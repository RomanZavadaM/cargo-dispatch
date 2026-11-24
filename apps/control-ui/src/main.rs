#![forbid(unsafe_code)]

use cargo_dispatch_control_ui_contracts::{
    ControlUiStatus, ListenerMode, RegisterTripInput, StartLanInput,
};
use leptos::{prelude::*, task::spawn_local};
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;
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
"#)]
extern "C" {
    #[wasm_bindgen(js_name = cargoControlInvoke)]
    fn cargo_control_invoke(command: &str, args_json: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = cargoControlRandomToken)]
    fn cargo_control_random_token() -> String;
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
    let message = RwSignal::new(String::new());
    let error = RwSignal::new(false);
    let busy = RwSignal::new(false);

    Effect::new(move |_| {
        if token.get_untracked().is_empty() {
            token.set(cargo_control_random_token());
        }
        spawn_local(async move {
            if let Ok(current) = load_status().await {
                status.set(Some(current));
            }
        });
    });

    let set_result = move |result: Result<ControlUiStatus, String>, success: &'static str| {
        busy.set(false);
        match result {
            Ok(current) => {
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
            set_result(result, "Рейс зареєстрований у Control.");
        });
    };

    let start_loopback = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let result = invoke("start_loopback", &serde_json::json!({})).await;
            set_result(result, "Loopback-трансляцію запущено.");
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
            set_result(result, "LAN test mode запущено.");
        });
    };

    let stop = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let result = invoke("stop_browser_server", &serde_json::json!({})).await;
            set_result(result, "Трансляцію вимкнено. Рейс лишився зареєстрованим.");
        });
    };

    view! {
        <main>
            <header>
                <p>"Cargo Dispatch"</p>
                <h1>"Control — трансляція рейсу"</h1>
                <p class="note">
                    "Control є джерелом рейсу. Це вікно лише керує browser-сесією і listener."
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
                            placeholder="trip-session-..."
                            prop:value=move || token.get()
                            on:input=move |event| token.set(event_target_value(&event))
                        />
                    </label>
                </div>
                <div class="actions">
                    <button class="primary" type="button" on:click=register disabled=move || busy.get()>
                        "Підключити папку рейсу"
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
                    "LAN не відкриває роутер/NAT автоматично. GPS у телефонному браузері через звичайний HTTP зазвичай потребуватиме HTTPS."
                </p>
            </section>

            <section>
                <h2>"3. Стан"</h2>
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
                <p
                    class=move || if error.get() { "error" } else { "ok" }
                    role="status"
                    aria-live="polite"
                >
                    {move || message.get()}
                </p>

                <h3>"Browser sessions"</h3>
                <ul>
                    <For
                        each=move || status.get().map(|current| current.sessions).unwrap_or_default()
                        key=|session| session.token.clone()
                        children=move |session| {
                            let trip = session.trip_id.clone();
                            let token_text = session.token.clone();
                            let url = session.driver_url.clone();
                            view! {
                                <li>
                                    <div><strong>"Trip: "</strong><code>{trip}</code></div>
                                    <div><strong>"Token: "</strong><code>{token_text}</code></div>
                                    {url.map(|driver_url| view! {
                                        <div>
                                            <strong>"Driver URL: "</strong>
                                            <a href=driver_url.clone() target="_blank" rel="noopener">
                                                {driver_url.clone()}
                                            </a>
                                        </div>
                                    })}
                                </li>
                            }
                        }
                    />
                </ul>
            </section>
        </main>
    }
}
