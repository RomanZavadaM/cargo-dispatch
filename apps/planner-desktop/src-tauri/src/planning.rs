//! Orders from the input file, route variants and the chosen route.

use super::*;
use cargo_dispatch_contracts::{PlannedDistance, PlannedDistanceSource, PlannedRouteEstimate};
use cargo_dispatch_planner::AvailableCapacity;
use cargo_dispatch_planner::proposals::{
    MAX_ORDERS, ProposalError, ProposalKind, ProposalOrder, ProposalSettings, StopAction,
    propose_routes, straight_line_km,
};
use cargo_dispatch_planner_ui_contracts::{
    AppliedPlanView, ApplyPlanInput, FigureDraft, FiguresDraft, InputVehicle, INPUT_SCHEMA_VERSION, InputOrder, InputPlace, MapLinkError,
    MapService, Money, OLDER_INPUT_SCHEMA_VERSION, PackageDraft, PlanKind, PlanProposalSetView,
    PlanProposalView, PlanStopAction, PlanStopView, PlannerPackageCargoInput, PlanningInput,
    PlanningStateView, ProposeInput, RoadState, SkipReason, SkippedOrder, VehicleType,
    VehicleTypeName, parse_distance_km, parse_drive_minutes, parse_moment, validate_input,
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
    #[serde(default)]
    package: Option<PackageDraft>,
    /// The one used last comes first.
    #[serde(default)]
    vehicle_types: Vec<VehicleType>,
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

/// The capacity check counts whole litres, so that sums compare exactly.
fn litres(volume_m3: Option<f64>) -> Option<u32> {
    // The conversion saturates; invalid volumes never reach the calculation.
    volume_m3.map(|volume| (volume * 1000.0).round() as u32)
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

/// The places a trip passes: from where the vehicle stands, through the points
/// of the route, and back when the vehicle has to return.
pub(crate) fn trip_path(vehicle: &InputVehicle, mut points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let start = vehicle.start.as_ref().and_then(coordinates);
    if let Some(start) = start.filter(|_| !points.is_empty()) {
        points.insert(0, (start.lat(), start.lon()));
        if vehicle.return_to_start {
            points.push((start.lat(), start.lon()));
        }
    }
    points
}

fn straight_distance(path: &[(f64, f64)]) -> Option<PlannedDistance> {
    let places: Vec<GeoPoint> = path
        .iter()
        .filter_map(|&(lat, lon)| GeoPoint::new(lat, lon).ok())
        .collect();
    let km: f64 = places
        .windows(2)
        .map(|pair| straight_line_km(pair[0], pair[1]))
        .sum();
    // The conversion saturates and a route on Earth stays far below the limit.
    let meters = (km * 1000.0).round() as u32;
    (meters > 0).then_some(PlannedDistance {
        meters,
        source: PlannedDistanceSource::StraightLine,
    })
}

/// What a package says about distance and time: the figures typed for this
/// version of the route, otherwise the straight-line distance and no time.
/// The flag reports a typed value that belongs to another version.
pub(crate) fn route_estimate(
    figures: &FiguresDraft,
    route_id: &str,
    revision: u64,
    path: &[(f64, f64)],
) -> Result<(Option<PlannedRouteEstimate>, bool), String> {
    let mut left_out = false;
    let mut current = |figure: &FigureDraft| {
        if figure.is_outdated(route_id, revision) {
            left_out = true;
            return None;
        }
        let value = figure.value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    };
    let typed_distance = current(&figures.distance_km);
    let typed_time = current(&figures.drive_time);
    let distance = match typed_distance {
        Some(text) => {
            let km = parse_distance_km(&text)
                .ok_or_else(|| "The typed kilometres cannot be read".to_owned())?;
            Some(PlannedDistance {
                // At most 100 000 km are accepted.
                meters: ((km * 1000.0).round() as u32).max(1),
                source: PlannedDistanceSource::Operator,
            })
        }
        None => straight_distance(path),
    };
    let drive_minutes = typed_time
        .map(|text| {
            parse_drive_minutes(&text)
                .ok_or_else(|| "The typed driving time cannot be read".to_owned())
        })
        .transpose()?;
    let known = distance.is_some() || drive_minutes.is_some();
    let estimate = known.then_some(PlannedRouteEstimate {
        distance,
        drive_minutes,
    });
    Ok((estimate, left_out))
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

    /// The vehicle of the planning document.
    pub(crate) fn vehicle(&self) -> Result<InputVehicle, String> {
        Ok(self.stored_planning()?.input.vehicle)
    }

    pub(crate) fn planning_view(&self) -> Result<PlanningStateView, String> {
        let stored = self.stored_planning()?;
        Ok(PlanningStateView {
            problems: validate_input(&stored.input),
            input: stored.input,
            routing: stored.routing,
            routing_profile: stored.routing_profile,
            cargos: stored.cargos,
            package: stored.package,
            vehicle_types: stored.vehicle_types,
        })
    }

    fn change_vehicle_types(
        &self,
        change: impl FnOnce(&mut Vec<VehicleType>),
    ) -> Result<Vec<VehicleType>, String> {
        self.change_planning(|stored| change(&mut stored.vehicle_types))?;
        Ok(self.stored_planning()?.vehicle_types)
    }

    /// Adds a type or replaces the one of the same name; it becomes the first.
    pub(crate) fn save_vehicle_type(
        &self,
        mut saved: VehicleType,
    ) -> Result<Vec<VehicleType>, String> {
        if !saved.is_usable() {
            return Err("Vehicle type needs a name and positive limits".to_owned());
        }
        saved.name = saved.name.trim().to_owned();
        self.change_vehicle_types(|types| {
            types.retain(|known| !known.is_named(&saved.name));
            types.insert(0, saved);
        })
    }

    /// Moves the chosen type to the head of the list.
    pub(crate) fn use_vehicle_type(
        &self,
        chosen: VehicleTypeName,
    ) -> Result<Vec<VehicleType>, String> {
        self.change_vehicle_types(|types| {
            if let Some(at) = types.iter().position(|known| known.is_named(&chosen.name)) {
                let found = types.remove(at);
                types.insert(0, found);
            }
        })
    }

    pub(crate) fn remove_vehicle_type(
        &self,
        removed: VehicleTypeName,
    ) -> Result<Vec<VehicleType>, String> {
        self.change_vehicle_types(|types| types.retain(|known| !known.is_named(&removed.name)))
    }

    /// Keeps the trip package section as typed; nothing in it is checked here.
    pub(crate) fn remember_package(&self, package: PackageDraft) -> Result<(), String> {
        self.change_planning(|stored| stored.package = Some(package))
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
        let mut input = self.stored_planning()?.input;
        // What is written follows the current version of the format.
        if input.schema_version == OLDER_INPUT_SCHEMA_VERSION {
            input.schema_version = INPUT_SCHEMA_VERSION.to_owned();
        }
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
                volume_litres: litres(order.cargo.volume_m3),
            });
        }

        let start = input.vehicle.start.as_ref().and_then(coordinates);
        let settings = ProposalSettings {
            start,
            return_to_start: input.vehicle.return_to_start,
            capacity: AvailableCapacity {
                payload_kg: input.vehicle.payload_kg,
                pallets: input.vehicle.pallets,
                volume_litres: litres(input.vehicle.volume_m3),
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
                peak_volume_m3: proposal.peak_volume_litres as f64 / 1000.0,
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

    /// The address of the working route in a map service: from where the vehicle
    /// stands, through the points, and back when the vehicle has to return.
    pub(crate) fn route_link(&self, service: MapService) -> Result<String, String> {
        let vehicle = self.vehicle()?;
        let points: Vec<(f64, f64)> = self
            .status()?
            .points
            .iter()
            .map(|point| (point.latitude, point.longitude))
            .collect();
        let path = trip_path(&vehicle, points);
        service.route_address(&path).map_err(|error| match error {
            MapLinkError::TooFewPoints => {
                "Opening a route needs at least two ordered points".to_owned()
            }
            MapLinkError::TooManyPoints => {
                "The map service takes no more than 11 points".to_owned()
            }
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

/// Called after every change of the form, so the file is written off the window's thread.
#[tauri::command]
pub(crate) async fn planner_planning_package(
    input: PackageDraft,
    state: State<'_, PlannerDesktopService>,
) -> Result<(), String> {
    state.remember_package(input)
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
pub(crate) fn planner_vehicle_type_save(
    input: VehicleType,
    state: State<'_, PlannerDesktopService>,
) -> Result<Vec<VehicleType>, String> {
    state.save_vehicle_type(input)
}

#[tauri::command]
pub(crate) fn planner_vehicle_type_use(
    input: VehicleTypeName,
    state: State<'_, PlannerDesktopService>,
) -> Result<Vec<VehicleType>, String> {
    state.use_vehicle_type(input)
}

#[tauri::command]
pub(crate) fn planner_vehicle_type_remove(
    input: VehicleTypeName,
    state: State<'_, PlannerDesktopService>,
) -> Result<Vec<VehicleType>, String> {
    state.remove_vehicle_type(input)
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
    use cargo_dispatch_planner_ui_contracts::{
        CargoRowDraft, InputCargo, PlanStopRef, PlannerTollPreference,
        ProfileDraft,
    };

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
        input.orders.push(order("A", 30.0, 31.0));
        input.orders.push(without_coordinates);
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
    fn the_route_address_starts_where_the_vehicle_stands_and_returns() {
        let service = PlannerDesktopService::default();
        let mut planned = input();
        planned.vehicle.start = Some(InputPlace {
            place: "Base".to_owned(),
            lat: Some(49.0),
            lon: Some(29.0),
            ..InputPlace::default()
        });
        planned.vehicle.return_to_start = true;
        service.update_planning(planned).expect("stored");
        let problem = service
            .route_link(MapService::Google)
            .expect_err("no route");
        assert!(problem.contains("at least two ordered points"));
        service
            .apply_plan(ApplyPlanInput {
                stops: [PlanStopAction::Pickup, PlanStopAction::Delivery]
                    .map(|action| PlanStopRef {
                        order_id: "A".to_owned(),
                        action,
                    })
                    .to_vec(),
            })
            .expect("applied");
        assert_eq!(
            service
                .route_link(MapService::GraphHopper)
                .expect("address"),
            "https://graphhopper.com/maps/?point=49.000000%2C29.000000\
             &point=50.000000%2C30.000000&point=50.000000%2C31.000000\
             &point=49.000000%2C29.000000&profile=truck"
        );
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
    fn a_version_1_0_file_is_read_and_saved_as_the_current_version() {
        let folder = std::env::temp_dir().join(format!(
            "cargo-planner-versions-{}",
            unix_time_ms().expect("clock")
        ));
        fs::create_dir_all(&folder).expect("folder");
        let old = folder.join("old.json");
        let text = r#"{"schema":"cargo-dispatch.planning-input","schema_version":"1.0",
            "vehicle":{"type":"Tent","payload_kg":20000}}"#;
        fs::write(&old, text).expect("old file");
        let service = PlannerDesktopService::default();
        let view = service.import_input(&old).expect("version 1.0");
        assert_eq!(view.problems, []);
        assert_eq!(view.input.vehicle.volume_m3, None);
        let saved = folder.join("saved.json");
        service.save_input(&saved).expect("saved");
        let again = service.import_input(&saved).expect("current version");
        assert_eq!(again.input.schema_version, INPUT_SCHEMA_VERSION);
        assert_eq!(again.problems, []);
        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn the_half_filled_package_form_survives_a_restart() {
        let folder = std::env::temp_dir().join(format!(
            "cargo-planner-package-{}",
            unix_time_ms().expect("clock")
        ));
        let journal = folder.join(PLANNER_JOURNAL_FILE);
        // A file written before the form was kept has no such field.
        fs::create_dir_all(&folder).expect("folder");
        fs::write(planning_path(&journal), r#"{"cargos":[]}"#).expect("old file");
        let first = PlannerDesktopService::open(journal.clone()).expect("open");
        assert!(first.planning_view().expect("view").package.is_none());
        let draft = PackageDraft {
            vehicle_type: "box".to_owned(),
            cargos: vec![CargoRowDraft {
                row_id: 3,
                weight_kg: "12 0".to_owned(),
                ..CargoRowDraft::default()
            }],
            next_cargo_row: 7,
            profile: ProfileDraft {
                height_m: "4,".to_owned(),
                ..ProfileDraft::default()
            },
            figures: FiguresDraft {
                distance_km: FigureDraft {
                    value: "640,".to_owned(),
                    route_id: "route".to_owned(),
                    revision: 2,
                },
                ..FiguresDraft::default()
            },
        };
        first.remember_package(draft.clone()).expect("remembered");
        drop(first);

        let second = PlannerDesktopService::open(journal).expect("reopen");
        assert_eq!(second.planning_view().expect("view").package, Some(draft));
        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn vehicle_types_are_kept_with_the_last_used_first() {
        let folder = std::env::temp_dir().join(format!(
            "cargo-planner-types-{}",
            unix_time_ms().expect("clock")
        ));
        let journal = folder.join(PLANNER_JOURNAL_FILE);
        fs::create_dir_all(&folder).expect("folder");
        // A file written before types were kept has no such list.
        fs::write(planning_path(&journal), r#"{"cargos":[]}"#).expect("old file");
        let first = PlannerDesktopService::open(journal.clone()).expect("open");
        assert!(
            first
                .planning_view()
                .expect("view")
                .vehicle_types
                .is_empty()
        );
        let named = |name: &str, payload_kg| VehicleType {
            name: name.to_owned(),
            payload_kg: Some(payload_kg),
            ..VehicleType::default()
        };
        let chosen = |name: &str| VehicleTypeName {
            name: name.to_owned(),
        };
        let names = |types: &[VehicleType]| -> Vec<String> {
            types.iter().map(|item| item.name.clone()).collect()
        };
        first
            .save_vehicle_type(named("Tent", 20_000))
            .expect("saved");
        first
            .save_vehicle_type(named("Reefer", 18_000))
            .expect("saved");
        first.save_vehicle_type(named("Van", 1_500)).expect("saved");
        let types = first.use_vehicle_type(chosen("tent ")).expect("used");
        assert_eq!(names(&types), ["Tent", "Van", "Reefer"]);
        let types = first
            .save_vehicle_type(named(" van", 2_000))
            .expect("replaced");
        assert_eq!(names(&types), ["van", "Tent", "Reefer"]);
        let types = first
            .remove_vehicle_type(chosen("Reefer"))
            .expect("removed");
        assert_eq!(names(&types), ["van", "Tent"]);
        for broken in [named(" ", 1), named("Empty", 0)] {
            let problem = first.save_vehicle_type(broken).expect_err("rejected");
            assert!(problem.starts_with("Vehicle type"));
        }
        drop(first);

        let second = PlannerDesktopService::open(journal).expect("reopen");
        let kept = second.planning_view().expect("view").vehicle_types;
        assert_eq!(names(&kept), ["van", "Tent"]);
        assert_eq!(kept[0].payload_kg, Some(2_000));
        let _ = fs::remove_dir_all(folder);
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
