use leptos::prelude::*;
use ui_probe_core::{
    ApplicationService, EditorField, IntakeMode, LoadRequest, RequestDraft, RoutePointStatus,
    RoutePreview, SortField, UiLanguage, UiTextKey, ValidationIssue, format_ui_date,
    format_ui_integer, localized_field_label, localized_validation_issue, ui_text,
};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

fn focus_later(id: &str) {
    let _ = js_sys::eval(&format!(
        "setTimeout(() => document.getElementById('{}')?.focus(), 0)",
        id
    ));
}

fn set_document_language(language: UiLanguage) {
    let _ = js_sys::eval(&format!(
        "document.documentElement.lang = '{}'",
        language.code()
    ));
}

#[component]
fn App() -> impl IntoView {
    let service = RwSignal::new(ApplicationService::synthetic());
    let language = RwSignal::new(UiLanguage::Uk);
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

    let route_draft = RwSignal::new(service.with(|service| service.route_draft()));
    let route_preview = RwSignal::new(None::<RoutePreview>);
    let route_message = RwSignal::new(String::new());

    let map_data = service.with(|service| service.map_probe_data());
    let map_routes = map_data.routes;
    let map_points = map_data.points;

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
            <section class="toolbar" id="language-toolbar">
                <label>
                    <span>{move || ui_text(language.get(), UiTextKey::Language)}</span>
                    <select
                        id="ui-language"
                        prop:value=move || language.get().code()
                        on:change=move |event| {
                            if let Some(next) = UiLanguage::from_code(&event_target_value(&event)) {
                                language.set(next);
                                set_document_language(next);
                            }
                        }
                    >
                        <option value="uk">"Українська"</option>
                        <option value="en">"English"</option>
                        <option value="de">"Deutsch"</option>
                    </select>
                </label>
                <p id="format-sample">
                    <span>{move || ui_text(language.get(), UiTextKey::FormatSample)}</span>
                    ": "
                    <strong id="number-format-sample">{move || format_ui_integer(language.get(), 12_345)}</strong>
                    " · "
                    <strong id="date-format-sample">{move || format_ui_date(language.get(), 2026, 11, 24)}</strong>
                </p>
            </section>
            <header>
                <p class="eyebrow">{move || ui_text(language.get(), UiTextKey::PlannerEyebrow)}</p>
                <h1 id="planner-title">{move || ui_text(language.get(), UiTextKey::PlannerTitle)}</h1>
                <p>
                    {move || format!(
                        "{} {} · revision {}",
                        format_ui_integer(language.get(), summary.total_requests as u64),
                        ui_text(language.get(), UiTextKey::SyntheticRequests),
                        summary.revision
                    )}
                </p>
            </header>

            <section class="metrics" aria-label="Probe summary">
                <article>
                    <strong>{summary.total_requests}</strong>
                    <span>{move || ui_text(language.get(), UiTextKey::Requests)}</span>
                </article>
                <article>
                    <strong>{summary.reserved_requests}</strong>
                    <span>{move || ui_text(language.get(), UiTextKey::Reserved)}</span>
                </article>
                <article>
                    <strong>{summary.assigned_requests}</strong>
                    <span>{move || ui_text(language.get(), UiTextKey::Assigned)}</span>
                </article>
                <article>
                    <strong id="selected-count">{move || selected.with(|ids| ids.len())}</strong>
                    <span>{move || ui_text(language.get(), UiTextKey::Selected)}</span>
                </article>
            </section>

            <section class="pool-section">
                <div class="toolbar">
                    <label>
                        <span>{move || ui_text(language.get(), UiTextKey::Filter)}</span>
                        <input
                            id="pool-filter"
                            type="search"
                            placeholder="Berlin, API, ID…"
                            on:input=move |event| filter.set(event_target_value(&event))
                        />
                    </label>
                    <div class="sorts" aria-label="Sort controls">
                        <button id="sort-age" type="button" on:click=move |_| set_sort(SortField::AgeMinutes)>
                            {move || ui_text(language.get(), UiTextKey::SortAge)}
                        </button>
                        <button id="sort-rate" type="button" on:click=move |_| set_sort(SortField::RateEur)>
                            {move || ui_text(language.get(), UiTextKey::SortRate)}
                        </button>
                    </div>
                    <p class="pool-status">
                        <strong id="pool-count">{move || rows.with(|rows| rows.len())}</strong>
                        " "
                        {move || ui_text(language.get(), UiTextKey::VisibleRows)}
                    </p>
                </div>

                <p class="note">
                    "The full filtered set is rendered. Filter, sort and form validation stay outside UI widgets."
                </p>

                <div class="table-wrap pool-scroll" id="pool-scroll" tabindex="0">
                    <table>
                        <thead>
                            <tr>
                                <th id="table-select">{move || ui_text(language.get(), UiTextKey::Select)}</th>
                                <th id="table-open">{move || ui_text(language.get(), UiTextKey::Open)}</th>
                                <th>"ID"</th>
                                <th id="table-age">{move || ui_text(language.get(), UiTextKey::Age)}</th>
                                <th id="table-source">{move || ui_text(language.get(), UiTextKey::Source)}</th>
                                <th id="table-origin">{move || ui_text(language.get(), UiTextKey::Origin)}</th>
                                <th id="table-destination">{move || ui_text(language.get(), UiTextKey::Destination)}</th>
                                <th>"kg"</th>
                                <th id="table-rate">{move || ui_text(language.get(), UiTextKey::Rate)}</th>
                                <th id="table-confidence">{move || ui_text(language.get(), UiTextKey::Confidence)}</th>
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
                                                    aria-label=move || format!(
                                                        "{} {}",
                                                        ui_text(language.get(), UiTextKey::Select),
                                                        id
                                                    )
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
                                                    aria-label=move || format!(
                                                        "{} {}",
                                                        ui_text(language.get(), UiTextKey::Open),
                                                        id
                                                    )
                                                    on:click=move |_| opened.set(Some(id))
                                                >
                                                    {move || ui_text(language.get(), UiTextKey::Open)}
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
                                        focus_later("editor-origin");
                                    }
                                }
                            >
                                {move || ui_text(language.get(), UiTextKey::EditRequest)}
                            </button>
                        </aside>
                    }
                })}

                <Show when=move || editor_draft.get().is_some()>
                    <section
                        id="request-editor"
                        class="editor"
                        aria-label=move || ui_text(language.get(), UiTextKey::RequestEditor)
                        on:keydown=move |event| {
                            if event.key() == "Escape" {
                                event.prevent_default();
                                if show_unsaved_warning.get_untracked() {
                                    show_unsaved_warning.set(false);
                                    focus_later("editor-close");
                                } else if editor_dirty.get_untracked() {
                                    show_unsaved_warning.set(true);
                                    focus_later("keep-editing");
                                } else {
                                    editor_draft.set(None);
                                    editor_baseline.set(None);
                                    editor_issues.set(Vec::new());
                                    editor_message.set(None);
                                    show_unsaved_warning.set(false);
                                    focus_later("edit-request");
                                }
                            }
                        }
                    >
                        <div class="editor-head">
                            <div>
                                <p class="eyebrow">{move || ui_text(language.get(), UiTextKey::RequestEditor)}</p>
                                <h2>{move || {
                                    editor_draft
                                        .get()
                                        .map(|draft| format!("Request #{}", draft.request_id))
                                        .unwrap_or_default()
                                }}</h2>
                            </div>
                            <button
                                id="editor-x"
                                class="editor-x"
                                type="button"
                                aria-label=move || ui_text(language.get(), UiTextKey::Close)
                                title=move || ui_text(language.get(), UiTextKey::Close)
                                on:click=move |_| {
                                    if editor_dirty.get() {
                                        show_unsaved_warning.set(true);
                                        focus_later("keep-editing");
                                    } else {
                                        editor_draft.set(None);
                                        editor_baseline.set(None);
                                        editor_issues.set(Vec::new());
                                        editor_message.set(None);
                                        show_unsaved_warning.set(false);
                                        focus_later("edit-request");
                                    }
                                }
                            >
                                "×"
                            </button>
                            <div class="editor-origin">
                                <span>
                                    {move || ui_text(language.get(), UiTextKey::SourceLabel)}
                                    " "
                                    <strong id="editor-source">{move || {
                                        editor_draft
                                            .get()
                                            .map(|draft| draft.source)
                                            .unwrap_or_default()
                                    }}</strong>
                                </span>
                                <span>
                                    {move || ui_text(language.get(), UiTextKey::ConfidenceLabel)}
                                    " "
                                    <strong id="editor-confidence">{move || {
                                        editor_draft
                                            .get()
                                            .map(|draft| format!("{}%", draft.confidence_percent))
                                            .unwrap_or_default()
                                    }}</strong>
                                </span>
                                <strong id="editor-dirty">
                                    {move || if editor_dirty.get() {
                                        ui_text(language.get(), UiTextKey::UnsavedChanges)
                                    } else {
                                        ui_text(language.get(), UiTextKey::SavedState)
                                    }}
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
                                {move || ui_text(language.get(), UiTextKey::Manual)}
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
                                {move || ui_text(language.get(), UiTextKey::PhoneOrder)}
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
                                    let label_key = field.key.clone();
                                    let english_label = field.label.clone();
                                    let input_id = format!("editor-{}", field.key);
                                    let required = field.required;
                                    view! {
                                        <label class="editor-field-wrap">
                                            <span>
                                                {move || localized_field_label(
                                                    language.get(),
                                                    &label_key,
                                                    &english_label,
                                                )}
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
                                <strong>{move || ui_text(language.get(), UiTextKey::CorrectForm)}</strong>
                                <ul>
                                    <For
                                        each=move || editor_issues.get()
                                        key=|issue| format!("{}:{}", issue.field_key, issue.message)
                                        children=move |issue: ValidationIssue| {
                                            let field_key = issue.field_key.clone();
                                            let localized_issue = issue.clone();
                                            view! {
                                                <li data-field=field_key>
                                                    {move || localized_validation_issue(
                                                        language.get(),
                                                        &localized_issue,
                                                    )}
                                                </li>
                                            }
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
                            <div
                                id="unsaved-warning"
                                class="unsaved-warning"
                                role="alertdialog"
                                aria-modal="true"
                                aria-labelledby="unsaved-title"
                                aria-describedby="unsaved-description"
                            >
                                <strong id="unsaved-title">{move || ui_text(language.get(), UiTextKey::ChangesNotSaved)}</strong>
                                <span id="unsaved-description">{move || ui_text(language.get(), UiTextKey::CloseWithoutSaving)}</span>
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
                                            focus_later("edit-request");
                                        }
                                    >
                                        {move || ui_text(language.get(), UiTextKey::DiscardClose)}
                                    </button>
                                    <button
                                        id="keep-editing"
                                        type="button"
                                        on:click=move |_| {
                                            show_unsaved_warning.set(false);
                                            focus_later("editor-close");
                                        }
                                    >
                                        {move || ui_text(language.get(), UiTextKey::ContinueEditing)}
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
                                    editor_message.set(Some(
                                        ui_text(language.get_untracked(), UiTextKey::Saved).to_owned(),
                                    ));
                                    show_unsaved_warning.set(false);
                                    opened.set(Some(draft.request_id));
                                }
                            >
                                {move || ui_text(language.get(), UiTextKey::Save)}
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
                                    focus_later("edit-request");
                                }
                            >
                                {move || ui_text(language.get(), UiTextKey::Cancel)}
                            </button>
                            <button
                                id="editor-close"
                                type="button"
                                on:click=move |_| {
                                    if editor_dirty.get() {
                                        show_unsaved_warning.set(true);
                                        focus_later("keep-editing");
                                    } else {
                                        editor_draft.set(None);
                                        editor_baseline.set(None);
                                        editor_issues.set(Vec::new());
                                        editor_message.set(None);
                                        show_unsaved_warning.set(false);
                                        focus_later("edit-request");
                                    }
                                }
                            >
                                {move || ui_text(language.get(), UiTextKey::Close)}
                            </button>
                        </div>
                    </section>
                </Show>

                <section id="route-panel" class="route-panel" aria-label="Working route">
                    <div class="route-head">
                        <div>
                            <p class="eyebrow">{move || ui_text(language.get(), UiTextKey::WorkingRoute)}</p>
                            <h2>{move || ui_text(language.get(), UiTextKey::RouteDraft)}</h2>
                        </div>
                        <div class="route-state">
                            <span>
                                "Saved revision: "
                                <strong id="saved-route-revision">
                                    {move || service.with(|service| service.route_snapshot().revision)}
                                </strong>
                            </span>
                            <span>
                                "Saved points: "
                                <strong id="saved-route-count">
                                    {move || service.with(|service| service.route_snapshot().points.len())}
                                </strong>
                            </span>
                            <span>
                                "Draft points: "
                                <strong id="draft-route-count">
                                    {move || route_draft.with(|draft| draft.points.len())}
                                </strong>
                            </span>
                        </div>
                    </div>

                    <div class="route-metrics" aria-label="Draft route metrics">
                        <span>
                            "Distance "
                            <strong id="draft-distance">
                                {move || route_draft.with(|draft| draft.metrics.distance_km)}
                            </strong>
                            " km"
                        </span>
                        <span>
                            "Empty "
                            <strong id="draft-empty-km">
                                {move || route_draft.with(|draft| draft.metrics.empty_km)}
                            </strong>
                            " km"
                        </span>
                        <span>
                            "Revenue €"
                            <strong id="draft-revenue">
                                {move || route_draft.with(|draft| draft.metrics.revenue_eur)}
                            </strong>
                        </span>
                        <span>
                            "Margin €"
                            <strong id="draft-margin">
                                {move || route_draft.with(|draft| draft.metrics.margin_eur)}
                            </strong>
                        </span>
                        <span>
                            "Risks "
                            <strong id="draft-risk-count">
                                {move || route_draft.with(|draft| draft.metrics.risk_count)}
                            </strong>
                        </span>
                    </div>

                    <div id="route-points" class="route-points">
                        <For
                            each=move || route_draft.with(|draft| draft.points.clone())
                            key=|point| {
                                (
                                    point.id,
                                    point.position,
                                    point.locked,
                                    point.status,
                                    point.cargo_id,
                                )
                            }
                            children=move |point| {
                                let id = point.id;
                                let cargo_id = point.cargo_id;
                                let locked = point.locked;
                                let status = match point.status {
                                    RoutePointStatus::Confirmed => "confirmed",
                                    RoutePointStatus::Tentative => "tentative",
                                };
                                let risks = if point.risk_flags.is_empty() {
                                    "none".to_owned()
                                } else {
                                    point.risk_flags.join(", ")
                                };
                                let call_task = point
                                    .call_task
                                    .clone()
                                    .unwrap_or_else(|| "—".to_owned());

                                view! {
                                    <article
                                        class="route-point"
                                        data-point-id=id.to_string()
                                        data-status=status
                                        data-locked=locked.to_string()
                                    >
                                        <div class="route-point-main">
                                            <strong class="route-position">{point.position}</strong>
                                            <div>
                                                <strong class="route-label">{point.label}</strong>
                                                <span class="route-point-meta">
                                                    {format!("{status} · risks: {risks} · call: {call_task}")}
                                                </span>
                                            </div>
                                        </div>
                                        <div class="route-actions">
                                            <button
                                                class="move-up"
                                                type="button"
                                                on:click=move |_| {
                                                    let mut next = route_draft.get_untracked();
                                                    let result = service.with(|service| {
                                                        service.move_route_point(&mut next, id, -1)
                                                    });
                                                    if result.applied {
                                                        route_draft.set(next);
                                                    }
                                                    route_preview.set(None);
                                                    route_message.set(result.message);
                                                }
                                            >
                                                "Up"
                                            </button>
                                            <button
                                                class="move-down"
                                                type="button"
                                                on:click=move |_| {
                                                    let mut next = route_draft.get_untracked();
                                                    let result = service.with(|service| {
                                                        service.move_route_point(&mut next, id, 1)
                                                    });
                                                    if result.applied {
                                                        route_draft.set(next);
                                                    }
                                                    route_preview.set(None);
                                                    route_message.set(result.message);
                                                }
                                            >
                                                "Down"
                                            </button>
                                            <button
                                                class="toggle-lock"
                                                type="button"
                                                on:click=move |_| {
                                                    let mut next = route_draft.get_untracked();
                                                    let result = service.with(|service| {
                                                        service.set_route_lock(&mut next, id, !locked)
                                                    });
                                                    if result.applied {
                                                        route_draft.set(next);
                                                    }
                                                    route_preview.set(None);
                                                    route_message.set(result.message);
                                                }
                                            >
                                                {if locked { "Unlock" } else { "Lock" }}
                                            </button>
                                            {cargo_id.map(|cargo_id| view! {
                                                <button
                                                    class="exclude-cargo"
                                                    type="button"
                                                    data-cargo-id=cargo_id.to_string()
                                                    on:click=move |_| {
                                                        let mut next = route_draft.get_untracked();
                                                        let result = service.with(|service| {
                                                            service.exclude_route_cargo(&mut next, cargo_id)
                                                        });
                                                        if result.applied {
                                                            route_draft.set(next);
                                                        }
                                                        route_preview.set(None);
                                                        route_message.set(result.message);
                                                    }
                                                >
                                                    "Exclude cargo"
                                                </button>
                                            })}
                                        </div>
                                    </article>
                                }
                            }
                        />
                    </div>

                    <p id="route-message" class="route-message" aria-live="polite">
                        {move || route_message.get()}
                    </p>

                    <div class="route-footer-actions">
                        <button
                            id="undo-route"
                            type="button"
                            on:click=move |_| {
                                route_draft.set(service.with(|service| service.route_draft()));
                                route_preview.set(None);
                                route_message.set("Unsaved route draft restored".to_owned());
                            }
                        >
                            "Undo draft"
                        </button>
                        <button
                            id="recalculate-route"
                            type="button"
                            on:click=move |_| {
                                let draft = route_draft.get_untracked();
                                let preview = service.with(|service| service.recalculate_route(&draft));
                                route_preview.set(Some(preview));
                                route_message.set("Recalculation ready for review".to_owned());
                            }
                        >
                            "Recalculate"
                        </button>
                    </div>

                    <Show when=move || route_preview.get().is_some()>
                        <aside id="route-preview" class="route-preview">
                            <h3>"Recalculation preview"</h3>
                            <div class="route-metrics">
                                <span>
                                    "Before €"
                                    <strong id="preview-before-revenue">
                                        {move || {
                                            route_preview
                                                .get()
                                                .map(|preview| preview.before_metrics.revenue_eur)
                                                .unwrap_or_default()
                                        }}
                                    </strong>
                                </span>
                                <span>
                                    "After €"
                                    <strong id="preview-after-revenue">
                                        {move || {
                                            route_preview
                                                .get()
                                                .map(|preview| preview.after_metrics.revenue_eur)
                                                .unwrap_or_default()
                                        }}
                                    </strong>
                                </span>
                                <span>
                                    "Excluded cargo "
                                    <strong id="preview-excluded-count">
                                        {move || {
                                            route_preview
                                                .get()
                                                .map(|preview| preview.excluded_cargo_ids.len())
                                                .unwrap_or_default()
                                        }}
                                    </strong>
                                </span>
                                <span>
                                    "Moved points "
                                    <strong id="preview-moved-count">
                                        {move || {
                                            route_preview
                                                .get()
                                                .map(|preview| preview.changed_order.len())
                                                .unwrap_or_default()
                                        }}
                                    </strong>
                                </span>
                            </div>
                            <button
                                id="accept-route"
                                type="button"
                                on:click=move |_| {
                                    let Some(preview) = route_preview.get_untracked() else {
                                        return;
                                    };
                                    let mut next_service = service.get_untracked();
                                    match next_service.accept_route_preview(&preview) {
                                        Ok(_) => {
                                            let fresh = next_service.route_draft();
                                            service.set(next_service);
                                            route_draft.set(fresh);
                                            route_preview.set(None);
                                            route_message.set("Recalculation accepted".to_owned());
                                        }
                                        Err(message) => {
                                            route_message.set(message);
                                        }
                                    }
                                }
                            >
                                "Accept recalculation"
                            </button>
                        </aside>
                    </Show>
                </section>


                <section id="map-probe" class="map-probe" aria-label="Map bridge">
                    <div class="route-head">
                        <div>
                            <p class="eyebrow">"Map"</p>
                            <h2>"Route alternatives and manual point"</h2>
                        </div>
                        <div class="route-state">
                            <span>
                                "Routes: "
                                <strong id="map-route-count">{map_routes.len()}</strong>
                            </span>
                            <span>
                                "Points: "
                                <strong id="map-point-count">{map_points.len()}</strong>
                            </span>
                            <span>
                                "Map generation: "
                                <strong id="map-generation">"0"</strong>
                            </span>
                        </div>
                    </div>

                    <div class="map-layout">
                        <div class="map-sidebar">
                            <div class="map-route-choices" aria-label="Route alternatives">
                                <For
                                    each=move || map_routes.clone()
                                    key=|route| route.id.clone()
                                    children=move |route| {
                                        let coordinates = route
                                            .coordinates
                                            .iter()
                                            .map(|point| format!("{},{}", point.lng, point.lat))
                                            .collect::<Vec<_>>()
                                            .join(";");
                                        view! {
                                            <button
                                                class="map-route-choice"
                                                type="button"
                                                data-route-id=route.id.clone()
                                                data-coordinates=coordinates
                                                data-selected=(route.id == "route-a").to_string()
                                            >
                                                {route.label}
                                            </button>
                                        }
                                    }
                                />
                            </div>

                            <div id="map-point-list" class="map-point-list">
                                <For
                                    each=move || map_points.clone()
                                    key=|point| point.id.clone()
                                    children=move |point| view! {
                                        <button
                                            class="map-point-choice"
                                            type="button"
                                            data-point-id=point.id.clone()
                                            data-lng=point.coordinate.lng.to_string()
                                            data-lat=point.coordinate.lat.to_string()
                                            data-manual=point.manual.to_string()
                                            data-selected="false"
                                        >
                                            {point.label}
                                        </button>
                                    }
                                />
                            </div>
                        </div>

                        <div>
                            <div id="maplibre-container" class="maplibre-container"></div>
                            <div class="map-status">
                                <span>
                                    "Selected route: "
                                    <strong id="map-selected-route">"route-a"</strong>
                                </span>
                                <span>
                                    "Selected point: "
                                    <strong id="map-selected-point"></strong>
                                </span>
                                <span>
                                    "Manual point: "
                                    <strong id="map-manual-coordinate">"24.030000, 49.015000"</strong>
                                </span>
                            </div>
                            <div class="route-footer-actions">
                                <button id="map-fit-all" type="button">"Fit all"</button>
                                <button id="map-remount" type="button">"Reopen map"</button>
                            </div>
                        </div>
                    </div>
                </section>

            </section>
        </main>
    }
}
