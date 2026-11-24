use dioxus::prelude::*;
use ui_probe_core::{ApplicationService, SortField};

fn main() {
    console_error_panic_hook::set_once();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut service = use_signal(ApplicationService::synthetic);
    let mut filter = use_signal(String::new);
    let mut selected = use_signal(|| None::<u64>);

    let summary = service.read().summary();
    let filter_value = filter.read().clone();
    let rows = service
        .read()
        .query(&filter_value, SortField::Id, true, None);

    rsx! {
        main { class: "shell", "data-probe": "dioxus",
            header {
                p { class: "eyebrow", "Cargo dispatch · secondary UI probe" }
                h1 { id: "dioxus-title", "Dioxus request pool" }
                p { "Same ui-probe-core application service; UI adapter only." }
            }

            section { class: "metrics",
                span { strong { id: "dioxus-total", "{summary.total_requests}" } " requests" }
                span { strong { "{summary.reserved_requests}" } " reserved" }
                span { strong { "{summary.assigned_requests}" } " assigned" }
                span { "revision " strong { id: "dioxus-revision", "{summary.revision}" } }
            }

            section { class: "toolbar",
                label {
                    span { "Filter" }
                    input {
                        id: "dioxus-filter",
                        value: "{filter_value}",
                        oninput: move |event| filter.set(event.value())
                    }
                }
                p { id: "dioxus-visible", "{rows.len()} visible rows" }
            }

            section { class: "table-wrap",
                table {
                    thead {
                        tr {
                            th { "Open" }
                            th { "ID" }
                            th { "Source" }
                            th { "Origin" }
                            th { "Destination" }
                            th { "Rate €" }
                        }
                    }
                    tbody { id: "dioxus-rows",
                        for row in rows {
                            {
                                let id = row.id;
                                rsx! {
                                    tr { class: "dioxus-row", "data-request-id": "{id}",
                                        td {
                                            button {
                                                class: "dioxus-open",
                                                "data-request-id": "{id}",
                                                onclick: move |_| selected.set(Some(id)),
                                                "Open"
                                            }
                                        }
                                        td { "{row.id}" }
                                        td { "{row.source}" }
                                        td { class: "dioxus-origin", "{row.origin}" }
                                        td { "{row.destination}" }
                                        td { "{row.rate_eur}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(request_id) = *selected.read() {
                RequestEditor { request_id, service, selected }
            }
        }
    }
}

#[component]
fn RequestEditor(
    request_id: u64,
    mut service: Signal<ApplicationService>,
    mut selected: Signal<Option<u64>>,
) -> Element {
    let initial = service
        .read()
        .editor_snapshot(request_id)
        .expect("selected request exists");
    let initial_origin = initial.field_value("origin").unwrap_or_default().to_owned();
    let mut origin = use_signal(|| initial_origin.clone());
    let mut message = use_signal(String::new);

    rsx! {
        section { class: "editor", id: "dioxus-editor",
            h2 { "Edit request {request_id}" }
            label {
                span { "Origin" }
                input {
                    id: "dioxus-origin-input",
                    value: "{origin}",
                    oninput: move |event| {
                        origin.set(event.value());
                        message.set(String::new());
                    }
                }
            }
            div { class: "editor-actions",
                button {
                    id: "dioxus-save",
                    onclick: move |_| {
                        let mut draft = service
                            .read()
                            .editor_snapshot(request_id)
                            .expect("selected request remains present");
                        draft.set_field("origin", origin.read().clone());
                        match service.write().save_draft(&draft) {
                            Ok(_) => message.set("Saved".to_owned()),
                            Err(issues) => {
                                let text = issues
                                    .into_iter()
                                    .map(|issue| issue.message)
                                    .collect::<Vec<_>>()
                                    .join("; ");
                                message.set(text);
                            }
                        }
                    },
                    "Save"
                }
                button {
                    id: "dioxus-close",
                    onclick: move |_| selected.set(None),
                    "Close"
                }
            }
            p { id: "dioxus-message", "{message}" }
        }
    }
}
