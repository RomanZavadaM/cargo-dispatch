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
"#)]
extern "C" {
    #[wasm_bindgen(js_name = cargoPlannerInvoke)]
    fn cargo_planner_invoke(command: &str, args_json: &str) -> js_sys::Promise;
    #[wasm_bindgen(js_name = cargoPlannerInitialLocale)]
    fn cargo_planner_initial_locale() -> String;
    #[wasm_bindgen(js_name = cargoPlannerSetLocale)]
    fn cargo_planner_set_locale(locale: &str);
}

fn js_error(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

async fn invoke<T: DeserializeOwned, A: Serialize>(command: &str, args: &A) -> Result<T, String> {
    let args_json = serde_json::to_string(args).map_err(|error| format!("command encode failed: {error}"))?;
    let value = JsFuture::from(cargo_planner_invoke(command, &args_json)).await.map_err(js_error)?;
    let json = value.as_string().ok_or_else(|| "Planner returned an unexpected response".to_owned())?;
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
                Err(problem) => { message.set(problem); error.set(true); }
            }
        });
    });

    let add_point = move |_| {
        if busy.get_untracked() { return; }
        let name = title.get_untracked().trim().to_owned();
        let lat = latitude.get_untracked().trim().parse::<f64>();
        let lon = longitude.get_untracked().trim().parse::<f64>();
        let (Ok(lat), Ok(lon)) = (lat, lon) else {
            message.set(strings(locale.get_untracked()).invalid_coordinates.to_owned());
            error.set(true);
            return;
        };
        if name.is_empty() || !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            message.set(strings(locale.get_untracked()).invalid_coordinates.to_owned());
            error.set(true);
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let input = PlannerPointInput { title: name, latitude: lat, longitude: lon };
            match invoke::<PlannerRouteView, _>("planner_add_point", &serde_json::json!({ "input": input })).await {
                Ok(current) => {
                    route.set(Some(current));
                    title.set(String::new()); latitude.set(String::new()); longitude.set(String::new());
                    message.set(strings(locale.get_untracked()).added.to_owned()); error.set(false);
                }
                Err(problem) => { message.set(problem); error.set(true); }
            }
            busy.set(false);
        });
    };

    let reset = move |_| {
        if busy.get_untracked() { return; }
        busy.set(true);
        spawn_local(async move {
            match invoke::<PlannerRouteView, _>("planner_reset_route", &serde_json::json!({})).await {
                Ok(current) => {
                    route.set(Some(current));
                    message.set(strings(locale.get_untracked()).reset_done.to_owned()); error.set(false);
                }
                Err(problem) => { message.set(problem); error.set(true); }
            }
            busy.set(false);
        });
    };

    let remove_point = move |point_id: String| {
        if busy.get_untracked() { return; }
        busy.set(true);
        spawn_local(async move {
            let input = PointActionInput { point_id };
            match invoke::<PlannerRouteView, _>("planner_remove_point", &serde_json::json!({ "input": input })).await {
                Ok(current) => {
                    route.set(Some(current));
                    message.set(strings(locale.get_untracked()).removed.to_owned()); error.set(false);
                }
                Err(problem) => { message.set(problem); error.set(true); }
            }
            busy.set(false);
        });
    };

    let move_point = move |point_id: String, delta: isize| {
        let Some(current) = route.get_untracked() else { return; };
        let Some(index) = current.points.iter().position(|point| point.point_id == point_id) else { return; };
        let Some(target) = index.checked_add_signed(delta) else { return; };
        if target >= current.points.len() { return; }
        let mut ids = current.points.iter().map(|point| point.point_id.clone()).collect::<Vec<_>>();
        ids.swap(index, target);
        busy.set(true);
        spawn_local(async move {
            let input = ReorderPointsInput { point_ids: ids };
            match invoke::<PlannerRouteView, _>("planner_reorder_points", &serde_json::json!({ "input": input })).await {
                Ok(current) => {
                    route.set(Some(current));
                    message.set(strings(locale.get_untracked()).reordered.to_owned()); error.set(false);
                }
                Err(problem) => { message.set(problem); error.set(true); }
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
