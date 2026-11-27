//! Orders from the input file, route variants and the chosen route.

use super::*;
use cargo_dispatch_planner::AvailableCapacity;
use cargo_dispatch_planner::proposals::{
    MAX_ORDERS, ProposalError, ProposalKind, ProposalOrder, ProposalSettings, StopAction,
    propose_routes,
};
use cargo_dispatch_planner_ui_contracts::{
    AppliedPlanView, ApplyPlanInput, InputOrder, InputPlace, Money, PlanKind, PlanProposalSetView,
    PlanProposalView, PlanStopAction, PlanStopView, PlannerPackageCargoInput, PlanningInput,
    PlanningStateView, ProposeInput, RoadState, SkipReason, SkippedOrder, parse_moment,
    validate_input,
};

const PLANNING_FILE: &str = "planner-input-v1.json";

/// Everything the operator typed that has to survive a restart. No secrets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct StoredPlanning {
    #[serde(default)]
    input: PlanningInput,
    #[serde(default)]
    routing: Option<PlannerRoutingConfigInput>,
    #[serde(default)]
    routing_profile: Option<PlannerRoutingProfileInput>,
    #[serde(default)]
    cargos: Vec<PlannerPackageCargoInput>,
}

pub(crate) fn planning_path(journal_path: &Path) -> PathBuf {
    journal_path.with_file_name(PLANNING_FILE)
}

/// Reads what was saved before. A damaged file is set aside, never overwritten.
pub(crate) fn load_planning(path: &Path) -> StoredPlanning {
    let Ok(text) = fs::read_to_string(path) else {
        return StoredPlanning::default();
    };
    serde_json::from_str(&text).unwrap_or_else(|_| {
        let _ = fs::rename(path, path.with_extension("damaged"));
        StoredPlanning::default()
    })
}

fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(text.as_bytes())?;
    file.sync_data()?;
    fs::rename(&temporary, path)
}

fn coordinates(place: &InputPlace) -> Option<GeoPoint> {
    GeoPoint::new(place.lat?, place.lon?).ok()
}

/// A date without a time means the whole day.
fn moment(text: Option<&String>, end_of_day: bool) -> Option<i64> {
    let text = text?;
    let minutes = parse_moment(text)?;
    Some(if end_of_day && text.len() == 10 {
        minutes + 1439
    } else {
        minutes
    })
}

/// A later stop must not have to be served before an earlier one may begin.
fn date_conflict(places: &[&InputPlace]) -> bool {
    let mut not_before = i64::MIN;
    places.iter().any(|place| {
        if let Some(earliest) = moment(place.earliest.as_ref(), false) {
            not_before = not_before.max(earliest);
        }
        moment(place.latest.as_ref(), true).is_some_and(|latest| latest < not_before)
    })
}

fn revenue(orders: &[&InputOrder]) -> Vec<Money> {
    let mut sums: BTreeMap<&str, f64> = BTreeMap::new();
    for rate in orders.iter().filter_map(|order| order.rate.as_ref()) {
        *sums.entry(rate.currency.as_str()).or_default() += rate.amount;
    }
    sums.into_iter()
        .map(|(currency, amount)| Money {
            amount,
            currency: currency.to_owned(),
        })
        .collect()
}

fn after_distance_cost(orders: &[&InputOrder], cost: Option<&Money>, km: f64) -> Option<Money> {
    let cost = cost?;
    let mut sum = 0.0;
    for order in orders {
        let rate = order.rate.as_ref()?;
        if rate.currency != cost.currency {
            return None;
        }
        sum += rate.amount;
    }
    Some(Money {
        amount: sum - cost.amount * km,
        currency: cost.currency.clone(),
    })
}

fn plan_kind(kind: ProposalKind) -> PlanKind {
    match kind {
        ProposalKind::Shortest => PlanKind::Shortest,
        ProposalKind::OneAtATime => PlanKind::OneAtATime,
        ProposalKind::LoadAllFirst => PlanKind::LoadAllFirst,
        ProposalKind::Alternative => PlanKind::Alternative,
    }
}

impl PlannerDesktopService {
    fn change_planning(&self, change: impl FnOnce(&mut StoredPlanning)) -> Result<(), String> {
        let mut stored = self
            .planning
            .lock()
            .map_err(|_| "Planner planning state unavailable".to_owned())?;
        let mut candidate = stored.clone();
        change(&mut candidate);
        if let Some(path) = &self.journal_path {
            let text = serde_json::to_string_pretty(&candidate)
                .map_err(|error| format!("Could not store planning input: {error}"))?;
            write_atomically(&planning_path(path), &text)
                .map_err(|error| format!("Could not store planning input: {error}"))?;
        }
        *stored = candidate;
        Ok(())
    }

