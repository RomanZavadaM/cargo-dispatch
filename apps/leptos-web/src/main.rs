use leptos::prelude::*;
use ui_probe_core::{ApplicationService, SortField};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let service = ApplicationService::synthetic();
    let summary = service.summary();
    let preview = service.query("", SortField::AgeMinutes, false, Some(25));

    view! {
        <main class="shell" data-probe="leptos">
            <header>
                <p class="eyebrow">"Cargo dispatch · UI probe"</p>
                <h1>"Synthetic planner pool"</h1>
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
            </section>

            <section>
                <h2>"Oldest 25 — preview only"</h2>
                <p class="note">
                    "P01 performance gate is not claimed yet; this foundation proves the shared application boundary and browser build."
                </p>
                <div class="table-wrap">
                    <table>
                        <thead>
                            <tr>
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
                                each=move || preview.clone()
                                key=|row| row.id
                                children=move |row| {
                                    view! {
                                        <tr data-row-id=row.id>
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
            </section>
        </main>
    }
}
