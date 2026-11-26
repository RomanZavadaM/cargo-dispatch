#![forbid(unsafe_code)]

use cargo_dispatch_domain::{PointId, WorkingRouteId};
use cargo_dispatch_geo::{TollPreference, VehicleRoutingProfile};
use cargo_dispatch_planner::{
    WorkingRoute, WorkingRouteState, edit_manual_working_route, manual_working_route,
};
use cargo_dispatch_planner_ui_contracts::{
    PlannerPointInput, PlannerPointView, PlannerRouteView, PlannerVehicleProfileInput,
    PlannerVehicleProfileView, PointActionInput, ReorderPointsInput,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{Manager, State};

const PLANNER_JOURNAL_SCHEMA: u32 = 1;
const PLANNER_JOURNAL_FILE: &str = "planner-draft-v1.jsonl";

#[derive(Debug, Clone)]
struct ManualPoint {
    point_id: PointId,
    title: String,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Clone)]
struct PlannerState {
    route: WorkingRoute,
    points: BTreeMap<PointId, ManualPoint>,
    vehicle_profile: Option<VehicleRoutingProfile>,
    next_serial: u64,
}

impl Default for PlannerState {
    fn default() -> Self {
        Self {
            route: manual_working_route(route_id(1), &[]).expect("empty manual route is valid"),
            points: BTreeMap::new(),
            vehicle_profile: None,
            next_serial: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PlannerDraftEvent {
    ResetRoute {
        route_serial: u64,
    },
    AddPoint {
        point_serial: u64,
        title: String,
        latitude: f64,
        longitude: f64,
    },
    RemovePoint {
        point_id: [u8; 16],
    },
    ReorderPoints {
        point_ids: Vec<[u8; 16]>,
    },
    SetVehicleProfile {
        profile: PlannerVehicleProfileInput,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlannerJournalRecord {
    schema: u32,
    event: PlannerDraftEvent,
}

struct PlannerDesktopService {
    inner: Mutex<PlannerState>,
    journal_path: Option<PathBuf>,
}

impl Default for PlannerDesktopService {
    fn default() -> Self {
        Self {
            inner: Mutex::new(PlannerState::default()),
            journal_path: None,
        }
    }
}

impl PlannerDesktopService {
    fn open(journal_path: PathBuf) -> Result<Self, String> {
        let state = recover_journal(&journal_path)?;
        Ok(Self {
            inner: Mutex::new(state),
            journal_path: Some(journal_path),
        })
    }

    fn append_event(&self, event: &PlannerDraftEvent) -> Result<(), String> {
        let Some(path) = &self.journal_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create Planner data folder: {error}"))?;
        }
        let record = PlannerJournalRecord {
            schema: PLANNER_JOURNAL_SCHEMA,
            event: event.clone(),
        };
        let mut encoded = serde_json::to_vec(&record)
            .map_err(|error| format!("Could not encode Planner journal record: {error}"))?;
        encoded.push(b'\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| format!("Could not open Planner journal: {error}"))?;
        file.write_all(&encoded)
            .map_err(|error| format!("Could not write Planner journal: {error}"))?;
        file.flush()
            .map_err(|error| format!("Could not flush Planner journal: {error}"))?;
        file.sync_data()
            .map_err(|error| format!("Could not sync Planner journal: {error}"))?;
        Ok(())
    }

    fn commit_event(
        &self,
        state: &mut PlannerState,
        event: PlannerDraftEvent,
    ) -> Result<PlannerRouteView, String> {
        let mut candidate = state.clone();
        apply_event(&mut candidate, &event)?;
        self.append_event(&event)?;
        *state = candidate;
        route_view(state)
    }

    fn status(&self) -> Result<PlannerRouteView, String> {
        let state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        route_view(&state)
    }

    fn reset(&self) -> Result<PlannerRouteView, String> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        let event = PlannerDraftEvent::ResetRoute {
            route_serial: state.next_serial,
        };
        self.commit_event(&mut state, event)
    }

    fn add_point(&self, input: PlannerPointInput) -> Result<PlannerRouteView, String> {
        validate_point(&input)?;
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        let event = PlannerDraftEvent::AddPoint {
            point_serial: state.next_serial,
            title: input.title.trim().to_owned(),
            latitude: input.latitude,
            longitude: input.longitude,
        };
        self.commit_event(&mut state, event)
    }

    fn remove_point(&self, input: PointActionInput) -> Result<PlannerRouteView, String> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        let id = point_from_text(&state, &input.point_id)?;
        self.commit_event(
            &mut state,
            PlannerDraftEvent::RemovePoint {
                point_id: id.as_bytes(),
            },
        )
    }

    fn reorder(&self, input: ReorderPointsInput) -> Result<PlannerRouteView, String> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        if input.point_ids.len() != state.route.points.len() {
            return Err("Reorder must contain every current point exactly once".to_owned());
        }
        let order = input
            .point_ids
            .iter()
            .map(|value| point_from_text(&state, value))
            .collect::<Result<Vec<_>, String>>()?;
        self.commit_event(
            &mut state,
            PlannerDraftEvent::ReorderPoints {
                point_ids: order.into_iter().map(PointId::as_bytes).collect(),
            },
        )
    }

    fn set_vehicle_profile(
        &self,
        input: PlannerVehicleProfileInput,
    ) -> Result<PlannerRouteView, String> {
        profile_from_input(&input)?;
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        self.commit_event(
            &mut state,
            PlannerDraftEvent::SetVehicleProfile { profile: input },
        )
    }
}

fn recover_journal(path: &Path) -> Result<PlannerState, String> {
    if !path
        .try_exists()
        .map_err(|error| format!("Could not inspect Planner journal: {error}"))?
    {
        return Ok(PlannerState::default());
    }

    let bytes =
        fs::read(path).map_err(|error| format!("Could not read Planner journal: {error}"))?;
    let mut state = PlannerState::default();
    let mut line_start = 0;

    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        let line = &bytes[line_start..index];
        line_start = index + 1;
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let record: PlannerJournalRecord = serde_json::from_slice(line)
            .map_err(|error| format!("Planner journal contains an invalid record: {error}"))?;
        if record.schema != PLANNER_JOURNAL_SCHEMA {
            return Err(format!(
                "Unsupported Planner journal schema: {}",
                record.schema
            ));
        }
        apply_event(&mut state, &record.event)?;
    }

    // A crash can leave only the final append without its newline. Because a command
    // is acknowledged only after write + flush + sync_data, that incomplete tail was
    // never committed and is intentionally ignored during recovery.
    Ok(state)
}

fn apply_event(state: &mut PlannerState, event: &PlannerDraftEvent) -> Result<(), String> {
    match event {
        PlannerDraftEvent::ResetRoute { route_serial } => {
            consume_serial(state, *route_serial)?;
            state.route = manual_working_route(route_id(*route_serial), &[])
                .map_err(|error| format!("Could not recover manual route: {error:?}"))?;
            state.points.clear();
            state.vehicle_profile = None;
        }
        PlannerDraftEvent::AddPoint {
            point_serial,
            title,
            latitude,
            longitude,
        } => {
            let input = PlannerPointInput {
                title: title.clone(),
                latitude: *latitude,
                longitude: *longitude,
            };
            validate_point(&input)?;
            consume_serial(state, *point_serial)?;
            let id = point_id(*point_serial);
            let mut order = state
                .route
                .points
                .iter()
                .map(|point| point.point_id)
                .collect::<Vec<_>>();
            order.push(id);
            let updated = edit_manual_working_route(&state.route, &order)
                .map_err(|error| format!("Planner rejected recovered point edit: {error:?}"))?;
            state.points.insert(
                id,
                ManualPoint {
                    point_id: id,
                    title: title.clone(),
                    latitude: *latitude,
                    longitude: *longitude,
                },
            );
            state.route = updated;
        }
        PlannerDraftEvent::RemovePoint { point_id: bytes } => {
            let id = PointId::from_bytes(*bytes)
                .map_err(|error| format!("Planner journal contains invalid point id: {error:?}"))?;
            if !state.points.contains_key(&id) {
                return Err("Planner journal removes an unknown point".to_owned());
            }
            let order = state
                .route
                .points
                .iter()
                .map(|point| point.point_id)
                .filter(|candidate| *candidate != id)
                .collect::<Vec<_>>();
            let updated = edit_manual_working_route(&state.route, &order)
                .map_err(|error| format!("Planner rejected recovered point removal: {error:?}"))?;
            state.route = updated;
            state.points.remove(&id);
        }
        PlannerDraftEvent::ReorderPoints { point_ids } => {
            let order = point_ids
                .iter()
                .copied()
                .map(|bytes| {
                    PointId::from_bytes(bytes).map_err(|error| {
                        format!("Planner journal contains invalid point id: {error:?}")
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let known = state.points.keys().copied().collect::<BTreeSet<_>>();
            let proposed = order.iter().copied().collect::<BTreeSet<_>>();
            if order.len() != state.route.points.len()
                || proposed.len() != order.len()
                || proposed != known
            {
                return Err("Planner journal contains an invalid point order".to_owned());
            }
            state.route = edit_manual_working_route(&state.route, &order)
                .map_err(|error| format!("Planner rejected recovered point order: {error:?}"))?;
        }
        PlannerDraftEvent::SetVehicleProfile { profile } => {
            state.vehicle_profile = Some(profile_from_input(profile)?);
        }
    }
    Ok(())
}

fn toll_preference(value: &str) -> Result<TollPreference, String> {
    match value {
        "allow" => Ok(TollPreference::Allow),
        "prefer_avoid" => Ok(TollPreference::PreferAvoid),
        "avoid" => Ok(TollPreference::Avoid),
        _ => Err("Потрібно явно вибрати ставлення до платних доріг".to_owned()),
    }
}

fn profile_from_input(input: &PlannerVehicleProfileInput) -> Result<VehicleRoutingProfile, String> {
    let hazmat = input
        .hazmat
        .ok_or_else(|| "Потрібно явно вказати, чи є небезпечний вантаж".to_owned())?;
    let hgv = input
        .hgv
        .ok_or_else(|| "Потрібно явно вказати, чи це вантажний автомобіль".to_owned())?;
    let exclude_unpaved = input
        .exclude_unpaved
        .ok_or_else(|| "Потрібно явно вказати ставлення до ґрунтових доріг".to_owned())?;
    let prefer_truck_routes = input
        .prefer_truck_routes
        .ok_or_else(|| "Потрібно явно вказати пріоритет вантажних доріг".to_owned())?;
    let toll_preference = input
        .toll_preference
        .as_deref()
        .ok_or_else(|| "Потрібно явно вибрати ставлення до платних доріг".to_owned())
        .and_then(toll_preference)?;

    let profile = VehicleRoutingProfile {
        height_m: input.height_m,
        width_m: input.width_m,
        length_m: input.length_m,
        gross_weight_t: input.gross_weight_t,
        axle_load_t: input.axle_load_t,
        axle_count: input.axle_count,
        hazmat,
        hgv,
        exclude_unpaved,
        prefer_truck_routes,
        toll_preference,
        max_speed_kmh: input.max_speed_kmh,
    };
    profile
        .validate()
        .map_err(|error| format!("Некоректний профіль авто: {error}"))?;
    Ok(profile)
}

fn profile_view(profile: &VehicleRoutingProfile) -> PlannerVehicleProfileView {
    let warnings = profile
        .warnings()
        .into_iter()
        .map(|warning| match warning {
            cargo_dispatch_geo::GeoWarning::MissingVehicleDimension(
                cargo_dispatch_geo::VehicleDimension::Height,
            ) => "height_m",
            cargo_dispatch_geo::GeoWarning::MissingVehicleDimension(
                cargo_dispatch_geo::VehicleDimension::Width,
            ) => "width_m",
            cargo_dispatch_geo::GeoWarning::MissingVehicleDimension(
                cargo_dispatch_geo::VehicleDimension::Length,
            ) => "length_m",
            cargo_dispatch_geo::GeoWarning::MissingVehicleDimension(
                cargo_dispatch_geo::VehicleDimension::GrossWeight,
            ) => "gross_weight_t",
            cargo_dispatch_geo::GeoWarning::MissingVehicleDimension(
                cargo_dispatch_geo::VehicleDimension::AxleLoad,
            ) => "axle_load_t",
            cargo_dispatch_geo::GeoWarning::MissingVehicleDimension(
                cargo_dispatch_geo::VehicleDimension::AxleCount,
            ) => "axle_count",
            cargo_dispatch_geo::GeoWarning::ApproximateEstimate => "approximate_estimate",
        })
        .map(str::to_owned)
        .collect();

    PlannerVehicleProfileView {
        height_m: profile.height_m,
        width_m: profile.width_m,
        length_m: profile.length_m,
        gross_weight_t: profile.gross_weight_t,
        axle_load_t: profile.axle_load_t,
        axle_count: profile.axle_count,
        hazmat: profile.hazmat,
        hgv: profile.hgv,
        exclude_unpaved: profile.exclude_unpaved,
        prefer_truck_routes: profile.prefer_truck_routes,
        toll_preference: match profile.toll_preference {
            TollPreference::Allow => "allow",
            TollPreference::PreferAvoid => "prefer_avoid",
            TollPreference::Avoid => "avoid",
        }
        .to_owned(),
        max_speed_kmh: profile.max_speed_kmh,
        fingerprint: profile.profile_fingerprint(),
        warnings,
    }
}

fn consume_serial(state: &mut PlannerState, serial: u64) -> Result<(), String> {
    if serial != state.next_serial {
        return Err(format!(
            "Planner journal serial mismatch: expected {}, got {serial}",
            state.next_serial
        ));
    }
    state.next_serial = serial.checked_add(1).ok_or("Planner id overflow")?;
    Ok(())
}

fn id_bytes(kind: u8, serial: u64) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[0] = kind;
    bytes[8..].copy_from_slice(&serial.to_be_bytes());
    bytes
}

fn route_id(serial: u64) -> WorkingRouteId {
    WorkingRouteId::from_bytes(id_bytes(0x52, serial)).expect("route id is non-nil")
}

fn point_id(serial: u64) -> PointId {
    PointId::from_bytes(id_bytes(0x50, serial)).expect("point id is non-nil")
}

fn hex_id(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_point(input: &PlannerPointInput) -> Result<(), String> {
    if input.title.trim().is_empty() {
        return Err("Point title is required".to_owned());
    }
    if !input.latitude.is_finite() || !(-90.0..=90.0).contains(&input.latitude) {
        return Err("Latitude must be between -90 and 90".to_owned());
    }
    if !input.longitude.is_finite() || !(-180.0..=180.0).contains(&input.longitude) {
        return Err("Longitude must be between -180 and 180".to_owned());
    }
    Ok(())
}

fn point_from_text(state: &PlannerState, value: &str) -> Result<PointId, String> {
    state
        .points
        .keys()
        .copied()
        .find(|id| hex_id(id.as_bytes()) == value)
        .ok_or_else(|| "Unknown point id".to_owned())
}

fn route_view(state: &PlannerState) -> Result<PlannerRouteView, String> {
    let points = state
        .route
        .points
        .iter()
        .enumerate()
        .map(|(index, route_point)| {
            let point = state
                .points
                .get(&route_point.point_id)
                .ok_or_else(|| "Planner point metadata is missing".to_owned())?;
            Ok(PlannerPointView {
                point_id: hex_id(point.point_id.as_bytes()),
                sequence: index + 1,
                title: point.title.clone(),
                latitude: point.latitude,
                longitude: point.longitude,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let state_name = match state.route.state {
        WorkingRouteState::Draft => "draft",
        WorkingRouteState::Selected => "selected",
        WorkingRouteState::Negotiating => "negotiating",
        WorkingRouteState::PartiallyConfirmed => "partially_confirmed",
        WorkingRouteState::Ready => "ready",
        WorkingRouteState::TransferredToControl => "transferred_to_control",
        WorkingRouteState::Replanning => "replanning",
        WorkingRouteState::Cancelled => "cancelled",
    };
    Ok(PlannerRouteView {
        working_route_id: hex_id(state.route.working_route_id.as_bytes()),
        revision: state.route.current_revision,
        state: state_name.to_owned(),
        points,
        vehicle_profile: state.vehicle_profile.as_ref().map(profile_view),
    })
}

#[tauri::command]
fn planner_status(state: State<'_, PlannerDesktopService>) -> Result<PlannerRouteView, String> {
    state.status()
}

#[tauri::command]
fn planner_reset_route(
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRouteView, String> {
    state.reset()
}

#[tauri::command]
fn planner_add_point(
    input: PlannerPointInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRouteView, String> {
    state.add_point(input)
}

#[tauri::command]
fn planner_remove_point(
    input: PointActionInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRouteView, String> {
    state.remove_point(input)
}

#[tauri::command]
fn planner_reorder_points(
    input: ReorderPointsInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRouteView, String> {
    state.reorder(input)
}

#[tauri::command]
fn planner_set_vehicle_profile(
    input: PlannerVehicleProfileInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRouteView, String> {
    state.set_vehicle_profile(input)
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data_dir)?;
            let service = PlannerDesktopService::open(app_data_dir.join(PLANNER_JOURNAL_FILE))
                .map_err(std::io::Error::other)?;
            app.manage(service);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            planner_status,
            planner_reset_route,
            planner_add_point,
            planner_remove_point,
            planner_reorder_points,
            planner_set_vehicle_profile
        ])
        .run(tauri::generate_context!())
        .expect("Cargo Dispatch Planner failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn point(title: &str, latitude: f64, longitude: f64) -> PlannerPointInput {
        PlannerPointInput {
            title: title.to_owned(),
            latitude,
            longitude,
        }
    }

    fn unique_journal(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cargo-dispatch-planner-{name}-{}-{nonce}.jsonl",
            std::process::id()
        ))
    }

    #[test]
    fn manual_route_service_uses_core_revision_for_add_reorder_remove() {
        let service = PlannerDesktopService::default();
        let initial = service.status().expect("initial");
        assert_eq!(initial.revision, 1);
        assert_eq!(initial.state, "draft");
        assert!(initial.points.is_empty());
        assert_eq!(
            service
                .add_point(point("A", 49.8393, 24.0296))
                .expect("A")
                .revision,
            2
        );
        assert_eq!(
            service
                .add_point(point("B", 49.8430, 24.0360))
                .expect("B")
                .revision,
            3
        );
        let current = service.add_point(point("C", 49.8500, 24.0500)).expect("C");
        assert_eq!(current.revision, 4);
        let reordered = service
            .reorder(ReorderPointsInput {
                point_ids: vec![
                    current.points[2].point_id.clone(),
                    current.points[0].point_id.clone(),
                    current.points[1].point_id.clone(),
                ],
            })
            .expect("reorder");
        assert_eq!(reordered.revision, 5);
        assert_eq!(
            reordered
                .points
                .iter()
                .map(|point| point.title.as_str())
                .collect::<Vec<_>>(),
            vec!["C", "A", "B"]
        );
        assert_eq!(reordered.state, "draft");
        let removed = service
            .remove_point(PointActionInput {
                point_id: reordered.points[1].point_id.clone(),
            })
            .expect("remove");
        assert_eq!(removed.revision, 6);
        assert_eq!(
            removed
                .points
                .iter()
                .map(|point| point.title.as_str())
                .collect::<Vec<_>>(),
            vec!["C", "B"]
        );
    }

    fn profile_input() -> PlannerVehicleProfileInput {
        PlannerVehicleProfileInput {
            height_m: None,
            width_m: Some(2.45),
            length_m: None,
            gross_weight_t: Some(12.0),
            axle_load_t: None,
            axle_count: Some(2),
            hazmat: Some(false),
            hgv: Some(true),
            exclude_unpaved: Some(true),
            prefer_truck_routes: Some(true),
            toll_preference: Some("prefer_avoid".to_owned()),
            max_speed_kmh: None,
        }
    }

    #[test]
    fn vehicle_profile_requires_explicit_non_numeric_choices() {
        let service = PlannerDesktopService::default();
        let mut input = profile_input();
        input.hazmat = None;
        assert!(service.set_vehicle_profile(input).is_err());
        assert!(service.status().expect("status").vehicle_profile.is_none());

        let configured = service
            .set_vehicle_profile(profile_input())
            .expect("configured profile");
        let profile = configured.vehicle_profile.expect("profile");
        assert_eq!(profile.width_m, Some(2.45));
        assert!(!profile.hazmat);
        assert!(profile.hgv);
        assert_eq!(profile.toll_preference, "prefer_avoid");
        assert!(profile.warnings.contains(&"height_m".to_owned()));
    }

    #[test]
    fn vehicle_profile_is_recovered_from_journal() {
        let path = unique_journal("profile-recovery");
        let service = PlannerDesktopService::open(path.clone()).expect("open journal");
        service
            .set_vehicle_profile(profile_input())
            .expect("save profile");
        drop(service);

        let recovered = PlannerDesktopService::open(path.clone()).expect("recover");
        let profile = recovered
            .status()
            .expect("status")
            .vehicle_profile
            .expect("profile");
        assert_eq!(profile.gross_weight_t, Some(12.0));
        assert_eq!(profile.axle_count, Some(2));
        assert!(profile.exclude_unpaved);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_coordinates_do_not_change_revision() {
        let service = PlannerDesktopService::default();
        assert!(service.add_point(point("bad", 91.0, 24.0)).is_err());
        assert_eq!(service.status().expect("status").revision, 1);
    }

    #[test]
    fn journal_recovers_route_order_revision_and_next_id() {
        let path = unique_journal("recovery");
        let service = PlannerDesktopService::open(path.clone()).expect("open journal");
        let first = service
            .add_point(point("Warehouse", 49.8393, 24.0296))
            .expect("first");
        let second = service
            .add_point(point("Customer", 49.8430, 24.0360))
            .expect("second");
        let reordered = service
            .reorder(ReorderPointsInput {
                point_ids: vec![
                    second.points[1].point_id.clone(),
                    first.points[0].point_id.clone(),
                ],
            })
            .expect("reorder");
        assert_eq!(reordered.revision, 4);
        drop(service);

        let recovered = PlannerDesktopService::open(path.clone()).expect("recover");
        let view = recovered.status().expect("status");
        assert_eq!(view.revision, 4);
        assert_eq!(
            view.points
                .iter()
                .map(|point| point.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Customer", "Warehouse"]
        );
        let after_restart = recovered
            .add_point(point("After restart", 49.8500, 24.0500))
            .expect("new point");
        assert_eq!(after_restart.revision, 5);
        assert_ne!(
            after_restart.points[2].point_id,
            after_restart.points[0].point_id
        );
        assert_ne!(
            after_restart.points[2].point_id,
            after_restart.points[1].point_id
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn recovery_ignores_only_an_incomplete_final_record() {
        let path = unique_journal("truncated-tail");
        let service = PlannerDesktopService::open(path.clone()).expect("open journal");
        let expected = service
            .add_point(point("Committed", 49.8393, 24.0296))
            .expect("point");
        drop(service);

        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open tail");
        file.write_all(br#"{"schema":1,"event":{"type":"add_point""#)
            .expect("write partial tail");
        file.sync_data().expect("sync partial tail");
        drop(file);

        let recovered = PlannerDesktopService::open(path.clone()).expect("recover");
        let view = recovered.status().expect("status");
        assert_eq!(view, expected);

        let _ = fs::remove_file(path);
    }
}
