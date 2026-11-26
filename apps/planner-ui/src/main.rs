#![forbid(unsafe_code)]

mod i18n;

use crate::i18n::{UiLocale, strings};
use cargo_dispatch_planner_ui_contracts::{
    PlannerPointInput, PlannerRouteView, PointActionInput, ReorderPointsInput,
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
  routeId: null,
  pointSetKey: ""
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
  if (!host || points.length === 0) {
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
  let center = {latitude: points[0].latitude, longitude: points[0].longitude};

  if (points.length === 1) {
    chosenZoom = 14;
  } else {
    for (let zoom = 18; zoom >= 3; zoom--) {
      const shown = points.map(point => plannerProject(point.latitude, point.longitude, zoom));
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

export function cargoPlannerMapRender(routeId, pointsJson) {
  const points = JSON.parse(pointsJson);
  const pointSetKey = points.map(point => point.point_id).sort().join("|");
  const shouldFit =
    !plannerMapState.initialized ||
    plannerMapState.routeId !== routeId ||
    plannerMapState.pointSetKey !== pointSetKey;

  plannerMapState.routeId = routeId;
  plannerMapState.pointSetKey = pointSetKey;
  plannerMapState.points = points;

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
    fn cargo_planner_map_render(route_id: &str, points_json: &str);
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

    Effect::new(move |_| {
        spawn_local(async move {
            match invoke::<PlannerRouteView, _>("planner_status", &serde_json::json!({})).await {
                Ok(current) => route.set(Some(current)),
                Err(problem) => {
                    message.set(problem);
                    error.set(true);
                }
            }
        });
    });

    Effect::new(move |_| {
        if let Some(current) = route.get()
            && let Ok(points_json) = serde_json::to_string(&current.points)
        {
            cargo_planner_map_render(&current.working_route_id, &points_json);
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
                    route.set(Some(current));
                    title.set(String::new());
                    latitude.set(String::new());
                    longitude.set(String::new());
                    message.set(strings(locale.get_untracked()).added.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(problem);
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
                    route.set(Some(current));
                    message.set(strings(locale.get_untracked()).reset_done.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(problem);
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
                    route.set(Some(current));
                    message.set(strings(locale.get_untracked()).removed.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(problem);
                    error.set(true);
                }
            }
            busy.set(false);
        });
    };

    let move_point = move |point_id: String, delta: isize| {
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
                    route.set(Some(current));
                    message.set(strings(locale.get_untracked()).reordered.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(problem);
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
                    <select prop:value=move || locale.get().code() on:change=move |event| {
                        let chosen = UiLocale::parse(&event_target_value(&event));
                        cargo_planner_set_locale(chosen.code());
                        locale.set(chosen);
                    }>
                        <option value="uk">"Українська"</option><option value="en">"English"</option>
                        <option value="de">"Deutsch"</option><option value="fr">"Français"</option>
                        <option value="es">"Español"</option><option value="ko">"한국어"</option>
                        <option value="ja">"日本語"</option><option value="zh-Hans">"简体中文"</option>
                    </select>
                </label>
            </header>

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

            <section>
                <h2>{move || strings(locale.get()).map_title}</h2>
                <p class="note">{move || strings(locale.get()).map_note}</p>
                <div id="planner-map" class="trip-map">
                    <div class="map-tiles"></div>
                    <div class="map-overlays"></div>
                    <div class="map-controls">
                        <button type="button" aria-label="Zoom in" on:click=move |_| cargo_planner_map_zoom(1)>"+"</button>
                        <button type="button" aria-label="Zoom out" on:click=move |_| cargo_planner_map_zoom(-1)>"−"</button>
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
                                                <button type="button" disabled=index == 0 on:click=move |_| move_point(id_up.clone(), -1)>{move || strings(locale.get()).move_up}</button>
                                                <button type="button" disabled=index + 1 >= total on:click=move |_| move_point(id_down.clone(), 1)>{move || strings(locale.get()).move_down}</button>
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
