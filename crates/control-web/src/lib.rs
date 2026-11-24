#![forbid(unsafe_code)]

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use cargo_dispatch_domain::PointId;
use cargo_dispatch_trip_files::TripFolder;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex, RwLock},
};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

const DRIVER_HTML: &str = r#"<!doctype html>
<html lang="uk">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Cargo Dispatch — рейс</title>
</head>
<body>
<main>
<h1>Cargo Dispatch — рейс</h1>
<p id="status">Завантаження…</p>
<ol id="points"></ol>
</main>
<script>
const token = decodeURIComponent(location.pathname.split("/").pop());
async function loadTrip() {
  const response = await fetch("/api/trip/" + encodeURIComponent(token), {cache: "no-store"});
  if (!response.ok) {
    document.getElementById("status").textContent = "Немає доступу до рейсу";
    return;
  }
  const trip = await response.json();
  document.getElementById("status").textContent =
    "Версія " + trip.trip_version + ", зміна " + trip.local_route_revision;
  const list = document.getElementById("points");
  list.replaceChildren();
  for (const point of trip.points) {
    const row = document.createElement("li");
    row.textContent = point.sequence + ". " + point.title +
      (point.completed ? " — виконано" : " — відкрито");
    if (!point.completed) {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = "Виконано";
      button.addEventListener("click", async () => {
        const result = await fetch(
          "/api/trip/" + encodeURIComponent(token) + "/complete/" + point.point_id,
          {method: "POST"}
        );
        if (result.ok) await loadTrip();
      });
      row.append(" ", button);
    }
    list.append(row);
  }
}
loadTrip();
setInterval(loadTrip, 2000);
</script>
</body>
</html>
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRegistrationError {
    InvalidToken,
    DuplicateToken,
    StateUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriverPointSnapshot {
    pub point_id: String,
    pub sequence: u32,
    pub title: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriverTripSnapshot {
    pub trip_id: String,
    pub trip_version: u64,
    pub local_route_revision: u64,
    pub points: Vec<DriverPointSnapshot>,
}

type SharedTrip = Arc<Mutex<TripFolder>>;

#[derive(Clone, Default)]
pub struct ControlBrowserState {
    sessions: Arc<RwLock<BTreeMap<String, SharedTrip>>>,
}

impl ControlBrowserState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_trip(
        &self,
        token: impl Into<String>,
        trip: TripFolder,
    ) -> Result<(), SessionRegistrationError> {
        let token = token.into();
        if !valid_token(&token) {
            return Err(SessionRegistrationError::InvalidToken);
        }
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| SessionRegistrationError::StateUnavailable)?;
        if sessions.contains_key(&token) {
            return Err(SessionRegistrationError::DuplicateToken);
        }
        sessions.insert(token, Arc::new(Mutex::new(trip)));
        Ok(())
    }

    fn trip(&self, token: &str) -> Result<Option<SharedTrip>, StatusCode> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(sessions.get(token).cloned())
    }
}

