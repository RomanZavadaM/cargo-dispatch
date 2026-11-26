#![forbid(unsafe_code)]

mod i18n;

// UI language is selected at runtime and persisted in the desktop webview.
// The same eight-language set is enforced by .github/scripts/check-localizations.py.
use crate::i18n::{UiLocale, localize_problem, strings};
use cargo_dispatch_control_ui_contracts::{
    ConfigureRoutingInput, ControlUiStatus, DashboardDocumentDirection, DashboardPoint,
    ListenerMode, OperatorDocumentInput, PickedFile, RegisterTripInput, ReorderRouteInput,
    RoutingAvailabilityState, RoutingProbeResult, SaveDocumentInput, SessionInput,
    StartGatewayInput, StartLanInput, TripDashboard, UpdateSessionMetadataInput,
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

    #[wasm_bindgen(js_name = cargoControlInitialLocale)]
    fn cargo_control_initial_locale() -> String;

    #[wasm_bindgen(js_name = cargoControlSetLocale)]
    fn cargo_control_set_locale(locale: &str);
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

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(ControlApp);
}

#[component]
fn ControlApp() -> impl IntoView {
    let locale = RwSignal::new(UiLocale::parse(&cargo_control_initial_locale()));
    let folder = RwSignal::new(String::new());
    let token = RwSignal::new(String::new());
    let lan_ip = RwSignal::new(String::new());
    let lan_port = RwSignal::new("8080".to_owned());
    let gateway_url = RwSignal::new(String::new());
    let gateway_secret = RwSignal::new(String::new());
    let routing_url = RwSignal::new(String::new());
    let routing_dataset = RwSignal::new(String::new());
    let routing_connect_timeout = RwSignal::new("1500".to_owned());
    let routing_request_timeout = RwSignal::new("10000".to_owned());
    let status = RwSignal::new(None::<ControlUiStatus>);
    let selected_token = RwSignal::new(String::new());
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
            let result = invoke("register_trip", &serde_json::json!({ "input": input })).await;
            set_status_result(result, strings(locale.get_untracked()).trip_registered);
        });
    };

    let start_loopback = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let result = invoke("start_loopback", &serde_json::json!({})).await;
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
        busy.set(true);
        let input = StartLanInput {
            ip: lan_ip.get_untracked(),
            port,
        };
        spawn_local(async move {
            let result = invoke("start_lan", &serde_json::json!({ "input": input })).await;
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
            let result = invoke("start_gateway", &serde_json::json!({ "input": input })).await;
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

            <section>
                <h2>{move || strings(locale.get()).connect_trip}</h2>
                <div class="grid">
                    <label>
                        {move || strings(locale.get()).trip_folder}
                        <input
                            id="trip-folder"
                            type="text"
                            placeholder=r"D:\CargoDispatch\active\trip-..."
                            prop:value=move || folder.get()
                            on:input=move |event| folder.set(event_target_value(&event))
                        />
                    </label>
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
                </div>
                <div class="actions">
                    <button type="button" on:click=pick_folder disabled=move || busy.get()>
                        {move || strings(locale.get()).choose_folder}
                    </button>
                    <button class="primary" type="button" on:click=register disabled=move || busy.get()>
                        {move || strings(locale.get()).connect}
                    </button>
                </div>
            </section>

            <section>
                <h2>{move || strings(locale.get()).broadcast}</h2>
                <div class="actions">
                    <button type="button" on:click=start_loopback disabled=move || busy.get()>
                        {move || strings(locale.get()).local_loopback}
                    </button>
                </div>
                <div class="grid">
                    <label>
                        {move || strings(locale.get()).lan_address}
                        <input
                            id="lan-ip"
                            type="text"
                            placeholder="192.168.1.25"
                            prop:value=move || lan_ip.get()
                            on:input=move |event| lan_ip.set(event_target_value(&event))
                        />
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
                <div class="actions">
                    <button class="primary" type="button" on:click=start_lan disabled=move || busy.get()>
                        {move || strings(locale.get()).enable_lan}
                    </button>
                    <button class="danger" type="button" on:click=stop disabled=move || busy.get()>
                        {move || strings(locale.get()).stop_broadcast}
                    </button>
                </div>
                <p class="note">{move || strings(locale.get()).lan_note}</p>
                <h3>{move || strings(locale.get()).public_gateway}</h3>
                <div class="grid">
                    <label>
                        {move || strings(locale.get()).gateway_address}
                        <input
                            id="gateway-url"
                            type="url"
                            placeholder="https://cargo.example.com"
                            prop:value=move || gateway_url.get()
                            on:input=move |event| gateway_url.set(event_target_value(&event))
                        />
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
                <div class="actions">
                    <button class="primary" type="button" on:click=start_gateway disabled=move || busy.get()>
                        {move || strings(locale.get()).enable_gateway}
                    </button>
                    <button type="button" on:click=stop_gateway disabled=move || busy.get()>
                        {move || strings(locale.get()).disable_gateway}
                    </button>
                </div>
                <p class="note">{move || strings(locale.get()).gateway_note}</p>
                <h3>{move || strings(locale.get()).routing_engine}</h3>
                <div class="grid">
                    <label>
                        {move || strings(locale.get()).valhalla_address}
                        <input
                            id="routing-url"
                            type="url"
                            placeholder="http://127.0.0.1:8002"
                            prop:value=move || routing_url.get()
                            on:input=move |event| routing_url.set(event_target_value(&event))
                        />
                    </label>
                    <label>
                        {move || strings(locale.get()).map_dataset}
                        <input
                            id="routing-dataset"
                            type="text"
                            placeholder="europe-2026-11"
                            prop:value=move || routing_dataset.get()
                            on:input=move |event| routing_dataset.set(event_target_value(&event))
                        />
                    </label>
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
            </section>

            <section>
                <h2>{move || strings(locale.get()).browser_sessions}</h2>
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
                <ul>
                    <For
                        each=move || status.get().map(|current| current.sessions).unwrap_or_default()
                        key=|session| session.token.clone()
                        children=move |session| {
                            let trip = session.trip_id.clone();
                            let session_token = session.token.clone();
                            let dashboard_token = session.token.clone();
                            let local_url = session.driver_url.clone();
                            let public_url = session.public_driver_url.clone();
                            let display_name = session.display_name.clone();
                            let endpoint_label = session.endpoint_label.clone();
                            let vehicle_label = session.vehicle_label.clone();
                            let vehicle_type = session.vehicle_type.clone();
                            let edit_display_name = RwSignal::new(session.display_name.clone());
                            let edit_endpoint_id =
                                RwSignal::new(session.endpoint_id.clone().unwrap_or_default());
                            let edit_endpoint_label =
                                RwSignal::new(session.endpoint_label.clone().unwrap_or_default());
                            let edit_vehicle_label =
                                RwSignal::new(session.vehicle_label.clone().unwrap_or_default());
                            let edit_vehicle_type =
                                RwSignal::new(session.vehicle_type.clone().unwrap_or_default());
                            let edit_token = session.token.clone();
                            view! {
                                <li>
                                    <div><strong>{display_name}</strong></div>
                                    {vehicle_label.map(|value| view! {
                                        <div><strong>{move || format!("{}: ", strings(locale.get()).vehicle_label)}</strong>{value}</div>
                                    })}
                                    {vehicle_type.map(|value| view! {
                                        <div><strong>{move || format!("{}: ", strings(locale.get()).vehicle_type)}</strong>{value}</div>
                                    })}
                                    {endpoint_label.map(|value| view! {
                                        <div><strong>{move || format!("{}: ", strings(locale.get()).endpoint_label)}</strong>{value}</div>
                                    })}
                                    <div><strong>{move || format!("{}: ", strings(locale.get()).trip)}</strong><code>{trip}</code></div>
                                    {public_url.map(|driver_url| {
                                        let href = driver_url.clone();
                                        let copy_url = driver_url.clone();
                                        view! {
                                            <div>
                                                <strong>{move || format!("{}: ", strings(locale.get()).public_driver_url)}</strong>
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
                                    {local_url.map(|driver_url| {
                                        let href = driver_url.clone();
                                        let copy_url = driver_url.clone();
                                        view! {
                                            <div>
                                                <strong>{move || format!("{}: ", strings(locale.get()).local_driver_url)}</strong>
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
                                </li>
                            }
                        }
                    />
                </ul>
            </section>

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
                    let location = trip.latest_location.clone();
                    let trip_id = trip.trip_id.clone();
                    let version = trip.trip_version;

                    view! {
                        <div>
                            <p>
                                <strong>{move || format!("{}: ", strings(locale.get()).trip)}</strong><code>{trip_id}</code>
                                {move || format!(" · {} ", strings(locale.get()).version)} {version}
                                {move || format!(" · {} ", strings(locale.get()).route_revision)} {revision}
                            </p>

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
                                        {format!("{} {}", strings(locale.get()).timestamp, location.observed_at_unix_ms)}
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
                                        view! {
                                            <li>
                                                <strong>{document.original_name}</strong>
                                                " · " {direction}
                                                " · " {document.mime_type}
                                                " · " {document.size} " " {move || strings(locale.get()).bytes}
                                                {document.point_id.map(|point| view! {
                                                    <span>{move || format!(" · {} ", strings(locale.get()).point)} <code>{point}</code></span>
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
            >
                {move || message.get()}
            </p>
        </main>
    }
}
