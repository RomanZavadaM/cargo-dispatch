//! The route in third-party map services and the figures the operator types.

use crate::i18n::UiLocale;
use crate::invoke;
use crate::orders_i18n::{K, text};
use cargo_dispatch_planner_ui_contracts::{
    FigureDraft, FiguresDraft, MapService, OpenRouteInput, OpenedRoute, PlannerRouteView,
    localize_problem, parse_distance_km, parse_drive_minutes,
};
use leptos::{prelude::*, task::spawn_local};

/// One typed value: where it is kept and when it is readable.
#[derive(Clone, Copy)]
struct Figure {
    label: K,
    mistake: K,
    read: fn(&FiguresDraft) -> &FigureDraft,
    write: fn(&mut FiguresDraft) -> &mut FigureDraft,
    readable: fn(&str) -> bool,
}

fn distance(figures: &FiguresDraft) -> &FigureDraft {
    &figures.distance_km
}

fn distance_mut(figures: &mut FiguresDraft) -> &mut FigureDraft {
    &mut figures.distance_km
}

fn drive_time(figures: &FiguresDraft) -> &FigureDraft {
    &figures.drive_time
}

fn drive_time_mut(figures: &mut FiguresDraft) -> &mut FigureDraft {
    &mut figures.drive_time
}

const DISTANCE: Figure = Figure {
    label: K::ManualKm,
    mistake: K::BadKm,
    read: distance,
    write: distance_mut,
    readable: |value| parse_distance_km(value).is_some(),
};

const DRIVE_TIME: Figure = Figure {
    label: K::ManualTime,
    mistake: K::BadTime,
    read: drive_time,
    write: drive_time_mut,
    readable: |value| parse_drive_minutes(value).is_some(),
};

/// Records that the value belongs to the route as it is now.
fn stamp(item: &mut FigureDraft, route: Option<&PlannerRouteView>) {
    if let Some(route) = route {
        item.route_id.clone_from(&route.working_route_id);
        item.revision = route.revision;
    }
}

fn figure_field(
    figure: Figure,
    locale: RwSignal<UiLocale>,
    route: RwSignal<Option<PlannerRouteView>>,
    figures: RwSignal<FiguresDraft>,
) -> impl IntoView {
    let value = move || (figure.read)(&figures.get()).value.clone();
    view! {
        <div>
            <label>
                {move || text(locale.get(), figure.label)}
                <input
                    prop:value=value
                    on:input=move |event| {
                        let entered = event_target_value(&event);
                        let current = route.get_untracked();
                        figures.update(|all| {
                            let item = (figure.write)(all);
                            item.value = entered;
                            stamp(item, current.as_ref());
                        });
                    }
                />
            </label>
            {move || {
                let typed = value();
                (!typed.trim().is_empty() && !(figure.readable)(&typed))
                    .then(|| view! { <p class="error">{text(locale.get(), figure.mistake)}</p> })
            }}
        </div>
    }
}

#[component]
pub(crate) fn RouteCheckPanel(
    locale: RwSignal<UiLocale>,
    message: RwSignal<String>,
    error: RwSignal<bool>,
    route: RwSignal<Option<PlannerRouteView>>,
    figures: RwSignal<FiguresDraft>,
) -> impl IntoView {
    let say = move |key: K| move || text(locale.get(), key);
    let address = RwSignal::new(String::new());
    // An address built for the previous route would mislead.
    Effect::new(move |_| {
        route.track();
        address.set(String::new());
    });
    let open = move |service: MapService| {
        spawn_local(async move {
            let args = serde_json::json!({ "input": OpenRouteInput { service } });
            match invoke::<OpenedRoute, _>("planner_open_route", &args).await {
                Ok(link) => {
                    address.set(link.url);
                    let note = if link.opened {
                        ""
                    } else {
                        text(locale.get_untracked(), K::BrowserFailed)
                    };
                    message.set(note.to_owned());
                    error.set(!link.opened);
                }
                Err(problem) => {
                    address.set(String::new());
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
        });
    };
    let outdated = move || {
        let Some(current) = route.get() else {
            return false;
        };
        let all = figures.get();
        [&all.distance_km, &all.drive_time]
            .iter()
            .any(|item| item.is_outdated(&current.working_route_id, current.revision))
    };
    let confirm = move |_| {
        let current = route.get_untracked();
        figures.update(|all| {
            stamp(&mut all.distance_km, current.as_ref());
            stamp(&mut all.drive_time, current.as_ref());
        });
    };
    view! {
        <section>
            <h2>{say(K::CheckTitle)}</h2>
            <p class="note">{say(K::CheckHint)}</p>
            <div class="actions">
                <button type="button" on:click=move |_| open(MapService::Google)>
                    {say(K::OpenGoogle)}
                </button>
                <button type="button" on:click=move |_| open(MapService::GraphHopper)>
                    {say(K::OpenGraphHopper)}
                </button>
            </div>
            {move || {
                (!address.get().is_empty())
                    .then(|| view! {
                        <div class="routing-grid">
                            <label>
                                {say(K::OpenedAddress)}
                                <input readonly=true prop:value=move || address.get() />
                            </label>
                        </div>
                    })
            }}
            <div class="routing-grid">
                {figure_field(DISTANCE, locale, route, figures)}
                {figure_field(DRIVE_TIME, locale, route, figures)}
            </div>
            {move || {
                outdated()
                    .then(|| view! {
                        <p class="error">{say(K::FiguresOutdated)}</p>
                        <div class="actions">
                            <button type="button" on:click=confirm>{say(K::FiguresConfirm)}</button>
                        </div>
                    })
            }}
            <p class="note">{say(K::FiguresNote)}</p>
        </section>
    }
}
