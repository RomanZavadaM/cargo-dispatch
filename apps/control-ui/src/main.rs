#![forbid(unsafe_code)]

mod i18n;

// UI language is selected at runtime and persisted in the desktop webview.
// The same eight-language set is enforced by .github/scripts/check-localizations.py.
use crate::i18n::{ControlStrings, UiLocale, localize_problem, strings};
use cargo_dispatch_control_ui_contracts::{
    AccessGuide, AccessStep, ConfigureRoutingInput, ControlUiStatus, CreateManualTripInput,
    CreateManualTripPointInput, DashboardDocumentDirection, DashboardPoint,
    DashboardVehicleRoutingProfile, ListenerMode, OpenPackageInput, OpenedPackage,
    OperatorDocumentInput, PickedFile, RegisterTripInput, ReorderRouteInput,
    RoutingAvailabilityState, RoutingProbeResult, RoutingTollPreference, SaveDocumentInput,
    SessionInput, StartGatewayInput, StartLanInput, TripDashboard, UpdateRoutingContextInput,
    UpdateSessionMetadataInput, format_file_size, is_supported_max_speed_kmh, point_label,
};
use leptos::{prelude::*, task::spawn_local};
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::{JsCast, closure::Closure, prelude::*};
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(inline_js = r#"
export function cargoControlInvoke(command, argsJson) {
  const args = JSON.parse(argsJson);
  return window.__TAURI__.core.invoke(command, args).then(result => JSON.stringify(result));
}
export function cargoControlRandomToken() {
  const bytes = new Uint8Array(18);
  crypto.getRandomValues(bytes);
  return "trip-" + Array.from(bytes, value => value.toString(16).padStart(2, "0")).join("");
}
export function cargoControlRandomId() {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  if (bytes.every(value => value === 0)) bytes[0] = 1;
  return Array.from(bytes, value => value.toString(16).padStart(2, "0")).join("");
}
export function cargoControlNow() {
  return Date.now();
}
export function cargoControlSetInterval(callback, ms) {
  return window.setInterval(callback, ms);
}
export function cargoControlSetTimeout(callback, ms) {
  return window.setTimeout(callback, ms);
}
export function cargoControlInitialLocale() {
  const saved = localStorage.getItem("cargo-dispatch-control-locale");
  const locale = saved || navigator.language || "en";
  document.documentElement.lang = locale;
  return locale;
}
export function cargoControlSetLocale(locale) {
  localStorage.setItem("cargo-dispatch-control-locale", locale);
  document.documentElement.lang = locale;
}
export function cargoControlWindowSessionToken() {
  return new URLSearchParams(window.location.search).get("session") || "";
}
export function cargoControlSetSessionWindowMode(enabled) {
  document.body.classList.toggle("session-window", enabled);
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = cargoControlInvoke)]
    fn cargo_control_invoke(command: &str, args_json: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = cargoControlRandomToken)]
    fn cargo_control_random_token() -> String;

    #[wasm_bindgen(js_name = cargoControlRandomId)]
    fn cargo_control_random_id() -> String;

    #[wasm_bindgen(js_name = cargoControlNow)]
    fn cargo_control_now() -> f64;

    #[wasm_bindgen(js_name = cargoControlSetInterval)]
    fn cargo_control_set_interval(callback: &js_sys::Function, milliseconds: i32) -> i32;

    #[wasm_bindgen(js_name = cargoControlSetTimeout)]
    fn cargo_control_set_timeout(callback: &js_sys::Function, milliseconds: i32) -> i32;

    #[wasm_bindgen(js_name = cargoControlInitialLocale)]
    fn cargo_control_initial_locale() -> String;

    #[wasm_bindgen(js_name = cargoControlSetLocale)]
    fn cargo_control_set_locale(locale: &str);

    #[wasm_bindgen(js_name = cargoControlWindowSessionToken)]
    fn cargo_control_window_session_token() -> String;

    #[wasm_bindgen(js_name = cargoControlSetSessionWindowMode)]
    fn cargo_control_set_session_window_mode(enabled: bool);
}

#[wasm_bindgen(inline_js = r#"
let controlMapState = {
  zoom: 6,
  centerLat: 49.0,
  centerLon: 31.0,
  initialized: false,
  bound: false,
  dragging: false,
  dragStartX: 0,
  dragStartY: 0,
  dragCenterX: 0,
  dragCenterY: 0,
  points: [],
  roadGeometry: null,
  location: null,
  tripId: null,
  dataKey: "",
  locationTitle: "Latest driver position"
};

function clampLat(latitude) {
  return Math.max(-85.05112878, Math.min(85.05112878, latitude));
}

function worldSize(zoom) {
  return 256 * Math.pow(2, zoom);
}

function project(latitude, longitude, zoom) {
  const size = worldSize(zoom);
  const lat = clampLat(latitude) * Math.PI / 180;
  const sin = Math.sin(lat);
  return {
    x: (longitude + 180) / 360 * size,
    y: (0.5 - Math.log((1 + sin) / (1 - sin)) / (4 * Math.PI)) * size
  };
}

function unproject(x, y, zoom) {
  const size = worldSize(zoom);
  const longitude = x / size * 360 - 180;
  const mercator = Math.PI * (1 - 2 * y / size);
  const latitude = Math.atan(Math.sinh(mercator)) * 180 / Math.PI;
  return {latitude: clampLat(latitude), longitude};
}

function coordinates() {
  const result = [];
  for (const point of controlMapState.points) {
    if (point.navigation) {
      result.push({
        latitude: point.navigation.latitude,
        longitude: point.navigation.longitude,
        type: "point",
        sequence: point.sequence,
        title: point.title,
        completed: point.completed
      });
    }
  }
  if (controlMapState.location) {
    result.push({
      latitude: controlMapState.location.latitude,
      longitude: controlMapState.location.longitude,
      type: "location",
      title: controlMapState.locationTitle
    });
  }
  return result;
}

function extentCoordinates() {
  const result = coordinates().map(item => ({
    latitude: item.latitude,
    longitude: item.longitude
  }));
  if (controlMapState.roadGeometry) {
    for (const point of controlMapState.roadGeometry.points) {
      result.push({
        latitude: point.latitude,
        longitude: point.longitude
      });
    }
  }
  return result;
}

function ensureHost() {
  const host = document.getElementById("control-map");
  if (!host || controlMapState.bound) return host;
  host.addEventListener("wheel", event => {
    event.preventDefault();
    cargoControlMapZoom(event.deltaY < 0 ? 1 : -1);
  }, {passive: false});
  host.addEventListener("pointerdown", event => {
    if (event.target.closest("button,a")) return;
    const center = project(controlMapState.centerLat, controlMapState.centerLon, controlMapState.zoom);
    controlMapState.dragging = true;
    controlMapState.dragStartX = event.clientX;
    controlMapState.dragStartY = event.clientY;
    controlMapState.dragCenterX = center.x;
    controlMapState.dragCenterY = center.y;
    host.setPointerCapture(event.pointerId);
  });
  host.addEventListener("pointermove", event => {
    if (!controlMapState.dragging) return;
    const x = controlMapState.dragCenterX - (event.clientX - controlMapState.dragStartX);
    const y = controlMapState.dragCenterY - (event.clientY - controlMapState.dragStartY);
    const center = unproject(x, y, controlMapState.zoom);
    controlMapState.centerLat = center.latitude;
    controlMapState.centerLon = center.longitude;
    render();
  });
  host.addEventListener("pointerup", event => {
    controlMapState.dragging = false;
    if (host.hasPointerCapture(event.pointerId)) host.releasePointerCapture(event.pointerId);
  });
  host.addEventListener("pointercancel", () => {
    controlMapState.dragging = false;
  });
  window.addEventListener("resize", render);
  controlMapState.bound = true;
  return host;
}

function render() {
  const host = ensureHost();
  if (!host) return;
  const width = Math.max(1, host.clientWidth);
  const height = Math.max(1, host.clientHeight);
  const zoom = controlMapState.zoom;
  const center = project(controlMapState.centerLat, controlMapState.centerLon, zoom);
  const left = center.x - width / 2;
  const top = center.y - height / 2;
  const tileLayer = host.querySelector(".map-tiles");
  const overlayLayer = host.querySelector(".map-overlays");
  if (!tileLayer || !overlayLayer) return;
  tileLayer.replaceChildren();
  overlayLayer.replaceChildren();

  const tileMinX = Math.floor(left / 256) - 1;
  const tileMaxX = Math.floor((left + width) / 256) + 1;
  const tileMinY = Math.floor(top / 256) - 1;
  const tileMaxY = Math.floor((top + height) / 256) + 1;
  const tileCount = Math.pow(2, zoom);
  for (let tileX = tileMinX; tileX <= tileMaxX; tileX++) {
    const wrappedX = ((tileX % tileCount) + tileCount) % tileCount;
    for (let tileY = tileMinY; tileY <= tileMaxY; tileY++) {
      if (tileY < 0 || tileY >= tileCount) continue;
      const image = document.createElement("img");
      image.alt = "";
      image.draggable = false;
      image.loading = "lazy";
      image.src = "https://tile.openstreetmap.org/" + zoom + "/" + wrappedX + "/" + tileY + ".png";
      image.style.left = (tileX * 256 - left) + "px";
      image.style.top = (tileY * 256 - top) + "px";
      image.addEventListener("error", () => image.remove());
      tileLayer.append(image);
    }
  }

  if (controlMapState.roadGeometry && controlMapState.roadGeometry.points.length >= 2) {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 " + width + " " + height);
    svg.setAttribute("width", String(width));
    svg.setAttribute("height", String(height));
    svg.style.position = "absolute";
    svg.style.inset = "0";
    const polyline = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
    const shownPoints = controlMapState.roadGeometry.points.map(point => {
      const shown = project(point.latitude, point.longitude, zoom);
      return (shown.x - left) + "," + (shown.y - top);
    });
    polyline.setAttribute("points", shownPoints.join(" "));
    polyline.setAttribute("fill", "none");
    polyline.setAttribute("stroke", "rgb(23, 105, 170)");
    polyline.setAttribute("stroke-width", "5");
    polyline.setAttribute("stroke-linecap", "round");
    polyline.setAttribute("stroke-linejoin", "round");
    svg.append(polyline);
    overlayLayer.append(svg);
  }

  for (const item of coordinates()) {
    const shown = project(item.latitude, item.longitude, zoom);
    const marker = document.createElement("div");
    marker.className = item.type === "location"
      ? "map-marker map-location-marker"
      : "map-marker" + (item.completed ? " completed" : "");
    marker.style.left = (shown.x - left) + "px";
    marker.style.top = (shown.y - top) + "px";
    marker.title = item.title;
    marker.textContent = item.type === "location" ? "" : String(item.sequence);
    overlayLayer.append(marker);
  }
}

function fit() {
  const host = ensureHost();
  const values = extentCoordinates();
  if (!host || !values.length) {
    controlMapState.initialized = true;
    render();
    return;
  }
  const width = Math.max(1, host.clientWidth - 72);
  const height = Math.max(1, host.clientHeight - 72);
  let chosenZoom = 3;
  let center = {latitude: values[0].latitude, longitude: values[0].longitude};
  for (let zoom = 18; zoom >= 3; zoom--) {
    const shown = values.map(item => project(item.latitude, item.longitude, zoom));
    const xs = shown.map(item => item.x);
    const ys = shown.map(item => item.y);
    const minX = Math.min(...xs);
    const maxX = Math.max(...xs);
    const minY = Math.min(...ys);
    const maxY = Math.max(...ys);
    if (maxX - minX <= width && maxY - minY <= height) {
      chosenZoom = zoom;
      center = unproject((minX + maxX) / 2, (minY + maxY) / 2, zoom);
      break;
    }
  }
  controlMapState.zoom = chosenZoom;
  controlMapState.centerLat = center.latitude;
  controlMapState.centerLon = center.longitude;
  controlMapState.initialized = true;
  render();
}