    fn stored_planning(&self) -> Result<StoredPlanning, String> {
        Ok(self
            .planning
            .lock()
            .map_err(|_| "Planner planning state unavailable".to_owned())?
            .clone())
    }

    pub(crate) fn remember_routing(
        &self,
        routing: Option<PlannerRoutingConfigInput>,
    ) -> Result<(), String> {
        self.change_planning(|stored| stored.routing = routing)
    }

    /// Applies the routing settings of the previous session.
    pub(crate) fn restore_routing(&self) {
        if let Ok(StoredPlanning {
            routing: Some(routing),
            ..
        }) = self.stored_planning()
        {
            // Settings that are no longer accepted stay in the form for correction.
            let _ = self.configure_routing(routing);
        }
    }

    pub(crate) fn planning_view(&self) -> Result<PlanningStateView, String> {
        let stored = self.stored_planning()?;
        Ok(PlanningStateView {
            problems: validate_input(&stored.input),
            input: stored.input,
            routing: stored.routing,
            routing_profile: stored.routing_profile,
            cargos: stored.cargos,
        })
    }

    pub(crate) fn update_planning(
        &self,
        input: PlanningInput,
    ) -> Result<PlanningStateView, String> {
        self.change_planning(|stored| stored.input = input)?;
        self.planning_view()
    }

