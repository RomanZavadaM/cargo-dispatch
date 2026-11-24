use leptos::prelude::*;
use ui_probe_core::{ApplicationService, LoadRequest, SortField};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let service = StoredValue::new(ApplicationService::synthetic());
    let summary = service.with_value(ApplicationService::summary);

    let filter = RwSignal::new(String::new());
    let sort = RwSignal::new(SortField::AgeMinutes);
    let ascending = RwSignal::new(false);
    let selected = RwSignal::new(Vec::<u64>::new());
    let opened = RwSignal::new(None::<u64>);

    let rows = Memo::new(move |_| {
        service.with_value(|service| {
            service.query(&filter.get(), sort.get(), ascending.get(), None)
        })
    });

    let opened_row = Memo::new(move |_| {
        opened
            .get()
            .and_then(|id| service.with_value(|service| service.get_request(id)))
    });

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
                <p class="eyebrow">"Cargo dispatch · UI probe · P01"</p>
                <h1>"5,000-request planner pool"</h1>
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
                    "P01 intentionally renders the full filtered set. Filter/sort are executed by the shared application service, not reimplemented in Leptos."
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
                                key=|row| row.id
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
                                            <td>{row.source}</td>
                                            <td>{row.origin}</td>
                                            <td>{row.destination}</td>
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
                    view! {
                        <aside id="row-detail" class="row-detail" aria-live="polite">
                            <strong>{format!("Request #{}", row.id)}</strong>
                            <span>{format!("{} → {}", row.origin, row.destination)}</span>
                            <span>{format!("{} kg · €{} · {} min old", row.weight_kg, row.rate_eur, row.age_minutes)}</span>
                        </aside>
                    }
                })}
            </section>
        </main>
    }
}