export function cargoControlMapRender(tripId, pointsJson, roadGeometryJson, locationJson, locationTitle) {
  const dataKey = tripId + "|" + pointsJson + "|" + roadGeometryJson + "|" + locationJson + "|" + locationTitle;
  if (controlMapState.initialized && controlMapState.dataKey === dataKey) return;
  controlMapState.points = JSON.parse(pointsJson);
  controlMapState.roadGeometry = JSON.parse(roadGeometryJson);
  controlMapState.location = JSON.parse(locationJson);
  controlMapState.locationTitle = locationTitle;
  if (controlMapState.tripId !== tripId) {
    controlMapState.tripId = tripId;
    controlMapState.initialized = false;
  }
  controlMapState.dataKey = dataKey;
  if (!controlMapState.initialized) fit();
  else render();
}

export function cargoControlMapZoom(delta) {
  controlMapState.zoom = Math.max(3, Math.min(18, controlMapState.zoom + delta));
  render();
}

export function cargoControlMapFit() {
  fit();
}

export function cargoControlCopyText(value, promptLabel) {
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(value).catch(() => {
      window.prompt(promptLabel, value);
    });
  } else {
    window.prompt(promptLabel, value);
  }
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = cargoControlMapRender)]
    fn cargo_control_map_render(
        trip_id: &str,
        points_json: &str,
        road_geometry_json: &str,
        location_json: &str,
        location_title: &str,
    );

    #[wasm_bindgen(js_name = cargoControlMapZoom)]
    fn cargo_control_map_zoom(delta: i32);

    #[wasm_bindgen(js_name = cargoControlMapFit)]
    fn cargo_control_map_fit();

    #[wasm_bindgen(js_name = cargoControlCopyText)]
    fn cargo_control_copy_text(value: &str, prompt_label: &str);
}

