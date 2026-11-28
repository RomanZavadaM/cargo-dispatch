//! Orders, vehicle and route variants: the first section of the Planner window.

use crate::i18n::UiLocale;
use crate::orders_i18n::{K, text};
use crate::package_draft::{cargo_rows, profile_from, snapshot};
use crate::{CargoDraft, RoutingProfileDraft, invoke};
use cargo_dispatch_planner_ui_contracts::{
    AppliedPlanView, ApplyPlanInput, InputOrder, InputPlace, InputProblem, InputProblemKind, Money,
    PackageDraft, PlanKind, PlanProposalSetView, PlanProposalView, PlanStopAction, PlanStopRef,
    PlannerPackageCargoInput, PlannerRouteView, PlannerRoutingProfileInput, PlannerTollPreference,
    PlanningInput, PlanningStateView, ProposeInput, RoadState, SkipReason, VehicleType,
    localize_problem,
};
use leptos::{prelude::*, task::spawn_local};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
struct Panel {
    locale: RwSignal<UiLocale>,
    message: RwSignal<String>,
    error: RwSignal<bool>,
    busy: RwSignal<bool>,
    document: RwSignal<PlanningInput>,
    problems: RwSignal<Vec<InputProblem>>,
    selected: RwSignal<BTreeSet<String>>,
    proposals: RwSignal<Option<PlanProposalSetView>>,
    /// The variant waiting for a second press before it replaces the route.
    confirming: RwSignal<Option<usize>>,
    /// Saved vehicle types, the one used last comes first.
    types: RwSignal<Vec<VehicleType>>,
    /// The vehicle type named in the trip package.
    package_type: RwSignal<String>,
}

impl Panel {
    fn say(self, key: K) -> impl Fn() -> &'static str + Copy {
        move || text(self.locale.get(), key)
    }

    fn report(self, problem: &str) {
        self.message.set(localize_problem(
            self.locale.get_untracked().code(),
            problem,
        ));
        self.error.set(true);
    }

    fn tell(self, key: K, error: bool) {
        let words = text(self.locale.get_untracked(), key);
        self.message.set(words.to_owned());
        self.error.set(error);
    }

    fn show(self, view: PlanningStateView) {
        let ids = view.input.orders.iter().map(|order| order.id.clone());
        self.selected.set(ids.collect());
        self.document.set(view.input);
        self.problems.set(view.problems);
        self.types.set(view.vehicle_types);
        self.proposals.set(None);
        self.confirming.set(None);
    }

    /// Changes the document, keeps it on disk and drops outdated variants.
    fn change(self, edit: impl FnOnce(&mut PlanningInput)) {
        self.document.update(edit);
        self.proposals.set(None);
        self.confirming.set(None);
        let input = self.document.get_untracked();
        spawn_local(async move {
            let args = serde_json::json!({ "input": input });
            match invoke::<PlanningStateView, _>("planner_planning_update", &args).await {
                Ok(view) => self.problems.set(view.problems),
                Err(problem) => self.report(&problem),
            }
        });
    }
}

fn whole<T: std::str::FromStr>(value: &str) -> Option<T> {
    let digits: String = value.chars().filter(|ch| !ch.is_whitespace()).collect();
    digits.parse().ok()
}

fn decimal(value: &str) -> Option<f64> {
    let digits: String = value.chars().filter(|ch| !ch.is_whitespace()).collect();
    digits.replace(',', ".").parse().ok()
}

