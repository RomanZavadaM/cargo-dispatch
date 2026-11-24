use leptos::prelude::*;
use ui_probe_core::{UiLanguage, UiTextKey, synthetic_driver_trip, ui_text};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(DriverApp);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DriverStatus {
    Driving,
    Arrived,
    Done,
}

fn driver_status_text(language: UiLanguage, status: DriverStatus) -> &'static str {
    let key = match status {
        DriverStatus::Driving => UiTextKey::Driving,
        DriverStatus::Arrived => UiTextKey::Arrived,
        DriverStatus::Done => UiTextKey::Done,
    };
    ui_text(language, key)
}

#[component]
fn DriverApp() -> impl IntoView {
    let trip = synthetic_driver_trip();
    let language = RwSignal::new(UiLanguage::Uk);
    let status = RwSignal::new(DriverStatus::Driving);
    let download_document = trip
        .documents
        .iter()
        .find(|document| document.mime_type == "application/pdf")
        .cloned()
        .expect("synthetic trip has a PDF");
    let navigator_url = format!(
        "https://www.google.com/maps/dir/?api=1&destination={},{}",
        trip.latitude, trip.longitude
    );

    view! {
        <main class="driver-shell" data-probe="driver-browser">
            <section class="card trip-head">
                <div>
                    <p class="eyebrow">"Cargo Dispatch · " {move || ui_text(language.get(), UiTextKey::DriverTitle)}</p>
                    <h1 id="driver-trip">{trip.trip_id.clone()}</h1>
                    <p id="driver-vehicle">{trip.vehicle.clone()}</p>
                </div>
                <div>
                    <label>
                        <span>{move || ui_text(language.get(), UiTextKey::Language)}</span>
                        <select
                            id="driver-language"
                            aria-label=move || ui_text(language.get(), UiTextKey::Language)
                            prop:value=move || language.get().code()
                            on:change=move |event| {
                                if let Some(next) = UiLanguage::from_code(&event_target_value(&event)) {
                                    language.set(next);
                                    let _ = js_sys::eval(&format!(
                                        "document.documentElement.lang = '{}'",
                                        next.code()
                                    ));
                                }
                            }
                        >
                            <option value="uk">"Українська"</option>
                            <option value="en">"English"</option>
                            <option value="de">"Deutsch"</option>
                        </select>
                    </label>
                    <strong id="driver-status" role="status" aria-live="polite">
                        {move || driver_status_text(language.get(), status.get())}
                    </strong>
                </div>
            </section>

            <section class="card">
                <p class="eyebrow">{move || ui_text(language.get(), UiTextKey::CurrentPoint)}</p>
                <strong id="current-point">{trip.current_point.clone()}</strong>
                <span id="current-task">{trip.task.clone()}</span>
                <button
                    id="status-button"
                    class="primary"
                    type="button"
                    on:click=move |_| {
                        let next = match status.get_untracked() {
                            DriverStatus::Driving => DriverStatus::Arrived,
                            DriverStatus::Arrived => DriverStatus::Done,
                            DriverStatus::Done => DriverStatus::Driving,
                        };
                        status.set(next);
                    }
                >
                    {move || ui_text(language.get(), UiTextKey::ChangeStatus)}
                </button>
            </section>

            <section class="card">
                <p class="eyebrow">{move || ui_text(language.get(), UiTextKey::Documents)}</p>
                <ul
                    id="driver-documents"
                    class="docs"
                    aria-label=move || ui_text(language.get(), UiTextKey::Documents)
                >
                    <For
                        each=move || trip.documents.clone()
                        key=|document| document.id.clone()
                        children=move |document| {
                            let required = document.required;
                            view! {
                                <li>
                                    <span>{document.name}</span>
                                    <strong>{move || if required {
                                        ui_text(language.get(), UiTextKey::Required)
                                    } else {
                                        ui_text(language.get(), UiTextKey::Optional)
                                    }}</strong>
                                </li>
                            }
                        }
                    />
                </ul>
                <label class="file-label" for="driver-file">{move || ui_text(language.get(), UiTextKey::AddFile)}</label>
                <input
                    id="driver-file"
                    type="file"
                    accept="image/*,application/pdf,video/*"
                    capture="environment"
                    on:change=move |_| {
                        let _ = js_sys::eval("window.cargoDriverReadSelectedFile()");
                    }
                />
                <span id="selected-file-metadata" aria-live="polite">{move || ui_text(language.get(), UiTextKey::NoFile)}</span>
                <a
                    id="document-download"
                    class="file-label"
                    href="data:application/pdf;base64,JVBERi0xLjQKJSBDYXJnbyBEaXNwYXRjaCBzeW50aGV0aWMgZG9jdW1lbnQKJSVFT0YK"
                    download=download_document.name.clone()
                    data-document-id=download_document.id.clone()
                    data-mime=download_document.mime_type.clone()
                >
                    {move || ui_text(language.get(), UiTextKey::DownloadDocument)}
                </a>
            </section>

            <section class="card">
                <p class="eyebrow">{move || ui_text(language.get(), UiTextKey::Phone)}</p>
                <div class="actions">
                    <button
                        id="geo-button"
                        type="button"
                        on:click=move |_| {
                            let _ = js_sys::eval("window.cargoDriverRequestLocation()");
                        }
                    >
                        {move || ui_text(language.get(), UiTextKey::CheckLocation)}
                    </button>
                    <a
                        id="navigator-link"
                        href=navigator_url
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        {move || ui_text(language.get(), UiTextKey::OpenNavigator)}
                    </a>
                </div>
                <span id="geo-status" aria-live="polite">"Координати ще не запитувались"</span>
            </section>
        </main>
    }
}