fn valid_token(token: &str) -> bool {
    (16..=128).contains(&token.len())
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn id_hex(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn parse_point_id(value: &str) -> Result<PointId, StatusCode> {
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    PointId::from_bytes(bytes).map_err(|_| StatusCode::BAD_REQUEST)
}

fn snapshot(trip: &TripFolder) -> DriverTripSnapshot {
    let state = trip.state();
    DriverTripSnapshot {
        trip_id: id_hex(state.trip_id.as_bytes()),
        trip_version: state.current_version.get(),
        local_route_revision: state.local_route_revision,
        points: state
            .route_points
            .iter()
            .map(|point| DriverPointSnapshot {
                point_id: id_hex(point.point_id.as_bytes()),
                sequence: point.sequence,
                title: point.title.clone(),
                completed: state.completed_point_ids.contains(&point.point_id),
            })
            .collect(),
    }
}

async fn driver_page(
    State(state): State<ControlBrowserState>,
    Path(token): Path<String>,
) -> Response {
    match state.trip(&token) {
        Ok(Some(_)) => Html(DRIVER_HTML).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => status.into_response(),
    }
}

async fn trip_snapshot(
    State(state): State<ControlBrowserState>,
    Path(token): Path<String>,
) -> Response {
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };
    let trip = match trip.lock() {
        Ok(trip) => trip,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    Json(snapshot(&trip)).into_response()
}

async fn complete_point(
    State(state): State<ControlBrowserState>,
    Path((token, point_id)): Path<(String, String)>,
) -> Response {
    let point_id = match parse_point_id(&point_id) {
        Ok(point_id) => point_id,
        Err(status) => return status.into_response(),
    };
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };
    let mut trip = match trip.lock() {
        Ok(trip) => trip,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if trip.complete(point_id).is_err() {
        return (StatusCode::CONFLICT, "point cannot be completed").into_response();
    }
    Json(snapshot(&trip)).into_response()
}

fn router(state: ControlBrowserState) -> Router {
    Router::new()
        .route("/driver/{token}", get(driver_page))
        .route("/api/trip/{token}", get(trip_snapshot))
        .route(
            "/api/trip/{token}/complete/{point_id}",
            post(complete_point),
        )
        .with_state(state)
}

pub struct LoopbackControlServer {
    address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<io::Result<()>>>,
}

impl LoopbackControlServer {
    pub async fn start(state: ControlBrowserState) -> io::Result<Self> {
        let listener =
            TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await?;
        let address = listener.local_addr()?;
        if address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
            return Err(io::Error::other("loopback server bound outside 127.0.0.1"));
        }

        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router(state))
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        Ok(Self {
            address,
            shutdown: Some(shutdown),
            task: Some(task),
        })
    }

    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    pub async fn shutdown(mut self) -> io::Result<()> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(task) = self.task.take() {
            task.await.map_err(io::Error::other)??;
        }
        Ok(())
    }
}

