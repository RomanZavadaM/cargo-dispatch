#![forbid(unsafe_code)]

mod i18n;
mod orders;
mod orders_i18n;
mod package_draft;
mod route_check;

use crate::i18n::{UiLocale, strings};
use crate::orders::OrdersPanel;
use crate::route_check::{RouteCheckPanel, package_figures};
use cargo_dispatch_planner_ui_contracts::{
    FiguresDraft, PlannerPackageCargoInput, PlannerPackageExportInput, PlannerPackageExportResult,
    PlannerPointInput, PlannerRouteView, PlannerRoutingAvailabilityState,
    PlannerRoutingConfigInput, PlannerRoutingPreviewInput, PlannerRoutingPreviewState,
    PlannerRoutingPreviewView, PlannerRoutingProbeResult, PlannerRoutingProfileInput,
    PlannerRoutingStatus, PlannerTollPreference, PointActionInput, ReorderPointsInput, VehicleType,
    is_supported_max_speed_kmh, localize_problem,
};
use leptos::{prelude::*, task::spawn_local};
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(inline_js = r#"
export function cargoPlannerInvoke(command, argsJson) {
  const args = JSON.parse(argsJson);
  return window.__TAURI__.core.invoke(command, args).then(result => JSON.stringify(result));
}
export function cargoPlannerInitialLocale() {
  const saved = localStorage.getItem("cargo-dispatch-planner-locale");
  const locale = saved || navigator.language || "en";
  document.documentElement.lang = locale;
  return locale;
}
export function cargoPlannerSetLocale(locale) {
  localStorage.setItem("cargo-dispatch-planner-locale", locale);
  document.documentElement.lang = locale;
}

let plannerMapState = {
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
  geometry: [],
  routeId: null,
  pointSetKey: "",
  geometryKey: ""
};

function clampPlannerLat(latitude) {
  return Math.max(-85.05112878, Math.min(85.05112878, latitude));
}

function plannerWorldSize(zoom) {
  return 256 * Math.pow(2, zoom);
}

function plannerProject(latitude, longitude, zoom) {
  const size = plannerWorldSize(zoom);
  const lat = clampPlannerLat(latitude) * Math.PI / 180;
  const sin = Math.sin(lat);
  return {
    x: (longitude + 180) / 360 * size,
    y: (0.5 - Math.log((1 + sin) / (1 - sin)) / (4 * Math.PI)) * size
  };
}

function plannerUnproject(x, y, zoom) {
  const size = plannerWorldSize(zoom);
  const longitude = x / size * 360 - 180;
  const mercator = Math.PI * (1 - 2 * y / size);
  const latitude = Math.atan(Math.sinh(mercator)) * 180 / Math.PI;
  return {latitude: clampPlannerLat(latitude), longitude};
}

function plannerMapHost() {
  const host = document.getElementById("planner-map");
  if (!host || plannerMapState.bound) return host;
  host.addEventListener("wheel", event => {
    event.preventDefault();
    cargoPlannerMapZoom(event.deltaY < 0 ? 1 : -1);
  }, {passive: false});
  host.addEventListener("pointerdown", event => {
    if (event.target.closest("button,a")) return;
    const center = plannerProject(
      plannerMapState.centerLat,
      plannerMapState.centerLon,
      plannerMapState.zoom
    );
    plannerMapState.dragging = true;
    plannerMapState.dragStartX = event.clientX;
    plannerMapState.dragStartY = event.clientY;
    plannerMapState.dragCenterX = center.x;
    plannerMapState.dragCenterY = center.y;
    host.setPointerCapture(event.pointerId);
  });
  host.addEventListener("pointermove", event => {
    if (!plannerMapState.dragging) return;
    const x = plannerMapState.dragCenterX - (event.clientX - plannerMapState.dragStartX);
    const y = plannerMapState.dragCenterY - (event.clientY - plannerMapState.dragStartY);
    const center = plannerUnproject(x, y, plannerMapState.zoom);
    plannerMapState.centerLat = center.latitude;
    plannerMapState.centerLon = center.longitude;
    plannerMapRender();
  });
  host.addEventListener("pointerup", event => {
    plannerMapState.dragging = false;
    if (host.hasPointerCapture(event.pointerId)) host.releasePointerCapture(event.pointerId);
  });
  host.addEventListener("pointercancel", () => {
    plannerMapState.dragging = false;
  });
  window.addEventListener("resize", plannerMapRender);
  plannerMapState.bound = true;
  return host;
}

function plannerMapRender() {
  const host = plannerMapHost();
  if (!host) return;
  const width = Math.max(1, host.clientWidth);
  const height = Math.max(1, host.clientHeight);
  const zoom = plannerMapState.zoom;
  const center = plannerProject(plannerMapState.centerLat, plannerMapState.centerLon, zoom);
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

  if (plannerMapState.geometry.length > 1) {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("class", "map-route");
    svg.setAttribute("viewBox", "0 0 " + width + " " + height);
    svg.setAttribute("preserveAspectRatio", "none");
    const line = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
    line.setAttribute(
      "points",
      plannerMapState.geometry
        .map(point => {
          const shown = plannerProject(point.latitude, point.longitude, zoom);
          return (shown.x - left) + "," + (shown.y - top);
        })
        .join(" ")
    );
    svg.append(line);
    overlayLayer.append(svg);
  }

  for (const point of plannerMapState.points) {
    const shown = plannerProject(point.latitude, point.longitude, zoom);
    const marker = document.createElement("div");
    marker.className = "map-marker";
    marker.style.left = (shown.x - left) + "px";
    marker.style.top = (shown.y - top) + "px";
    marker.title = point.title;
    marker.textContent = String(point.sequence);
    overlayLayer.append(marker);
  }
}

function plannerMapFit() {
  const host = plannerMapHost();
  const points = plannerMapState.points;
  const fitPoints = plannerMapState.geometry.length > 0
    ? [...points, ...plannerMapState.geometry]
    : points;
  if (!host || fitPoints.length === 0) {
    plannerMapState.zoom = 6;
    plannerMapState.centerLat = 49.0;
    plannerMapState.centerLon = 31.0;
    plannerMapState.initialized = true;
    plannerMapRender();
    return;
  }

  const width = Math.max(1, host.clientWidth - 72);
  const height = Math.max(1, host.clientHeight - 72);
  let chosenZoom = 3;
  let center = {latitude: fitPoints[0].latitude, longitude: fitPoints[0].longitude};

  if (fitPoints.length === 1) {
    chosenZoom = 14;
  } else {
    for (let zoom = 18; zoom >= 3; zoom--) {
      const shown = fitPoints.map(point => plannerProject(point.latitude, point.longitude, zoom));
      const xs = shown.map(item => item.x);
      const ys = shown.map(item => item.y);
      const minX = Math.min(...xs);
      const maxX = Math.max(...xs);
      const minY = Math.min(...ys);
      const maxY = Math.max(...ys);
      if (maxX - minX <= width && maxY - minY <= height) {
        chosenZoom = zoom;
        center = plannerUnproject((minX + maxX) / 2, (minY + maxY) / 2, zoom);
        break;
      }
    }
  }

  plannerMapState.zoom = chosenZoom;
  plannerMapState.centerLat = center.latitude;
  plannerMapState.centerLon = center.longitude;
  plannerMapState.initialized = true;
  plannerMapRender();
}

export function cargoPlannerMapRender(routeId, pointsJson, geometryJson) {
  const points = JSON.parse(pointsJson);
  const geometry = JSON.parse(geometryJson);
  const pointSetKey = points.map(point => point.point_id).sort().join("|");
  const geometryKey = geometryJson;
  const shouldFit =
    !plannerMapState.initialized ||
    plannerMapState.routeId !== routeId ||
    plannerMapState.pointSetKey !== pointSetKey ||
    plannerMapState.geometryKey !== geometryKey;

  plannerMapState.routeId = routeId;
  plannerMapState.pointSetKey = pointSetKey;
  plannerMapState.geometryKey = geometryKey;
  plannerMapState.points = points;
  plannerMapState.geometry = geometry;

  if (!plannerMapHost()) {
    window.setTimeout(() => {
      if (shouldFit) plannerMapFit();
      else plannerMapRender();
    }, 0);
    return;
  }
  if (shouldFit) plannerMapFit();
  else plannerMapRender();
}

export function cargoPlannerMapZoom(delta) {
  plannerMapState.zoom = Math.max(3, Math.min(18, plannerMapState.zoom + delta));
  plannerMapState.initialized = true;
  plannerMapRender();
}

export function cargoPlannerMapFit() {
  plannerMapFit();
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = cargoPlannerInvoke)]
    fn cargo_planner_invoke(command: &str, args_json: &str) -> js_sys::Promise;
    #[wasm_bindgen(js_name = cargoPlannerInitialLocale)]
    fn cargo_planner_initial_locale() -> String;
    #[wasm_bindgen(js_name = cargoPlannerSetLocale)]
    fn cargo_planner_set_locale(locale: &str);
    #[wasm_bindgen(js_name = cargoPlannerMapRender)]
    fn cargo_planner_map_render(route_id: &str, points_json: &str, geometry_json: &str);
    #[wasm_bindgen(js_name = cargoPlannerMapZoom)]
    fn cargo_planner_map_zoom(delta: i32);
    #[wasm_bindgen(js_name = cargoPlannerMapFit)]
    fn cargo_planner_map_fit();
}