    pub(crate) fn import_input(&self, path: &Path) -> Result<PlanningStateView, String> {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("Could not read input file: {error}"))?;
        let input: PlanningInput = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|error| format!("Input file is not valid: {error}"))?;
        self.update_planning(input)
    }

    pub(crate) fn save_input(&self, path: &Path) -> Result<(), String> {
        let input = self.stored_planning()?.input;
        let text = serde_json::to_string_pretty(&input)
            .map_err(|error| format!("Could not save input file: {error}"))?;
        write_atomically(path, &text).map_err(|error| format!("Could not save input file: {error}"))
    }

    pub(crate) async fn propose(
        &self,
        request: ProposeInput,
    ) -> Result<PlanProposalSetView, String> {
        if let Some(profile) = request.routing_profile.clone() {
            self.change_planning(|stored| stored.routing_profile = Some(profile))?;
        }
        let input = self.stored_planning()?.input;
        let problems = validate_input(&input);
        if problems
            .iter()
            .any(|problem| problem.field.starts_with("vehicle."))
        {
            return Err("Correct the vehicle data before planning".to_owned());
        }

        let mut skipped = Vec::new();
        let mut chosen: Vec<&InputOrder> = Vec::new();
        let mut candidates = Vec::new();
        for id in &request.order_ids {
            let (index, order) = input
                .orders
                .iter()
                .enumerate()
                .find(|(_, order)| &order.id == id)
                .ok_or_else(|| "Unknown order selected".to_owned())?;
            let prefix = format!("orders[{index}].");
            let mut skip = |reason| {
                skipped.push(SkippedOrder {
                    order_id: order.id.clone(),
                    reason,
                });
            };
            if problems
                .iter()
                .any(|problem| problem.field.starts_with(&prefix))
            {
                skip(SkipReason::Invalid);
                continue;
            }
            let (Some(pickup), Some(delivery)) =
                (coordinates(&order.pickup), coordinates(&order.delivery))
            else {
                skip(SkipReason::MissingCoordinates);
                continue;
            };
            chosen.push(order);
            candidates.push(ProposalOrder {
                pickup,
                delivery,
                weight_kg: order.cargo.weight_kg,
                pallets: order.cargo.pallets,
            });
        }

        let start = input.vehicle.start.as_ref().and_then(coordinates);
        let settings = ProposalSettings {
            start,
            return_to_start: input.vehicle.return_to_start,
            capacity: AvailableCapacity {
                payload_kg: input.vehicle.payload_kg,
                pallets: input.vehicle.pallets,
            },
        };
        let set = match propose_routes(&candidates, settings) {
            Ok(set) => set,
            Err(ProposalError::NoOrders) => {
                return Ok(PlanProposalSetView {
                    proposals: Vec::new(),
                    skipped,
                    exhaustive: true,
                });
            }
            Err(ProposalError::TooManyOrders) => {
                return Err(format!(
                    "Select no more than {MAX_ORDERS} orders for one calculation"
                ));
            }
        };
        skipped.extend(set.too_large.iter().map(|&index| SkippedOrder {
            order_id: chosen[index].id.clone(),
            reason: SkipReason::TooLarge,
        }));

        let routing = self
            .routing
            .lock()
            .map_err(|_| "Planner routing state unavailable".to_owned())?
            .clone();
        let road = match (routing, request.routing_profile) {
            (Some(config), Some(profile)) => {
                let client = ValhallaHttpClient::new(config)
                    .map_err(|error| format!("Could not create Valhalla client: {error}"))?;
                Some((client, routing_profile(profile)?))
            }
            _ => None,
        };

        let mut proposals = Vec::new();
        for proposal in set.proposals {
            let places: Vec<&InputPlace> = proposal
                .stops
                .iter()
                .map(|stop| match stop.action {
                    StopAction::Pickup => &chosen[stop.order].pickup,
                    StopAction::Delivery => &chosen[stop.order].delivery,
                })
                .collect();
            let included: Vec<&InputOrder> = proposal
                .stops
                .iter()
                .filter(|stop| stop.action == StopAction::Pickup)
                .map(|stop| chosen[stop.order])
                .collect();

            let mut road_state = RoadState::NotRequested;
            let mut road_km = None;
            let mut road_hours = None;
            if let Some((client, profile)) = &road {
                let mut points: Vec<GeoPoint> = places
                    .iter()
                    .filter_map(|place| coordinates(place))
                    .collect();
                if let Some(start) = start {
                    points.insert(0, start);
                    if input.vehicle.return_to_start {
                        points.push(start);
                    }
                }
                road_state = RoadState::Failed;
                let request = RouteRequest::new(points, profile.clone())
                    .map_err(|error| format!("Invalid routing preview request: {error}"))?;
                if let Ok(result) = client.route(&request).await
                    && result.status == NormalizedStatus::Ok
                {
                    road_state = RoadState::Ok;
                    road_km = result.distance_m.map(|meters| meters / 1000.0);
                    road_hours = result.duration_s.map(|seconds| seconds / 3600.0);
                }
            }

            let stops = proposal
                .stops
                .iter()
                .zip(&places)
                .map(|(stop, place)| PlanStopView {
                    order_id: chosen[stop.order].id.clone(),
                    action: match stop.action {
                        StopAction::Pickup => PlanStopAction::Pickup,
                        StopAction::Delivery => PlanStopAction::Delivery,
                    },
                    place: place.place.clone(),
                    cargo: chosen[stop.order].cargo.title.clone(),
                })
                .collect();
            proposals.push(PlanProposalView {
                kind: plan_kind(proposal.kind),
                stops,
                straight_km: proposal.straight_km,
                empty_km: proposal.empty_km,
                road_state,
                road_km,
                road_hours,
                peak_weight_kg: proposal.peak_weight_kg,
                peak_pallets: proposal.peak_pallets,
                needs_check: proposal.needs_check,
                date_conflict: date_conflict(&places),
                revenue: revenue(&included),
                result_after_distance_cost: after_distance_cost(
                    &included,
                    input.vehicle.cost_per_km.as_ref(),
                    road_km.unwrap_or(proposal.straight_km),
                ),
            });
        }
        // With every road known, the road decides which variant is the shortest.
        if proposals.iter().all(|item| item.road_km.is_some()) {
            proposals.sort_by(|a, b| {
                a.road_km
                    .unwrap_or(f64::MAX)
                    .total_cmp(&b.road_km.unwrap_or(f64::MAX))
            });
            for (index, item) in proposals.iter_mut().enumerate() {
                if index == 0 {
                    item.kind = PlanKind::Shortest;
                } else if item.kind == PlanKind::Shortest {
                    item.kind = PlanKind::Alternative;
                }
            }
        }

        Ok(PlanProposalSetView {
            proposals,
            skipped,
            exhaustive: set.exhaustive,
        })
    }

    /// Replaces the working route with the chosen order of stops.
    pub(crate) fn apply_plan(&self, plan: ApplyPlanInput) -> Result<AppliedPlanView, String> {
        let input = self.stored_planning()?.input;
        let mut cargos: Vec<PlannerPackageCargoInput> = Vec::new();
        let mut order_of_row: Vec<&str> = Vec::new();
        let mut points = Vec::new();
        let mut delivered = BTreeSet::new();
        // Check the whole plan before the current route is touched.
        for stop in &plan.stops {
            let order = input
                .orders
                .iter()
                .find(|order| order.id == stop.order_id)
                .ok_or_else(|| "Unknown order selected".to_owned())?;
            let row = order_of_row.iter().position(|id| *id == order.id);
            let (place, kind, row) = match (stop.action, row) {
                (PlanStopAction::Pickup, None) => {
                    order_of_row.push(order.id.as_str());
                    cargos.push(PlannerPackageCargoInput {
                        row_id: u64::try_from(order_of_row.len()).unwrap_or(u64::MAX),
                        title: order.cargo.title.clone(),
                        weight_kg: order.cargo.weight_kg,
                        pallets: order.cargo.pallets,
                        pickup_point_id: None,
                        unload_point_id: None,
                    });
                    (&order.pickup, PointKind::Load, cargos.len() - 1)
                }
                (PlanStopAction::Delivery, Some(row)) if delivered.insert(row) => {
                    (&order.delivery, PointKind::Unload, row)
                }
                _ => {
                    return Err(
                        "A route needs the pickup and the delivery of every order".to_owned()
                    );
                }
            };
            let location =
                coordinates(place).ok_or_else(|| "Order has no coordinates".to_owned())?;
            let title = format!("{} — {}", place.place.trim(), order.cargo.title.trim());
            points.push((title, location, kind, row));
        }
        if delivered.len() != cargos.len() || points.is_empty() {
            return Err("A route needs the pickup and the delivery of every order".to_owned());
        }

        let route = {
            let mut state = self
                .inner
                .lock()
                .map_err(|_| "Planner state unavailable".to_owned())?;
            let reset = PlannerDraftEvent::ResetRoute {
                route_serial: state.next_serial,
            };
            let mut view = self.commit_event(&mut state, reset)?;
            for (title, location, kind, row) in points {
                let serial = state.next_serial;
                let identifier = hex_id(point_id(serial).as_bytes());
                if kind == PointKind::Load {
                    cargos[row].pickup_point_id = Some(identifier);
                } else {
                    cargos[row].unload_point_id = Some(identifier);
                }
                let event = PlannerDraftEvent::AddPoint {
                    point_serial: serial,
                    title,
                    latitude: location.lat(),
                    longitude: location.lon(),
                    kind,
                };
                view = self.commit_event(&mut state, event)?;
            }
            view
        };
        let remembered = cargos.clone();
        self.change_planning(|stored| stored.cargos = remembered)?;
        Ok(AppliedPlanView {
            route,
            cargos,
            vehicle_type: input.vehicle.vehicle_type.unwrap_or_default(),
        })
    }
}