impl Drop for LoopbackControlServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_contracts::{TripPackageHeader, TripPointKind, TripRoutePoint, TripVersion};
    use cargo_dispatch_domain::{OperatorId, TripId, WorkingRouteId};
    use cargo_dispatch_planner::PlannerTripExportDraft;
    use cargo_dispatch_trip_files::{TripFolder, export_package};
    use std::{fs, path::Path};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn create_trip(
        root: &Path,
        name: &str,
        trip_byte: u8,
        first_point_byte: u8,
    ) -> io::Result<(TripId, Vec<TripRoutePoint>, TripFolder)> {
        let package = root.join(format!("{name}.zip"));
        let folder = root.join(name);
        let trip_id = TripId::from_bytes([trip_byte; 16]).expect("synthetic trip id");
        let route: Vec<_> = (0..3_u8)
            .map(|offset| TripRoutePoint {
                point_id: PointId::from_bytes([first_point_byte + offset; 16])
                    .expect("synthetic point id"),
                sequence: u32::from(offset) + 1,
                kind: TripPointKind::Load,
                title: format!("Synthetic point {}", offset + 1),
            })
            .collect();
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(
                trip_id,
                TripVersion::new(1).expect("version"),
                None,
            )
            .expect("header"),
            working_route_id: WorkingRouteId::from_bytes([trip_byte + 1; 16])
                .expect("route id"),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([trip_byte + 2; 16]).expect("operator"),
            ordered_point_ids: route.iter().map(|point| point.point_id).collect(),
        };
        export_package(&package, &draft, &route)?;
        let folder = TripFolder::import(&folder, &package)?;
        Ok((trip_id, route, folder))
    }

    async fn http_request(
        address: SocketAddr,
        method: &str,
        path: &str,
    ) -> io::Result<(u16, String)> {
        let mut stream = tokio::net::TcpStream::connect(address).await?;
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        let response = String::from_utf8(response)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let (headers, body) = response
            .split_once("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP response"))?;
        let status = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP status"))?;
        Ok((status, body.to_owned()))
    }

    #[tokio::test]
    async fn loopback_browser_clients_share_control_truth_and_keep_trips_isolated() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let folder_a_path = tmp.path().join("trip-a");
        let folder_b_path = tmp.path().join("trip-b");
        let (trip_a, route_a, folder_a) = create_trip(tmp.path(), "trip-a", 10, 20)?;
        let (trip_b, route_b, folder_b) = create_trip(tmp.path(), "trip-b", 30, 40)?;

        let state = ControlBrowserState::new();
        assert_eq!(
            state.register_trip("short", folder_a),
            Err(SessionRegistrationError::InvalidToken)
        );

        let folder_a = TripFolder::open(&folder_a_path, trip_a)?;
        state
            .register_trip("trip-a-token-0001", folder_a)
            .expect("register trip A");
        state
            .register_trip("trip-b-token-0002", folder_b)
            .expect("register trip B");

        let (_, _, duplicate_folder) = create_trip(tmp.path(), "duplicate", 60, 70)?;
        assert_eq!(
            state.register_trip("trip-a-token-0001", duplicate_folder),
            Err(SessionRegistrationError::DuplicateToken)
        );

        let server = LoopbackControlServer::start(state.clone()).await?;
        assert_eq!(server.address().ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));

        let (status, _) =
            http_request(server.address(), "GET", "/driver/wrong-token-0000").await?;
        assert_eq!(status, 404);

        let (status, page) =
            http_request(server.address(), "GET", "/driver/trip-a-token-0001").await?;
        assert_eq!(status, 200);
        assert!(page.contains("Cargo Dispatch"));

        let (status, body_before) =
            http_request(server.address(), "GET", "/api/trip/trip-a-token-0001").await?;
        assert_eq!(status, 200);
        let before: DriverTripSnapshot = serde_json::from_str(&body_before)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        assert!(before.points.iter().all(|point| !point.completed));

        let point_a = id_hex(route_a[1].point_id.as_bytes());
        let (status, _) = http_request(
            server.address(),
            "POST",
            &format!("/api/trip/trip-a-token-0001/complete/{point_a}"),
        )
        .await?;
        assert_eq!(status, 200);

        let (_, client_one_body) =
            http_request(server.address(), "GET", "/api/trip/trip-a-token-0001").await?;
        let (_, client_two_body) =
            http_request(server.address(), "GET", "/api/trip/trip-a-token-0001").await?;
        let client_one: DriverTripSnapshot = serde_json::from_str(&client_one_body)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let client_two: DriverTripSnapshot = serde_json::from_str(&client_two_body)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        assert_eq!(client_one, client_two);
        assert!(client_one.points[1].completed);

        let (_, trip_b_body) =
            http_request(server.address(), "GET", "/api/trip/trip-b-token-0002").await?;
        let trip_b_snapshot: DriverTripSnapshot = serde_json::from_str(&trip_b_body)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        assert!(trip_b_snapshot.points.iter().all(|point| !point.completed));

        let (status, _) = http_request(
            server.address(),
            "POST",
            &format!("/api/trip/trip-b-token-0002/complete/{point_a}"),
        )
        .await?;
        assert_eq!(status, 409);

        server.shutdown().await?;
        drop(state);

        let durable_a = TripFolder::open(&folder_a_path, trip_a)?;
        assert!(durable_a
            .state()
            .completed_point_ids
            .contains(&route_a[1].point_id));

        let durable_b = TripFolder::open(&folder_b_path, trip_b)?;
        assert!(durable_b.state().completed_point_ids.is_empty());
        assert_eq!(durable_b.state().route_points[0].point_id, route_b[0].point_id);

        fs::remove_file(tmp.path().join("trip-a.zip")).ok();
        Ok(())
    }
}
