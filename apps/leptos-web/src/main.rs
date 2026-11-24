use leptos::prelude::*;
use ui_probe_core::{
    ApplicationService, EditorField, IntakeMode, LoadRequest, RequestDraft, SortField,
    ValidationIssue,
};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let service = RwSignal::new(ApplicationService::synthetic());
    let summary = service.with(ApplicationService::summary);

    let filter = RwSignal::new(String::new());
    let sort = RwSignal::new(SortField::AgeMinutes);
    let ascending = RwSignal::new(false);
    let selected = RwSignal::new(Vec::<u64>::new());
    let opened = RwSignal::new(None::<u64>);

    let editor_baseline = RwSignal::new(None::<RequestDraft>);
    let editor_draft = RwSignal::new(None::<RequestDraft>);
    let editor_issues = RwSignal::new(Vec::<ValidationIssue>::new());
    let editor_message = RwSignal::new(None::<String>);
    let show_unsaved_warning = RwSignal::new(false);

    let rows = Memo::new(move |_| {
        service.with(|service| service.query(&filter.get(), sort.get(), ascending.get(), None))
    });

    let opened_row = Memo::new(move |_| {
        opened
            .get()
            .and_then(|id| service.with(|service| service.get_request(id)))
    });

    let editor_dirty = Memo::new(move |_| editor_baseline.get() != editor_draft.get());

    let set_sort = move |field: SortField| {
        if sort.get_untracked() == field {
            ascending.update(|value| *value = !*value);
        } else {
            sort.set(field);
            ascending.set(false);
        }
    };

    view! {
        <main class="shell" data-probe="leptos">
            <header>
                <p class="eyebrow">"Cargo dispatch · UI probe"</p>
                <h1>"Planner pool and request editor"</h1>
                <p>
                    {format!(
                        "{} synthetic requests · revision {}",
                        summary.total_requests, summary.revision
                    )}
                </p>
            </header>

            <section class="metrics" aria-label="Probe summary">
                <article>
                    <strong>{summary.total_requests}</strong>
                    <span>"requests"</span>
                </article>
                <article>
                    <strong>{summary.reserved_requests}</strong>
                    <span>"reserved"</span>
                </article>
                <article>
                    <strong>{summary.assigned_requests}</strong>
                    <span>"assigned"</span>
                </article>
                <article>
                    <strong id="selected-count">{move || selected.with(|ids| ids.len())}</strong>
                    <span>"selected"</span>
                </article>
            </section>

            <section class="pool-section">
                <div class="toolbar">
                    <label>
                        <span>"Filter"</span>
                        <input
                            id="pool-filter"
                            type="search"
                            placeholder="Berlin, API, ID…"
                            on:input=move |event| filter.set(event_target_value(&event))
                        />
                    </label>
                    <div class="sorts" aria-label="Sort controls">
                        <button id="sort-age" type="button" on:click=move |_| set_sort(SortField::AgeMinutes)>
                            "Sort age"
                        </button>
                        <button id="sort-rate" type="button" on:click=move |_| set_sort(SortField::RateEur)>
                            "Sort rate"
                        </button>
                    </div>
                    <p class="pool-status">
                        <strong id="pool-count">{move || rows.with(|rows| rows.len())}</strong>
                        " visible rows"
                    </p>
                </div>

                <p class="note">
                    "The full filtered set is rendered. Filter, sort and form validation stay outside UI widgets."
                </p>

                <div class="table-wrap pool-scroll" id="pool-scroll" tabindex="0">
                    <table>
                        <thead>
                            <tr>
                                <th>"Select"</th>
                                <th>"Open"</th>
                                <th>"ID"</th>
                                <th>"Age"</th>
                                <th>"Source"</th>
                                <th>"Origin"</th>
                                <th>"Destination"</th>
                                <th>"kg"</th>
                                <th>"Rate €"</th>
                                <th>"Confidence"</th>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || rows.get()
                                key=|row| {
                                    (
                                        row.id,
                                        row.age_minutes,
                                        row.source.clone(),
                                        row.origin.clone(),
                                        row.destination.clone(),
                                        row.weight_kg,
                                        row.rate_eur,
                                        row.confidence_percent,
                                    )
                                }
                                children=move |row: LoadRequest| {
                                    let id = row.id;
                                    let rate = row.rate_eur.to_string();
                                    let age = row.age_minutes.to_string();
                                    view! {
                                        <tr
                                            class="pool-row"
                                            data-row-id=id.to_string()
                                            data-rate=rate
                                            data-age=age
                                        >
                                            <td>
                                                <input
                                                    class="row-select"
                                                    type="checkbox"
                                                    aria-label=format!("Select request {id}")
                                                    prop:checked=move || selected.with(|ids| ids.contains(&id))
                                                    on:change=move |event| {
                                                        let checked = event_target_checked(&event);
                                                        selected.update(|ids| {
                                                            if checked {
                                                                if !ids.contains(&id) {
                                                                    ids.push(id);
                                                                }
                                                            } else {
                                                                ids.retain(|selected_id| *selected_id != id);
                                                            }
                                                        });
                                                    }
                                                />
                                            </td>
                                            <td>
                                                <button
                                                    class="open-row"
                                                    type="button"
                                                    aria-label=format!("Open request {id}")
                                                    on:click=move |_| opened.set(Some(id))
                                                >
                                                    "Open"
                                                </button>
                                            </td>
                                            <td>{row.id}</td>
                                            <td>{row.age_minutes}</td>
                                            <td class="source-cell">{row.source}</td>
                                            <td class="origin-cell">{row.origin}</td>
                                            <td class="destination-cell">{row.destination}</td>
                                            <td>{row.weight_kg}</td>
                                            <td>{row.rate_eur}</td>
                                            <td>{format!("{}%", row.confidence_percent)}</td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </div>

                {move || opened_row.get().map(|row| {
                    let id = row.id;
                    view! {
                        <aside id="row-detail" class="row-detail" aria-live="polite">
                            <strong>{format!("Request #{}", row.id)}</strong>
                            <span>{format!("{} → {}", row.origin, row.destination)}</span>
                            <span>{format!("{} kg · €{} · {} min old", row.weight_kg, row.rate_eur, row.age_minutes)}</span>
                            <button
                                id="edit-request"
                                type="button"
                                on:click=move |_| {
                                    if let Some(draft) = service.with(|service| service.editor_snapshot(id)) {
                                        editor_baseline.set(Some(draft.clone()));
                                        editor_draft.set(Some(draft));
                                        editor_issues.set(Vec::new());
                                        editor_message.set(None);
                                        show_unsaved_warning.set(false);
                                    }
                                }
                            >
                                "Edit request"
                            </button>
                        </aside>
                    }
                })}

                <Show when=move || editor_draft.get().is_some()>
                    <section id="request-editor" class="editor" aria-label="Request editor">
                        <div class="editor-head">
                            <div>
                                <p class="eyebrow">"Request editor"</p>
                                <h2>{move || {
                                    editor_draft
                                        .get()
                                        .map(|draft| format!("Request #{}", draft.request_id))
                                        .unwrap_or_default()
                                }}</h2>
                            </div>
                            <div class="editor-origin">
                                <span>
                                    "Source: "
                                    <strong id="editor-source">{move || {
                                        editor_draft
                                            .get()
                                            .map(|draft| draft.source)
                                            .unwrap_or_default()
                                    }}</strong>
                                </span>
                                <span>
                                    "Confidence: "
                                    <strong id="editor-confidence">{move || {
                                        editor_draft
                                            .get()
                                            .map(|draft| format!("{}%", draft.confidence_percent))
                                            .unwrap_or_default()
                                    }}</strong>
                                </span>
                                <strong id="editor-dirty">
                                    {move || if editor_dirty.get() { "Unsaved changes" } else { "Saved state" }}
                                </strong>
                            </div>
                        </div>

                        <div class="mode-switch" aria-label="Intake mode">
                            <button
                                id="mode-manual"
                                type="button"
                                aria-pressed=move || {
                                    editor_draft
                                        .get()
                                        .is_some_and(|draft| draft.mode == IntakeMode::Manual)
                                        .to_string()
                                }
                                on:click=move |_| {
                                    editor_draft.update(|current| {
                                        if let Some(draft) = current {
                                            draft.mode = IntakeMode::Manual;
                                        }
                                    });
                                    editor_message.set(None);
                                }
                            >
                                "Manual"
                            </button>
                            <button
                                id="mode-phone"
                                type="button"
                                aria-pressed=move || {
                                    editor_draft
                                        .get()
                                        .is_some_and(|draft| draft.mode == IntakeMode::Phone)
                                        .to_string()
                                }
                                on:click=move |_| {
                                    editor_draft.update(|current| {
                                        if let Some(draft) = current {
                                            draft.mode = IntakeMode::Phone;
                                        }
                                    });
                                    editor_message.set(None);
                                }
                            >
                                "Phone order"
                            </button>
                        </div>

                        <div class="editor-grid">
                            <For
                                each=move || {
                                    editor_draft
                                        .get()
                                        .map(|draft| draft.visible_fields())
                                        .unwrap_or_default()
                                }
                                key=|field| field.key.clone()
                                children=move |field: EditorField| {
                                    let key = field.key.clone();
                                    let input_id = format!("editor-{}", field.key);
                                    let required = field.required;
                                    view! {
                                        <label class="editor-field-wrap">
                                            <span>
                                                {field.label}
                                                {if required { " *" } else { "" }}
                                            </span>
                                            <input
                                                id=input_id
                                                class="editor-field"
                                                data-field-key=key.clone()
                                                prop:value=field.value
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    editor_draft.update(|current| {
                                                        if let Some(draft) = current {
                                                            draft.set_field(&key, value);
                                                        }
                                                    });
                                                    editor_message.set(None);
                                                }
                                            />
                                        </label>
                                    }
                                }
                            />
                        </div>

                        <Show when=move || !editor_issues.get().is_empty()>
                            <div id="validation-errors" class="validation-errors" role="alert">
                                <strong>"Please correct the form:"</strong>
                                <ul>
                                    <For
                                        each=move || editor_issues.get()
                                        key=|issue| format!("{}:{}", issue.field_key, issue.message)
                                        children=move |issue: ValidationIssue| view! {
                                            <li data-field=issue.field_key>{issue.message}</li>
                                        }
                                    />
                                </ul>
                            </div>
                        </Show>

                        <Show when=move || editor_message.get().is_some()>
                            <p id="editor-saved" class="saved-message">
                                {move || editor_message.get().unwrap_or_default()}
                            </p>
                        </Show>

                        <Show when=move || show_unsaved_warning.get()>
                            <div id="unsaved-warning" class="unsaved-warning" role="alertdialog">
                                <strong>"Changes are not saved."</strong>
                                <span>"Close without saving, or continue editing."</span>
                                <div>
                                    <button
                                        id="discard-close"
                                        type="button"
                                        on:click=move |_| {
                                            editor_draft.set(None);
                                            editor_baseline.set(None);
                                            editor_issues.set(Vec::new());
                                            editor_message.set(None);
                                            show_unsaved_warning.set(false);
                                        }
                                    >
                                        "Discard and close"
                                    </button>
                                    <button
                                        id="keep-editing"
                                        type="button"
                                        on:click=move |_| show_unsaved_warning.set(false)
                                    >
                                        "Continue editing"
                                    </button>
                                </div>
                            </div>
                        </Show>

                        <div class="editor-actions">
                            <button
                                id="editor-save"
                                type="button"
                                on:click=move |_| {
                                    let Some(draft) = editor_draft.get() else {
                                        return;
                                    };
                                    let issues = service.with(|service| service.validate_draft(&draft));
                                    if !issues.is_empty() {
                                        editor_issues.set(issues);
                                        editor_message.set(None);
                                        return;
                                    }

                                    service.update(|service| {
                                        service
                                            .save_draft(&draft)
                                            .expect("draft was validated immediately before save");
                                    });
                                    let fresh = service.with(|service| service.editor_snapshot(draft.request_id));
                                    editor_baseline.set(fresh.clone());
                                    editor_draft.set(fresh);
                                    editor_issues.set(Vec::new());
                                    editor_message.set(Some("Saved".to_owned()));
                                    show_unsaved_warning.set(false);
                                    opened.set(Some(draft.request_id));
                                }
                            >
                                "Save"
                            </button>
                            <button
                                id="editor-cancel"
                                type="button"
                                on:click=move |_| {
                                    editor_draft.set(None);
                                    editor_baseline.set(None);
                                    editor_issues.set(Vec::new());
                                    editor_message.set(None);
                                    show_unsaved_warning.set(false);
                                }
                            >
                                "Cancel"
                            </button>
                            <button
                                id="editor-close"
                                type="button"
                                on:click=move |_| {
                                    if editor_dirty.get() {
                                        show_unsaved_warning.set(true);
                                    } else {
                                        editor_draft.set(None);
                                        editor_baseline.set(None);
                                        editor_issues.set(Vec::new());
                                        editor_message.set(None);
                                        show_unsaved_warning.set(false);
                                    }
                                }
                            >
                                "Close"
                            </button>
                        </div>
                    </section>
                </Show>
            </section>
        </main>
    }
}
