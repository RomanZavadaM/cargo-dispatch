#![forbid(unsafe_code)]

use cargo_dispatch_control_ui_contracts::{
    ControlUiStatus, ListenerMode, ListenerStatus, RegisterTripInput, SessionStatus, StartLanInput,
};
use cargo_dispatch_control_web::{ControlAccessMode, ControlBrowserState, LoopbackControlServer};
use cargo_dispatch_domain::TripId;
use cargo_dispatch_trip_files::TripFolder;
use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Mutex,
};
use tauri::State;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Debug, Clone)]
struct RegisteredSession {
    trip_id: String,
    token: String,
}

struct DesktopControlService {
    browser: ControlBrowserState,
    server: AsyncMutex<Option<LoopbackControlServer>>,
    sessions: Mutex<BTreeMap<String, RegisteredSession>>,
}

impl Default for DesktopControlService {
    fn default() -> Self {
        Self {
            browser: ControlBrowserState::new(),
            server: AsyncMutex::new(None),
            sessions: Mutex::new(BTreeMap::new()),
        }
    }
}

fn parse_id_bytes(value: &str) -> Result<[u8; 16], String> {
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Trip ID має містити рівно 32 hex-символи".to_owned());
    }
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| "Некоректний Trip ID".to_owned())?;
    }
    Ok(bytes)
}

fn parse_trip_id(value: &str) -> Result<TripId, String> {
    TripId::from_bytes(parse_id_bytes(value)?)
        .map_err(|error| format!("Некоректний Trip ID: {error:?}"))
}

fn driver_url(address: SocketAddr, token: &str) -> String {
    format!("http://{address}/driver/{token}")
}

impl DesktopControlService {
    async fn register_trip(&self, input: RegisterTripInput) -> Result<ControlUiStatus, String> {
        let token = input.token.trim().to_owned();
        let trip_id_text = input.trip_id.trim().to_ascii_lowercase();
        let trip_id = parse_trip_id(&trip_id_text)?;
        let folder_path = PathBuf::from(input.folder_path.trim());
        if folder_path.as_os_str().is_empty() {
            return Err("Потрібно вказати папку рейсу".to_owned());
        }

        {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?;
            if sessions.contains_key(&token) {
                return Err("Цей token уже зареєстрований".to_owned());
            }
            if sessions
                .values()
                .any(|session| session.trip_id == trip_id_text)
            {
                return Err("Цей рейс уже зареєстрований у Control".to_owned());
            }
        }

        let folder = tokio::task::spawn_blocking(move || TripFolder::open(&folder_path, trip_id))
            .await
            .map_err(|error| format!("Не вдалося відкрити задачу читання рейсу: {error}"))?
            .map_err(|error| format!("Не вдалося відкрити папку рейсу: {error}"))?;

        self.browser
            .register_trip(token.clone(), folder)
            .map_err(|error| format!("Не вдалося створити browser session: {error:?}"))?;

        self.sessions
            .lock()
            .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?
            .insert(
                token.clone(),
                RegisteredSession {
                    trip_id: trip_id_text,
                    token,
                },
            );

        self.status().await
    }

    async fn start_loopback(&self) -> Result<ControlUiStatus, String> {
        let mut server = self.server.lock().await;
        if server.is_some() {
            return Err("Browser listener уже запущений".to_owned());
        }
        let started = LoopbackControlServer::start(self.browser.clone())
            .await
            .map_err(|error| format!("Не вдалося запустити loopback: {error}"))?;
        *server = Some(started);
        drop(server);
        self.status().await
    }

    async fn start_lan(&self, input: StartLanInput) -> Result<ControlUiStatus, String> {
        let ip: IpAddr = input
            .ip
            .trim()
            .parse()
            .map_err(|_| "Некоректна LAN-адреса".to_owned())?;
        let address = SocketAddr::new(ip, input.port);

        let mut server = self.server.lock().await;
        if server.is_some() {
            return Err("Browser listener уже запущений; спочатку вимкніть його".to_owned());
        }
        let started = LoopbackControlServer::start_lan(self.browser.clone(), address)
            .await
            .map_err(|error| format!("Не вдалося запустити LAN test mode: {error}"))?;
        *server = Some(started);
        drop(server);
        self.status().await
    }

    async fn stop(&self) -> Result<ControlUiStatus, String> {
        let current = self.server.lock().await.take();
        if let Some(server) = current {
            server
                .shutdown()
                .await
                .map_err(|error| format!("Не вдалося зупинити listener: {error}"))?;
        }
        self.status().await
    }