fn filled(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn shown<T: ToString>(value: Option<T>) -> String {
    value.map(|item| item.to_string()).unwrap_or_default()
}

fn cargo_draft(cargo: PlannerPackageCargoInput) -> CargoDraft {
    CargoDraft {
        row_id: cargo.row_id,
        title: cargo.title,
        weight_kg: shown(cargo.weight_kg),
        pallets: shown(cargo.pallets),
        pickup_point_id: cargo.pickup_point_id.unwrap_or_default(),
        unload_point_id: cargo.unload_point_id.unwrap_or_default(),
    }
}

fn profile_draft(profile: &PlannerRoutingProfileInput) -> RoutingProfileDraft {
    let choice = |value: bool| if value { "yes" } else { "no" }.to_owned();
    RoutingProfileDraft {
        height_m: shown(profile.height_m),
        width_m: shown(profile.width_m),
        length_m: shown(profile.length_m),
        gross_weight_t: shown(profile.gross_weight_t),
        axle_load_t: shown(profile.axle_load_t),
        axle_count: shown(profile.axle_count),
        hazmat: choice(profile.hazmat),
        hgv: choice(profile.hgv),
        exclude_unpaved: choice(profile.exclude_unpaved),
        prefer_truck_routes: choice(profile.prefer_truck_routes),
        toll_preference: match profile.toll_preference {
            PlannerTollPreference::Allow => "allow",
            PlannerTollPreference::PreferAvoid => "prefer_avoid",
            PlannerTollPreference::Avoid => "avoid",
        }
        .to_owned(),
        max_speed_kmh: shown(profile.max_speed_kmh),
    }
}

fn field(
    panel: Panel,
    label: K,
    value: String,
    apply: impl Fn(&mut PlanningInput, String) + Copy + 'static,
) -> impl IntoView {
    view! {
        <label>
            {panel.say(label)}
            <input
                prop:value=value
                on:change=move |event| {
                    let entered = event_target_value(&event);
                    panel.change(|input| apply(input, entered));
                }
            />
        </label>
    }
}

/// Which end of an order an edit belongs to.
fn end_of(order: &mut InputOrder, delivery: bool) -> &mut InputPlace {
    if delivery {
        &mut order.delivery
    } else {
        &mut order.pickup
    }
}

fn place_fields(
    panel: Panel,
    index: usize,
    delivery: bool,
    place: &InputPlace,
) -> impl IntoView + use<> {
    let title = if delivery { K::Delivery } else { K::Pickup };
    let edit = move |input: &mut PlanningInput, change: &dyn Fn(&mut InputPlace)| {
        if let Some(order) = input.orders.get_mut(index) {
            change(end_of(order, delivery));
        }
    };
    view! {
        <div class="routing-grid">
            {field(panel, title, place.place.clone(), move |input, value| {
                edit(input, &|place| place.place = value.clone());
            })}
            {field(panel, K::Latitude, shown(place.lat), move |input, value| {
                edit(input, &|place| place.lat = decimal(&value));
            })}
            {field(panel, K::Longitude, shown(place.lon), move |input, value| {
                edit(input, &|place| place.lon = decimal(&value));
            })}
            {field(panel, K::DateFrom, shown(place.earliest.clone()), move |input, value| {
                edit(input, &|place| place.earliest = filled(&value));
            })}
            {field(panel, K::DateTo, shown(place.latest.clone()), move |input, value| {
                edit(input, &|place| place.latest = filled(&value));
            })}
        </div>
    }
}

fn order_card(panel: Panel, index: usize, order: InputOrder) -> impl IntoView {
    let id = order.id.clone();
    let checked_id = order.id.clone();
    let rate = order.rate.clone().unwrap_or_default();
    let money = move |input: &mut PlanningInput, change: &dyn Fn(&mut Money)| {
        if let Some(order) = input.orders.get_mut(index) {
            let mut rate = order.rate.clone().unwrap_or_default();
            change(&mut rate);
            order.rate = (rate.amount != 0.0 || !rate.currency.is_empty()).then_some(rate);
        }
    };
    view! {
        <div class="cargo-card">
            <div class="package-heading">
                <label>
                    <span>
                        <input
                            type="checkbox"
                            prop:checked=move || panel.selected.get().contains(&checked_id)
                            on:change=move |event| {
                                let on = event_target_checked(&event);
                                let id = id.clone();
                                panel.selected.update(|chosen| {
                                    if on {
                                        chosen.insert(id);
                                    } else {
                                        chosen.remove(&id);
                                    }
                                });
                                panel.proposals.set(None);
                            }
                        />
                        " " {order.id.clone()}
                        {order.source.clone().map(|source| format!(" · {source}"))}
                    </span>
                </label>
                <button
                    type="button"
                    class="danger"
                    on:click=move |_| panel.change(|input| {
                        if index < input.orders.len() {
                            input.orders.remove(index);
                        }
                    })
                >
                    {panel.say(K::Remove)}
                </button>
            </div>
            <div class="routing-grid">
                {field(panel, K::Cargo, order.cargo.title.clone(), move |input, value| {
                    if let Some(order) = input.orders.get_mut(index) {
                        order.cargo.title = value;
                    }
                })}
                {field(panel, K::Weight, shown(order.cargo.weight_kg), move |input, value| {
                    if let Some(order) = input.orders.get_mut(index) {
                        order.cargo.weight_kg = whole(&value);
                    }
                })}
                {field(panel, K::Volume, shown(order.cargo.volume_m3), move |input, value| {
                    if let Some(order) = input.orders.get_mut(index) {
                        order.cargo.volume_m3 = decimal(&value);
                    }
                })}
                {field(panel, K::Pallets, shown(order.cargo.pallets), move |input, value| {
                    if let Some(order) = input.orders.get_mut(index) {
                        order.cargo.pallets = whole(&value);
                    }
                })}
                {field(
                    panel,
                    K::Rate,
                    shown(order.rate.as_ref().map(|rate| rate.amount)),
                    move |input, value| {
                        money(input, &|rate| rate.amount = decimal(&value).unwrap_or(0.0));
                    },
                )}
                {field(panel, K::Currency, rate.currency, move |input, value| {
                    money(input, &|rate| rate.currency = value.trim().to_uppercase());
                })}
                {field(panel, K::Contact, shown(order.contact.clone()), move |input, value| {
                    if let Some(order) = input.orders.get_mut(index) {
                        order.contact = filled(&value);
                    }
                })}
            </div>
            {place_fields(panel, index, false, &order.pickup)}
            {place_fields(panel, index, true, &order.delivery)}
            {order.note.map(|note| view! { <p class="note">{note}</p> })}
        </div>
    }
}

/// Sends a change of the saved type list and shows the list that results.
fn change_types(panel: Panel, command: &'static str, input: serde_json::Value, done: Option<K>) {
    spawn_local(async move {
        let args = serde_json::json!({ "input": input });
        match invoke::<Vec<VehicleType>, _>(command, &args).await {
            Ok(types) => {
                panel.types.set(types);
                if let Some(done) = done {
                    panel.tell(done, false);
                }
            }
            Err(problem) => panel.report(&problem),
        }
    });
}

fn choose_type(panel: Panel, name: &str) {
    let saved = panel.types.get_untracked();
    let Some(chosen) = saved.into_iter().find(|known| known.name == name) else {
        return;
    };
    panel.package_type.set(chosen.name.clone());
    let named = serde_json::json!({ "name": chosen.name });
    panel.change(move |input| chosen.apply_to(&mut input.vehicle));
    change_types(panel, "planner_vehicle_type_use", named, None);
}

fn save_type(panel: Panel) {
    let Some(current) = VehicleType::of(&panel.document.get_untracked().vehicle) else {
        panel.tell(K::TypeNeedsName, true);
        return;
    };
    panel.package_type.set(current.name.clone());
    let input = serde_json::json!(current);
    change_types(panel, "planner_vehicle_type_save", input, Some(K::TypeSaved));
}

/// One line of the list: the name and what the vehicle takes.
fn type_line(locale: UiLocale, kind: &VehicleType) -> String {
    let mut parts = vec![kind.name.clone()];
    if let Some(weight) = kind.payload_kg {
        parts.push(format!("{weight} kg"));
    }
    if let Some(volume) = kind.volume_m3 {
        parts.push(format!("{volume} m³"));
    }
    if let Some(pallets) = kind.pallets {
        parts.push(format!("{}: {pallets}", text(locale, K::Pallets)));
    }
    parts.extend(kind.features.clone());
    parts.join(" · ")
}

/// The list the vehicle is chosen from.
fn type_picker(panel: Panel, current: Option<VehicleType>) -> impl IntoView {
    view! {
        <div class="routing-grid">
            <label>
                {panel.say(K::TypeList)}
                <select on:change=move |event| {
                    choose_type(panel, &event_target_value(&event));
                }>
                    {move || {
                        let locale = panel.locale.get();
                        let types = panel.types.get();
                        // Shown as chosen only while every value matches the saved type.
                        let chosen = types.iter().position(|known| Some(known) == current.as_ref());
                        let none = chosen.is_none();
                        let options = types
                            .iter()
                            .enumerate()
                            .map(|(at, known)| {
                                let selected = chosen == Some(at);
                                let line = type_line(locale, known);
                                view! {
                                    <option value=known.name.clone() prop:selected=selected>
                                        {line}
                                    </option>
                                }
                            })
                            .collect_view();
                        view! {
                            <option value="" prop:selected=none>
                                {text(locale, K::TypeChoose)}
                            </option>
                            {options}
                        }
                    }}
                </select>
            </label>
        </div>
    }
}

/// Keeps the typed values as a type, or takes the named type off the list.
fn type_actions(panel: Panel, current: Option<VehicleType>) -> impl IntoView {
    let name = current.map(|kind| kind.name);
    let listed = name.clone();
    let unlisted = move || {
        let types = panel.types.get();
        !listed
            .as_ref()
            .is_some_and(|name| types.iter().any(|known| known.is_named(name)))
    };
    view! {
        <div class="actions">
            <button type="button" on:click=move |_| save_type(panel)>
                {panel.say(K::SaveType)}
            </button>
            <button
                type="button"
                class="danger"
                disabled=unlisted
                on:click=move |_| {
                    if let Some(name) = &name {
                        let input = serde_json::json!({ "name": name });
                        let done = Some(K::TypeRemoved);
                        change_types(panel, "planner_vehicle_type_remove", input, done);
                    }
                }
            >
                {panel.say(K::RemoveType)}
            </button>
        </div>
    }
}

fn vehicle_fields(panel: Panel, document: &PlanningInput) -> impl IntoView + use<> {
    let vehicle = document.vehicle.clone();
    let start = vehicle.start.clone().unwrap_or_default();
    let cost = vehicle.cost_per_km.clone().unwrap_or_default();
    let current = VehicleType::of(&vehicle);
    let begin = move |input: &mut PlanningInput, change: &dyn Fn(&mut InputPlace)| {
        let mut place = input.vehicle.start.clone().unwrap_or_default();
        change(&mut place);
        input.vehicle.start = (place != InputPlace::default()).then_some(place);
    };
    let money = move |input: &mut PlanningInput, change: &dyn Fn(&mut Money)| {
        let mut cost = input.vehicle.cost_per_km.clone().unwrap_or_default();
        change(&mut cost);
        input.vehicle.cost_per_km =
            (cost.amount != 0.0 || !cost.currency.is_empty()).then_some(cost);
    };
    view! {
        <h3>{panel.say(K::Vehicle)}</h3>
        {type_picker(panel, current.clone())}
        <div class="routing-grid">
            {field(panel, K::VehicleType, shown(vehicle.vehicle_type), |input, value| {
                input.vehicle.vehicle_type = filled(&value);
            })}
            {field(panel, K::Payload, shown(vehicle.payload_kg), |input, value| {
                input.vehicle.payload_kg = whole(&value);
            })}
            {field(panel, K::Volume, shown(vehicle.volume_m3), |input, value| {
                input.vehicle.volume_m3 = decimal(&value);
            })}
            {field(panel, K::Pallets, shown(vehicle.pallets), |input, value| {
                input.vehicle.pallets = whole(&value);
            })}
            {field(panel, K::Features, shown(vehicle.features), |input, value| {
                input.vehicle.features = filled(&value);
            })}
        </div>
        {type_actions(panel, current)}
        <div class="routing-grid">
            {field(panel, K::Start, start.place.clone(), move |input, value| {
                begin(input, &|place| place.place = value.clone());
            })}
            {field(panel, K::Latitude, shown(start.lat), move |input, value| {
                begin(input, &|place| place.lat = decimal(&value));
            })}
            {field(panel, K::Longitude, shown(start.lon), move |input, value| {
                begin(input, &|place| place.lon = decimal(&value));
            })}
            {field(
                panel,
                K::CostPerKm,
                shown(vehicle.cost_per_km.as_ref().map(|cost| cost.amount)),
                move |input, value| {
                    money(input, &|cost| cost.amount = decimal(&value).unwrap_or(0.0));
                },
            )}
            {field(panel, K::Currency, cost.currency, move |input, value| {
                money(input, &|cost| cost.currency = value.trim().to_uppercase());
            })}
            <label>
                <span>
                    <input
                        type="checkbox"
                        prop:checked=vehicle.return_to_start
                        on:change=move |event| {
                            let on = event_target_checked(&event);
                            panel.change(|input| input.vehicle.return_to_start = on);
                        }
                    />
                    " " {panel.say(K::ReturnToStart)}
                </span>
            </label>
        </div>
    }
}

fn add_order(panel: Panel) {
    panel.change(|input| {
        let mut number = input.orders.len() + 1;
        while input
            .orders
            .iter()
            .any(|order| order.id == format!("M{number}"))
        {
            number += 1;
        }
        let order = InputOrder {
            id: format!("M{number}"),
            ..InputOrder::default()
        };
        panel.selected.update(|chosen| {
            chosen.insert(order.id.clone());
        });
        input.orders.push(order);
    });
}

fn problem_text(locale: UiLocale, problem: &InputProblem) -> String {
    let key = match problem.kind {
        InputProblemKind::Schema => K::ProblemSchema,
        InputProblemKind::Required => K::ProblemRequired,
        InputProblemKind::Duplicate => K::ProblemDuplicate,
        InputProblemKind::NotPositive => K::ProblemNotPositive,
        InputProblemKind::Coordinate | InputProblemKind::CoordinatePair => K::ProblemCoordinate,
        InputProblemKind::DateFormat => K::ProblemDateFormat,
        InputProblemKind::DateOrder => K::ProblemDateOrder,
        InputProblemKind::Currency => K::ProblemCurrency,
        InputProblemKind::EstimatedName => K::ProblemEstimated,
    };
    format!("{}: {}", problem.field, text(locale, key))
}

fn money_list(values: &[Money]) -> String {
    values
        .iter()
        .map(|money| format!("{:.0} {}", money.amount, money.currency))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The figures of one variant as short sentences.
fn figures(locale: UiLocale, proposal: &PlanProposalView) -> Vec<String> {
    let say = |key| text(locale, key);
    let mut lines = Vec::new();
    match (proposal.road_state, proposal.road_km, proposal.road_hours) {
        (RoadState::Ok, Some(km), Some(hours)) => {
            lines.push(format!(
                "{}: {km:.0} km, {hours:.1} {}",
                say(K::Road),
                say(K::Hours)
            ));
            if hours > 9.0 {
                lines.push(say(K::OverNineHours).to_owned());
            }
        }
        (RoadState::NotRequested, ..) => lines.push(say(K::RoadNotRequested).to_owned()),
        _ => lines.push(say(K::RoadFailed).to_owned()),
    }
    lines.push(format!(
        "{}: {:.0} km; {}: {:.0} km",
        say(K::Straight),
        proposal.straight_km,
        say(K::Empty),
        proposal.empty_km
    ));
    let volume = if proposal.peak_volume_m3 > 0.0 {
        format!(", {:.1} m³", proposal.peak_volume_m3)
    } else {
        String::new()
    };
    lines.push(format!(
        "{}: {} kg, {} {}{volume}",
        say(K::Peak),
        proposal.peak_weight_kg,
        proposal.peak_pallets,
        say(K::Pallets)
    ));
    if !proposal.revenue.is_empty() {
        lines.push(format!(
            "{}: {}",
            say(K::Revenue),
            money_list(&proposal.revenue)
        ));
    }
    if let Some(result) = &proposal.result_after_distance_cost {
        let value = money_list(std::slice::from_ref(result));
        lines.push(format!("{}: {value}", say(K::Result)));
    }
    if proposal.needs_check {
        lines.push(say(K::NeedsCheck).to_owned());
    }
    if proposal.date_conflict {
        lines.push(say(K::DateConflict).to_owned());
    }
    lines
}

#[component]
pub(crate) fn OrdersPanel(
    locale: RwSignal<UiLocale>,
    message: RwSignal<String>,
    error: RwSignal<bool>,
    route: RwSignal<Option<PlannerRouteView>>,
    profile: RwSignal<RoutingProfileDraft>,
    vehicle_type: RwSignal<String>,
    vehicle_types: RwSignal<Vec<VehicleType>>,
    cargos: RwSignal<Vec<CargoDraft>>,
    next_cargo_row: RwSignal<u64>,
) -> impl IntoView {
    let panel = Panel {
        locale,
        message,
        error,
        busy: RwSignal::new(false),
        document: RwSignal::new(PlanningInput::default()),
        problems: RwSignal::new(Vec::new()),
        selected: RwSignal::new(BTreeSet::new()),
        proposals: RwSignal::new(None),
        confirming: RwSignal::new(None),
        types: vehicle_types,
        package_type: vehicle_type,
    };
    let fill_cargos = move |rows: Vec<PlannerPackageCargoInput>| {
        if let Some(last) = rows.iter().map(|row| row.row_id).max() {
            next_cargo_row.set(last + 1);
            cargos.set(rows.into_iter().map(cargo_draft).collect());
        }
    };

    // Nothing is saved before the saved form is back, or defaults would replace it.
    let restored = RwSignal::new(false);
    // One save at a time and always the newest form, so an older one never lands last.
    let waiting = StoredValue::new(None::<PackageDraft>);
    let sending = StoredValue::new(false);
    Effect::new(move |_| {
        if !restored.get() {
            return;
        }
        let draft = snapshot(
            &vehicle_type.get(),
            &cargos.get(),
            next_cargo_row.get(),
            &profile.get(),
        );
        waiting.set_value(Some(draft));
        if sending.get_value() {
            return;
        }
        sending.set_value(true);
        spawn_local(async move {
            loop {
                let mut next = None;
                waiting.update_value(|draft| next = draft.take());
                let Some(draft) = next else {
                    break;
                };
                let args = serde_json::json!({ "input": draft });
                if let Err(problem) = invoke::<(), _>("planner_planning_package", &args).await {
                    panel.report(&problem);
                }
            }
            sending.set_value(false);
        });
    });

    // What was typed before the last exit comes back.
    spawn_local(async move {
        let args = serde_json::json!({});
        match invoke::<PlanningStateView, _>("planner_planning_status", &args).await {
            Ok(view) => {
                if let Some(saved) = &view.routing_profile {
                    profile.set(profile_draft(saved));
                }
                if let Some(saved) = &view.input.vehicle.vehicle_type {
                    vehicle_type.set(saved.clone());
                }
                fill_cargos(view.cargos.clone());
                if let Some(draft) = &view.package {
                    vehicle_type.set(draft.vehicle_type.clone());
                    let (rows, next) = cargo_rows(draft);
                    next_cargo_row.set(next);
                    cargos.set(rows);
                    profile.set(profile_from(&draft.profile));
                }
                panel.show(view);
                restored.set(true);
            }
            Err(problem) => panel.report(&problem),
        }
    });

    let open_file = move |_| {
        panel.busy.set(true);
        spawn_local(async move {
            let args = serde_json::json!({});
            match invoke::<Option<PlanningStateView>, _>("planner_planning_open", &args).await {
                Ok(Some(view)) => {
                    panel.show(view);
                    panel.message.set(String::new());
                }
                Ok(None) => {}
                Err(problem) => panel.report(&problem),
            }
            panel.busy.set(false);
        });
    };
    let save_file = move |_| {
        panel.busy.set(true);
        spawn_local(async move {
            let args = serde_json::json!({});
            match invoke::<Option<String>, _>("planner_planning_save", &args).await {
                Ok(Some(path)) => {
                    let saved = text(panel.locale.get_untracked(), K::Saved);
                    panel.message.set(format!("{saved}: {path}"));
                    panel.error.set(false);
                }
                Ok(None) => {}
                Err(problem) => panel.report(&problem),
            }
            panel.busy.set(false);
        });
    };
    let propose = move |_| {
        let input = ProposeInput {
            order_ids: panel
                .document
                .get_untracked()
                .orders
                .iter()
                .map(|order| order.id.clone())
                .filter(|id| panel.selected.get_untracked().contains(id))
                .collect(),
            routing_profile: profile.get_untracked().input().ok(),
        };
        panel.busy.set(true);
        panel.confirming.set(None);
        spawn_local(async move {
            let args = serde_json::json!({ "input": input });
            match invoke::<PlanProposalSetView, _>("planner_planning_propose", &args).await {
                Ok(set) => {
                    panel.proposals.set(Some(set));
                    panel.message.set(String::new());
                }
                Err(problem) => panel.report(&problem),
            }
            panel.busy.set(false);
        });
    };
    let choose = move |index: usize, stops: Vec<PlanStopRef>| {
        let has_route = route
            .get_untracked()
            .is_some_and(|current| !current.points.is_empty());
        if has_route && panel.confirming.get_untracked() != Some(index) {
            panel.confirming.set(Some(index));
            return;
        }
        panel.confirming.set(None);
        panel.busy.set(true);
        spawn_local(async move {
            let args = serde_json::json!({ "input": ApplyPlanInput { stops } });
            match invoke::<AppliedPlanView, _>("planner_planning_apply", &args).await {
                Ok(applied) => {
                    route.set(Some(applied.route));
                    vehicle_type.set(applied.vehicle_type);
                    fill_cargos(applied.cargos);
                    let done = text(panel.locale.get_untracked(), K::Chosen);
                    panel.message.set(done.to_owned());
                    panel.error.set(false);
                }
                Err(problem) => panel.report(&problem),
            }
            panel.busy.set(false);
        });
    };

    let variants = move || {
        panel.proposals.get().map(|set| {
            let cards = set
                .proposals
                .into_iter()
                .enumerate()
                .map(|(index, proposal)| {
                    let kind = match proposal.kind {
                        PlanKind::Shortest => K::KindShortest,
                        PlanKind::OneAtATime => K::KindOneAtATime,
                        PlanKind::LoadAllFirst => K::KindLoadAllFirst,
                        PlanKind::Alternative => K::KindAlternative,
                    };
                    let stops: Vec<PlanStopRef> = proposal
                        .stops
                        .iter()
                        .map(|stop| PlanStopRef {
                            order_id: stop.order_id.clone(),
                            action: stop.action,
                        })
                        .collect();
                    let listed = proposal
                        .stops
                        .iter()
                        .map(|stop| {
                            let action = match stop.action {
                                PlanStopAction::Pickup => K::Pickup,
                                PlanStopAction::Delivery => K::Delivery,
                            };
                            let line = format!(" — {} ({})", stop.place, stop.cargo);
                            view! { <li><strong>{panel.say(action)}</strong>{line}</li> }
                        })
                        .collect_view();
                    let summary = proposal.clone();
                    view! {
                        <div class="cargo-card">
                            <h3>{format!("{}. ", index + 1)}{panel.say(kind)}</h3>
                            <ol>{listed}</ol>
                            {move || figures(panel.locale.get(), &summary)
                                .into_iter()
                                .map(|line| view! { <div>{line}</div> })
                                .collect_view()}
                            <div class="actions">
                                <button
                                    type="button"
                                    class="primary"
                                    disabled=move || panel.busy.get()
                                    on:click=move |_| choose(index, stops.clone())
                                >
                                    {move || {
                                        let waiting = panel.confirming.get() == Some(index);
                                        let key = if waiting { K::ChooseAgain } else { K::Choose };
                                        text(panel.locale.get(), key)
                                    }}
                                </button>
                            </div>
                        </div>
                    }
                })
                .collect_view();
            let skipped = set
                .skipped
                .into_iter()
                .map(|item| {
                    let reason = match item.reason {
                        SkipReason::Invalid => K::SkipInvalid,
                        SkipReason::MissingCoordinates => K::SkipCoordinates,
                        SkipReason::TooLarge => K::SkipTooLarge,
                    };
                    view! { <p class="error">{item.order_id}": "{panel.say(reason)}</p> }
                })
                .collect_view();
            view! {
                <div class="cargo-list">
                    {(!set.exhaustive).then(|| view! { <p class="note">{panel.say(K::Heuristic)}</p> })}
                    {skipped}
                    {cards}
                </div>
            }
        })
    };

    view! {
        <section>
            <h2>{panel.say(K::Title)}</h2>
            <p class="note">{panel.say(K::Hint)}</p>
            <div class="actions">
                <button type="button" disabled=move || panel.busy.get() on:click=open_file>
                    {panel.say(K::OpenFile)}
                </button>
                <button type="button" disabled=move || panel.busy.get() on:click=save_file>
                    {panel.say(K::SaveFile)}
                </button>
                <button type="button" on:click=move |_| add_order(panel)>
                    {panel.say(K::AddOrder)}
                </button>
            </div>
            {move || {
                let locale = panel.locale.get();
                panel
                    .problems
                    .get()
                    .iter()
                    .map(|problem| view! { <p class="error">{problem_text(locale, problem)}</p> })
                    .collect_view()
            }}
            {move || {
                let document = panel.document.get();
                let vehicle = vehicle_fields(panel, &document);
                let orders = document
                    .orders
                    .into_iter()
                    .enumerate()
                    .map(|(index, order)| order_card(panel, index, order))
                    .collect_view();
                view! { {vehicle}<div class="cargo-list">{orders}</div> }
            }}
            <div class="actions">
                <button
                    type="button"
                    class="primary"
                    disabled=move || panel.busy.get() || panel.selected.get().is_empty()
                    on:click=propose
                >
                    {panel.say(K::Propose)}
                </button>
            </div>
            {variants}
        </section>
    }
}
