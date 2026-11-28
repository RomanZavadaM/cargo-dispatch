//! One field for what the operator copies in Google Maps.

use crate::i18n::UiLocale;
use crate::invoke;
use crate::orders_i18n::{K, text};
use cargo_dispatch_planner_ui_contracts::{PlaceInput, PlaceView, localize_problem};
use leptos::{prelude::*, task::spawn_local};

/// A pasted text arrives at once; typing adds one character at a time.
const PASTED_AT_ONCE: usize = 8;

/// Reads the point from the pasted text and hands it to `take`.
pub(crate) fn paste_field(
    locale: RwSignal<UiLocale>,
    message: RwSignal<String>,
    error: RwSignal<bool>,
    take: impl Fn(PlaceView) + Copy + 'static,
) -> impl IntoView {
    let typed = RwSignal::new(String::new());
    let resolve = move |entered: String| {
        if entered.trim().is_empty() {
            return;
        }
        spawn_local(async move {
            let args = serde_json::json!({ "input": PlaceInput { text: entered } });
            match invoke::<PlaceView, _>("planner_place_resolve", &args).await {
                Ok(found) => {
                    // The field may be gone by now; the point is still taken.
                    let _ = typed.try_set(String::new());
                    take(found);
                    let done = text(locale.get_untracked(), K::PointTaken);
                    message.set(done.to_owned());
                    error.set(false);
                }
                Err(problem) => {
                    message.set(localize_problem(locale.get_untracked().code(), &problem));
                    error.set(true);
                }
            }
        });
    };
    view! {
        <label>
            {move || text(locale.get(), K::PasteMap)}
            <input
                prop:value=move || typed.get()
                on:input=move |event| {
                    let entered = event_target_value(&event);
                    let before = typed.get_untracked().chars().count();
                    let pasted = entered.chars().count() >= before + PASTED_AT_ONCE;
                    typed.set(entered.clone());
                    if pasted {
                        resolve(entered);
                    }
                }
                on:change=move |event| resolve(event_target_value(&event))
            />
        </label>
    }
}
