use leptos::prelude::*;
use ui_probe_core::synthetic_driver_trip;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(DriverApp);
}

#[component]
fn DriverApp() -> impl IntoView {
    let trip = synthetic_driver_trip();
    let status = RwSignal::new("У дорозі".to_owned());
    let navigator_url = format!(
        "https://www.google.com/maps/dir/?api=1&destination={},{}",
        trip.latitude, trip.longitude
    );

    view! {
        <main class="driver-shell" data-probe="driver-browser">
            <section class="card trip-head">
                <div>
                    <p class="eyebrow">"Cargo Dispatch · водій"</p>
                    <h1 id="driver-trip">{trip.trip_id.clone()}</h1>
                    <p id="driver-vehicle">{trip.vehicle.clone()}</p>
                </div>
                <strong id="driver-status">{move || status.get()}</strong>
            </section>

            <section class="card">
                <p class="eyebrow">"Поточна точка"</p>
                <strong id="current-point">{trip.current_point.clone()}</strong>
                <span id="current-task">{trip.task.clone()}</span>
                <button
                    id="status-button"
                    class="primary"
                    type="button"
                    on:click=move |_| {
                        let next = match status.get_untracked().as_str() {
                            "У дорозі" => "Прибув",
                            "Прибув" => "Завдання виконано",
                            _ => "У дорозі",
                        };
                        status.set(next.to_owned());
                    }
                >
                    "Змінити статус"
                </button>
            </section>

            <section class="card">
                <p class="eyebrow">"Документи"</p>
                <ul id="driver-documents" class="docs">
                    <For
                        each=move || trip.documents.clone()
                        key=|document| document.id.clone()
                        children=move |document| view! {
                            <li>
                                <span>{document.name}</span>
                                <strong>{if document.required { "обов’язковий" } else { "за потреби" }}</strong>
                            </li>
                        }
                    />
                </ul>
                <label class="file-label" for="driver-file">"Додати фото / PDF / файл"</label>
                <input
                    id="driver-file"
                    type="file"
                    accept="image/*,application/pdf,video/*"
                    capture="environment"
                />
            </section>

            <section class="card">
                <p class="eyebrow">"Телефон"</p>
                <div class="actions">
                    <button
                        id="geo-button"
                        type="button"
                        on:click=move |_| {
                            let _ = js_sys::eval("window.cargoDriverRequestLocation()");
                        }
                    >
                        "Перевірити моє місце"
                    </button>
                    <a
                        id="navigator-link"
                        href=navigator_url
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        "Відкрити навігатор"
                    </a>
                </div>
                <span id="geo-status" aria-live="polite">"Координати ще не запитувались"</span>
            </section>
        </main>
    }
}
