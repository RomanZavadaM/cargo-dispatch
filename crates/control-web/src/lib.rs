#![forbid(unsafe_code)]

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Path, Request, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use cargo_dispatch_domain::{DocumentId, PointId};
use cargo_dispatch_trip_files::{
    DocumentDirection, DocumentStoreDraft, DriverLocationSample, StoredDocument, TripFolder,
};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex, RwLock},
};
use tempfile::NamedTempFile;
use tokio::{
    fs::File as TokioFile,
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};

const MAX_UPLOAD_BYTES: u64 = 512 * 1024 * 1024;
const STREAM_CHUNK_BYTES: usize = 64 * 1024;

const DRIVER_HTML: &str = r#"<!doctype html>
<html lang="uk">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Cargo Dispatch — рейс</title>
<style>
body{font-family:system-ui,sans-serif;margin:0;background:#f6f7f8;color:#202124}
main{max-width:760px;margin:auto;padding:16px}
section{background:white;border:1px solid #ddd;border-radius:12px;padding:14px;margin:12px 0}
button,input{font:inherit}.row{display:flex;gap:8px;align-items:center;flex-wrap:wrap}
.point{padding:8px 0;border-bottom:1px solid #eee}.point:last-child{border-bottom:0}
small{color:#666}.error{color:#a40000}.ok{color:#176b2c}
</style>
</head>
<body>
<main>
<h1>Cargo Dispatch — рейс</h1>
<p id="status">Завантаження…</p>

<section>
<h2>Маршрут</h2>
<div id="points"></div>
</section>

<section>
<h2>Документи / фото / відео</h2>
<div id="documents"></div>
<label>Прив'язати до точки:
<select id="filePoint"><option value="">Рейс у цілому</option></select>
</label>
<div class="row">
<input id="file" type="file" accept="image/*,video/*,application/pdf,*/*">
<input id="camera" type="file" accept="image/*,video/*" capture="environment">
<button id="sendFile" type="button">Передати в Control</button>
</div>
<p id="fileStatus"></p>
</section>

<section>
<h2>Геолокація</h2>
<p id="location">Ще не передавалась</p>
<div class="row">
<button id="startLocation" type="button">Почати передачу геолокації</button>
<button id="stopLocation" type="button" disabled>Зупинити</button>
</div>
<p id="locationStatus"></p>
</section>
</main>
<script>
const token = decodeURIComponent(location.pathname.split("/").pop());
let currentTrip = null;
let watchId = null;
let lastLocationSentAt = 0;

function endpoint(path) {
  return "/api/trip/" + encodeURIComponent(token) + path;
}

function newDocumentId() {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  if (bytes.every(value => value === 0)) bytes[0] = 1;
  return Array.from(bytes, value => value.toString(16).padStart(2, "0")).join("");
}

async function api(path, options) {
  const response = await fetch(endpoint(path), Object.assign({cache: "no-store"}, options || {}));
  if (!response.ok) throw new Error("HTTP " + response.status);
  return response;
}

async function loadTrip() {
  try {
    const response = await api("");
    currentTrip = await response.json();
    document.getElementById("status").textContent =
      "Версія " + currentTrip.trip_version + ", зміна " + currentTrip.local_route_revision;
    renderRoute();
    renderDocuments();
    renderLocation();
  } catch (error) {
    document.getElementById("status").textContent = "Немає доступу до рейсу";
    document.getElementById("status").className = "error";
  }
}

function renderRoute() {
  const host = document.getElementById("points");
  host.replaceChildren();
  const pointSelect = document.getElementById("filePoint");
  const selectedPoint = pointSelect.value;
  pointSelect.replaceChildren();
  const tripOption = document.createElement("option");
  tripOption.value = "";
  tripOption.textContent = "Рейс у цілому";
  pointSelect.append(tripOption);
  for (const point of currentTrip.points) {
    const option = document.createElement("option");
    option.value = point.point_id;
    option.textContent = point.sequence + ". " + point.title;
    pointSelect.append(option);
  }
  if (Array.from(pointSelect.options).some(option => option.value === selectedPoint)) {
    pointSelect.value = selectedPoint;
  }

  const openPoints = currentTrip.points.filter(point => !point.completed);
  for (const point of currentTrip.points) {
    const row = document.createElement("div");
    row.className = "point";
    const text = document.createElement("span");
    text.textContent = point.sequence + ". " + point.title +
      (point.completed ? " — виконано" : " — відкрито");
    row.append(text);
    if (point.navigation) {
      const nav = document.createElement("div");
      const address = point.navigation.address ? point.navigation.address + " — " : "";
      nav.textContent = address +
        point.navigation.latitude.toFixed(6) + ", " +
        point.navigation.longitude.toFixed(6) + " ";
      const link = document.createElement("a");
      link.href = "https://www.google.com/maps/dir/?api=1&destination=" +
        encodeURIComponent(point.navigation.latitude + "," + point.navigation.longitude);
      link.target = "_blank";
      link.rel = "noopener";
      link.textContent = "Відкрити навігацію";
      nav.append(link);
      row.append(nav);
    }

    if (!point.completed) {
      const done = document.createElement("button");
      done.type = "button";
      done.textContent = "Виконано";
      done.addEventListener("click", async () => {
        try {
          await api("/complete/" + point.point_id, {method: "POST"});
          await loadTrip();
        } catch (error) {
          alert("Control не прийняв зміну: " + error.message);
        }
      });
      row.append(" ", done);

      const openIndex = openPoints.findIndex(item => item.point_id === point.point_id);
      for (const move of [-1, 1]) {
        const button = document.createElement("button");
        button.type = "button";
        button.textContent = move < 0 ? "↑" : "↓";
        button.disabled =
          (move < 0 && openIndex === 0) ||
          (move > 0 && openIndex === openPoints.length - 1);
        button.addEventListener("click", async () => {
          const ids = openPoints.map(item => item.point_id);
          const target = openIndex + move;
          [ids[openIndex], ids[target]] = [ids[target], ids[openIndex]];
          try {
            await api("/route/order", {
              method: "POST",
              headers: {"Content-Type": "application/json"},
              body: JSON.stringify({
                based_on_revision: currentTrip.local_route_revision,
                open_point_ids: ids
              })
            });
            await loadTrip();
          } catch (error) {
            alert("Маршрут уже змінився або Control відхилив порядок: " + error.message);
            await loadTrip();
          }
        });
        row.append(" ", button);
      }
    }
    host.append(row);
  }
}

function renderDocuments() {
  const host = document.getElementById("documents");
  host.replaceChildren();
  if (!currentTrip.documents.length) {
    host.textContent = "Документів ще немає";
    return;
  }
  const list = document.createElement("ul");
  for (const item of currentTrip.documents) {
    const row = document.createElement("li");
    const link = document.createElement("a");
    link.href = endpoint("/documents/" + item.document_id);
    link.textContent = item.original_name;
    const direction = item.direction === "from_operator" ? "Control → водій" : "водій → Control";
    row.append(link, " — " + direction + " — " + item.size + " байт");
    list.append(row);
  }
  host.append(list);
}

function renderLocation() {
  const host = document.getElementById("location");
  if (!currentTrip.latest_location) {
    host.textContent = "Ще не передавалась";
    return;
  }
  const item = currentTrip.latest_location;
  const accuracy = item.accuracy_m == null ? "" : ", ±" + item.accuracy_m.toFixed(1) + " м";
  host.textContent =
    item.latitude.toFixed(6) + ", " + item.longitude.toFixed(6) + accuracy +
    " — " + new Date(item.observed_at_unix_ms).toLocaleString();
}

document.getElementById("sendFile").addEventListener("click", async () => {
  const input = document.getElementById("file");
  const camera = document.getElementById("camera");
  const status = document.getElementById("fileStatus");
  const file =
    input.files && input.files.length ? input.files[0] :
    camera.files && camera.files.length ? camera.files[0] : null;
  if (!file) {
    status.textContent = "Спочатку виберіть файл або фото/відео";
    status.className = "error";
    return;
  }
  const documentId = newDocumentId();
  status.textContent = "Передача…";
  status.className = "";
  try {
    const headers = {
      "Content-Type": file.type || "application/octet-stream",
      "X-File-Name": encodeURIComponent(file.name),
      "X-Created-At-Unix-Ms": String(Date.now())
    };
    const pointId = document.getElementById("filePoint").value;
    if (pointId) headers["X-Point-Id"] = pointId;
    await api("/documents/" + documentId, {
      method: "POST",
      headers,
      body: file
    });
    input.value = "";
    camera.value = "";
    status.textContent = "Файл прийнятий Control";
    status.className = "ok";
    await loadTrip();
  } catch (error) {
    status.textContent = "Передача не вдалася: " + error.message;
    status.className = "error";
  }
});

document.getElementById("startLocation").addEventListener("click", () => {
  const status = document.getElementById("locationStatus");
  if (!navigator.geolocation) {
    status.textContent = "Браузер не підтримує геолокацію";
    status.className = "error";
    return;
  }
  if (!window.isSecureContext &&
      location.hostname !== "127.0.0.1" &&
      location.hostname !== "localhost") {
    status.textContent =
      "Для геолокації телефонний браузер зазвичай вимагає HTTPS. LAN HTTP можна використовувати для маршруту й файлів.";
    status.className = "error";
  }
  watchId = navigator.geolocation.watchPosition(async position => {
    const now = Date.now();
    if (now - lastLocationSentAt < 10000) return;
    lastLocationSentAt = now;
    try {
      await api("/location", {
        method: "POST",
        headers: {"Content-Type": "application/json"},
        body: JSON.stringify({
          latitude: position.coords.latitude,
          longitude: position.coords.longitude,
          accuracy_m: position.coords.accuracy,
          observed_at_unix_ms: Math.round(position.timestamp || now)
        })
      });
      status.textContent = "Позиція прийнята Control";
      status.className = "ok";
      await loadTrip();
    } catch (error) {
      status.textContent = "Control не прийняв позицію: " + error.message;
      status.className = "error";
    }
  }, error => {
    status.textContent = "Геолокація недоступна: " + error.message;
    status.className = "error";
  }, {enableHighAccuracy: true, maximumAge: 5000, timeout: 15000});
  document.getElementById("startLocation").disabled = true;
  document.getElementById("stopLocation").disabled = false;
});

document.getElementById("stopLocation").addEventListener("click", () => {
  if (watchId != null) navigator.geolocation.clearWatch(watchId);
  watchId = null;
  document.getElementById("startLocation").disabled = false;
  document.getElementById("stopLocation").disabled = true;
});

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverNavigationSnapshot {
    pub latitude: f64,
    pub longitude: f64,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverPointSnapshot {
    pub point_id: String,
    pub sequence: u32,
    pub title: String,
    pub completed: bool,
    pub navigation: Option<DriverNavigationSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverLocationSnapshot {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: Option<f64>,
    pub observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriverDocumentSnapshot {
    pub document_id: String,
    pub direction: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: u64,
    pub point_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverTripSnapshot {
    pub trip_id: String,
    pub trip_version: u64,
    pub local_route_revision: u64,
    pub points: Vec<DriverPointSnapshot>,
    pub documents: Vec<DriverDocumentSnapshot>,
    pub latest_location: Option<DriverLocationSnapshot>,
}

#[derive(Debug, Clone, Deserialize)]
struct RouteOrderRequest {
    based_on_revision: u64,
    open_point_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LocationUpdate {
    latitude: f64,
    longitude: f64,
    accuracy_m: Option<f64>,
    observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAccessMode {
    Loopback,
    Lan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlServerInfo {
    pub mode: ControlAccessMode,
    pub address: SocketAddr,
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

fn parse_uuid_bytes(value: &str) -> Result<[u8; 16], StatusCode> {
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    Ok(bytes)
}

fn parse_point_id(value: &str) -> Result<PointId, StatusCode> {
    PointId::from_bytes(parse_uuid_bytes(value)?).map_err(|_| StatusCode::BAD_REQUEST)
}

fn parse_document_id(value: &str) -> Result<DocumentId, StatusCode> {
    DocumentId::from_bytes(parse_uuid_bytes(value)?).map_err(|_| StatusCode::BAD_REQUEST)
}

fn location_sample(update: LocationUpdate) -> Result<DriverLocationSample, StatusCode> {
    if !update.latitude.is_finite()
        || !(-90.0..=90.0).contains(&update.latitude)
        || !update.longitude.is_finite()
        || !(-180.0..=180.0).contains(&update.longitude)
        || update.observed_at_unix_ms == 0
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let latitude_e7 = (update.latitude * 10_000_000.0).round();
    let longitude_e7 = (update.longitude * 10_000_000.0).round();
    if latitude_e7 < f64::from(i32::MIN)
        || latitude_e7 > f64::from(i32::MAX)
        || longitude_e7 < f64::from(i32::MIN)
        || longitude_e7 > f64::from(i32::MAX)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let accuracy_cm = match update.accuracy_m {
        Some(value)
            if value.is_finite() && value >= 0.0 && value <= f64::from(u32::MAX) / 100.0 =>
        {
            Some((value * 100.0).round() as u32)
        }
        Some(_) => return Err(StatusCode::BAD_REQUEST),
        None => None,
    };

    DriverLocationSample {
        latitude_e7: latitude_e7 as i32,
        longitude_e7: longitude_e7 as i32,
        accuracy_cm,
        observed_at_unix_ms: update.observed_at_unix_ms,
    }
    .validate()
    .map_err(|_| StatusCode::BAD_REQUEST)
}

fn document_snapshot(document: &StoredDocument) -> DriverDocumentSnapshot {
    DriverDocumentSnapshot {
        document_id: id_hex(document.document_id.as_bytes()),
        direction: match document.direction {
            DocumentDirection::FromOperator => "from_operator",
            DocumentDirection::FromDriver => "from_driver",
        }
        .to_owned(),
        original_name: document.original_name.clone(),
        mime_type: document.mime_type.clone(),
        size: document.size,
        point_id: document.point_id.map(|point| id_hex(point.as_bytes())),
    }
}

fn location_snapshot(sample: DriverLocationSample) -> DriverLocationSnapshot {
    DriverLocationSnapshot {
        latitude: sample.latitude(),
        longitude: sample.longitude(),
        accuracy_m: sample.accuracy_m(),
        observed_at_unix_ms: sample.observed_at_unix_ms,
    }
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
                navigation: point
                    .navigation
                    .as_ref()
                    .map(|navigation| DriverNavigationSnapshot {
                        latitude: navigation.latitude(),
                        longitude: navigation.longitude(),
                        address: navigation.address.clone(),
                    }),
            })
            .collect(),
        documents: trip.documents().iter().map(document_snapshot).collect(),
        latest_location: trip.latest_location().map(location_snapshot),
    }
}

fn decode_uri_component(value: &str) -> Result<String, StatusCode> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(StatusCode::BAD_REQUEST);
            }
            let high = (bytes[index + 1] as char)
                .to_digit(16)
                .ok_or(StatusCode::BAD_REQUEST)?;
            let low = (bytes[index + 2] as char)
                .to_digit(16)
                .ok_or(StatusCode::BAD_REQUEST)?;
            decoded.push(((high << 4) | low) as u8);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| StatusCode::BAD_REQUEST)
}

fn validate_original_name(value: &str) -> Result<(), StatusCode> {
    if value.trim().is_empty()
        || value.len() > 512
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

fn encode_header_filename(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(*byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn header_text(headers: &HeaderMap, name: &'static str) -> Result<String, StatusCode> {
    headers
        .get(name)
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map(str::to_owned)
        .map_err(|_| StatusCode::BAD_REQUEST)
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

async fn reorder_route(
    State(state): State<ControlBrowserState>,
    Path(token): Path<String>,
    Json(request): Json<RouteOrderRequest>,
) -> Response {
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };
    let mut trip = match trip.lock() {
        Ok(trip) => trip,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let shown = trip.state();
    if shown.local_route_revision != request.based_on_revision {
        return (StatusCode::CONFLICT, "stale route revision").into_response();
    }
    let order = match request
        .open_point_ids
        .iter()
        .map(|value| parse_point_id(value))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(order) => order,
        Err(status) => return status.into_response(),
    };
    if trip.driver_order(&shown, &order).is_err() {
        return (StatusCode::CONFLICT, "route order rejected").into_response();
    }
    Json(snapshot(&trip)).into_response()
}

async fn update_location(
    State(state): State<ControlBrowserState>,
    Path(token): Path<String>,
    Json(update): Json<LocationUpdate>,
) -> Response {
    let sample = match location_sample(update) {
        Ok(sample) => sample,
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
    if trip.record_location(sample).is_err() {
        return (StatusCode::CONFLICT, "location not stored").into_response();
    }
    Json(location_snapshot(sample)).into_response()
}

async fn upload_document(
    State(state): State<ControlBrowserState>,
    Path((token, document_id)): Path<(String, String)>,
    request: Request,
) -> Response {
    let document_id = match parse_document_id(&document_id) {
        Ok(document_id) => document_id,
        Err(status) => return status.into_response(),
    };
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };

    let headers = request.headers();
    if let Some(length) = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        && length > MAX_UPLOAD_BYTES
    {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }

    let original_name = match header_text(headers, "x-file-name")
        .and_then(|value| decode_uri_component(&value))
        .and_then(|value| {
            validate_original_name(&value)?;
            Ok(value)
        }) {
        Ok(value) => value,
        Err(status) => return status.into_response(),
    };
    let mime_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty() && value.len() <= 256)
        .unwrap_or("application/octet-stream")
        .to_owned();
    let created_at_unix_ms = match header_text(headers, "x-created-at-unix-ms")
        .and_then(|value| value.parse::<u64>().map_err(|_| StatusCode::BAD_REQUEST))
    {
        Ok(value) if value > 0 => value,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let point_id = match headers.get("x-point-id") {
        Some(value) => match value
            .to_str()
            .map_err(|_| StatusCode::BAD_REQUEST)
            .and_then(parse_point_id)
        {
            Ok(value) => Some(value),
            Err(status) => return status.into_response(),
        },
        None => None,
    };

    let temporary = match NamedTempFile::new() {
        Ok(file) => file,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let std_file = match temporary.reopen() {
        Ok(file) => file,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let mut file = TokioFile::from_std(std_file);
    let mut stream = request.into_body().into_data_stream();
    let mut total = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
        total = match total.checked_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX)) {
            Some(total) if total <= MAX_UPLOAD_BYTES => total,
            _ => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        };
        if file.write_all(&chunk).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    if file.sync_all().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    drop(file);

    let draft = DocumentStoreDraft {
        document_id,
        direction: DocumentDirection::FromDriver,
        original_name,
        mime_type,
        created_at_unix_ms,
        point_id,
    };
    let stored = match tokio::task::spawn_blocking(move || {
        let _keep_temporary_alive = &temporary;
        let mut trip = trip
            .lock()
            .map_err(|_| io::Error::other("trip state unavailable"))?;
        trip.store_document(draft, temporary.path())
    })
    .await
    {
        Ok(Ok(stored)) => stored,
        Ok(Err(error)) if error.kind() == io::ErrorKind::InvalidData => {
            return StatusCode::CONFLICT.into_response();
        }
        _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    Json(document_snapshot(&stored)).into_response()
}

fn file_body(file: TokioFile) -> Body {
    let stream = stream::unfold(Some(file), |state| async move {
        let mut file = state?;
        let mut buffer = vec![0_u8; STREAM_CHUNK_BYTES];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(read) => {
                buffer.truncate(read);
                Some((Ok::<Bytes, io::Error>(Bytes::from(buffer)), Some(file)))
            }
            Err(error) => Some((Err(error), None)),
        }
    });
    Body::from_stream(stream)
}

async fn download_document(
    State(state): State<ControlBrowserState>,
    Path((token, document_id)): Path<(String, String)>,
) -> Response {
    let document_id = match parse_document_id(&document_id) {
        Ok(document_id) => document_id,
        Err(status) => return status.into_response(),
    };
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };

    let verified = match tokio::task::spawn_blocking(move || {
        let trip = trip
            .lock()
            .map_err(|_| io::Error::other("trip state unavailable"))?;
        trip.verified_document_path(document_id)
    })
    .await
    {
        Ok(Ok(value)) => value,
        Ok(Err(error)) if error.kind() == io::ErrorKind::NotFound => {
            return StatusCode::NOT_FOUND.into_response();
        }
        Ok(Err(_)) => return StatusCode::CONFLICT.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let (document, path) = verified;
    let file = match TokioFile::open(path).await {
        Ok(file) => file,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let mut response = Response::new(file_body(file));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&document.mime_type) {
        headers.insert(CONTENT_TYPE, value);
    } else {
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
    }
    if let Ok(value) = HeaderValue::from_str(&document.size.to_string()) {
        headers.insert(CONTENT_LENGTH, value);
    }
    let disposition = format!(
        "attachment; filename*=UTF-8''{}",
        encode_header_filename(&document.original_name)
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(CONTENT_DISPOSITION, value);
    }
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn router(state: ControlBrowserState) -> Router {
    Router::new()
        .route("/driver/{token}", get(driver_page))
        .route("/api/trip/{token}", get(trip_snapshot))
        .route(
            "/api/trip/{token}/complete/{point_id}",
            post(complete_point),
        )
        .route("/api/trip/{token}/route/order", post(reorder_route))
        .route("/api/trip/{token}/location", post(update_location))
        .route(
            "/api/trip/{token}/documents/{document_id}",
            get(download_document).post(upload_document),
        )
        .with_state(state)
}

fn is_private_lan_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() && !ip.is_loopback() && !ip.is_unspecified(),
        IpAddr::V6(ip) => {
            let first = ip.segments()[0];
            let unique_local = first & 0xfe00 == 0xfc00;
            let link_local = first & 0xffc0 == 0xfe80;
            (unique_local || link_local) && !ip.is_loopback() && !ip.is_unspecified()
        }
    }
}

fn validate_lan_address(address: SocketAddr) -> io::Result<()> {
    if address.port() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "LAN test port must be explicit",
        ));
    }
    if !is_private_lan_ip(address.ip()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "LAN test address must be an explicit private/local interface",
        ));
    }
    Ok(())
}

pub struct LoopbackControlServer {
    info: ControlServerInfo,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<io::Result<()>>>,
}

impl LoopbackControlServer {
    async fn start_bound(
        state: ControlBrowserState,
        requested: SocketAddr,
        mode: ControlAccessMode,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(requested).await?;
        let address = listener.local_addr()?;
        match mode {
            ControlAccessMode::Loopback if !address.ip().is_loopback() => {
                return Err(io::Error::other(
                    "loopback server bound outside loopback interface",
                ));
            }
            ControlAccessMode::Lan => validate_lan_address(address)?,
            _ => {}
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
            info: ControlServerInfo { mode, address },
            shutdown: Some(shutdown),
            task: Some(task),
        })
    }

    pub async fn start(state: ControlBrowserState) -> io::Result<Self> {
        Self::start_bound(
            state,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            ControlAccessMode::Loopback,
        )
        .await
    }

    pub async fn start_lan(state: ControlBrowserState, address: SocketAddr) -> io::Result<Self> {
        validate_lan_address(address)?;
        Self::start_bound(state, address, ControlAccessMode::Lan).await
    }

    #[must_use]
    pub const fn info(&self) -> ControlServerInfo {
        self.info
    }

    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.info.address
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
    use cargo_dispatch_contracts::{
        TripNavigationTarget, TripPackageHeader, TripPointKind, TripRoutePoint, TripVersion,
    };
    use cargo_dispatch_domain::{OperatorId, TripId, WorkingRouteId};
    use cargo_dispatch_planner::PlannerTripExportDraft;
    use cargo_dispatch_trip_files::{DocumentStoreDraft, TripFolder, export_package};
    use std::{fs, net::Ipv6Addr, path::Path};
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
                navigation: Some(TripNavigationTarget {
                    latitude_e7: 498_393_000 + i32::from(offset) * 1_000,
                    longitude_e7: 240_296_000 + i32::from(offset) * 1_000,
                    address: Some(format!("Synthetic address {}", offset + 1)),
                }),
            })
            .collect();
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(trip_id, TripVersion::new(1).expect("version"), None)
                .expect("header"),
            working_route_id: WorkingRouteId::from_bytes([trip_byte + 1; 16]).expect("route id"),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([trip_byte + 2; 16]).expect("operator"),
            ordered_point_ids: route.iter().map(|point| point.point_id).collect(),
        };
        export_package(&package, &draft, &route)?;
        let folder = TripFolder::import(&folder, &package)?;
        Ok((trip_id, route, folder))
    }

    fn response_parts(response: Vec<u8>) -> io::Result<(u16, String, Vec<u8>)> {
        let split = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP response"))?;
        let headers = String::from_utf8(response[..split].to_vec())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let status = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP status"))?;
        Ok((status, headers, response[split + 4..].to_vec()))
    }

    async fn http_request(
        address: SocketAddr,
        method: &str,
        path: &str,
        headers: &[(&str, String)],
        body: &[u8],
    ) -> io::Result<(u16, String, Vec<u8>)> {
        let mut stream = tokio::net::TcpStream::connect(address).await?;
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (name, value) in headers {
            request.push_str(name);
            request.push_str(": ");
            request.push_str(value);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).await?;
        stream.write_all(body).await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        response_parts(response)
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        address: SocketAddr,
        path: &str,
    ) -> io::Result<T> {
        let (status, _, body) = http_request(address, "GET", path, &[], &[]).await?;
        if status != 200 {
            return Err(io::Error::other(format!("unexpected HTTP {status}")));
        }
        serde_json::from_slice(&body)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    #[test]
    fn lan_mode_requires_explicit_private_address_and_port() {
        assert!(validate_lan_address("192.168.1.25:8080".parse().unwrap()).is_ok());
        assert!(validate_lan_address("10.10.1.5:9090".parse().unwrap()).is_ok());
        assert!(validate_lan_address("[fd00::10]:8080".parse().unwrap()).is_ok());
        assert!(validate_lan_address("0.0.0.0:8080".parse().unwrap()).is_err());
        assert!(validate_lan_address("127.0.0.1:8080".parse().unwrap()).is_err());
        assert!(validate_lan_address("8.8.8.8:8080".parse().unwrap()).is_err());
        assert!(validate_lan_address("192.168.1.25:0".parse().unwrap()).is_err());
        assert!(!is_private_lan_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[tokio::test]
    async fn browser_exchanges_route_location_and_binary_files_with_control() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let folder_a_path = tmp.path().join("trip-a");
        let folder_b_path = tmp.path().join("trip-b");
        let (trip_a, route_a, mut folder_a) = create_trip(tmp.path(), "trip-a", 10, 20)?;
        let (trip_b, route_b, folder_b) = create_trip(tmp.path(), "trip-b", 30, 40)?;

        let operator_file = tmp.path().join("operator-ttn.pdf");
        fs::write(&operator_file, b"synthetic operator pdf")?;
        let operator_document = DocumentId::from_bytes([80; 16]).unwrap();
        folder_a.store_document(
            DocumentStoreDraft {
                document_id: operator_document,
                direction: DocumentDirection::FromOperator,
                original_name: "TTN-test.pdf".to_owned(),
                mime_type: "application/pdf".to_owned(),
                created_at_unix_ms: 100,
                point_id: Some(route_a[0].point_id),
            },
            &operator_file,
        )?;

        let state = ControlBrowserState::new();
        state
            .register_trip("trip-a-token-0001", folder_a)
            .expect("register trip A");
        state
            .register_trip("trip-b-token-0002", folder_b)
            .expect("register trip B");

        let server = LoopbackControlServer::start(state.clone()).await?;
        assert_eq!(server.info().mode, ControlAccessMode::Loopback);
        assert_eq!(server.address().ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));

        let (status, _, page) = http_request(
            server.address(),
            "GET",
            "/driver/trip-a-token-0001",
            &[],
            &[],
        )
        .await?;
        assert_eq!(status, 200);
        assert!(String::from_utf8_lossy(&page).contains("Передати в Control"));

        let mut trip: DriverTripSnapshot =
            get_json(server.address(), "/api/trip/trip-a-token-0001").await?;
        assert_eq!(trip.documents.len(), 1);
        assert!(trip.latest_location.is_none());

        let order_body = serde_json::to_vec(&serde_json::json!({
            "based_on_revision": trip.local_route_revision,
            "open_point_ids": [
                id_hex(route_a[2].point_id.as_bytes()),
                id_hex(route_a[0].point_id.as_bytes()),
                id_hex(route_a[1].point_id.as_bytes())
            ]
        }))?;
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            "/api/trip/trip-a-token-0001/route/order",
            &[("Content-Type", "application/json".to_owned())],
            &order_body,
        )
        .await?;
        assert_eq!(status, 200);

        trip = get_json(server.address(), "/api/trip/trip-a-token-0001").await?;
        assert_eq!(
            trip.points[0].point_id,
            id_hex(route_a[2].point_id.as_bytes())
        );
        let navigation = trip.points[0]
            .navigation
            .as_ref()
            .expect("navigation target delivered");
        assert_eq!(navigation.address.as_deref(), Some("Synthetic address 3"));
        assert!((navigation.latitude - 49.8395).abs() < 0.000001);
        assert!((navigation.longitude - 24.0298).abs() < 0.000001);

        let location_body = serde_json::to_vec(&serde_json::json!({
            "latitude": 49.8393,
            "longitude": 24.0296,
            "accuracy_m": 8.5,
            "observed_at_unix_ms": 1_795_550_000_000_u64
        }))?;
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            "/api/trip/trip-a-token-0001/location",
            &[("Content-Type", "application/json".to_owned())],
            &location_body,
        )
        .await?;
        assert_eq!(status, 200);

        let driver_document = DocumentId::from_bytes([81; 16]).unwrap();
        let driver_document_hex = id_hex(driver_document.as_bytes());
        let video_bytes = b"synthetic-driver-video-bytes";
        let upload_path = format!("/api/trip/trip-a-token-0001/documents/{driver_document_hex}");
        let upload_headers = [
            ("Content-Type", "video/mp4".to_owned()),
            ("X-File-Name", "driver-video.mp4".to_owned()),
            ("X-Created-At-Unix-Ms", "1795550000100".to_owned()),
            ("X-Point-Id", id_hex(route_a[2].point_id.as_bytes())),
        ];
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            &upload_path,
            &upload_headers,
            video_bytes,
        )
        .await?;
        assert_eq!(status, 200);

        let (status, _, _) = http_request(
            server.address(),
            "POST",
            &upload_path,
            &upload_headers,
            video_bytes,
        )
        .await?;
        assert_eq!(status, 200);

        let different = b"different bytes for same document id";
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            &upload_path,
            &upload_headers,
            different,
        )
        .await?;
        assert_eq!(status, 409);

        let photo_document = DocumentId::from_bytes([82; 16]).unwrap();
        let photo_document_hex = id_hex(photo_document.as_bytes());
        let photo_path = format!("/api/trip/trip-a-token-0001/documents/{photo_document_hex}");
        let photo_bytes = b"synthetic-driver-photo-bytes";
        let photo_headers = [
            ("Content-Type", "image/jpeg".to_owned()),
            ("X-File-Name", "driver-photo.jpg".to_owned()),
            ("X-Created-At-Unix-Ms", "1795550000200".to_owned()),
            ("X-Point-Id", id_hex(route_a[0].point_id.as_bytes())),
        ];
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            &photo_path,
            &photo_headers,
            photo_bytes,
        )
        .await?;
        assert_eq!(status, 200);

        trip = get_json(server.address(), "/api/trip/trip-a-token-0001").await?;
        assert_eq!(trip.documents.len(), 3);
        let video = trip
            .documents
            .iter()
            .find(|item| item.document_id == driver_document_hex)
            .expect("driver video listed");
        assert_eq!(video.direction, "from_driver");
        assert_eq!(video.mime_type, "video/mp4");
        assert_eq!(
            video.point_id.as_deref(),
            Some(id_hex(route_a[2].point_id.as_bytes()).as_str())
        );
        let photo = trip
            .documents
            .iter()
            .find(|item| item.document_id == photo_document_hex)
            .expect("driver photo listed");
        assert_eq!(photo.mime_type, "image/jpeg");
        assert_eq!(
            photo.point_id.as_deref(),
            Some(id_hex(route_a[0].point_id.as_bytes()).as_str())
        );
        let location = trip.latest_location.expect("location stored");
        assert!((location.latitude - 49.8393).abs() < 0.000001);
        assert!((location.longitude - 24.0296).abs() < 0.000001);

        let operator_download = format!(
            "/api/trip/trip-a-token-0001/documents/{}",
            id_hex(operator_document.as_bytes())
        );
        let (status, headers, body) =
            http_request(server.address(), "GET", &operator_download, &[], &[]).await?;
        assert_eq!(status, 200);
        assert!(headers.contains("application/pdf"));
        assert_eq!(body, b"synthetic operator pdf");

        let (status, headers, body) =
            http_request(server.address(), "GET", &upload_path, &[], &[]).await?;
        assert_eq!(status, 200);
        assert!(headers.contains("video/mp4"));
        assert_eq!(body, video_bytes);

        let (status, headers, body) =
            http_request(server.address(), "GET", &photo_path, &[], &[]).await?;
        assert_eq!(status, 200);
        assert!(headers.contains("image/jpeg"));
        assert_eq!(body, photo_bytes);

        let wrong_trip_download =
            format!("/api/trip/trip-b-token-0002/documents/{driver_document_hex}");
        let (status, _, _) =
            http_request(server.address(), "GET", &wrong_trip_download, &[], &[]).await?;
        assert_eq!(status, 404);

        let trip_b_snapshot: DriverTripSnapshot =
            get_json(server.address(), "/api/trip/trip-b-token-0002").await?;
        assert!(trip_b_snapshot.documents.is_empty());
        assert!(trip_b_snapshot.latest_location.is_none());
        assert_eq!(
            trip_b_snapshot.points[0].point_id,
            id_hex(route_b[0].point_id.as_bytes())
        );

        server.shutdown().await?;
        drop(state);

        let recovered_a = TripFolder::open(&folder_a_path, trip_a)?;
        assert_eq!(
            recovered_a.state().route_points[0].point_id,
            route_a[2].point_id
        );
        assert_eq!(
            recovered_a
                .latest_location()
                .expect("durable location")
                .latitude_e7,
            498_393_000
        );
        assert_eq!(recovered_a.documents().len(), 3);
        let (_, stored_video) = recovered_a.verified_document_path(driver_document)?;
        assert_eq!(fs::read(stored_video)?, video_bytes);
        let (_, stored_photo) = recovered_a.verified_document_path(photo_document)?;
        assert_eq!(fs::read(stored_photo)?, photo_bytes);

        let recovered_b = TripFolder::open(&folder_b_path, trip_b)?;
        assert!(recovered_b.documents().is_empty());
        assert!(recovered_b.latest_location().is_none());
        assert_eq!(
            recovered_b.state().route_points[0].point_id,
            route_b[0].point_id
        );

        Ok(())
    }
}
