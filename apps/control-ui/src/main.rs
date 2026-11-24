#![forbid(unsafe_code)]

use cargo_dispatch_control_ui_contracts::{
    ControlUiStatus, DashboardDocumentDirection, DashboardPoint, ListenerMode,
    OperatorDocumentInput, PickedFile, RegisterTripInput, ReorderRouteInput, SessionInput,
    StartLanInput, TripDashboard,
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
                message.set(problem);
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
    let folder = RwSignal::new(String::new());
    let token = RwSignal::new(String::new());
    let lan_ip = RwSignal::new(String::new());
    let lan_port = RwSignal::new("8080".to_owned());
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
                message.set(problem);
                error.set(true);
            }
        }
    };

    let refresh_dashboard = move |_| {
        let active = selected_token.get_untracked();
        if active.is_empty() {
            message.set("Спочатку виберіть browser session.".to_owned());
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
                    message.set(problem);
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
                    message.set(problem);
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
        message.set("Відкриваю папку рейсу…".to_owned());
        error.set(false);
        let input = RegisterTripInput {
            folder_path: folder.get_untracked(),
            token: token.get_untracked(),
        };
        spawn_local(async move {
            let result = invoke("register_trip", &serde_json::json!({ "input": input })).await;
            set_status_result(result, "Рейс зареєстрований у Control.");
        });
    };

    let start_loopback = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let result = invoke("start_loopback", &serde_json::json!({})).await;
            set_status_result(result, "Loopback-трансляцію запущено.");
        });
    };

    let start_lan = move |_| {
        if busy.get_untracked() {
            return;
        }
        let port = match lan_port.get_untracked().trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                message.set("Порт має бути числом 1–65535.".to_owned());
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
            set_status_result(result, "LAN test mode запущено.");
        });
    };

    let stop = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let result = invoke("stop_browser_server", &serde_json::json!({})).await;
            set_status_result(result, "Трансляцію вимкнено. Рейс лишився зареєстрованим.");
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
                    message.set(problem);
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
            message.set("Спочатку виберіть файл.".to_owned());
            error.set(true);
            return;
        };
        if active.is_empty() {
            message.set("Спочатку виберіть browser session.".to_owned());
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
                    message.set("Файл записаний у рейс і доступний водієві.".to_owned());
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
                <p>"Cargo Dispatch"</p>
                <h1>"Control — рейс і трансляція"</h1>
                <p class="note">
                    "Control є єдиним джерелом факту рейсу. Desktop UI і Driver Browser працюють з тим самим durable state."
                </p>
            </header>

            <section>
                <h2>"1. Підключити рейс"</h2>
                <div class="grid">
                    <label>
                        "Папка активного рейсу"
                        <input
                            id="trip-folder"
                            type="text"
                            placeholder=r"D:\CargoDispatch\active\trip-..."
                            prop:value=move || folder.get()
                            on:input=move |event| folder.set(event_target_value(&event))
                        />
                    </label>
                    <label>
                        "Session token (згенеровано автоматично)"
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
                        "Вибрати папку…"
                    </button>
                    <button class="primary" type="button" on:click=register disabled=move || busy.get()>
                        "Підключити рейс"
                    </button>
                </div>
            </section>

            <section>
                <h2>"2. Трансляція"</h2>
                <div class="actions">
                    <button type="button" on:click=start_loopback disabled=move || busy.get()>
                        "Локально 127.0.0.1"
                    </button>
                </div>
                <div class="grid">
                    <label>
                        "LAN-адреса цього комп'ютера"
                        <input
                            id="lan-ip"
                            type="text"
                            placeholder="192.168.1.25"
                            prop:value=move || lan_ip.get()
                            on:input=move |event| lan_ip.set(event_target_value(&event))
                        />
                    </label>
                    <label>
                        "Порт"
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
                        "Увімкнути LAN test mode"
                    </button>
                    <button class="danger" type="button" on:click=stop disabled=move || busy.get()>
                        "Вимкнути трансляцію"
                    </button>
                </div>
                <p class="note">
                    "LAN не відкриває роутер/NAT автоматично. Route/files/media працюють напряму; GPS у телефонному браузері через HTTP зазвичай потребує HTTPS."
                </p>
            </section>

            <section>
                <h2>"3. Browser sessions"</h2>
                <div class="status">
                    {move || {
                        status.get().map(|current| {
                            match current.listener {
                                Some(listener) => {
                                    let mode = match listener.mode {
                                        ListenerMode::Loopback => "loopback",
                                        ListenerMode::Lan => "LAN",
                                    };
                                    format!("Listener: {mode} · {}", listener.address)
                                }
                                None => "Listener вимкнений".to_owned(),
                            }
                        }).unwrap_or_else(|| "Читаю стан…".to_owned())
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
                            let url = session.driver_url.clone();
                            view! {
                                <li>
                                    <div><strong>"Trip: "</strong><code>{trip}</code></div>
                                    <div><strong>"Token: "</strong><code>{session_token}</code></div>
                                    {url.map(|driver_url| {
                                        let href = driver_url.clone();
                                        view! {
                                            <div>
                                                <strong>"Driver URL: "</strong>
                                                <a href=href target="_blank" rel="noopener">{driver_url}</a>
                                            </div>
                                        }
                                    })}
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
                                        "Відкрити в dashboard"
                                    </button>
                                </li>
                            }
                        }
                    />
                </ul>
            </section>

            <section>
                <h2>"4. Живий рейс"</h2>
                <div class="actions">
                    <button type="button" on:click=refresh_dashboard>"Оновити"</button>
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
                                <strong>"Trip: "</strong><code>{trip_id}</code>
                                " · version " {version}
                                " · route revision " {revision}
                            </p>

                            <h3>"Маршрут"</h3>
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
                                        view! {
                                            <div class="point">
                                                <strong>{format!("{}. {}", point.sequence, point.title)}</strong>
                                                <span>{if point.completed { " — виконано" } else { " — відкрито" }}</span>
                                                {navigation.map(|nav| view! {
                                                    <div class="note">
                                                        {nav.address.unwrap_or_default()}
                                                        " · "
                                                        {format!("{:.6}, {:.6}", nav.latitude, nav.longitude)}
                                                    </div>
                                                })}
                                                {(!point.completed).then(|| view! {
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

                            <h3>"Остання позиція водія"</h3>
                            {match location {
                                Some(location) => view! {
                                    <p>
                                        {format!("{:.6}, {:.6}", location.latitude, location.longitude)}
                                        {location.accuracy_m.map(|accuracy| format!(" · ±{accuracy:.1} м")).unwrap_or_default()}
                                        " · "
                                        {format!("timestamp {}", location.observed_at_unix_ms)}
                                    </p>
                                }.into_any(),
                                None => view! { <p class="note">"Позиція ще не надходила."</p> }.into_any(),
                            }}

                            <h3>"Документи / фото / відео"</h3>
                            <ul>
                                <For
                                    each=move || documents.clone()
                                    key=|document| document.document_id.clone()
                                    children=move |document| {
                                        let document_id = document.document_id.clone();
                                        let direction = match document.direction {
                                            DashboardDocumentDirection::FromOperator => "Control → водій",
                                            DashboardDocumentDirection::FromDriver => "водій → Control",
                                        };
                                        view! {
                                            <li>
                                                <strong>{document.original_name}</strong>
                                                " · " {direction}
                                                " · " {document.mime_type}
                                                " · " {document.size} " байт"
                                                {document.point_id.map(|point| view! {
                                                    <span>" · point " <code>{point}</code></span>
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
                                                                        "Файл збережено: {path}"
                                                                    ));
                                                                    error.set(false);
                                                                }
                                                                Ok(None) => {}
                                                                Err(problem) => {
                                                                    message.set(problem);
                                                                    error.set(true);
                                                                }
                                                            }
                                                            busy.set(false);
                                                        });
                                                    }
                                                >
                                                    "Зберегти…"
                                                </button>
                                            </li>
                                        }
                                    }
                                />
                            </ul>

                            <h3>"Передати файл водієві"</h3>
                            <div class="actions">
                                <button type="button" on:click=pick_file disabled=move || busy.get()>
                                    "Вибрати файл…"
                                </button>
                                <span>
                                    {move || picked_file.get()
                                        .map(|file| file.original_name)
                                        .unwrap_or_else(|| "Файл не вибрано".to_owned())}
                                </span>
                            </div>
                            <label>
                                "Прив'язати до точки"
                                <select
                                    on:change=move |event| attachment_point.set(event_target_value(&event))
                                >
                                    <option value="">"Рейс у цілому"</option>
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
                                    "Передати водієві"
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