fn js_error(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

async fn invoke<T: DeserializeOwned, A: Serialize>(command: &str, args: &A) -> Result<T, String> {
    let args_json = serde_json::to_string(args)
        .map_err(|error| format!("Не вдалося підготувати команду: {error}"))?;
    let value = JsFuture::from(cargo_control_invoke(command, &args_json))
        .await
        .map_err(js_error)?;
    let json = value
        .as_string()
        .ok_or_else(|| "Control повернув неочікувану відповідь".to_owned())?;
    serde_json::from_str(&json).map_err(|error| format!("Не вдалося прочитати відповідь: {error}"))
}

async fn load_status() -> Result<ControlUiStatus, String> {
    invoke("control_status", &serde_json::json!({})).await
}

async fn load_dashboard(token: String) -> Result<TripDashboard, String> {
    invoke(
        "trip_dashboard",
        &serde_json::json!({ "input": SessionInput { token } }),
    )
    .await
}

/// The moment in the date and time format of the interface language.
fn local_time(locale: UiLocale, unix_ms: u64) -> String {
    let moment = js_sys::Date::new(&JsValue::from_f64(unix_ms as f64));
    String::from(moment.to_locale_string(locale.code(), &JsValue::UNDEFINED))
}

fn reordered_open_point_ids(
    points: &[DashboardPoint],
    point_id: &str,
    delta: isize,
) -> Option<Vec<String>> {
    let index = points.iter().position(|item| item.point_id == point_id)?;
    let target = index.checked_add_signed(delta)?;
    if target >= points.len() {
        return None;
    }
    let mut ids = points
        .iter()
        .map(|item| item.point_id.clone())
        .collect::<Vec<_>>();
    ids.swap(index, target);
    Some(ids)
}

fn submit_reorder(
    input: ReorderRouteInput,
    dashboard: RwSignal<Option<TripDashboard>>,
    message: RwSignal<String>,
    error: RwSignal<bool>,
    locale: RwSignal<UiLocale>,
) {
    spawn_local(async move {
        match invoke::<TripDashboard, _>(
            "reorder_trip_route",
            &serde_json::json!({ "input": input }),
        )
        .await
        {
            Ok(current) => {
                dashboard.set(Some(current));
                error.set(false);
            }
            Err(problem) => {
                message.set(localize_problem(locale.get_untracked(), &problem));
                error.set(true);
            }
        }
    });
}

fn submit_session_metadata(
    input: UpdateSessionMetadataInput,
    status: RwSignal<Option<ControlUiStatus>>,
    message: RwSignal<String>,
    error: RwSignal<bool>,
    locale: RwSignal<UiLocale>,
) {
    spawn_local(async move {
        match invoke::<ControlUiStatus, _>(
            "update_session_metadata",
            &serde_json::json!({ "input": input }),
        )
        .await
        {
            Ok(current) => {
                status.set(Some(current));
                error.set(false);
            }
            Err(problem) => {
                message.set(localize_problem(locale.get_untracked(), &problem));
                error.set(true);
            }
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoutingDraftError {
    InvalidNumber,
    ChoiceRequired,
}

#[derive(Debug, Clone, Default)]
struct RoutingProfileDraft {
    height_m: String,
    width_m: String,
    length_m: String,
    gross_weight_t: String,
    axle_load_t: String,
    axle_count: String,
    hazmat: String,
    hgv: String,
    exclude_unpaved: String,
    prefer_truck_routes: String,
    toll_preference: String,
    max_speed_kmh: String,
}

impl RoutingProfileDraft {
    fn from_profile(profile: Option<&DashboardVehicleRoutingProfile>) -> Self {
        let Some(profile) = profile else {
            return Self::default();
        };
        Self {
            height_m: profile
                .height_m
                .map(|value| value.to_string())
                .unwrap_or_default(),
            width_m: profile
                .width_m
                .map(|value| value.to_string())
                .unwrap_or_default(),
            length_m: profile
                .length_m
                .map(|value| value.to_string())
                .unwrap_or_default(),
            gross_weight_t: profile
                .gross_weight_t
                .map(|value| value.to_string())
                .unwrap_or_default(),
            axle_load_t: profile
                .axle_load_t
                .map(|value| value.to_string())
                .unwrap_or_default(),
            axle_count: profile
                .axle_count
                .map(|value| value.to_string())
                .unwrap_or_default(),
            hazmat: if profile.hazmat { "yes" } else { "no" }.to_owned(),
            hgv: if profile.hgv { "yes" } else { "no" }.to_owned(),
            exclude_unpaved: if profile.exclude_unpaved { "yes" } else { "no" }.to_owned(),
            prefer_truck_routes: if profile.prefer_truck_routes {
                "yes"
            } else {
                "no"
            }
            .to_owned(),
            toll_preference: match profile.toll_preference {
                RoutingTollPreference::Allow => "allow",
                RoutingTollPreference::PreferAvoid => "prefer_avoid",
                RoutingTollPreference::Avoid => "avoid",
            }
            .to_owned(),
            max_speed_kmh: profile
                .max_speed_kmh
                .map(|value| value.to_string())
                .unwrap_or_default(),
        }
    }

    fn optional_positive_f64(value: &str) -> Result<Option<f64>, RoutingDraftError> {
        let value = value.trim();
        if value.is_empty() {
            return Ok(None);
        }
        let parsed = value
            .parse::<f64>()
            .map_err(|_| RoutingDraftError::InvalidNumber)?;
        if !parsed.is_finite() || parsed <= 0.0 {
            return Err(RoutingDraftError::InvalidNumber);
        }
        Ok(Some(parsed))
    }

    fn optional_max_speed_kmh(value: &str) -> Result<Option<f64>, RoutingDraftError> {
        match Self::optional_positive_f64(value)? {
            Some(speed) if !is_supported_max_speed_kmh(speed) => {
                Err(RoutingDraftError::InvalidNumber)
            }
            speed => Ok(speed),
        }
    }

    fn optional_positive_u8(value: &str) -> Result<Option<u8>, RoutingDraftError> {
        let value = value.trim();
        if value.is_empty() {
            return Ok(None);
        }
        let parsed = value
            .parse::<u8>()
            .map_err(|_| RoutingDraftError::InvalidNumber)?;
        if parsed == 0 {
            return Err(RoutingDraftError::InvalidNumber);
        }
        Ok(Some(parsed))
    }

    fn explicit_bool(value: &str) -> Result<bool, RoutingDraftError> {
        match value {
            "yes" => Ok(true),
            "no" => Ok(false),
            _ => Err(RoutingDraftError::ChoiceRequired),
        }
    }

    fn profile(&self) -> Result<DashboardVehicleRoutingProfile, RoutingDraftError> {
        let toll_preference = match self.toll_preference.as_str() {
            "allow" => RoutingTollPreference::Allow,
            "prefer_avoid" => RoutingTollPreference::PreferAvoid,
            "avoid" => RoutingTollPreference::Avoid,
            _ => return Err(RoutingDraftError::ChoiceRequired),
        };
        Ok(DashboardVehicleRoutingProfile {
            height_m: Self::optional_positive_f64(&self.height_m)?,
            width_m: Self::optional_positive_f64(&self.width_m)?,
            length_m: Self::optional_positive_f64(&self.length_m)?,
            gross_weight_t: Self::optional_positive_f64(&self.gross_weight_t)?,
            axle_load_t: Self::optional_positive_f64(&self.axle_load_t)?,
            axle_count: Self::optional_positive_u8(&self.axle_count)?,
            hazmat: Self::explicit_bool(&self.hazmat)?,
            hgv: Self::explicit_bool(&self.hgv)?,
            exclude_unpaved: Self::explicit_bool(&self.exclude_unpaved)?,
            prefer_truck_routes: Self::explicit_bool(&self.prefer_truck_routes)?,
            toll_preference,
            max_speed_kmh: Self::optional_max_speed_kmh(&self.max_speed_kmh)?,
        })
    }
}

fn routing_profile_summary(profile: &DashboardVehicleRoutingProfile, locale: UiLocale) -> String {
    let text = strings(locale);
    let mut parts = Vec::new();
    if let Some(value) = profile.height_m {
        parts.push(format!("{}: {value}", text.height_m));
    }
    if let Some(value) = profile.width_m {
        parts.push(format!("{}: {value}", text.width_m));
    }
    if let Some(value) = profile.length_m {
        parts.push(format!("{}: {value}", text.length_m));
    }
    if let Some(value) = profile.gross_weight_t {
        parts.push(format!("{}: {value}", text.gross_weight_t));
    }
    if let Some(value) = profile.axle_load_t {
        parts.push(format!("{}: {value}", text.axle_load_t));
    }
    if let Some(value) = profile.axle_count {
        parts.push(format!("{}: {value}", text.axle_count));
    }
    parts.push(format!(
        "{}: {}",
        text.hazmat,
        if profile.hazmat { text.yes } else { text.no }
    ));
    parts.push(format!(
        "{}: {}",
        text.hgv,
        if profile.hgv { text.yes } else { text.no }
    ));
    parts.push(format!(
        "{}: {}",
        text.exclude_unpaved,
        if profile.exclude_unpaved {
            text.yes
        } else {
            text.no
        }
    ));
    parts.push(format!(
        "{}: {}",
        text.prefer_truck_routes,
        if profile.prefer_truck_routes {
            text.yes
        } else {
            text.no
        }
    ));
    let toll = match profile.toll_preference {
        RoutingTollPreference::Allow => text.toll_allow,
        RoutingTollPreference::PreferAvoid => text.toll_prefer_avoid,
        RoutingTollPreference::Avoid => text.toll_avoid,
    };
    parts.push(format!("{}: {toll}", text.toll_preference));
    if let Some(value) = profile.max_speed_kmh {
        parts.push(format!("{}: {value}", text.max_speed_kmh));
    }
    parts.join(" · ")
}

fn submit_routing_context(
    input: UpdateRoutingContextInput,
    status: RwSignal<Option<ControlUiStatus>>,
    selected_token: RwSignal<String>,
    dashboard: RwSignal<Option<TripDashboard>>,
    message: RwSignal<String>,
    error: RwSignal<bool>,
    locale: RwSignal<UiLocale>,
) {
    let target = input.token.clone();
    spawn_local(async move {
        match invoke::<TripDashboard, _>(
            "update_routing_context",
            &serde_json::json!({ "input": input }),
        )
        .await
        {
            Ok(current) => {
                if selected_token.get_untracked() == target {
                    dashboard.set(Some(current));
                }
                if let Ok(current_status) = load_status().await {
                    status.set(Some(current_status));
                }
                message.set(
                    strings(locale.get_untracked())
                        .routing_context_saved
                        .to_owned(),
                );
                error.set(false);
            }
            Err(problem) => {
                message.set(localize_problem(locale.get_untracked(), &problem));
                error.set(true);
            }
        }
    });
}

#[derive(Clone)]
struct ManualPointDraft {
    point_id: String,
    title: RwSignal<String>,
    latitude: RwSignal<String>,
    longitude: RwSignal<String>,
    address: RwSignal<String>,
}

impl ManualPointDraft {
    fn new() -> Self {
        Self {
            point_id: cargo_control_random_id(),
            title: RwSignal::new(String::new()),
            latitude: RwSignal::new(String::new()),
            longitude: RwSignal::new(String::new()),
            address: RwSignal::new(String::new()),
        }
    }

    fn input(&self) -> Result<CreateManualTripPointInput, ()> {
        let title = self.title.get_untracked().trim().to_owned();
        if title.is_empty() {
            return Err(());
        }
        let latitude = self
            .latitude
            .get_untracked()
            .trim()
            .parse::<f64>()
            .map_err(|_| ())?;
        let longitude = self
            .longitude
            .get_untracked()
            .trim()
            .parse::<f64>()
            .map_err(|_| ())?;
        if !latitude.is_finite()
            || !longitude.is_finite()
            || !(-90.0..=90.0).contains(&latitude)
            || !(-180.0..=180.0).contains(&longitude)
        {
            return Err(());
        }
        let address = self.address.get_untracked().trim().to_owned();
        Ok(CreateManualTripPointInput {
            point_id: self.point_id.clone(),
            title,
            latitude,
            longitude,
            address: (!address.is_empty()).then_some(address),
        })
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(ControlApp);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessDialog {
    Add,
    Connect,
    Road,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectChoice {
    Local,
    Lan,
    Internet,
}

async fn stop_running_listener(running: bool) -> Result<(), String> {
    if running {
        invoke::<ControlUiStatus, _>("stop_browser_server", &serde_json::json!({})).await?;
    }
    Ok(())
}

fn access_step_title(step: AccessStep, text: &ControlStrings) -> &'static str {
    match step {
        AccessStep::Connect => text.broadcast,
        AccessStep::AddTrip => text.connect_trip,
        AccessStep::ShareLink => text.step_share,
    }
}

fn access_step_hint(step: AccessStep, text: &ControlStrings) -> &'static str {
    match step {
        AccessStep::Connect => text.step_connect_hint,
        AccessStep::AddTrip => text.step_add_hint,
        AccessStep::ShareLink => text.step_share_hint,
    }
}

#[component]
fn ControlApp() -> impl IntoView {
    let locale = RwSignal::new(UiLocale::parse(&cargo_control_initial_locale()));
    let folder = RwSignal::new(String::new());
    let token = RwSignal::new(String::new());
    let manual_folder = RwSignal::new(String::new());
    let manual_token = RwSignal::new(cargo_control_random_token());
    let manual_trip_id = RwSignal::new(cargo_control_random_id());
    let manual_display_name = RwSignal::new(String::new());
    let manual_points = RwSignal::new(vec![ManualPointDraft::new(), ManualPointDraft::new()]);
    let lan_ip = RwSignal::new(String::new());
    let lan_candidates = RwSignal::new(Vec::<String>::new());
    let lan_port = RwSignal::new("8080".to_owned());
    let gateway_url = RwSignal::new(String::new());
    let gateway_secret = RwSignal::new(String::new());
    let routing_url = RwSignal::new(String::new());
    let routing_dataset = RwSignal::new(String::new());
    let routing_connect_timeout = RwSignal::new("1500".to_owned());
    let routing_request_timeout = RwSignal::new("10000".to_owned());
    let status = RwSignal::new(None::<ControlUiStatus>);
    let selected_token = RwSignal::new(String::new());
    let dialog = RwSignal::new(None::<AccessDialog>);
    let add_manual = RwSignal::new(false);
    let connect_choice = RwSignal::new(ConnectChoice::Local);
    let window_session_token = cargo_control_window_session_token();
    let session_window = !window_session_token.is_empty();
    if session_window {
        selected_token.set(window_session_token);
        cargo_control_set_session_window_mode(true);
    }
    let dashboard = RwSignal::new(None::<TripDashboard>);
    let picked_file = RwSignal::new(None::<PickedFile>);
    let attachment_point = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());
    let error = RwSignal::new(false);
    let busy = RwSignal::new(false);

    Effect::new(move |_| {
        if token.get_untracked().is_empty() {
            token.set(cargo_control_random_token());
        }
        spawn_local(async move {
            if let Ok(current) = load_status().await {
                if selected_token.get_untracked().is_empty()
                    && let Some(first) = current.sessions.first()
                {
                    selected_token.set(first.token.clone());
                }
                if let Some(routing) = current.routing.as_ref() {
                    routing_url.set(routing.base_url.clone());
                    routing_dataset.set(routing.map_dataset_id.clone());
                    routing_connect_timeout.set(routing.connect_timeout_ms.to_string());
                    routing_request_timeout.set(routing.request_timeout_ms.to_string());
                }
                status.set(Some(current));
            }
        });

        let refresh = Closure::wrap(Box::new(move || {
            let active = selected_token.get_untracked();
            if active.is_empty() {
                return;
            }
            spawn_local(async move {
                if let Ok(current) = load_dashboard(active).await {
                    dashboard.set(Some(current));
                }
            });
        }) as Box<dyn FnMut()>);
        let _ =
            cargo_control_set_interval(refresh.as_ref().unchecked_ref::<js_sys::Function>(), 2_000);
        refresh.forget();
    });

    Effect::new(move |_| {
        if let Some(current) = dashboard.get()
            && let (Ok(points_json), Ok(road_geometry_json), Ok(location_json)) = (
                serde_json::to_string(&current.points),
                serde_json::to_string(&current.road_geometry),
                serde_json::to_string(&current.latest_location),
            )
        {
            cargo_control_map_render(
                &current.trip_id,
                &points_json,
                &road_geometry_json,
                &location_json,
                strings(locale.get()).latest_driver_position,
            );
        }
    });

    Effect::new(move |_| {
        let shown = message.get();
        if shown.is_empty() || error.get() {
            return;
        }
        let hide = Closure::once_into_js(move || {
            if !error.get_untracked() && message.get_untracked() == shown {
                message.set(String::new());
            }
        });
        cargo_control_set_timeout(hide.unchecked_ref(), 5_000);
    });

    // The program finds this computer's network address; typing stays possible.
    Effect::new(move |_| {
        if dialog.get() != Some(AccessDialog::Connect) {
            return;
        }
        spawn_local(async move {
            let Ok(found) = invoke::<Vec<String>, _>("lan_addresses", &serde_json::json!({})).await
            else {
                return;
            };
            let current = lan_ip.get_untracked();
            let outdated =
                lan_candidates.get_untracked().contains(&current) && !found.contains(&current);
            if (current.is_empty() || outdated)
                && let Some(first) = found.first()
            {
                lan_ip.set(first.clone());
            }
            lan_candidates.set(found);
        });
    });

    let set_status_result = move |result: Result<ControlUiStatus, String>,
                                  success: &'static str| {
        busy.set(false);
        match result {
            Ok(current) => {
                if selected_token.get_untracked().is_empty()
                    && let Some(first) = current.sessions.first()
                {
                    selected_token.set(first.token.clone());
                }
                status.set(Some(current));
                message.set(success.to_owned());
                error.set(false);
            }
            Err(problem) => {
                message.set(localize_problem(locale.get_untracked(), &problem));
                error.set(true);
            }
        }
    };

    let refresh_dashboard = move |_| {
        let active = selected_token.get_untracked();
        if active.is_empty() {
            message.set(
                strings(locale.get_untracked())
                    .choose_session_first
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        spawn_local(async move {
            match load_dashboard(active).await {
                Ok(current) => {
                    dashboard.set(Some(current));
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked(), &problem));
                    error.set(true);
                }
            }
        });
    };

    let pick_folder = move |_| {
        spawn_local(async move {
            match invoke::<Option<String>, _>("pick_trip_folder", &serde_json::json!({})).await {
                Ok(Some(path)) => {
                    folder.set(path);
                    error.set(false);
                }
                Ok(None) => {}
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked(), &problem));
                    error.set(true);
                }
            }
        });
    };

    let open_package = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        message.set(strings(locale.get_untracked()).opening_trip.to_owned());
        error.set(false);
        let input = OpenPackageInput {
            token: token.get_untracked(),
        };
        spawn_local(async move {
            let result: Result<Option<OpenedPackage>, String> = invoke(
                "open_planner_package",
                &serde_json::json!({ "input": input }),
            )
            .await;
            match result {
                Ok(None) => {
                    busy.set(false);
                    message.set(String::new());
                }
                Ok(Some(opened)) => {
                    dialog.set(None);
                    token.set(cargo_control_random_token());
                    selected_token.set(opened.token);
                    let done = strings(locale.get_untracked()).package_opened;
                    set_status_result(Ok(opened.status), done);
                }
                Err(problem) => {
                    let done = strings(locale.get_untracked()).package_opened;
                    set_status_result(Err(problem), done);
                }
            }
        });
    };

    let register = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        message.set(strings(locale.get_untracked()).opening_trip.to_owned());
        error.set(false);
        let input = RegisterTripInput {
            folder_path: folder.get_untracked(),
            token: token.get_untracked(),
        };
        spawn_local(async move {
            let result: Result<ControlUiStatus, String> =
                invoke("register_trip", &serde_json::json!({ "input": input })).await;
            if result.is_ok() {
                dialog.set(None);
                folder.set(String::new());
                token.set(cargo_control_random_token());
            }
            set_status_result(result, strings(locale.get_untracked()).trip_registered);
        });
    };

    let pick_manual_folder = move |_| {
        spawn_local(async move {
            match invoke::<Option<String>, _>("pick_trip_folder", &serde_json::json!({})).await {
                Ok(Some(path)) => {
                    manual_folder.set(path);
                    error.set(false);
                }
                Ok(None) => {}
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked(), &problem));
                    error.set(true);
                }
            }
        });
    };

    let create_manual = move |_| {
        if busy.get_untracked() {
            return;
        }
        let points = match manual_points
            .get_untracked()
            .iter()
            .map(ManualPointDraft::input)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(points) if !points.is_empty() => points,
            _ => {
                message.set(
                    strings(locale.get_untracked())
                        .manual_point_invalid
                        .to_owned(),
                );
                error.set(true);
                return;
            }
        };
        let created_token = manual_token.get_untracked();
        let input = CreateManualTripInput {
            folder_path: manual_folder.get_untracked(),
            token: created_token.clone(),
            trip_id: manual_trip_id.get_untracked(),
            display_name: {
                let value = manual_display_name.get_untracked().trim().to_owned();
                (!value.is_empty()).then_some(value)
            },
            points,
        };
        busy.set(true);
        message.set(strings(locale.get_untracked()).opening_trip.to_owned());
        error.set(false);
        spawn_local(async move {
            match invoke::<ControlUiStatus, _>(
                "create_manual_trip",
                &serde_json::json!({ "input": input }),
            )
            .await
            {
                Ok(current) => {
                    status.set(Some(current));
                    selected_token.set(created_token.clone());
                    match load_dashboard(created_token).await {
                        Ok(current) => dashboard.set(Some(current)),
                        Err(problem) => {
                            message.set(localize_problem(locale.get_untracked(), &problem));
                            error.set(true);
                            busy.set(false);
                            return;
                        }
                    }
                    manual_folder.set(String::new());
                    manual_display_name.set(String::new());
                    manual_token.set(cargo_control_random_token());
                    manual_trip_id.set(cargo_control_random_id());
                    manual_points.set(vec![ManualPointDraft::new(), ManualPointDraft::new()]);
                    dialog.set(None);
                    message.set(
                        strings(locale.get_untracked())
                            .manual_trip_created
                            .to_owned(),
                    );
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let start_loopback = move |_| {
        if busy.get_untracked() {
            return;
        }
        let listener = status.get_untracked().and_then(|current| current.listener);
        if listener
            .as_ref()
            .is_some_and(|active| active.mode == ListenerMode::Loopback)
        {
            dialog.set(None);
            return;
        }
        busy.set(true);
        let running = listener.is_some();
        spawn_local(async move {
            let result: Result<ControlUiStatus, String> = match stop_running_listener(running).await
            {
                Ok(()) => invoke("start_loopback", &serde_json::json!({})).await,
                Err(problem) => Err(problem),
            };
            if result.is_ok() {
                dialog.set(None);
            } else if let Ok(current) = load_status().await {
                status.set(Some(current));
            }
            set_status_result(result, strings(locale.get_untracked()).loopback_started);
        });
    };

    let start_lan = move |_| {
        if busy.get_untracked() {
            return;
        }
        let port = match lan_port.get_untracked().trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                message.set(strings(locale.get_untracked()).invalid_port.to_owned());
                error.set(true);
                return;
            }
        };
        let ip = lan_ip.get_untracked().trim().to_owned();
        let Ok(address) = ip.parse::<std::net::IpAddr>() else {
            message.set(localize_problem(
                locale.get_untracked(),
                "Некоректна LAN-адреса",
            ));
            error.set(true);
            return;
        };
        let requested = std::net::SocketAddr::new(address, port).to_string();
        let listener = status.get_untracked().and_then(|current| current.listener);
        if listener
            .as_ref()
            .is_some_and(|active| active.mode == ListenerMode::Lan && active.address == requested)
        {
            dialog.set(None);
            return;
        }
        busy.set(true);
        let input = StartLanInput { ip, port };
        let running = listener.is_some();
        spawn_local(async move {
            let result: Result<ControlUiStatus, String> = match stop_running_listener(running).await
            {
                Ok(()) => invoke("start_lan", &serde_json::json!({ "input": input })).await,
                Err(problem) => Err(problem),
            };
            if result.is_ok() {
                dialog.set(None);
            } else if let Ok(current) = load_status().await {
                status.set(Some(current));
            }
            set_status_result(result, strings(locale.get_untracked()).lan_started);
        });
    };

    let start_gateway = move |_| {
        if busy.get_untracked() {
            return;
        }
        let base_url = gateway_url.get_untracked().trim().to_owned();
        let secret = gateway_secret.get_untracked();
        if base_url.is_empty() {
            message.set(
                strings(locale.get_untracked())
                    .gateway_url_required
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        if secret.len() < 32 {
            message.set(strings(locale.get_untracked()).secret_too_short.to_owned());
            error.set(true);
            return;
        }

        busy.set(true);
        let input = StartGatewayInput {
            gateway_base_url: base_url,
            control_secret: secret,
        };
        spawn_local(async move {
            let result: Result<ControlUiStatus, String> =
                invoke("start_gateway", &serde_json::json!({ "input": input })).await;
            if result.is_ok() {
                dialog.set(None);
            }
            set_status_result(result, strings(locale.get_untracked()).gateway_started);
        });
    };

    let stop_gateway = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let result = invoke("stop_gateway", &serde_json::json!({})).await;
            set_status_result(result, strings(locale.get_untracked()).gateway_stopped);
        });
    };

    let stop = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let result = invoke("stop_browser_server", &serde_json::json!({})).await;
            set_status_result(result, strings(locale.get_untracked()).broadcast_stopped);
        });
    };

    let configure_routing = move |_| {
        if busy.get_untracked() {
            return;
        }
        let base_url = routing_url.get_untracked().trim().to_owned();
        let map_dataset_id = routing_dataset.get_untracked().trim().to_owned();
        if base_url.is_empty() {
            message.set(
                strings(locale.get_untracked())
                    .routing_url_required
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        if map_dataset_id.is_empty() {
            message.set(
                strings(locale.get_untracked())
                    .routing_dataset_required
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        let connect_timeout_ms = match routing_connect_timeout
            .get_untracked()
            .trim()
            .parse::<u64>()
        {
            Ok(value) if (100..=30_000).contains(&value) => value,
            _ => {
                message.set(
                    strings(locale.get_untracked())
                        .invalid_routing_timeout
                        .to_owned(),
                );
                error.set(true);
                return;
            }
        };
        let request_timeout_ms = match routing_request_timeout
            .get_untracked()
            .trim()
            .parse::<u64>()
        {
            Ok(value) if (250..=120_000).contains(&value) && value >= connect_timeout_ms => value,
            _ => {
                message.set(
                    strings(locale.get_untracked())
                        .invalid_routing_timeout
                        .to_owned(),
                );
                error.set(true);
                return;
            }
        };
        busy.set(true);
        let input = ConfigureRoutingInput {
            base_url,
            map_dataset_id,
            connect_timeout_ms,
            request_timeout_ms,
        };
        spawn_local(async move {
            let result = invoke("configure_routing", &serde_json::json!({ "input": input })).await;
            set_status_result(result, strings(locale.get_untracked()).routing_configured);
        });
    };

    let test_routing = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match invoke::<RoutingProbeResult, _>("test_routing", &serde_json::json!({})).await {
                Ok(probe) => {
                    let text = match probe.state {
                        RoutingAvailabilityState::NotConfigured => strings(locale.get_untracked())
                            .routing_not_configured
                            .to_owned(),
                        RoutingAvailabilityState::Ok => {
                            let version = probe
                                .engine_version
                                .as_deref()
                                .map(|value| format!(" · Valhalla {value}"))
                                .unwrap_or_default();
                            format!(
                                "{}{version}",
                                strings(locale.get_untracked()).routing_available
                            )
                        }
                        RoutingAvailabilityState::Timeout => {
                            strings(locale.get_untracked()).routing_timeout.to_owned()
                        }
                        RoutingAvailabilityState::EngineUnavailable => {
                            strings(locale.get_untracked())
                                .routing_unavailable
                                .to_owned()
                        }
                        RoutingAvailabilityState::DatasetMissing => strings(locale.get_untracked())
                            .routing_dataset_missing
                            .to_owned(),
                        RoutingAvailabilityState::EngineError => {
                            strings(locale.get_untracked()).routing_error.to_owned()
                        }
                    };
                    error.set(probe.state != RoutingAvailabilityState::Ok);
                    message.set(text);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let disable_routing = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let result = invoke("disable_routing", &serde_json::json!({})).await;
            set_status_result(result, strings(locale.get_untracked()).routing_disabled);
        });
    };

    let pick_file = move |_| {
        spawn_local(async move {
            match invoke::<Option<PickedFile>, _>("pick_operator_file", &serde_json::json!({}))
                .await
            {
                Ok(file) => {
                    picked_file.set(file);
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked(), &problem));
                    error.set(true);
                }
            }
        });
    };

    let send_file = move |_| {
        if busy.get_untracked() {
            return;
        }
        let active = selected_token.get_untracked();
        let Some(file) = picked_file.get_untracked() else {
            message.set(strings(locale.get_untracked()).choose_file_first.to_owned());
            error.set(true);
            return;
        };
        if active.is_empty() {
            message.set(
                strings(locale.get_untracked())
                    .choose_session_first
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        busy.set(true);
        let point = attachment_point.get_untracked();
        let input = OperatorDocumentInput {
            token: active,
            document_id: cargo_control_random_id(),
            source_path: file.path,
            original_name: file.original_name,
            mime_type: file.mime_type,
            point_id: (!point.is_empty()).then_some(point),
            created_at_unix_ms: cargo_control_now().round() as u64,
        };
        spawn_local(async move {
            match invoke::<TripDashboard, _>(
                "send_operator_document",
                &serde_json::json!({ "input": input }),
            )
            .await
            {
                Ok(current) => {
                    dashboard.set(Some(current));
                    picked_file.set(None);
                    message.set(strings(locale.get_untracked()).file_sent.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let access_guide = move || {
        let current = status.get();
        let has_trip = current
            .as_ref()
            .is_some_and(|value| !value.sessions.is_empty());
        let listener_on = current
            .as_ref()
            .is_some_and(|value| value.listener.is_some());
        let gateway_on = current
            .as_ref()
            .is_some_and(|value| value.gateway.is_some());
        let through_internet = gateway_on || connect_choice.get() == ConnectChoice::Internet;
        AccessGuide::new(through_internet, has_trip, listener_on, gateway_on)
    };

    let access_status = move || {
        let text = strings(locale.get());
        let Some(current) = status.get() else {
            return text.reading_state.to_owned();
        };
        let mut parts = Vec::new();
        if let Some(listener) = current.listener {
            let mode = match listener.mode {
                ListenerMode::Loopback => text.mode_local,
                ListenerMode::Lan => text.mode_lan,
            };
            parts.push(mode.to_owned());
        }
        if current.gateway.is_some() {
            parts.push(text.mode_internet.to_owned());
        }
        let value = if parts.is_empty() {
            text.access_off.to_owned()
        } else {
            parts.join(" + ")
        };
        format!("{}: {value}", text.access_status)
    };

    view! {
        <main>
            <header>
                <div class="actions">
                    <p>"Cargo Dispatch"</p>
                    <label>
                        {move || strings(locale.get()).language}
                        <select
                            id="control-language"
                            prop:value=move || locale.get().code()
                            on:change=move |event| {
                                let next = UiLocale::parse(&event_target_value(&event));
                                locale.set(next);
                                cargo_control_set_locale(next.code());
                            }
                        >
                            <For
                                each=move || UiLocale::ALL
                                key=|item| item.code()
                                children=move |item| view! {
                                    <option value=item.code()>{item.label()}</option>
                                }
                            />
                        </select>
                    </label>
                </div>
                <h1>{move || strings(locale.get()).title}</h1>
                <p class="note">{move || strings(locale.get()).subtitle}</p>
            </header>

            <div class="toolbar manager-only">
                <button class="primary" type="button" on:click=move |_| dialog.set(Some(AccessDialog::Add))>
                    {move || strings(locale.get()).connect_trip}
                </button>
                <button class="primary" type="button" on:click=move |_| dialog.set(Some(AccessDialog::Connect))>
                    {move || strings(locale.get()).broadcast}
                </button>
                <button type="button" on:click=move |_| dialog.set(Some(AccessDialog::Road))>
                    {move || strings(locale.get()).routing_engine}
                </button>
                <button type="button" on:click=move |_| dialog.set(Some(AccessDialog::Help))>
                    {move || strings(locale.get()).help_title}
                </button>
            </div>

            <div class="access-line manager-only">
                <strong>{access_status}</strong>
                <button
                    class="danger"
                    type="button"
                    class:hidden=move || status.get().and_then(|current| current.listener).is_none()
                    on:click=stop
                    disabled=move || busy.get()
                >
                    {move || strings(locale.get()).stop_broadcast}
                </button>
                <button
                    class="danger"
                    type="button"
                    class:hidden=move || status.get().and_then(|current| current.gateway).is_none()
                    on:click=stop_gateway
                    disabled=move || busy.get()
                >
                    {move || strings(locale.get()).disable_gateway}
                </button>
            </div>

            <section class="manager-only">
                <h2>{move || strings(locale.get()).next_title}</h2>
                <ol class="steps">
                    {move || {
                        let guide = access_guide();
                        let text = strings(locale.get());
                        guide
                            .order
                            .into_iter()
                            .enumerate()
                            .map(|(index, step)| {
                                let class = if step == guide.current() {
                                    "now"
                                } else if guide.is_done(step) {
                                    "done"
                                } else {
                                    ""
                                };
                                let mark = if guide.is_done(step) {
                                    "✓".to_owned()
                                } else {
                                    format!("{}.", index + 1)
                                };
                                view! { <li class=class>{format!("{mark} {}", access_step_title(step, text))}</li> }
                            })
                            .collect_view()
                    }}
                </ol>
                <p>{move || access_step_hint(access_guide().current(), strings(locale.get()))}</p>
                <ul class:hidden=move || access_guide().current() != AccessStep::ShareLink>
                    {move || {
                        status
                            .get()
                            .map(|current| current.driver_links())
                            .unwrap_or_default()
                            .into_iter()
                            .map(|link| {
                                let href = link.url.clone();
                                let copy_url = link.url.clone();
                                let through_internet = link.through_internet;
                                view! {
                                    <li>
                                        <div><strong>{link.trip_name}</strong></div>
                                        <div>
                                            {move || {
                                                let text = strings(locale.get());
                                                if through_internet {
                                                    text.public_driver_url
                                                } else {
                                                    text.local_driver_url
                                                }
                                            }}
                                        </div>
                                        <code class="link-text">{link.url}</code>
                                        <div class="actions">
                                            <a href=href target="_blank" rel="noopener">{move || strings(locale.get()).open}</a>
                                            <button
                                                type="button"
                                                on:click=move |_| cargo_control_copy_text(
                                                    &copy_url,
                                                    strings(locale.get_untracked()).copy_driver_url,
                                                )
                                            >
                                                {move || strings(locale.get()).copy}
                                            </button>
                                        </div>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
                <div class="actions">
                    <button
                        class="primary"
                        type="button"
                        class:hidden=move || access_guide().current() == AccessStep::ShareLink
                        on:click=move |_| {
                            dialog.set(Some(match access_guide().current() {
                                AccessStep::Connect => AccessDialog::Connect,
                                AccessStep::AddTrip | AccessStep::ShareLink => AccessDialog::Add,
                            }));
                        }
                    >
                        {move || access_step_title(access_guide().current(), strings(locale.get()))}
                    </button>
                </div>
            </section>

            <section class="manager-only">
                <h2>{move || strings(locale.get()).browser_sessions}</h2>
                <p class="note" class:hidden=move || status.get().is_some_and(|current| !current.sessions.is_empty())>
                    {move || strings(locale.get()).no_trips}
                </p>
                <ul>
                    <For
                        each=move || status.get().map(|current| current.sessions).unwrap_or_default()
                        key=|session| session.token.clone()
                        children=move |session| {
                            let trip = session.trip_id.clone();
                            let session_token = session.token.clone();
                            let dashboard_token = session.token.clone();
                            let live_token = session.token.clone();
                            let live = Memo::new(move |_| {
                                status.get().and_then(|current| current.session(&live_token).cloned())
                            });
                            let edit_display_name = RwSignal::new(session.display_name.clone());
                            let edit_endpoint_id =
                                RwSignal::new(session.endpoint_id.clone().unwrap_or_default());
                            let edit_endpoint_label =
                                RwSignal::new(session.endpoint_label.clone().unwrap_or_default());
                            let edit_vehicle_label =
                                RwSignal::new(session.vehicle_label.clone().unwrap_or_default());
                            let edit_vehicle_type =
                                RwSignal::new(session.vehicle_type.clone().unwrap_or_default());
                            let edit_routing_profile = RwSignal::new(
                                RoutingProfileDraft::from_profile(session.routing_profile.as_ref()),
                            );
                            let edit_token = session.token.clone();
                            let routing_token_apply = session.token.clone();
                            let routing_token_clear = session.token.clone();
                            view! {
                                <li>
                                    <div><strong>{move || live.get().map(|value| value.display_name)}</strong></div>
                                    {move || live.get().and_then(|value| value.vehicle_label).map(|value| view! {
                                        <div><strong>{move || format!("{}: ", strings(locale.get()).vehicle_label)}</strong>{value}</div>
                                    })}
                                    {move || live.get().and_then(|value| value.vehicle_type).map(|value| view! {
                                        <div><strong>{move || format!("{}: ", strings(locale.get()).vehicle_type)}</strong>{value}</div>
                                    })}
                                    {move || live.get().and_then(|value| value.endpoint_label).map(|value| view! {
                                        <div><strong>{move || format!("{}: ", strings(locale.get()).endpoint_label)}</strong>{value}</div>
                                    })}
                                    <div><strong>{move || format!("{}: ", strings(locale.get()).trip)}</strong><code>{trip}</code></div>
                                    {move || live.get().and_then(|value| value.public_driver_url).map(|driver_url| {
                                        let href = driver_url.clone();
                                        let copy_url = driver_url.clone();
                                        let shown_url = driver_url.clone();
                                        view! {
                                            <div>
                                                <strong>{move || format!("{}: ", strings(locale.get()).public_driver_url)}</strong>
                                                <code class="link-text">{shown_url}</code>
                                                <div class="actions">
                                                    <a href=href target="_blank" rel="noopener">{move || strings(locale.get()).open}</a>
                                                    <button
                                                        type="button"
                                                        on:click=move |_| cargo_control_copy_text(
                                                            &copy_url,
                                                            strings(locale.get_untracked()).copy_driver_url,
                                                        )
                                                    >
                                                        {move || strings(locale.get()).copy}
                                                    </button>
                                                </div>
                                            </div>
                                        }
                                    })}
                                    {move || live.get().and_then(|value| value.driver_url).map(|driver_url| {
                                        let href = driver_url.clone();
                                        let copy_url = driver_url.clone();
                                        let shown_url = driver_url.clone();
                                        view! {
                                            <div>
                                                <strong>{move || format!("{}: ", strings(locale.get()).local_driver_url)}</strong>
                                                <code class="link-text">{shown_url}</code>
                                                <div class="actions">
                                                    <a href=href target="_blank" rel="noopener">{move || strings(locale.get()).open}</a>
                                                    <button
                                                        type="button"
                                                        on:click=move |_| cargo_control_copy_text(
                                                            &copy_url,
                                                            strings(locale.get_untracked()).copy_driver_url,
                                                        )
                                                    >
                                                        {move || strings(locale.get()).copy}
                                                    </button>
                                                </div>
                                            </div>
                                        }
                                    })}
                                    <details>
                                        <summary>{move || strings(locale.get()).technical_data}</summary>
                                        <label>
                                            {move || strings(locale.get()).session_display_name}
                                            <input
                                                type="text"
                                                prop:value=move || edit_display_name.get()
                                                on:input=move |event| edit_display_name.set(event_target_value(&event))
                                            />
                                        </label>
                                        <label>
                                            {move || strings(locale.get()).endpoint_id}
                                            <input
                                                type="text"
                                                maxlength="32"
                                                prop:value=move || edit_endpoint_id.get()
                                                on:input=move |event| edit_endpoint_id.set(event_target_value(&event))
                                            />
                                        </label>
                                        <label>
                                            {move || strings(locale.get()).endpoint_label}
                                            <input
                                                type="text"
                                                prop:value=move || edit_endpoint_label.get()
                                                on:input=move |event| edit_endpoint_label.set(event_target_value(&event))
                                            />
                                        </label>
                                        <label>
                                            {move || strings(locale.get()).vehicle_label}
                                            <input
                                                type="text"
                                                prop:value=move || edit_vehicle_label.get()
                                                on:input=move |event| edit_vehicle_label.set(event_target_value(&event))
                                            />
                                        </label>
                                        <label>
                                            {move || strings(locale.get()).vehicle_type}
                                            <input
                                                type="text"
                                                prop:value=move || edit_vehicle_type.get()
                                                on:input=move |event| edit_vehicle_type.set(event_target_value(&event))
                                            />
                                        </label>
                                        <button
                                            type="button"
                                            on:click=move |_| {
                                                let endpoint = edit_endpoint_id.get_untracked();
                                                submit_session_metadata(
                                                    UpdateSessionMetadataInput {
                                                        token: edit_token.clone(),
                                                        display_name: edit_display_name.get_untracked(),
                                                        endpoint_id: (!endpoint.trim().is_empty()).then_some(endpoint),
                                                        endpoint_label: Some(edit_endpoint_label.get_untracked()),
                                                        vehicle_label: Some(edit_vehicle_label.get_untracked()),
                                                        vehicle_type: Some(edit_vehicle_type.get_untracked()),
                                                    },
                                                    status,
                                                    message,
                                                    error,
                                                    locale,
                                                );
                                            }
                                        >
                                            {move || strings(locale.get()).save}
                                        </button>
                                        <h4>{move || strings(locale.get()).routing_context}</h4>
                                        <p class="note">{move || strings(locale.get()).routing_context_note}</p>
                                        <div class="grid">
                                            <label>
                                                {move || strings(locale.get()).height_m}
                                                <input type="number" step="0.01" min="0"
                                                    prop:value=move || edit_routing_profile.get().height_m
                                                    on:input=move |event| edit_routing_profile.update(|draft| draft.height_m = event_target_value(&event))
                                                />
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).width_m}
                                                <input type="number" step="0.01" min="0"
                                                    prop:value=move || edit_routing_profile.get().width_m
                                                    on:input=move |event| edit_routing_profile.update(|draft| draft.width_m = event_target_value(&event))
                                                />
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).length_m}
                                                <input type="number" step="0.01" min="0"
                                                    prop:value=move || edit_routing_profile.get().length_m
                                                    on:input=move |event| edit_routing_profile.update(|draft| draft.length_m = event_target_value(&event))
                                                />
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).gross_weight_t}
                                                <input type="number" step="0.01" min="0"
                                                    prop:value=move || edit_routing_profile.get().gross_weight_t
                                                    on:input=move |event| edit_routing_profile.update(|draft| draft.gross_weight_t = event_target_value(&event))
                                                />
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).axle_load_t}
                                                <input type="number" step="0.01" min="0"
                                                    prop:value=move || edit_routing_profile.get().axle_load_t
                                                    on:input=move |event| edit_routing_profile.update(|draft| draft.axle_load_t = event_target_value(&event))
                                                />
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).axle_count}
                                                <input type="number" step="1" min="1" max="255"
                                                    prop:value=move || edit_routing_profile.get().axle_count
                                                    on:input=move |event| edit_routing_profile.update(|draft| draft.axle_count = event_target_value(&event))
                                                />
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).hazmat}
                                                <select
                                                    prop:value=move || edit_routing_profile.get().hazmat
                                                    on:change=move |event| edit_routing_profile.update(|draft| draft.hazmat = event_target_value(&event))
                                                >
                                                    <option value="">{move || strings(locale.get()).not_specified}</option>
                                                    <option value="yes">{move || strings(locale.get()).yes}</option>
                                                    <option value="no">{move || strings(locale.get()).no}</option>
                                                </select>
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).hgv}
                                                <select
                                                    prop:value=move || edit_routing_profile.get().hgv
                                                    on:change=move |event| edit_routing_profile.update(|draft| draft.hgv = event_target_value(&event))
                                                >
                                                    <option value="">{move || strings(locale.get()).not_specified}</option>
                                                    <option value="yes">{move || strings(locale.get()).yes}</option>
                                                    <option value="no">{move || strings(locale.get()).no}</option>
                                                </select>
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).exclude_unpaved}
                                                <select
                                                    prop:value=move || edit_routing_profile.get().exclude_unpaved
                                                    on:change=move |event| edit_routing_profile.update(|draft| draft.exclude_unpaved = event_target_value(&event))
                                                >
                                                    <option value="">{move || strings(locale.get()).not_specified}</option>
                                                    <option value="yes">{move || strings(locale.get()).yes}</option>
                                                    <option value="no">{move || strings(locale.get()).no}</option>
                                                </select>
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).prefer_truck_routes}
                                                <select
                                                    prop:value=move || edit_routing_profile.get().prefer_truck_routes
                                                    on:change=move |event| edit_routing_profile.update(|draft| draft.prefer_truck_routes = event_target_value(&event))
                                                >
                                                    <option value="">{move || strings(locale.get()).not_specified}</option>
                                                    <option value="yes">{move || strings(locale.get()).yes}</option>
                                                    <option value="no">{move || strings(locale.get()).no}</option>
                                                </select>
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).toll_preference}
                                                <select
                                                    prop:value=move || edit_routing_profile.get().toll_preference
                                                    on:change=move |event| edit_routing_profile.update(|draft| draft.toll_preference = event_target_value(&event))
                                                >
                                                    <option value="">{move || strings(locale.get()).not_specified}</option>
                                                    <option value="allow">{move || strings(locale.get()).toll_allow}</option>
                                                    <option value="prefer_avoid">{move || strings(locale.get()).toll_prefer_avoid}</option>
                                                    <option value="avoid">{move || strings(locale.get()).toll_avoid}</option>
                                                </select>
                                            </label>
                                            <label>
                                                {move || strings(locale.get()).max_speed_kmh}
                                                <input type="number" step="0.1" min="10" max="252"
                                                    prop:value=move || edit_routing_profile.get().max_speed_kmh
                                                    on:input=move |event| edit_routing_profile.update(|draft| draft.max_speed_kmh = event_target_value(&event))
                                                />
                                            </label>
                                        </div>
                                        <div class="actions">
                                            <button
                                                class="primary"
                                                type="button"
                                                on:click=move |_| {
                                                    match edit_routing_profile.get_untracked().profile() {
                                                        Ok(profile) => submit_routing_context(
                                                            UpdateRoutingContextInput {
                                                                token: routing_token_apply.clone(),
                                                                profile: Some(profile),
                                                            },
                                                            status,
                                                            selected_token,
                                                            dashboard,
                                                            message,
                                                            error,
                                                            locale,
                                                        ),
                                                        Err(RoutingDraftError::InvalidNumber) => {
                                                            message.set(strings(locale.get_untracked()).invalid_routing_value.to_owned());
                                                            error.set(true);
                                                        }
                                                        Err(RoutingDraftError::ChoiceRequired) => {
                                                            message.set(strings(locale.get_untracked()).routing_choice_required.to_owned());
                                                            error.set(true);
                                                        }
                                                    }
                                                }
                                            >
                                                {move || strings(locale.get()).apply_routing_context}
                                            </button>
                                            <button
                                                type="button"
                                                on:click=move |_| {
                                                    submit_routing_context(
                                                        UpdateRoutingContextInput {
                                                            token: routing_token_clear.clone(),
                                                            profile: None,
                                                        },
                                                        status,
                                                        selected_token,
                                                        dashboard,
                                                        message,
                                                        error,
                                                        locale,
                                                    );
                                                    edit_routing_profile.set(RoutingProfileDraft::default());
                                                }
                                            >
                                                {move || strings(locale.get()).clear_routing_context}
                                            </button>
                                        </div>
                                        <div><strong>{move || format!("{}: ", strings(locale.get()).token)}</strong><code>{session_token}</code></div>
                                    </details>
                                    <button
                                        type="button"
                                        on:click=move |_| {
                                            selected_token.set(dashboard_token.clone());
                                            spawn_local({
                                                let token = dashboard_token.clone();
                                                async move {
                                                    if let Ok(current) = load_dashboard(token).await {
                                                        dashboard.set(Some(current));
                                                    }
                                                }
                                            });
                                        }
                                    >
                                        {move || strings(locale.get()).open_dashboard}
                                    </button>
                                    <button
                                        type="button"
                                        on:click=move |_| {
                                            let token = session.token.clone();
                                            spawn_local(async move {
                                                let result = invoke::<(), _>(
                                                    "open_session_window",
                                                    &serde_json::json!({
                                                        "input": SessionInput { token }
                                                    }),
                                                )
                                                .await;
                                                if let Err(problem) = result {
                                                    message.set(localize_problem(
                                                        locale.get_untracked(),
                                                        &problem,
                                                    ));
                                                    error.set(true);
                                                }
                                            });
                                        }
                                    >
                                        {move || strings(locale.get()).open_session_window}
                                    </button>
                                </li>
                            }
                        }
                    />
                </ul>
            </section>

            <div class="access-dialog manager-only" class:open=move || dialog.get() == Some(AccessDialog::Add)>
                <div class="access-box">
                    <div class="access-head">
                        <h2>{move || strings(locale.get()).connect_trip}</h2>
                        <button type="button" on:click=move |_| dialog.set(None)>
                            {move || strings(locale.get()).close}
                        </button>
                    </div>
                    <div class="warn" class:hidden=move || status.get().and_then(|current| current.gateway).is_none()>
                        <p>{move || strings(locale.get()).add_blocked}</p>
                        <button type="button" on:click=stop_gateway disabled=move || busy.get()>
                            {move || strings(locale.get()).disable_gateway}
                        </button>
                    </div>
                    <label class="choice" class:selected=move || !add_manual.get()>
                        <input type="radio" name="add-kind" prop:checked=move || !add_manual.get() on:change=move |_| add_manual.set(false) />
                        <span><strong>{move || strings(locale.get()).existing_trip}</strong></span>
                    </label>
                    <label class="choice" class:selected=move || add_manual.get()>
                        <input type="radio" name="add-kind" prop:checked=move || add_manual.get() on:change=move |_| add_manual.set(true) />
                        <span><strong>{move || strings(locale.get()).manual_trip}</strong></span>
                    </label>
                <div class:hidden=move || add_manual.get()>
                <div class="actions">
                    <button class="primary" type="button" on:click=open_package disabled=move || busy.get()>
                        {move || strings(locale.get()).open_package}
                    </button>
                </div>
                <p class="note">{move || strings(locale.get()).open_package_hint}</p>
                <div class="grid">
                    <label>
                        {move || strings(locale.get()).trip_folder}
                        <input
                            id="trip-folder"
                            type="text"
                            prop:value=move || folder.get()
                            on:input=move |event| folder.set(event_target_value(&event))
                        />
                        <span class="hint">{move || format!("{} {}", strings(locale.get()).example_prefix, r"D:\CargoDispatch\active\trip-01")}</span>
                    </label>
                </div>
                <details>
                    <summary>{move || strings(locale.get()).advanced}</summary>
                    <label>
                        {move || strings(locale.get()).session_token}
                        <input
                            id="session-token"
                            type="text"
                            minlength="16"
                            prop:value=move || token.get()
                            on:input=move |event| token.set(event_target_value(&event))
                        />
                    </label>
                </details>
                <div class="actions">
                    <button type="button" on:click=pick_folder disabled=move || busy.get()>
                        {move || strings(locale.get()).choose_folder}
                    </button>
                    <button class="primary" type="button" on:click=register disabled=move || busy.get()>
                        {move || strings(locale.get()).connect}
                    </button>
                </div>

                </div>

                <div class:hidden=move || !add_manual.get()>
                <p class="note">{move || strings(locale.get()).manual_trip_note}</p>
                <div class="grid">
                    <label>
                        {move || strings(locale.get()).trip_folder}
                        <input
                            type="text"
                            prop:value=move || manual_folder.get()
                            on:input=move |event| manual_folder.set(event_target_value(&event))
                        />
                        <span class="hint">{move || format!("{} {}", strings(locale.get()).example_prefix, r"D:\CargoDispatch\active\manual-trip")}</span>
                    </label>
                    <label>
                        {move || strings(locale.get()).session_display_name}
                        <input
                            type="text"
                            prop:value=move || manual_display_name.get()
                            on:input=move |event| manual_display_name.set(event_target_value(&event))
                        />
                    </label>
                </div>
                <details>
                    <summary>{move || strings(locale.get()).advanced}</summary>
                    <label>
                        {move || strings(locale.get()).session_token}
                        <input
                            type="text"
                            minlength="16"
                            prop:value=move || manual_token.get()
                            on:input=move |event| manual_token.set(event_target_value(&event))
                        />
                    </label>
                </details>
                <div class="actions">
                    <button type="button" on:click=pick_manual_folder disabled=move || busy.get()>
                        {move || strings(locale.get()).choose_folder}
                    </button>
                </div>
                <h4>{move || strings(locale.get()).manual_points}</h4>
                <For
                    each=move || manual_points.get()
                    key=|point| point.point_id.clone()
                    children=move |point| {
                        let remove_id = point.point_id.clone();
                        view! {
                            <div class="manual-point">
                                <div class="grid">
                                    <label>
                                        {move || strings(locale.get()).point_title}
                                        <input
                                            type="text"
                                            prop:value=move || point.title.get()
                                            on:input=move |event| point.title.set(event_target_value(&event))
                                        />
                                    </label>
                                    <label>
                                        {move || strings(locale.get()).latitude}
                                        <input
                                            type="number"
                                            step="0.000001"
                                            min="-90"
                                            max="90"
                                            prop:value=move || point.latitude.get()
                                            on:input=move |event| point.latitude.set(event_target_value(&event))
                                        />
                                    </label>
                                    <label>
                                        {move || strings(locale.get()).longitude}
                                        <input
                                            type="number"
                                            step="0.000001"
                                            min="-180"
                                            max="180"
                                            prop:value=move || point.longitude.get()
                                            on:input=move |event| point.longitude.set(event_target_value(&event))
                                        />
                                    </label>
                                    <label>
                                        {move || strings(locale.get()).address_optional}
                                        <input
                                            type="text"
                                            prop:value=move || point.address.get()
                                            on:input=move |event| point.address.set(event_target_value(&event))
                                        />
                                    </label>
                                </div>
                                <div class="actions">
                                    <button
                                        type="button"
                                        on:click=move |_| {
                                            manual_points.update(|rows| {
                                                rows.retain(|row| row.point_id != remove_id);
                                            });
                                        }
                                        disabled=move || busy.get()
                                    >
                                        {move || strings(locale.get()).remove_point}
                                    </button>
                                </div>
                            </div>
                        }
                    }
                />
                <div class="actions">
                    <button
                        type="button"
                        on:click=move |_| manual_points.update(|rows| rows.push(ManualPointDraft::new()))
                        disabled=move || busy.get()
                    >
                        {move || strings(locale.get()).add_point}
                    </button>
                    <button class="primary" type="button" on:click=create_manual disabled=move || busy.get()>
                        {move || strings(locale.get()).create_trip}
                    </button>
                </div>
                </div>
                </div>
            </div>


            <div class="access-dialog manager-only" class:open=move || dialog.get() == Some(AccessDialog::Connect)>
                <div class="access-box">
                    <div class="access-head">
                        <h2>{move || strings(locale.get()).broadcast}</h2>
                        <button type="button" on:click=move |_| dialog.set(None)>
                            {move || strings(locale.get()).close}
                        </button>
                    </div>
                    <label class="choice" class:selected=move || connect_choice.get() == ConnectChoice::Local>
                        <input type="radio" name="connect-choice" prop:checked=move || connect_choice.get() == ConnectChoice::Local on:change=move |_| connect_choice.set(ConnectChoice::Local) />
                        <span><strong>{move || strings(locale.get()).mode_local}</strong><span class="hint">{move || strings(locale.get()).mode_local_hint}</span></span>
                    </label>
                    <label class="choice" class:selected=move || connect_choice.get() == ConnectChoice::Lan>
                        <input type="radio" name="connect-choice" prop:checked=move || connect_choice.get() == ConnectChoice::Lan on:change=move |_| connect_choice.set(ConnectChoice::Lan) />
                        <span><strong>{move || strings(locale.get()).mode_lan}</strong><span class="hint">{move || strings(locale.get()).mode_lan_hint}</span></span>
                    </label>
                    <label class="choice" class:selected=move || connect_choice.get() == ConnectChoice::Internet>
                        <input type="radio" name="connect-choice" prop:checked=move || connect_choice.get() == ConnectChoice::Internet on:change=move |_| connect_choice.set(ConnectChoice::Internet) />
                        <span><strong>{move || strings(locale.get()).mode_internet}</strong><span class="hint">{move || strings(locale.get()).mode_internet_hint}</span></span>
                    </label>
                <p class="note" class:hidden=move || status.get().and_then(|current| current.listener).is_none() || connect_choice.get() == ConnectChoice::Internet>
                    {move || strings(locale.get()).listener_busy}
                </p>

                <div class:hidden=move || connect_choice.get() != ConnectChoice::Local>
                    <p class="note">{move || strings(locale.get()).no_fields}</p>
                    <div class="actions">
                        <button class="primary" type="button" on:click=start_loopback disabled=move || busy.get()>
                            {move || strings(locale.get()).local_loopback}
                        </button>
                    </div>
                </div>

                <div class:hidden=move || connect_choice.get() != ConnectChoice::Lan>
                    <div class="grid">
                        <label>
                            {move || strings(locale.get()).lan_address}
                            <input
                                id="lan-ip"
                                type="text"
                                prop:value=move || lan_ip.get()
                                on:input=move |event| lan_ip.set(event_target_value(&event))
                            />
                            <span class="hint">{move || {
                                let text = strings(locale.get());
                                if lan_candidates.get().is_empty() {
                                    format!("{} {} 192.168.1.25", text.lan_not_found, text.example_prefix)
                                } else {
                                    text.lan_found.to_owned()
                                }
                            }}</span>
                        </label>
                        <label>
                            {move || strings(locale.get()).port}
                            <input
                                id="lan-port"
                                type="number"
                                min="1"
                                max="65535"
                                prop:value=move || lan_port.get()
                                on:input=move |event| lan_port.set(event_target_value(&event))
                            />
                        </label>
                    </div>
                    <div class="actions" class:hidden=move || { lan_candidates.get().len() < 2 }>
                        <span class="hint">{move || strings(locale.get()).lan_other}</span>
                        {move || {
                            lan_candidates
                                .get()
                                .into_iter()
                                .map(|address| {
                                    let chosen = address.clone();
                                    let shown = address.clone();
                                    view! {
                                        <button
                                            type="button"
                                            class:primary=move || lan_ip.get() == shown
                                            on:click=move |_| lan_ip.set(chosen.clone())
                                        >
                                            {address}
                                        </button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                    <p class="note">{move || strings(locale.get()).lan_note}</p>
                    <div class="actions">
                        <button class="primary" type="button" on:click=start_lan disabled=move || busy.get()>
                            {move || strings(locale.get()).enable_lan}
                        </button>
                    </div>
                </div>

                <div class:hidden=move || connect_choice.get() != ConnectChoice::Internet>
                    <h3>{move || strings(locale.get()).public_gateway}</h3>
                    <p class="note">{move || strings(locale.get()).internet_order_note}</p>
                    <div class="grid">
                        <label>
                            {move || strings(locale.get()).gateway_address}
                            <input
                                id="gateway-url"
                                type="url"
                                prop:value=move || gateway_url.get()
                                on:input=move |event| gateway_url.set(event_target_value(&event))
                            />
                            <span class="hint">{move || format!("{} {}", strings(locale.get()).example_prefix, r"https://cargo.example.com")}</span>
                        </label>
                        <label>
                            {move || strings(locale.get()).control_secret}
                            <input
                                id="gateway-secret"
                                type="password"
                                minlength="32"
                                autocomplete="off"
                                prop:value=move || gateway_secret.get()
                                on:input=move |event| gateway_secret.set(event_target_value(&event))
                            />
                        </label>
                    </div>
                    <p class="note">{move || strings(locale.get()).gateway_note}</p>
                    <div class="actions">
                        <button class="primary" type="button" on:click=start_gateway disabled=move || busy.get()>
                            {move || strings(locale.get()).enable_gateway}
                        </button>
                    </div>
                </div>

                <div class="actions">
                    <button class="danger" type="button" class:hidden=move || status.get().and_then(|current| current.listener).is_none() on:click=stop disabled=move || busy.get()>
                        {move || strings(locale.get()).stop_broadcast}
                    </button>
                    <button type="button" class:hidden=move || status.get().and_then(|current| current.gateway).is_none() on:click=stop_gateway disabled=move || busy.get()>
                        {move || strings(locale.get()).disable_gateway}
                    </button>
                </div>
                <details>
                <summary>{move || strings(locale.get()).advanced}</summary>
                <div class="status">
                    {move || {
                        status.get().map(|current| {
                            let local = match current.listener {
                                Some(listener) => {
                                    let mode = match listener.mode {
                                        ListenerMode::Loopback => "loopback",
                                        ListenerMode::Lan => "LAN",
                                    };
                                    format!(
                                        "{}: {mode} · {}",
                                        strings(locale.get()).listener,
                                        listener.address
                                    )
                                }
                                None => strings(locale.get()).listener_off.to_owned(),
                            };
                            let public = match current.gateway {
                                Some(gateway) => format!(
                                    "{}: {} · {} {}/{}",
                                    strings(locale.get()).gateway,
                                    gateway.base_url,
                                    strings(locale.get()).active,
                                    gateway.active_sessions,
                                    gateway.total_sessions
                                ),
                                None => strings(locale.get()).gateway_off.to_owned(),
                            };
                            format!("{local} · {public}")
                        }).unwrap_or_else(|| strings(locale.get()).reading_state.to_owned())
                    }}
                </div>
                </details>
                </div>
            </div>

            <div class="access-dialog manager-only" class:open=move || dialog.get() == Some(AccessDialog::Road)>
                <div class="access-box">
                    <div class="access-head">
                        <h2>{move || strings(locale.get()).routing_engine}</h2>
                        <button type="button" on:click=move |_| dialog.set(None)>
                            {move || strings(locale.get()).close}
                        </button>
                    </div>
                <p>{move || strings(locale.get()).road_optional}</p>
                <div class="grid">
                    <label>
                        {move || strings(locale.get()).valhalla_address}
                        <input
                            id="routing-url"
                            type="url"
                            prop:value=move || routing_url.get()
                            on:input=move |event| routing_url.set(event_target_value(&event))
                        />
                        <span class="hint">{move || format!("{} {}", strings(locale.get()).example_prefix, r"http://127.0.0.1:8002")}</span>
                    </label>
                    <label>
                        {move || strings(locale.get()).map_dataset}
                        <input
                            id="routing-dataset"
                            type="text"
                            prop:value=move || routing_dataset.get()
                            on:input=move |event| routing_dataset.set(event_target_value(&event))
                        />
                        <span class="hint">{move || format!("{} {}", strings(locale.get()).example_prefix, r"europe-2026-11")}</span>
                    </label>
                </div>
                <details>
                    <summary>{move || strings(locale.get()).advanced}</summary>
                    <div class="grid">
                    <label>
                        {move || strings(locale.get()).connect_timeout_ms}
                        <input
                            id="routing-connect-timeout"
                            type="number"
                            min="100"
                            max="30000"
                            prop:value=move || routing_connect_timeout.get()
                            on:input=move |event| routing_connect_timeout.set(event_target_value(&event))
                        />
                    </label>
                    <label>
                        {move || strings(locale.get()).request_timeout_ms}
                        <input
                            id="routing-request-timeout"
                            type="number"
                            min="250"
                            max="120000"
                            prop:value=move || routing_request_timeout.get()
                            on:input=move |event| routing_request_timeout.set(event_target_value(&event))
                        />
                    </label>
                    </div>
                </details>
                <div class="actions">
                    <button class="primary" type="button" on:click=configure_routing disabled=move || busy.get()>
                        {move || strings(locale.get()).configure_routing}
                    </button>
                    <button type="button" on:click=test_routing disabled=move || busy.get()>
                        {move || strings(locale.get()).check_routing}
                    </button>
                    <button type="button" on:click=disable_routing disabled=move || busy.get()>
                        {move || strings(locale.get()).disable_routing}
                    </button>
                </div>
                <div class="status">
                    {move || {
                        status
                            .get()
                            .and_then(|current| current.routing)
                            .map(|routing| {
                                format!(
                                    "{} · {} · {} / {} ms",
                                    routing.base_url,
                                    routing.map_dataset_id,
                                    routing.connect_timeout_ms,
                                    routing.request_timeout_ms
                                )
                            })
                            .unwrap_or_else(|| {
                                strings(locale.get()).routing_not_configured.to_owned()
                            })
                    }}
                </div>
                <p class="note">{move || strings(locale.get()).routing_note}</p>
                </div>
            </div>


            <div class="access-dialog manager-only" class:open=move || dialog.get() == Some(AccessDialog::Help)>
                <div class="access-box">
                    <div class="access-head">
                        <h2>{move || strings(locale.get()).help_title}</h2>
                        <button type="button" on:click=move |_| dialog.set(None)>
                            {move || strings(locale.get()).close}
                        </button>
                    </div>
                    <p>{move || strings(locale.get()).help_local}</p>
                    <p>{move || strings(locale.get()).help_internet}</p>
                    <p>{move || strings(locale.get()).road_optional}</p>
                    <p>{move || strings(locale.get()).help_restart}</p>
                </div>
            </div>

            <section id="control-map-section">
                <h2>{move || strings(locale.get()).trip_map}</h2>
                <div id="control-map" class="trip-map" aria-label=move || strings(locale.get()).active_trip_map>
                    <div class="map-tiles" aria-hidden="true"></div>
                    <div class="map-overlays" aria-hidden="true"></div>
                    <div class="map-controls">
                        <button type="button" aria-label=move || strings(locale.get()).zoom_in on:click=move |_| cargo_control_map_zoom(1)> "+"</button>
                        <button type="button" aria-label=move || strings(locale.get()).zoom_out on:click=move |_| cargo_control_map_zoom(-1)> "−"</button>
                        <button type="button" on:click=move |_| cargo_control_map_fit()> {move || strings(locale.get()).fit_trip}</button>
                    </div>
                    <a
                        class="map-attribution"
                        href="https://www.openstreetmap.org/copyright"
                        target="_blank"
                        rel="noopener"
                    >
                        "© OpenStreetMap contributors"
                    </a>
                </div>
                <p class="note">{move || strings(locale.get()).map_note}</p>
            </section>

            <section>
                <h2>{move || strings(locale.get()).live_trip}</h2>
                <div class="actions">
                    <button type="button" on:click=refresh_dashboard>{move || strings(locale.get()).refresh}</button>
                </div>
                {move || dashboard.get().map(|trip| {
                    let revision = trip.local_route_revision;
                    let open_points = trip
                        .points
                        .iter()
                        .filter(|point| !point.completed)
                        .cloned()
                        .collect::<Vec<_>>();
                    let route_rows = trip.points.clone();
                    let documents = trip.documents.clone();
                    let document_points = trip.points.clone();
                    let location = trip.latest_location.clone();
                    let planning_basis = trip.planning_basis.clone();
                    let planning_basis_missing = planning_basis.is_none();
                    let actual_context = trip.actual_context.clone();
                    let actual_device = actual_context
                        .endpoint_label
                        .clone()
                        .or(actual_context.endpoint_id.clone());
                    let actual_vehicle = actual_context.vehicle_label.clone();
                    let actual_vehicle_type = actual_context.vehicle_type.clone();
                    let actual_routing_profile = actual_context.routing_profile.clone();
                    let trip_id = trip.trip_id.clone();
                    let version = trip.trip_version;

                    view! {
                        <div>
                            <p>
                                <strong>{move || format!("{}: ", strings(locale.get()).trip)}</strong><code>{trip_id}</code>
                                {move || format!(" · {} ", strings(locale.get()).version)} {version}
                                {move || format!(" · {} ", strings(locale.get()).route_revision)} {revision}
                            </p>

                            <div class="context-grid">
                                <div class="context-card">
                                    <h3>{move || strings(locale.get()).planning_basis}</h3>
                                    {planning_basis.map(|basis| {
                                        let vehicle_type = basis.vehicle_type;
                                        let cargos = basis.cargos;
                                        let estimate = basis.route_estimate;
                                        let distance = estimate.and_then(|plan| {
                                            plan.distance_m.map(|meters| (meters, plan.distance_typed))
                                        });
                                        let drive_minutes = estimate.and_then(|plan| plan.drive_minutes);
                                        view! {
                                            <div>
                                                <p>
                                                    <strong>{move || format!("{}: ", strings(locale.get()).planned_vehicle)}</strong>
                                                    {vehicle_type}
                                                </p>
                                                {distance.map(|(meters, typed)| {
                                                    let km = f64::from(meters) / 1000.0;
                                                    let value = if typed {
                                                        format!("{km:.1} km")
                                                    } else {
                                                        format!("≈ {km:.0} km")
                                                    };
                                                    view! {
                                                        <p class="planned-distance">
                                                            <strong>{move || format!("{}: ", strings(locale.get()).planned_distance)}</strong>
                                                            {value}
                                                            " "
                                                            {move || {
                                                                let texts = strings(locale.get());
                                                                if typed {
                                                                    texts.planned_distance_typed
                                                                } else {
                                                                    texts.planned_distance_straight
                                                                }
                                                            }}
                                                        </p>
                                                    }
                                                })}
                                                {drive_minutes.map(|minutes| view! {
                                                    <p class="planned-drive-time">
                                                        <strong>{move || format!("{}: ", strings(locale.get()).planned_drive_time)}</strong>
                                                        {format!("{}:{:02}", minutes / 60, minutes % 60)}
                                                    </p>
                                                })}
                                                <h4>{move || strings(locale.get()).planned_cargos}</h4>
                                                {cargos.is_empty().then(|| view! {
                                                    <p class="note">{move || strings(locale.get()).not_specified}</p>
                                                })}
                                                <For
                                                    each=move || cargos.clone()
                                                    key=|cargo| cargo.cargo_id.clone()
                                                    children=move |cargo| {
                                                        let weight = cargo.weight_kg;
                                                        let pallets = cargo.pallets;
                                                        view! {
                                                            <div class="context-item">
                                                                <strong>{cargo.title}</strong>
                                                                {weight.map(|value| view! {
                                                                    <span>
                                                                        " · "
                                                                        {value}
                                                                        " "
                                                                        {move || strings(locale.get()).weight_kg}
                                                                    </span>
                                                                })}
                                                                {pallets.map(|value| view! {
                                                                    <span>
                                                                        " · "
                                                                        {value}
                                                                        " "
                                                                        {move || strings(locale.get()).pallets}
                                                                    </span>
                                                                })}
                                                            </div>
                                                        }
                                                    }
                                                />
                                            </div>
                                        }
                                    })}
                                    {planning_basis_missing.then(|| view! {
                                        <p class="note">{move || strings(locale.get()).not_specified}</p>
                                    })}
                                </div>

                                <div class="context-card">
                                    <h3>{move || strings(locale.get()).actual_context}</h3>
                                    <p>
                                        <strong>{move || format!("{}: ", strings(locale.get()).endpoint_label)}</strong>
                                        {actual_device.unwrap_or_else(|| "—".to_owned())}
                                    </p>
                                    <p>
                                        <strong>{move || format!("{}: ", strings(locale.get()).vehicle_label)}</strong>
                                        {actual_vehicle.unwrap_or_else(|| "—".to_owned())}
                                    </p>
                                    <p>
                                        <strong>{move || format!("{}: ", strings(locale.get()).vehicle_type)}</strong>
                                        {actual_vehicle_type.unwrap_or_else(|| "—".to_owned())}
                                    </p>
                                    <h4>{move || strings(locale.get()).routing_context}</h4>
                                    {match actual_routing_profile {
                                        Some(profile) => view! {
                                            <p class="note">
                                                {move || routing_profile_summary(&profile, locale.get())}
                                            </p>
                                        }.into_any(),
                                        None => view! {
                                            <p class="note">{move || strings(locale.get()).not_specified}</p>
                                        }.into_any(),
                                    }}
                                </div>
                            </div>

                            <h3>{move || strings(locale.get()).route}</h3>
                            <div>
                                <For
                                    each=move || route_rows.clone()
                                    key=|point| point.point_id.clone()
                                    children=move |point| {
                                        let point_id_up = point.point_id.clone();
                                        let point_id_down = point.point_id.clone();
                                        let open_up = open_points.clone();
                                        let open_down = open_points.clone();
                                        let navigation = point.navigation.clone();
                                        let completed = point.completed;
                                        view! {
                                            <div class="point">
                                                <strong>{format!("{}. {}", point.sequence, point.title)}</strong>
                                                <span>{move || {
                                                    if completed {
                                                        format!(" — {}", strings(locale.get()).completed)
                                                    } else {
                                                        format!(" — {}", strings(locale.get()).open_status)
                                                    }
                                                }}</span>
                                                {navigation.map(|nav| view! {
                                                    <div class="note">
                                                        {nav.address.unwrap_or_default()}
                                                        " · "
                                                        {format!("{:.6}, {:.6}", nav.latitude, nav.longitude)}
                                                    </div>
                                                })}
                                                {(!completed).then(|| view! {
                                                    <span class="actions">
                                                        <button
                                                            type="button"
                                                            on:click=move |_| {
                                                                if let Some(ids) = reordered_open_point_ids(
                                                                    &open_up,
                                                                    &point_id_up,
                                                                    -1,
                                                                ) {
                                                                    submit_reorder(
                                                                        ReorderRouteInput {
                                                                            token: selected_token.get_untracked(),
                                                                            based_on_revision: revision,
                                                                            open_point_ids: ids,
                                                                        },
                                                                        dashboard,
                                                                        message,
                                                                        error,
                                                                        locale,
                                                                    );
                                                                }
                                                            }
                                                        >
                                                            "↑"
                                                        </button>
                                                        <button
                                                            type="button"
                                                            on:click=move |_| {
                                                                if let Some(ids) = reordered_open_point_ids(
                                                                    &open_down,
                                                                    &point_id_down,
                                                                    1,
                                                                ) {
                                                                    submit_reorder(
                                                                        ReorderRouteInput {
                                                                            token: selected_token.get_untracked(),
                                                                            based_on_revision: revision,
                                                                            open_point_ids: ids,
                                                                        },
                                                                        dashboard,
                                                                        message,
                                                                        error,
                                                                        locale,
                                                                    );
                                                                }
                                                            }
                                                        >
                                                            "↓"
                                                        </button>
                                                    </span>
                                                })}
                                            </div>
                                        }
                                    }
                                />
                            </div>

                            <h3>{move || strings(locale.get()).latest_driver_position}</h3>
                            {match location {
                                Some(location) => view! {
                                    <p>
                                        {format!("{:.6}, {:.6}", location.latitude, location.longitude)}
                                        {location.accuracy_m.map(|accuracy| {
                                            format!(" · ±{accuracy:.1} {}", strings(locale.get()).meter_unit)
                                        }).unwrap_or_default()}
                                        " · "
                                        {format!(
                                            "{} {}",
                                            strings(locale.get()).timestamp,
                                            local_time(locale.get(), location.observed_at_unix_ms),
                                        )}
                                    </p>
                                }.into_any(),
                                None => view! { <p class="note">{move || strings(locale.get()).no_position}</p> }.into_any(),
                            }}

                            <h3>{move || strings(locale.get()).documents_media}</h3>
                            <ul>
                                <For
                                    each=move || documents.clone()
                                    key=|document| document.document_id.clone()
                                    children=move |document| {
                                        let document_id = document.document_id.clone();
                                        let direction = match document.direction {
                                            DashboardDocumentDirection::FromOperator => strings(locale.get_untracked()).from_operator,
                                            DashboardDocumentDirection::FromDriver => strings(locale.get_untracked()).from_driver,
                                        };
                                        let size = document.size;
                                        let attached = document
                                            .point_id
                                            .as_deref()
                                            .map(|id| point_label(&document_points, id));
                                        view! {
                                            <li>
                                                <strong>{document.original_name}</strong>
                                                " · " {direction}
                                                " · " {document.mime_type}
                                                " · " {move || format_file_size(locale.get().code(), size)}
                                                {attached.map(|label| view! {
                                                    <span>{move || format!(" · {} {label}", strings(locale.get()).point)}</span>
                                                })}
                                                <button
                                                    type="button"
                                                    disabled=move || busy.get()
                                                    on:click=move |_| {
                                                        let input = SaveDocumentInput {
                                                            token: selected_token.get_untracked(),
                                                            document_id: document_id.clone(),
                                                        };
                                                        busy.set(true);
                                                        spawn_local(async move {
                                                            match invoke::<Option<String>, _>(
                                                                "save_session_document",
                                                                &serde_json::json!({ "input": input }),
                                                            )
                                                            .await
                                                            {
                                                                Ok(Some(path)) => {
                                                                    message.set(format!(
                                                                        "{}: {path}",
                                                                        strings(locale.get_untracked()).file_saved
                                                                    ));
                                                                    error.set(false);
                                                                }
                                                                Ok(None) => {}
                                                                Err(problem) => {
                                                                    message.set(localize_problem(locale.get_untracked(), &problem));
                                                                    error.set(true);
                                                                }
                                                            }
                                                            busy.set(false);
                                                        });
                                                    }
                                                >
                                                    {move || strings(locale.get()).save}
                                                </button>
                                            </li>
                                        }
                                    }
                                />
                            </ul>

                            <h3>{move || strings(locale.get()).send_file}</h3>
                            <div class="actions">
                                <button type="button" on:click=pick_file disabled=move || busy.get()>
                                    {move || strings(locale.get()).choose_file}
                                </button>
                                <span>
                                    {move || picked_file.get()
                                        .map(|file| file.original_name)
                                        .unwrap_or_else(|| strings(locale.get()).no_file.to_owned())}
                                </span>
                            </div>
                            <label>
                                {move || strings(locale.get()).attach_point}
                                <select
                                    on:change=move |event| attachment_point.set(event_target_value(&event))
                                >
                                    <option value="">{move || strings(locale.get()).whole_trip}</option>
                                    <For
                                        each=move || trip.points.clone()
                                        key=|point| point.point_id.clone()
                                        children=move |point| view! {
                                            <option value=point.point_id>
                                                {format!("{}. {}", point.sequence, point.title)}
                                            </option>
                                        }
                                    />
                                </select>
                            </label>
                            <div class="actions">
                                <button class="primary" type="button" on:click=send_file disabled=move || busy.get()>
                                    {move || strings(locale.get()).send_to_driver}
                                </button>
                            </div>
                        </div>
                    }
                })}
            </section>

            <p
                class=move || if error.get() { "error" } else { "ok" }
                role="status"
                aria-live="polite"
                hidden=move || message.get().is_empty()
            >
                {move || message.get()}
            </p>
        </main>
    }
}