    async fn status(&self) -> Result<ControlUiStatus, String> {
        let listener = self.server.lock().await.as_ref().map(|server| {
            let info = server.info();
            ListenerStatus {
                mode: match info.mode {
                    ControlAccessMode::Loopback => ListenerMode::Loopback,
                    ControlAccessMode::Lan => ListenerMode::Lan,
                },
                address: info.address.to_string(),
            }
        });

        let address = self
            .server
            .lock()
            .await
            .as_ref()
            .map(LoopbackControlServer::address);
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?
            .values()
            .map(|session| SessionStatus {
                trip_id: session.trip_id.clone(),
                token: session.token.clone(),
                driver_url: address.map(|address| driver_url(address, &session.token)),
            })
            .collect();

        Ok(ControlUiStatus { listener, sessions })
    }
}

#[tauri::command]
async fn control_status(
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.status().await
}

#[tauri::command]
async fn register_trip(
    input: RegisterTripInput,
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.register_trip(input).await
}

#[tauri::command]
async fn start_loopback(
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.start_loopback().await
}

#[tauri::command]
async fn start_lan(
    input: StartLanInput,
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.start_lan(input).await
}

#[tauri::command]
async fn stop_browser_server(
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.stop().await
}

pub fn run() {
    tauri::Builder::default()
        .manage(DesktopControlService::default())
        .invoke_handler(tauri::generate_handler![
            control_status,
            register_trip,
            start_loopback,
            start_lan,
            stop_browser_server,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Cargo Dispatch Control");
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_contracts::{TripPackageHeader, TripPointKind, TripRoutePoint, TripVersion};
    use cargo_dispatch_domain::{OperatorId, PointId, WorkingRouteId};
    use cargo_dispatch_planner::PlannerTripExportDraft;
    use cargo_dispatch_trip_files::export_package;

    fn hex(bytes: [u8; 16]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn create_trip(root: &std::path::Path) -> std::io::Result<(TripId, PathBuf)> {
        let trip = TripId::from_bytes([21; 16]).expect("trip");
        let point = PointId::from_bytes([22; 16]).expect("point");
        let route = vec![TripRoutePoint {
            point_id: point,
            sequence: 1,
            kind: TripPointKind::Load,
            title: "Synthetic point".to_owned(),
            navigation: None,
        }];
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(trip, TripVersion::new(1).expect("version"), None)
                .expect("header"),
            working_route_id: WorkingRouteId::from_bytes([23; 16]).expect("route"),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([24; 16]).expect("operator"),
            ordered_point_ids: vec![point],
        };
        let package = root.join("desktop-trip.zip");
        let folder = root.join("desktop-trip");
        export_package(&package, &draft, &route)?;
        TripFolder::import(&folder, &package)?;
        Ok((trip, folder))
    }

    #[test]
    fn trip_id_parser_requires_exact_hex() {
        assert!(parse_trip_id("abc").is_err());
        assert!(parse_trip_id("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
        assert!(parse_trip_id("15151515151515151515151515151515").is_ok());
    }

    #[tokio::test]
    async fn desktop_service_keeps_trip_registered_when_listener_stops() -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let (trip, folder) = create_trip(tmp.path())?;
        let service = DesktopControlService::default();
        let token = "desktop-trip-token-0001";

        let registered = service
            .register_trip(RegisterTripInput {
                folder_path: folder.to_string_lossy().into_owned(),
                trip_id: hex(trip.as_bytes()),
                token: token.to_owned(),
            })
            .await
            .expect("trip registers");
        assert_eq!(registered.sessions.len(), 1);
        assert!(registered.listener.is_none());

        let running = service.start_loopback().await.expect("loopback starts");
        let listener = running.listener.expect("listener status");
        assert_eq!(listener.mode, ListenerMode::Loopback);
        assert!(listener.address.starts_with("127.0.0.1:"));
        assert!(
            running.sessions[0]
                .driver_url
                .as_deref()
                .is_some_and(|url| url.contains(token))
        );

        assert!(service.start_loopback().await.is_err());
        let stopped = service.stop().await.expect("listener stops");
        assert!(stopped.listener.is_none());
        assert_eq!(stopped.sessions.len(), 1);
        assert!(stopped.sessions[0].driver_url.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn desktop_service_does_not_bypass_lan_validation() {
        let service = DesktopControlService::default();
        assert!(
            service
                .start_lan(StartLanInput {
                    ip: "0.0.0.0".to_owned(),
                    port: 8080,
                })
                .await
                .is_err()
        );
        assert!(
            service
                .start_lan(StartLanInput {
                    ip: "127.0.0.1".to_owned(),
                    port: 8080,
                })
                .await
                .is_err()
        );
        assert!(
            service
                .start_lan(StartLanInput {
                    ip: "8.8.8.8".to_owned(),
                    port: 8080,
                })
                .await
                .is_err()
        );
    }
}