fn js_error(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

async fn invoke<T: DeserializeOwned, A: Serialize>(command: &str, args: &A) -> Result<T, String> {
    let args_json =
        serde_json::to_string(args).map_err(|error| format!("command encode failed: {error}"))?;
    let value = JsFuture::from(cargo_planner_invoke(command, &args_json))
        .await
        .map_err(js_error)?;
    let json = value
        .as_string()
        .ok_or_else(|| "Planner returned an unexpected response".to_owned())?;
    serde_json::from_str(&json).map_err(|error| format!("response decode failed: {error}"))
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
    fn optional_positive_f64(value: &str) -> Result<Option<f64>, RoutingDraftError> {
        let value = value.trim();
        if value.is_empty() {
            return Ok(None);
        }
        let parsed = value
            .replace(',', ".")
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

    fn is_empty(&self) -> bool {
        [
            self.height_m.as_str(),
            self.width_m.as_str(),
            self.length_m.as_str(),
            self.gross_weight_t.as_str(),
            self.axle_load_t.as_str(),
            self.axle_count.as_str(),
            self.hazmat.as_str(),
            self.hgv.as_str(),
            self.exclude_unpaved.as_str(),
            self.prefer_truck_routes.as_str(),
            self.toll_preference.as_str(),
            self.max_speed_kmh.as_str(),
        ]
        .iter()
        .all(|value| value.trim().is_empty())
    }

    fn input(&self) -> Result<PlannerRoutingProfileInput, RoutingDraftError> {
        let toll_preference = match self.toll_preference.as_str() {
            "allow" => PlannerTollPreference::Allow,
            "prefer_avoid" => PlannerTollPreference::PreferAvoid,
            "avoid" => PlannerTollPreference::Avoid,
            _ => return Err(RoutingDraftError::ChoiceRequired),
        };
        Ok(PlannerRoutingProfileInput {
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

#[derive(Debug, Clone)]
struct CargoDraft {
    row_id: u64,
    title: String,
    weight_kg: String,
    pallets: String,
    pickup_point_id: String,
    unload_point_id: String,
}

impl CargoDraft {
    fn new(row_id: u64) -> Self {
        Self {
            row_id,
            title: String::new(),
            weight_kg: String::new(),
            pallets: String::new(),
            pickup_point_id: String::new(),
            unload_point_id: String::new(),
        }
    }

    fn optional_positive_u32(value: &str) -> Result<Option<u32>, ()> {
        let value = value.trim();
        if value.is_empty() {
            return Ok(None);
        }
        let parsed = value.parse::<u32>().map_err(|_| ())?;
        if parsed == 0 {
            return Err(());
        }
        Ok(Some(parsed))
    }

    fn optional_positive_u16(value: &str) -> Result<Option<u16>, ()> {
        let value = value.trim();
        if value.is_empty() {
            return Ok(None);
        }
        let parsed = value.parse::<u16>().map_err(|_| ())?;
        if parsed == 0 {
            return Err(());
        }
        Ok(Some(parsed))
    }

    fn input(&self) -> Result<Option<PlannerPackageCargoInput>, ()> {
        let is_empty = self.title.trim().is_empty()
            && self.weight_kg.trim().is_empty()
            && self.pallets.trim().is_empty()
            && self.pickup_point_id.is_empty()
            && self.unload_point_id.is_empty();
        if is_empty {
            return Ok(None);
        }
        let title = self.title.trim();
        if title.is_empty() {
            return Err(());
        }
        Ok(Some(PlannerPackageCargoInput {
            row_id: self.row_id,
            title: title.to_owned(),
            weight_kg: Self::optional_positive_u32(&self.weight_kg)?,
            pallets: Self::optional_positive_u16(&self.pallets)?,
            pickup_point_id: (!self.pickup_point_id.is_empty())
                .then(|| self.pickup_point_id.clone()),
            unload_point_id: (!self.unload_point_id.is_empty())
                .then(|| self.unload_point_id.clone()),
        }))
    }
}

/// Each option carries the selection, so a saved language shows from the start.
const LANGUAGES: [(&str, &str); 8] = [
    ("uk", "Українська"),
    ("en", "English"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("es", "Español"),
    ("ko", "한국어"),
    ("ja", "日本語"),
    ("zh-Hans", "简体中文"),
];

/// One text of a cargo row, followed as the row changes.
fn cargo_value(
    rows: RwSignal<Vec<CargoDraft>>,
    row_id: u64,
    pick: fn(&CargoDraft) -> &String,
) -> impl Fn() -> String + Copy {
    move || {
        rows.with(|rows| {
            rows.iter()
                .find(|row| row.row_id == row_id)
                .map(|row| pick(row).clone())
                .unwrap_or_default()
        })
    }
}

fn probe_message(state: PlannerRoutingAvailabilityState, locale: UiLocale) -> &'static str {
    let text = strings(locale);
    match state {
        PlannerRoutingAvailabilityState::NotConfigured => text.routing_not_configured,
        PlannerRoutingAvailabilityState::Ok => text.routing_available,
        PlannerRoutingAvailabilityState::Timeout => text.routing_timeout,
        PlannerRoutingAvailabilityState::EngineUnavailable => text.routing_unavailable,
        PlannerRoutingAvailabilityState::DatasetMissing => text.routing_dataset_missing,
        PlannerRoutingAvailabilityState::EngineError => text.routing_error,
    }
}

fn preview_message(state: PlannerRoutingPreviewState, locale: UiLocale) -> &'static str {
    let text = strings(locale);
    match state {
        PlannerRoutingPreviewState::Ok => text.preview_updated,
        PlannerRoutingPreviewState::NotConfigured => text.routing_not_configured,
        PlannerRoutingPreviewState::NoRoute => text.preview_no_route,
        PlannerRoutingPreviewState::Timeout => text.preview_timeout,
        PlannerRoutingPreviewState::EngineUnavailable => text.preview_unavailable,
        PlannerRoutingPreviewState::DatasetMissing => text.preview_dataset_missing,
        PlannerRoutingPreviewState::EngineError => text.preview_error,
        PlannerRoutingPreviewState::StaleDraft => text.preview_stale,
    }
}

fn warning_label(code: &str, locale: UiLocale) -> String {
    let text = strings(locale);
    match code {
        "missing_height_m" => text.height_m,
        "missing_width_m" => text.width_m,
        "missing_length_m" => text.length_m,
        "missing_gross_weight_t" => text.gross_weight_t,
        "missing_axle_load_t" => text.axle_load_t,
        "missing_axle_count" => text.axle_count,
        _ => code,
    }
    .to_owned()
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(PlannerApp);
}

#[component]
fn PlannerApp() -> impl IntoView {
    let locale = RwSignal::new(UiLocale::parse(&cargo_planner_initial_locale()));
    let route = RwSignal::new(None::<PlannerRouteView>);
    let title = RwSignal::new(String::new());
    let latitude = RwSignal::new(String::new());
    let longitude = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());
    let error = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let routing_status = RwSignal::new(None::<PlannerRoutingStatus>);
    let preview = RwSignal::new(None::<PlannerRoutingPreviewView>);
    let routing_url = RwSignal::new(String::new());
    let routing_dataset = RwSignal::new(String::new());
    let connect_timeout_ms = RwSignal::new("1000".to_owned());
    let request_timeout_ms = RwSignal::new("10000".to_owned());
    let routing_profile = RwSignal::new(RoutingProfileDraft::default());
    let export_vehicle_type = RwSignal::new(String::new());
    let vehicle_types = RwSignal::new(Vec::<VehicleType>::new());
    let figures = RwSignal::new(FiguresDraft::default());
    let export_cargos = RwSignal::new(vec![CargoDraft::new(1)]);
    let next_cargo_row = RwSignal::new(2_u64);
    let export_result = RwSignal::new(None::<PlannerPackageExportResult>);

    Effect::new(move |_| {
        spawn_local(async move {
            match invoke::<PlannerRouteView, _>("planner_status", &serde_json::json!({})).await {
                Ok(current) => route.set(Some(current)),
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
        });
    });

    Effect::new(move |_| {
        spawn_local(async move {
            match invoke::<Option<PlannerRoutingStatus>, _>(
                "planner_routing_status",
                &serde_json::json!({}),
            )
            .await
            {
                Ok(current) => {
                    if let Some(current) = &current {
                        routing_url.set(current.base_url.clone());
                        routing_dataset.set(current.map_dataset_id.clone());
                        connect_timeout_ms.set(current.connect_timeout_ms.to_string());
                        request_timeout_ms.set(current.request_timeout_ms.to_string());
                    }
                    routing_status.set(current);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
        });
    });

    Effect::new(move |_| {
        let _ = routing_profile.get();
        preview.set(None);
        export_result.set(None);
    });

    // A point that left the route cannot stay attached to a cargo.
    Effect::new(move |_| {
        let Some(current) = route.get() else {
            return;
        };
        let gone =
            |id: &str| !id.is_empty() && !current.points.iter().any(|point| point.point_id == id);
        if export_cargos.with_untracked(|rows| {
            rows.iter()
                .any(|row| gone(&row.pickup_point_id) || gone(&row.unload_point_id))
        }) {
            export_cargos.update(|rows| {
                for row in rows {
                    if gone(&row.pickup_point_id) {
                        row.pickup_point_id.clear();
                    }
                    if gone(&row.unload_point_id) {
                        row.unload_point_id.clear();
                    }
                }
            });
        }
    });

    Effect::new(move |_| {
        if let Some(current) = route.get()
            && let Ok(points_json) = serde_json::to_string(&current.points)
        {
            let geometry = preview
                .get()
                .filter(|value| {
                    value.state == PlannerRoutingPreviewState::Ok
                        && value.working_route_id == current.working_route_id
                        && value.revision == current.revision
                })
                .map(|value| value.geometry)
                .unwrap_or_default();
            if let Ok(geometry_json) = serde_json::to_string(&geometry) {
                cargo_planner_map_render(&current.working_route_id, &points_json, &geometry_json);
            }
        }
    });

    let add_point = move |_| {
        if busy.get_untracked() {
            return;
        }
        let name = title.get_untracked().trim().to_owned();
        let lat = latitude.get_untracked().trim().parse::<f64>();
        let lon = longitude.get_untracked().trim().parse::<f64>();
        let (Ok(lat), Ok(lon)) = (lat, lon) else {
            message.set(
                strings(locale.get_untracked())
                    .invalid_coordinates
                    .to_owned(),
            );
            error.set(true);
            return;
        };
        if name.is_empty() || !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            message.set(
                strings(locale.get_untracked())
                    .invalid_coordinates
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let input = PlannerPointInput {
                title: name,
                latitude: lat,
                longitude: lon,
            };
            match invoke::<PlannerRouteView, _>(
                "planner_add_point",
                &serde_json::json!({ "input": input }),
            )
            .await
            {
                Ok(current) => {
                    preview.set(None);
                    export_result.set(None);
                    route.set(Some(current));
                    title.set(String::new());
                    latitude.set(String::new());
                    longitude.set(String::new());
                    message.set(strings(locale.get_untracked()).added.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let reset = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match invoke::<PlannerRouteView, _>("planner_reset_route", &serde_json::json!({})).await
            {
                Ok(current) => {
                    preview.set(None);
                    export_result.set(None);
                    route.set(Some(current));
                    message.set(strings(locale.get_untracked()).reset_done.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let remove_point = move |point_id: String| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let input = PointActionInput { point_id };
            match invoke::<PlannerRouteView, _>(
                "planner_remove_point",
                &serde_json::json!({ "input": input }),
            )
            .await
            {
                Ok(current) => {
                    preview.set(None);
                    export_result.set(None);
                    route.set(Some(current));
                    message.set(strings(locale.get_untracked()).removed.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let move_point = move |point_id: String, delta: isize| {
        if busy.get_untracked() {
            return;
        }
        let Some(current) = route.get_untracked() else {
            return;
        };
        let Some(index) = current
            .points
            .iter()
            .position(|point| point.point_id == point_id)
        else {
            return;
        };
        let Some(target) = index.checked_add_signed(delta) else {
            return;
        };
        if target >= current.points.len() {
            return;
        }
        let mut ids = current
            .points
            .iter()
            .map(|point| point.point_id.clone())
            .collect::<Vec<_>>();
        ids.swap(index, target);
        busy.set(true);
        spawn_local(async move {
            let input = ReorderPointsInput { point_ids: ids };
            match invoke::<PlannerRouteView, _>(
                "planner_reorder_points",
                &serde_json::json!({ "input": input }),
            )
            .await
            {
                Ok(current) => {
                    preview.set(None);
                    export_result.set(None);
                    route.set(Some(current));
                    message.set(strings(locale.get_untracked()).reordered.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let configure_routing = move |_| {
        if busy.get_untracked() {
            return;
        }
        let base_url = routing_url.get_untracked().trim().to_owned();
        let map_dataset_id = routing_dataset.get_untracked().trim().to_owned();
        let Ok(connect_timeout) = connect_timeout_ms.get_untracked().trim().parse::<u64>() else {
            message.set(strings(locale.get_untracked()).routing_error.to_owned());
            error.set(true);
            return;
        };
        let Ok(request_timeout) = request_timeout_ms.get_untracked().trim().parse::<u64>() else {
            message.set(strings(locale.get_untracked()).routing_error.to_owned());
            error.set(true);
            return;
        };
        busy.set(true);
        spawn_local(async move {
            let input = PlannerRoutingConfigInput {
                base_url,
                map_dataset_id,
                connect_timeout_ms: connect_timeout,
                request_timeout_ms: request_timeout,
            };
            match invoke::<PlannerRoutingStatus, _>(
                "planner_configure_routing",
                &serde_json::json!({ "input": input }),
            )
            .await
            {
                Ok(current) => {
                    preview.set(None);
                    routing_status.set(Some(current));
                    message.set(
                        strings(locale.get_untracked())
                            .routing_configured
                            .to_owned(),
                    );
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
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
            match invoke::<(), _>("planner_disable_routing", &serde_json::json!({})).await {
                Ok(()) => {
                    preview.set(None);
                    routing_status.set(None);
                    message.set(strings(locale.get_untracked()).routing_disabled.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let test_routing = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match invoke::<PlannerRoutingProbeResult, _>(
                "planner_test_routing",
                &serde_json::json!({}),
            )
            .await
            {
                Ok(result) => {
                    let is_ok = result.state == PlannerRoutingAvailabilityState::Ok;
                    message.set(probe_message(result.state, locale.get_untracked()).to_owned());
                    error.set(!is_ok);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let calculate_preview = move |_| {
        if busy.get_untracked() {
            return;
        }
        let Some(current) = route.get_untracked() else {
            return;
        };
        if current.points.len() < 2 {
            message.set(
                strings(locale.get_untracked())
                    .preview_needs_points
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        let profile = match routing_profile.get_untracked().input() {
            Ok(profile) => profile,
            Err(RoutingDraftError::InvalidNumber) => {
                message.set(
                    strings(locale.get_untracked())
                        .invalid_profile_value
                        .to_owned(),
                );
                error.set(true);
                return;
            }
            Err(RoutingDraftError::ChoiceRequired) => {
                message.set(
                    strings(locale.get_untracked())
                        .routing_choice_required
                        .to_owned(),
                );
                error.set(true);
                return;
            }
        };
        let expected_profile = profile.clone();
        busy.set(true);
        spawn_local(async move {
            let input = PlannerRoutingPreviewInput { profile };
            match invoke::<PlannerRoutingPreviewView, _>(
                "planner_preview_route",
                &serde_json::json!({ "input": input }),
            )
            .await
            {
                Ok(result) => {
                    let state = result.state;
                    let current_route = route.get_untracked();
                    let profile_still_current = routing_profile
                        .get_untracked()
                        .input()
                        .is_ok_and(|current| current == expected_profile);
                    let still_current = profile_still_current
                        && current_route.as_ref().is_some_and(|current| {
                            current.working_route_id == result.working_route_id
                                && current.revision == result.revision
                        });
                    if still_current {
                        preview.set(Some(result));
                        message.set(preview_message(state, locale.get_untracked()).to_owned());
                        error.set(state != PlannerRoutingPreviewState::Ok);
                    } else {
                        preview.set(None);
                        message.set(strings(locale.get_untracked()).preview_stale.to_owned());
                        error.set(true);
                    }
                }
                Err(problem) => {
                    preview.set(None);
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let add_cargo = move |_| {
        let row_id = next_cargo_row.get_untracked();
        next_cargo_row.set(row_id.saturating_add(1));
        export_cargos.update(|rows| rows.push(CargoDraft::new(row_id)));
        export_result.set(None);
    };

    let remove_cargo = move |row_id: u64| {
        export_cargos.update(|rows| rows.retain(|row| row.row_id != row_id));
        export_result.set(None);
    };

    let export_package = move |_| {
        if busy.get_untracked() {
            return;
        }
        let Some(current) = route.get_untracked() else {
            return;
        };
        if current.points.is_empty() {
            message.set(
                strings(locale.get_untracked())
                    .package_needs_points
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        let vehicle_type = export_vehicle_type.get_untracked().trim().to_owned();
        if vehicle_type.is_empty() {
            message.set(
                strings(locale.get_untracked())
                    .vehicle_type_required
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        let cargos = match export_cargos
            .get_untracked()
            .iter()
            .map(CargoDraft::input)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(rows) => rows.into_iter().flatten().collect::<Vec<_>>(),
            Err(()) => {
                message.set(strings(locale.get_untracked()).invalid_cargo.to_owned());
                error.set(true);
                return;
            }
        };
        if cargos.is_empty() {
            message.set(
                strings(locale.get_untracked())
                    .package_needs_cargo
                    .to_owned(),
            );
            error.set(true);
            return;
        }
        let routing_profile_draft = routing_profile.get_untracked();
        let routing_profile_input = if routing_profile_draft.is_empty() {
            None
        } else {
            match routing_profile_draft.input() {
                Ok(profile) => Some(profile),
                Err(RoutingDraftError::InvalidNumber) => {
                    message.set(
                        strings(locale.get_untracked())
                            .invalid_profile_value
                            .to_owned(),
                    );
                    error.set(true);
                    return;
                }
                Err(RoutingDraftError::ChoiceRequired) => {
                    message.set(
                        strings(locale.get_untracked())
                            .routing_choice_required
                            .to_owned(),
                    );
                    error.set(true);
                    return;
                }
            }
        };
        let routing_preview = if routing_profile_input.is_some() {
            preview.get_untracked().filter(|value| {
                value.state == PlannerRoutingPreviewState::Ok
                    && value.working_route_id == current.working_route_id
                    && value.revision == current.revision
            })
        } else {
            None
        };

        busy.set(true);
        spawn_local(async move {
            let selected =
                invoke::<Option<String>, _>("planner_pick_export_path", &serde_json::json!({}))
                    .await;
            let path = match selected {
                Ok(Some(path)) => path,
                Ok(None) => {
                    message.set(
                        strings(locale.get_untracked())
                            .package_dialog_cancelled
                            .to_owned(),
                    );
                    error.set(false);
                    busy.set(false);
                    return;
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                    busy.set(false);
                    return;
                }
            };

            let input = PlannerPackageExportInput {
                path,
                vehicle_type,
                cargos,
                routing_profile: routing_profile_input,
                routing_preview,
                figures: figures.get_untracked(),
            };
            match invoke::<PlannerPackageExportResult, _>(
                "planner_export_package",
                &serde_json::json!({ "input": input }),
            )
            .await
            {
                Ok(result) => {
                    export_result.set(Some(result));
                    message.set(strings(locale.get_untracked()).package_exported.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    export_result.set(None);
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    view! {
        <main>
            <header>
                <div><h1>{move || strings(locale.get()).app_title}</h1><div class="note">{move || strings(locale.get()).subtitle}</div></div>
                <label class="language">
                    {move || strings(locale.get()).language}
                    <select on:change=move |event| {
                        let chosen = UiLocale::parse(&event_target_value(&event));
                        cargo_planner_set_locale(chosen.code());
                        locale.set(chosen);
                    }>
                        {LANGUAGES
                            .into_iter()
                            .map(|(code, name)| view! {
                                <option value=code prop:selected=move || { locale.get().code() == code }>
                                    {name}
                                </option>
                            })
                            .collect_view()}
                    </select>
                </label>
            </header>

            <OrdersPanel
                locale=locale
                message=message
                error=error
                route=route
                profile=routing_profile
                vehicle_type=export_vehicle_type
                vehicle_types=vehicle_types
                cargos=export_cargos
                next_cargo_row=next_cargo_row
                typed_figures=figures
            />

            <section>
                <h2>{move || strings(locale.get()).manual_route}</h2>
                <div class="status">{move || route.get().map(|value| format!(
                    "{}: {} · {}", strings(locale.get()).revision, value.revision, value.working_route_id
                )).unwrap_or_default()}</div>
                <p class="note">{move || strings(locale.get()).route_note}</p>
                <div class="grid">
                    <label>{move || strings(locale.get()).point_title}
                        <input prop:value=move || title.get() on:input=move |e| title.set(event_target_value(&e)) />
                    </label>
                    <label>{move || strings(locale.get()).latitude}
                        <input inputmode="decimal" prop:value=move || latitude.get() on:input=move |e| latitude.set(event_target_value(&e)) />
                    </label>
                    <label>{move || strings(locale.get()).longitude}
                        <input inputmode="decimal" prop:value=move || longitude.get() on:input=move |e| longitude.set(event_target_value(&e)) />
                    </label>
                </div>
                <div class="actions">
                    <button class="primary" type="button" on:click=add_point disabled=move || busy.get()>{move || strings(locale.get()).add_point}</button>
                    <button type="button" on:click=reset disabled=move || busy.get()>{move || strings(locale.get()).reset_route}</button>
                </div>
            </section>

            <RouteCheckPanel
                locale=locale
                message=message
                error=error
                route=route
                figures=figures
            />

            <section>
                <h2>{move || strings(locale.get()).routing_title}</h2>
                <p class="note">{move || strings(locale.get()).routing_note}</p>
                <div class="routing-grid">
                    <label>
                        {move || strings(locale.get()).routing_url}
                        <input
                            prop:value=move || routing_url.get()
                            on:input=move |event| routing_url.set(event_target_value(&event))
                        />
                    </label>
                    <label>
                        {move || strings(locale.get()).routing_dataset}
                        <input
                            prop:value=move || routing_dataset.get()
                            on:input=move |event| routing_dataset.set(event_target_value(&event))
                        />
                    </label>
                    <label>
                        {move || strings(locale.get()).connect_timeout_ms}
                        <input
                            type="number"
                            min="100"
                            max="30000"
                            prop:value=move || connect_timeout_ms.get()
                            on:input=move |event| connect_timeout_ms.set(event_target_value(&event))
                        />
                    </label>
                    <label>
                        {move || strings(locale.get()).request_timeout_ms}
                        <input
                            type="number"
                            min="250"
                            max="120000"
                            prop:value=move || request_timeout_ms.get()
                            on:input=move |event| request_timeout_ms.set(event_target_value(&event))
                        />
                    </label>
                </div>
                <div class="actions">
                    <button class="primary" type="button" on:click=configure_routing disabled=move || busy.get()>
                        {move || strings(locale.get()).save_routing}
                    </button>
                    <button type="button" on:click=test_routing disabled=move || busy.get()>
                        {move || strings(locale.get()).test_routing}
                    </button>
                    <button type="button" on:click=disable_routing disabled=move || busy.get()>
                        {move || strings(locale.get()).disable_routing}
                    </button>
                </div>
                <div class="status">
                    {move || routing_status.get().map(|value| {
                        format!("{} · {}", value.base_url, value.map_dataset_id)
                    }).unwrap_or_else(|| strings(locale.get()).routing_not_configured.to_owned())}
                </div>

                <h3>{move || strings(locale.get()).planning_profile}</h3>
                <p class="note">{move || strings(locale.get()).planning_profile_note}</p>
                <div class="routing-grid">
                    <label>{move || strings(locale.get()).height_m}
                        <input inputmode="decimal"
                            prop:value=move || routing_profile.get().height_m
                            on:input=move |event| routing_profile.update(|draft| draft.height_m = event_target_value(&event))
                        />
                    </label>
                    <label>{move || strings(locale.get()).width_m}
                        <input inputmode="decimal"
                            prop:value=move || routing_profile.get().width_m
                            on:input=move |event| routing_profile.update(|draft| draft.width_m = event_target_value(&event))
                        />
                    </label>
                    <label>{move || strings(locale.get()).length_m}
                        <input inputmode="decimal"
                            prop:value=move || routing_profile.get().length_m
                            on:input=move |event| routing_profile.update(|draft| draft.length_m = event_target_value(&event))
                        />
                    </label>
                    <label>{move || strings(locale.get()).gross_weight_t}
                        <input inputmode="decimal"
                            prop:value=move || routing_profile.get().gross_weight_t
                            on:input=move |event| routing_profile.update(|draft| draft.gross_weight_t = event_target_value(&event))
                        />
                    </label>
                    <label>{move || strings(locale.get()).axle_load_t}
                        <input inputmode="decimal"
                            prop:value=move || routing_profile.get().axle_load_t
                            on:input=move |event| routing_profile.update(|draft| draft.axle_load_t = event_target_value(&event))
                        />
                    </label>
                    <label>{move || strings(locale.get()).axle_count}
                        <input inputmode="numeric"
                            prop:value=move || routing_profile.get().axle_count
                            on:input=move |event| routing_profile.update(|draft| draft.axle_count = event_target_value(&event))
                        />
                    </label>
                    <label>{move || strings(locale.get()).hazmat}
                        <select
                            prop:value=move || routing_profile.get().hazmat
                            on:change=move |event| routing_profile.update(|draft| draft.hazmat = event_target_value(&event))
                        >
                            <option value="">{move || strings(locale.get()).not_specified}</option>
                            <option value="yes">{move || strings(locale.get()).yes}</option>
                            <option value="no">{move || strings(locale.get()).no}</option>
                        </select>
                    </label>
                    <label>{move || strings(locale.get()).hgv}
                        <select
                            prop:value=move || routing_profile.get().hgv
                            on:change=move |event| routing_profile.update(|draft| draft.hgv = event_target_value(&event))
                        >
                            <option value="">{move || strings(locale.get()).not_specified}</option>
                            <option value="yes">{move || strings(locale.get()).yes}</option>
                            <option value="no">{move || strings(locale.get()).no}</option>
                        </select>
                    </label>
                    <label>{move || strings(locale.get()).exclude_unpaved}
                        <select
                            prop:value=move || routing_profile.get().exclude_unpaved
                            on:change=move |event| routing_profile.update(|draft| draft.exclude_unpaved = event_target_value(&event))
                        >
                            <option value="">{move || strings(locale.get()).not_specified}</option>
                            <option value="yes">{move || strings(locale.get()).yes}</option>
                            <option value="no">{move || strings(locale.get()).no}</option>
                        </select>
                    </label>
                    <label>{move || strings(locale.get()).prefer_truck_routes}
                        <select
                            prop:value=move || routing_profile.get().prefer_truck_routes
                            on:change=move |event| routing_profile.update(|draft| draft.prefer_truck_routes = event_target_value(&event))
                        >
                            <option value="">{move || strings(locale.get()).not_specified}</option>
                            <option value="yes">{move || strings(locale.get()).yes}</option>
                            <option value="no">{move || strings(locale.get()).no}</option>
                        </select>
                    </label>
                    <label>{move || strings(locale.get()).toll_preference}
                        <select
                            prop:value=move || routing_profile.get().toll_preference
                            on:change=move |event| routing_profile.update(|draft| draft.toll_preference = event_target_value(&event))
                        >
                            <option value="">{move || strings(locale.get()).not_specified}</option>
                            <option value="allow">{move || strings(locale.get()).toll_allow}</option>
                            <option value="prefer_avoid">{move || strings(locale.get()).toll_prefer_avoid}</option>
                            <option value="avoid">{move || strings(locale.get()).toll_avoid}</option>
                        </select>
                    </label>
                    <label>{move || strings(locale.get()).max_speed_kmh}
                        <input inputmode="decimal"
                            prop:value=move || routing_profile.get().max_speed_kmh
                            on:input=move |event| routing_profile.update(|draft| draft.max_speed_kmh = event_target_value(&event))
                        />
                    </label>
                </div>
                <div class="actions">
                    <button class="primary" type="button" on:click=calculate_preview disabled=move || busy.get()>
                        {move || strings(locale.get()).calculate_preview}
                    </button>
                </div>
                {move || match preview.get() {
                    Some(value) => {
                        let distance = value.distance_m.map(|distance| distance / 1000.0);
                        let duration = value.duration_s.map(|seconds| seconds / 60.0);
                        let warnings = value
                            .warnings
                            .iter()
                            .map(|warning| warning_label(warning, locale.get()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        view! {
                            <div class="preview-summary">
                                {distance.map(|value| view! {
                                    <span><strong>{move || strings(locale.get()).distance_km}</strong>{format!(": {value:.1}")}</span>
                                })}
                                {duration.map(|value| view! {
                                    <span><strong>{move || strings(locale.get()).duration_min}</strong>{format!(": {value:.0}")}</span>
                                })}
                                {(!warnings.is_empty()).then(|| view! {
                                    <span><strong>{move || strings(locale.get()).warnings}</strong>{format!(": {warnings}")}</span>
                                })}
                            </div>
                        }.into_any()
                    }
                    None => view! { <span></span> }.into_any(),
                }}
            </section>

            <section>
                <h2>{move || strings(locale.get()).package_export_title}</h2>
                <p class="note">{move || strings(locale.get()).package_export_note}</p>
                <div class="routing-grid">
                    <label>
                        {move || strings(locale.get()).vehicle_type}
                        <select on:change=move |event| {
                            export_vehicle_type.set(event_target_value(&event));
                            export_result.set(None);
                        }>
                            {move || {
                                let current = export_vehicle_type.get();
                                let mut names: Vec<String> = vehicle_types
                                    .get()
                                    .into_iter()
                                    .map(|known| known.name)
                                    .collect();
                                // A name from an order file or an earlier version stays
                                // until another one is chosen.
                                if !current.is_empty() && !names.contains(&current) {
                                    names.insert(0, current.clone());
                                }
                                names.insert(0, String::new());
                                names
                                    .into_iter()
                                    .map(|name| {
                                        let selected = name == current;
                                        let line = if name.is_empty() {
                                            "—".to_owned()
                                        } else {
                                            name.clone()
                                        };
                                        view! {
                                            <option value=name prop:selected=selected>
                                                {line}
                                            </option>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </select>
                    </label>
                </div>

                <div class="package-heading">
                    <h3>{move || strings(locale.get()).cargos}</h3>
                    <button type="button" on:click=add_cargo disabled=move || busy.get()>
                        {move || strings(locale.get()).add_cargo}
                    </button>
                </div>

                <div class="cargo-list">
                    <For
                        each=move || export_cargos.get()
                        key=|cargo| cargo.row_id
                        children=move |cargo| {
                            let row_id = cargo.row_id;
                            let row_title = cargo_value(export_cargos, row_id, |row| &row.title);
                            let row_weight =
                                cargo_value(export_cargos, row_id, |row| &row.weight_kg);
                            let row_pallets =
                                cargo_value(export_cargos, row_id, |row| &row.pallets);
                            let row_pickup =
                                cargo_value(export_cargos, row_id, |row| &row.pickup_point_id);
                            let row_unload =
                                cargo_value(export_cargos, row_id, |row| &row.unload_point_id);
                            view! {
                                <div class="cargo-card package-cargo">
                                    <div class="cargo-grid">
                                        <label>
                                            {move || strings(locale.get()).cargo_title}
                                            <input
                                                prop:value=row_title
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    export_cargos.update(|rows| {
                                                        if let Some(row) = rows.iter_mut().find(|row| row.row_id == row_id) {
                                                            row.title = value;
                                                        }
                                                    });
                                                    export_result.set(None);
                                                }
                                            />
                                        </label>
                                        <label>
                                            {move || strings(locale.get()).cargo_weight_kg}
                                            <input
                                                inputmode="numeric"
                                                prop:value=row_weight
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    export_cargos.update(|rows| {
                                                        if let Some(row) = rows.iter_mut().find(|row| row.row_id == row_id) {
                                                            row.weight_kg = value;
                                                        }
                                                    });
                                                    export_result.set(None);
                                                }
                                            />
                                        </label>
                                        <label>
                                            {move || strings(locale.get()).cargo_pallets}
                                            <input
                                                inputmode="numeric"
                                                prop:value=row_pallets
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    export_cargos.update(|rows| {
                                                        if let Some(row) = rows.iter_mut().find(|row| row.row_id == row_id) {
                                                            row.pallets = value;
                                                        }
                                                    });
                                                    export_result.set(None);
                                                }
                                            />
                                        </label>
                                        <label>
                                            {move || strings(locale.get()).pickup_point}
                                            <select
                                                on:change=move |event| {
                                                    let value = event_target_value(&event);
                                                    export_cargos.update(|rows| {
                                                        if let Some(row) = rows.iter_mut().find(|row| row.row_id == row_id) {
                                                            row.pickup_point_id = value;
                                                        }
                                                    });
                                                    export_result.set(None);
                                                }
                                            >
                                                <option value="" prop:selected=move || row_pickup().is_empty()>
                                                    {move || strings(locale.get()).choose_point}
                                                </option>
                                                <For
                                                    each=move || route.get().map(|value| value.points).unwrap_or_default()
                                                    // The number is part of the text, so a moved point is drawn anew.
                                                    key=|point| format!("{}:{}", point.point_id, point.sequence)
                                                    children=move |point| {
                                                        let value = point.point_id.clone();
                                                        let own = value.clone();
                                                        view! {
                                                            <option value=value prop:selected=move || { row_pickup() == own }>
                                                                {format!("{}. {}", point.sequence, point.title)}
                                                            </option>
                                                        }
                                                    }
                                                />
                                            </select>
                                        </label>
                                        <label>
                                            {move || strings(locale.get()).unload_point}
                                            <select
                                                on:change=move |event| {
                                                    let value = event_target_value(&event);
                                                    export_cargos.update(|rows| {
                                                        if let Some(row) = rows.iter_mut().find(|row| row.row_id == row_id) {
                                                            row.unload_point_id = value;
                                                        }
                                                    });
                                                    export_result.set(None);
                                                }
                                            >
                                                <option value="" prop:selected=move || row_unload().is_empty()>
                                                    {move || strings(locale.get()).choose_point}
                                                </option>
                                                <For
                                                    each=move || route.get().map(|value| value.points).unwrap_or_default()
                                                    // The number is part of the text, so a moved point is drawn anew.
                                                    key=|point| format!("{}:{}", point.point_id, point.sequence)
                                                    children=move |point| {
                                                        let value = point.point_id.clone();
                                                        let own = value.clone();
                                                        view! {
                                                            <option value=value prop:selected=move || { row_unload() == own }>
                                                                {format!("{}. {}", point.sequence, point.title)}
                                                            </option>
                                                        }
                                                    }
                                                />
                                            </select>
                                        </label>
                                    </div>
                                    <div class="actions">
                                        <button
                                            class="danger"
                                            type="button"
                                            on:click=move |_| remove_cargo(row_id)
                                            disabled=move || busy.get()
                                        >
                                            {move || strings(locale.get()).remove_cargo}
                                        </button>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>

                <div class="actions">
                    <button class="primary" type="button" on:click=export_package disabled=move || busy.get()>
                        {move || strings(locale.get()).export_package}
                    </button>
                </div>

                {move || export_result.get().map(|result| {
                    let sent = result.clone();
                    let geometry = if result.geometry_included {
                        strings(locale.get()).geometry_included
                    } else {
                        strings(locale.get()).geometry_not_included
                    };
                    view! {
                        <div class="status package-result">
                            <div><strong>{move || strings(locale.get()).package_file}</strong>{format!(": {}", result.path)}</div>
                            <div><strong>{move || strings(locale.get()).package_schema}</strong>{format!(": {}.{}", result.schema_major, result.schema_minor)}</div>
                            <div><strong>{move || strings(locale.get()).trip_version}</strong>{format!(": {}", result.trip_version)}</div>
                            <div><strong>{move || strings(locale.get()).cargo_count}</strong>{format!(": {}", result.cargo_count)}</div>
                            <div>{geometry}</div>
                            {move || package_figures(locale.get(), &sent)
                                .into_iter()
                                .map(|line| view! { <div>{line}</div> })
                                .collect_view()}
                        </div>
                    }
                })}
            </section>

            <section>
                <h2>{move || strings(locale.get()).map_title}</h2>
                <p class="note">{move || strings(locale.get()).map_note}</p>
                <div id="planner-map" class="trip-map">
                    <div class="map-tiles"></div>
                    <div class="map-overlays"></div>
                    <div class="map-controls">
                        <button type="button" aria-label=move || strings(locale.get()).map_zoom_in on:click=move |_| cargo_planner_map_zoom(1)>"+"</button>
                        <button type="button" aria-label=move || strings(locale.get()).map_zoom_out on:click=move |_| cargo_planner_map_zoom(-1)>"−"</button>
                        <button type="button" on:click=move |_| cargo_planner_map_fit()>{move || strings(locale.get()).map_fit}</button>
                    </div>
                    <a
                        class="map-attribution"
                        href="https://www.openstreetmap.org/copyright"
                        target="_blank"
                        rel="noopener noreferrer"
                    >"© OpenStreetMap"</a>
                </div>
            </section>

            <section>
                {move || match route.get() {
                    Some(current) if !current.points.is_empty() => {
                        let total = current.points.len();
                        view! {
                            <ul class="route-list">
                                <For each=move || current.points.clone() key=|point| point.point_id.clone() children=move |point| {
                                    let id_up = point.point_id.clone();
                                    let id_down = point.point_id.clone();
                                    let id_remove = point.point_id.clone();
                                    let index = point.sequence - 1;
                                    view! {
                                        <li class="route-item">
                                            <span class="sequence">{point.sequence}</span>
                                            <div><div class="point-title">{point.title}</div><div class="coords">{format!("{:.6}, {:.6}", point.latitude, point.longitude)}</div></div>
                                            <div class="actions">
                                                <button type="button" disabled={index == 0} on:click=move |_| move_point(id_up.clone(), -1)>{move || strings(locale.get()).move_up}</button>
                                                <button type="button" disabled={index + 1 >= total} on:click=move |_| move_point(id_down.clone(), 1)>{move || strings(locale.get()).move_down}</button>
                                                <button class="danger" type="button" on:click=move |_| remove_point(id_remove.clone())>{move || strings(locale.get()).remove}</button>
                                            </div>
                                        </li>
                                    }
                                } />
                            </ul>
                        }.into_any()
                    }
                    _ => view! { <p class="note">{move || strings(locale.get()).empty_route}</p> }.into_any(),
                }}
            </section>

            <Show when=move || !message.get().is_empty()>
                <p class=move || if error.get() { "error" } else { "ok" }>{move || message.get()}</p>
            </Show>
        </main>
    }
}