async fn pick_input_file(app: AppHandle, save: bool) -> Result<Option<PathBuf>, String> {
    tokio::task::spawn_blocking(move || {
        let dialog = app.dialog().file().add_filter("JSON", &["json"]);
        let picked = if save {
            dialog
                .set_file_name("cargo-dispatch-orders.json")
                .blocking_save_file()
        } else {
            dialog.blocking_pick_file()
        };
        picked
            .map(|value| {
                value
                    .into_path()
                    .map_err(|error| format!("Could not read input file: {error}"))
            })
            .transpose()
    })
    .await
    .map_err(|error| format!("Could not open input file dialog: {error}"))?
}

#[tauri::command]
pub(crate) fn planner_planning_status(
    state: State<'_, PlannerDesktopService>,
) -> Result<PlanningStateView, String> {
    state.planning_view()
}

#[tauri::command]
pub(crate) fn planner_planning_update(
    input: PlanningInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlanningStateView, String> {
    state.update_planning(input)
}

/// `None` when the operator closed the dialog.
#[tauri::command]
pub(crate) async fn planner_planning_open(
    app: AppHandle,
    state: State<'_, PlannerDesktopService>,
) -> Result<Option<PlanningStateView>, String> {
    match pick_input_file(app, false).await? {
        Some(path) => state.import_input(&path).map(Some),
        None => Ok(None),
    }
}

#[tauri::command]
pub(crate) async fn planner_planning_save(
    app: AppHandle,
    state: State<'_, PlannerDesktopService>,
) -> Result<Option<String>, String> {
    match pick_input_file(app, true).await? {
        Some(path) => {
            state.save_input(&path)?;
            Ok(Some(path.to_string_lossy().into_owned()))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub(crate) async fn planner_planning_propose(
    input: ProposeInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlanProposalSetView, String> {
    state.propose(input).await
}

#[tauri::command]
pub(crate) fn planner_planning_apply(
    input: ApplyPlanInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<AppliedPlanView, String> {
    state.apply_plan(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_planner_ui_contracts::{InputCargo, PlanStopRef, PlannerTollPreference};

    fn order(id: &str, from: f64, to: f64) -> InputOrder {
        let place = |name: &str, lon: f64| InputPlace {
            place: name.to_owned(),
            lat: Some(50.0),
            lon: Some(lon),
            ..InputPlace::default()
        };
        InputOrder {
            id: id.to_owned(),
            cargo: InputCargo {
                title: format!("Cargo {id}"),
                weight_kg: Some(1_000),
                ..InputCargo::default()
            },
            pickup: place("Alpha", from),
            delivery: place("Beta", to),
            rate: Some(Money {
                amount: 1_000.0,
                currency: "UAH".to_owned(),
            }),
            ..InputOrder::default()
        }
    }

    fn input() -> PlanningInput {
        let mut input = PlanningInput::default();
        input.vehicle.payload_kg = Some(20_000);
        input.vehicle.cost_per_km = Some(Money {
            amount: 1.0,
            currency: "UAH".to_owned(),
        });
        let mut without_coordinates = order("B", 31.0, 32.0);
        without_coordinates.pickup.lat = None;
        without_coordinates.pickup.lon = None;
        input.orders = vec![order("A", 30.0, 31.0), without_coordinates];
        input
    }

    #[tokio::test]
    async fn orders_become_variants_and_the_chosen_one_becomes_the_route() {
        let service = PlannerDesktopService::default();
        service.update_planning(input()).expect("stored");
        let set = service
            .propose(ProposeInput {
                order_ids: vec!["A".to_owned(), "B".to_owned()],
                routing_profile: None,
            })
            .await
            .expect("proposals");
        assert_eq!(set.skipped.len(), 1);
        assert_eq!(set.skipped[0].reason, SkipReason::MissingCoordinates);
        let best = &set.proposals[0];
        assert_eq!(best.road_state, RoadState::NotRequested);
        let result = best
            .result_after_distance_cost
            .as_ref()
            .expect("same currency");
        assert!((result.amount - (1_000.0 - best.straight_km)).abs() < 0.001);

        let applied = service
            .apply_plan(ApplyPlanInput {
                stops: best
                    .stops
                    .iter()
                    .map(|stop| PlanStopRef {
                        order_id: stop.order_id.clone(),
                        action: stop.action,
                    })
                    .collect(),
            })
            .expect("applied");
        assert_eq!(applied.route.points.len(), 2);
        assert_eq!(applied.cargos.len(), 1);
        assert_eq!(
            applied.cargos[0].pickup_point_id.as_deref(),
            Some(applied.route.points[0].point_id.as_str())
        );
        let state = service.inner.lock().expect("state");
        let kinds: Vec<_> = package_route_points(&state)
            .expect("points")
            .into_iter()
            .map(|point| point.kind)
            .collect();
        assert_eq!(kinds, [TripPointKind::Load, TripPointKind::Unload]);
    }

    #[test]
    fn a_delivery_without_its_pickup_leaves_the_route_untouched() {
        let service = PlannerDesktopService::default();
        service.update_planning(input()).expect("stored");
        let problem = service
            .apply_plan(ApplyPlanInput {
                stops: vec![PlanStopRef {
                    order_id: "A".to_owned(),
                    action: PlanStopAction::Delivery,
                }],
            })
            .expect_err("rejected");
        assert!(problem.starts_with("A route needs"));
        assert!(service.status().expect("status").points.is_empty());
    }

    #[test]
    fn input_and_routing_settings_survive_a_restart() {
        let folder = std::env::temp_dir().join(format!(
            "cargo-planner-planning-{}",
            unix_time_ms().expect("clock")
        ));
        let journal = folder.join(PLANNER_JOURNAL_FILE);
        let first = PlannerDesktopService::open(journal.clone()).expect("open");
        first.update_planning(input()).expect("stored");
        first
            .remember_routing(Some(PlannerRoutingConfigInput {
                base_url: "http://127.0.0.1:8002".to_owned(),
                map_dataset_id: "synthetic".to_owned(),
                connect_timeout_ms: 1_000,
                request_timeout_ms: 5_000,
            }))
            .expect("remembered");
        first
            .change_planning(|stored| {
                stored.routing_profile = Some(PlannerRoutingProfileInput {
                    height_m: Some(4.0),
                    width_m: None,
                    length_m: None,
                    gross_weight_t: None,
                    axle_load_t: None,
                    axle_count: None,
                    hazmat: false,
                    hgv: true,
                    exclude_unpaved: false,
                    prefer_truck_routes: true,
                    toll_preference: PlannerTollPreference::Allow,
                    max_speed_kmh: None,
                });
            })
            .expect("remembered");
        drop(first);

        let second = PlannerDesktopService::open(journal).expect("reopen");
        let view = second.planning_view().expect("view");
        assert_eq!(view.input, input());
        assert_eq!(view.routing_profile.expect("profile").height_m, Some(4.0));
        assert!(second.routing.lock().expect("routing").is_some());
        let _ = fs::remove_dir_all(folder);
    }
}
