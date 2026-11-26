#![forbid(unsafe_code)]

use cargo_dispatch_domain::{PointId, WorkingRouteId};
use cargo_dispatch_geo::{
    GeoPoint, GeoWarning, NormalizedStatus, RouteRequest, TollPreference, VehicleDimension,
    VehicleRoutingProfile,
    valhalla_http::{ValhallaHttpClient, ValhallaHttpError, ValhallaRuntimeConfig},
};
use cargo_dispatch_planner::{
    WorkingRoute, WorkingRouteState, edit_manual_working_route, manual_working_route,
};
use cargo_dispatch_planner_ui_contracts::{
    PlannerPointInput, PlannerPointView, PlannerRoadGeometryPoint, PlannerRouteView,
    PlannerRoutingAvailabilityState, PlannerRoutingConfigInput, PlannerRoutingPreviewInput,
    PlannerRoutingPreviewState, PlannerRoutingPreviewView, PlannerRoutingProbeResult,
    PlannerRoutingProfileInput, PlannerRoutingStatus, PlannerTollPreference, PointActionInput,
    ReorderPointsInput,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
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
    next_serial: u64,
}

impl Default for PlannerState {
    fn default() -> Self {
        Self {
            route: manual_working_route(route_id(1), &[]).expect("empty manual route is valid"),
            points: BTreeMap::new(),
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlannerJournalRecord {
    schema: u32,
    event: PlannerDraftEvent,
}

struct PlannerDesktopService {
    inner: Mutex<PlannerState>,
    journal_path: Option<PathBuf>,
    routing: Mutex<Option<ValhallaRuntimeConfig>>,
}

impl Default for PlannerDesktopService {
    fn default() -> Self {
        Self {
            inner: Mutex::new(PlannerState::default()),
            journal_path: None,
            routing: Mutex::new(None),
        }
    }
}

impl PlannerDesktopService {
    fn open(journal_path: PathBuf) -> Result<Self, String> {
        let state = recover_journal(&journal_path)?;
        Ok(Self {
            inner: Mutex::new(state),
            journal_path: Some(journal_path),
            routing: Mutex::new(None),
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

    fn routing_status(&self) -> Result<Option<PlannerRoutingStatus>, String> {
        let routing = self
            .routing
            .lock()
            .map_err(|_| "Planner routing state unavailable".to_owned())?;
        Ok(routing.as_ref().map(|config| PlannerRoutingStatus {
            base_url: config.base_url().to_string(),
            map_dataset_id: config.map_dataset_id().to_owned(),
            connect_timeout_ms: config.connect_timeout().as_millis() as u64,
            request_timeout_ms: config.request_timeout().as_millis() as u64,
        }))
    }

    fn configure_routing(
        &self,
        input: PlannerRoutingConfigInput,
    ) -> Result<PlannerRoutingStatus, String> {
        let base_url = input.base_url.trim();
        let map_dataset_id = input.map_dataset_id.trim();
        if base_url.is_empty() {
            return Err("Valhalla address is required".to_owned());
        }
        if map_dataset_id.is_empty() {
            return Err("Valhalla map dataset id is required".to_owned());
        }
        if !(100..=30_000).contains(&input.connect_timeout_ms) {
            return Err("Connect timeout must be 100–30000 ms".to_owned());
        }
        if !(250..=120_000).contains(&input.request_timeout_ms)
            || input.request_timeout_ms < input.connect_timeout_ms
        {
            return Err(
                "Request timeout must be 250–120000 ms and not shorter than connect timeout"
                    .to_owned(),
            );
        }
        let config = ValhallaRuntimeConfig::new(
            base_url,
            map_dataset_id,
            Duration::from_millis(input.connect_timeout_ms),
            Duration::from_millis(input.request_timeout_ms),
        )
        .map_err(|error| format!("Invalid Valhalla configuration: {error}"))?;
        let status = PlannerRoutingStatus {
            base_url: config.base_url().to_string(),
            map_dataset_id: config.map_dataset_id().to_owned(),
            connect_timeout_ms: config.connect_timeout().as_millis() as u64,
            request_timeout_ms: config.request_timeout().as_millis() as u64,
        };
        *self
            .routing
            .lock()
            .map_err(|_| "Planner routing state unavailable".to_owned())? = Some(config);
        Ok(status)
    }

    fn disable_routing(&self) -> Result<(), String> {
        *self
            .routing
            .lock()
            .map_err(|_| "Planner routing state unavailable".to_owned())? = None;
        Ok(())
    }

    async fn test_routing(&self) -> Result<PlannerRoutingProbeResult, String> {
        let config = self
            .routing
            .lock()
            .map_err(|_| "Planner routing state unavailable".to_owned())?
            .clone();
        let Some(config) = config else {
            return Ok(PlannerRoutingProbeResult {
                state: PlannerRoutingAvailabilityState::NotConfigured,
                engine_version: None,
                tileset_last_modified: None,
            });
        };
        let availability = ValhallaHttpClient::new(config)
            .map_err(|error| format!("Could not create Valhalla client: {error}"))?
            .availability()
            .await
            .map_err(|error| format!("Could not test Valhalla: {error}"))?;
        Ok(PlannerRoutingProbeResult {
            state: routing_availability_state(availability.status),
            engine_version: availability.engine_version,
            tileset_last_modified: availability.tileset_last_modified,
        })
    }

    async fn preview_route(
        &self,
        input: PlannerRoutingPreviewInput,
    ) -> Result<PlannerRoutingPreviewView, String> {
        let profile = routing_profile(input.profile)?;
        let profile_fingerprint = profile.profile_fingerprint();
        let (working_route_id, revision, points) = {
            let state = self
                .inner
                .lock()
                .map_err(|_| "Planner state unavailable".to_owned())?;
            if state.route.points.len() < 2 {
                return Err("Route preview needs at least two ordered points".to_owned());
            }
            let points = state
                .route
                .points
                .iter()
                .map(|route_point| {
                    let point = state
                        .points
                        .get(&route_point.point_id)
                        .ok_or_else(|| "Planner point metadata is missing".to_owned())?;
                    GeoPoint::new(point.latitude, point.longitude)
                        .map_err(|error| format!("Invalid Planner point: {error}"))
                })
                .collect::<Result<Vec<_>, String>>()?;
            (
                hex_id(state.route.working_route_id.as_bytes()),
                state.route.current_revision,
                points,
            )
        };

        let config = self
            .routing
            .lock()
            .map_err(|_| "Planner routing state unavailable".to_owned())?
            .clone();
        let Some(config) = config else {
            return Ok(empty_preview(
                working_route_id,
                revision,
                PlannerRoutingPreviewState::NotConfigured,
                profile_fingerprint,
                None,
                None,
            ));
        };
        let map_dataset_id = config.map_dataset_id().to_owned();
        let request = RouteRequest::new(points, profile)
            .map_err(|error| format!("Invalid routing preview request: {error}"))?;
        let client = ValhallaHttpClient::new(config)
            .map_err(|error| format!("Could not create Valhalla client: {error}"))?;
        let result = match client.route(&request).await {
            Ok(result) => result,
            Err(ValhallaHttpError::Unavailable(status)) => {
                return Ok(empty_preview(
                    working_route_id,
                    revision,
                    preview_state(status),
                    profile_fingerprint,
                    None,
                    Some(map_dataset_id),
                ));
            }
            Err(error) => return Err(format!("Valhalla preview failed: {error}")),
        };

        {
            let state = self
                .inner
                .lock()
                .map_err(|_| "Planner state unavailable".to_owned())?;
            if hex_id(state.route.working_route_id.as_bytes()) != working_route_id
                || state.route.current_revision != revision
            {
                return Ok(empty_preview(
                    working_route_id,
                    revision,
                    PlannerRoutingPreviewState::StaleDraft,
                    profile_fingerprint,
                    Some(result.identity.engine_version().to_owned()),
                    Some(result.identity.map_dataset_id().to_owned()),
                ));
            }
        }

        let state = preview_state(result.status);
        let geometry = if state == PlannerRoutingPreviewState::Ok {
            result
                .geometry
                .iter()
                .map(|point| PlannerRoadGeometryPoint {
                    latitude: point.lat(),
                    longitude: point.lon(),
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(PlannerRoutingPreviewView {
            working_route_id,
            revision,
            state,
            engine_version: Some(result.identity.engine_version().to_owned()),
            map_dataset_id: Some(result.identity.map_dataset_id().to_owned()),
            profile_fingerprint,
            distance_m: result.distance_m,
            duration_s: result.duration_s,
            geometry,
            warnings: result.warnings.into_iter().map(warning_code).collect(),
        })
    }
}

fn routing_profile(input: PlannerRoutingProfileInput) -> Result<VehicleRoutingProfile, String> {
    let profile = VehicleRoutingProfile {
        height_m: input.height_m,
        width_m: input.width_m,
        length_m: input.length_m,
        gross_weight_t: input.gross_weight_t,
        axle_load_t: input.axle_load_t,
        axle_count: input.axle_count,
        hazmat: input.hazmat,
        hgv: input.hgv,
        exclude_unpaved: input.exclude_unpaved,
        prefer_truck_routes: input.prefer_truck_routes,
        toll_preference: match input.toll_preference {
            PlannerTollPreference::Allow => TollPreference::Allow,
            PlannerTollPreference::PreferAvoid => TollPreference::PreferAvoid,
            PlannerTollPreference::Avoid => TollPreference::Avoid,
        },
        max_speed_kmh: input.max_speed_kmh,
    };
    profile
        .validate()
        .map_err(|error| format!("Invalid Planner routing profile: {error}"))?;
    Ok(profile)
}

fn routing_availability_state(status: NormalizedStatus) -> PlannerRoutingAvailabilityState {
    match status {
        NormalizedStatus::Ok => PlannerRoutingAvailabilityState::Ok,
        NormalizedStatus::Timeout => PlannerRoutingAvailabilityState::Timeout,
        NormalizedStatus::EngineUnavailable => PlannerRoutingAvailabilityState::EngineUnavailable,
        NormalizedStatus::DatasetMissing => PlannerRoutingAvailabilityState::DatasetMissing,
        NormalizedStatus::EngineError
        | NormalizedStatus::NoRoute
        | NormalizedStatus::InvalidPoint
        | NormalizedStatus::InvalidProfile => PlannerRoutingAvailabilityState::EngineError,
    }
}

fn preview_state(status: NormalizedStatus) -> PlannerRoutingPreviewState {
    match status {
        NormalizedStatus::Ok => PlannerRoutingPreviewState::Ok,
        NormalizedStatus::NoRoute => PlannerRoutingPreviewState::NoRoute,
        NormalizedStatus::Timeout => PlannerRoutingPreviewState::Timeout,
        NormalizedStatus::EngineUnavailable => PlannerRoutingPreviewState::EngineUnavailable,
        NormalizedStatus::DatasetMissing => PlannerRoutingPreviewState::DatasetMissing,
        NormalizedStatus::EngineError
        | NormalizedStatus::InvalidPoint
        | NormalizedStatus::InvalidProfile => PlannerRoutingPreviewState::EngineError,
    }
}

fn warning_code(warning: GeoWarning) -> String {
    match warning {
        GeoWarning::MissingVehicleDimension(VehicleDimension::Height) => "missing_height_m",
        GeoWarning::MissingVehicleDimension(VehicleDimension::Width) => "missing_width_m",
        GeoWarning::MissingVehicleDimension(VehicleDimension::Length) => "missing_length_m",
        GeoWarning::MissingVehicleDimension(VehicleDimension::GrossWeight) => {
            "missing_gross_weight_t"
        }
        GeoWarning::MissingVehicleDimension(VehicleDimension::AxleLoad) => "missing_axle_load_t",
        GeoWarning::MissingVehicleDimension(VehicleDimension::AxleCount) => "missing_axle_count",
        GeoWarning::ApproximateEstimate => "approximate_estimate",
    }
    .to_owned()
}

fn empty_preview(
    working_route_id: String,
    revision: u64,
    state: PlannerRoutingPreviewState,
    profile_fingerprint: String,
    engine_version: Option<String>,
    map_dataset_id: Option<String>,
) -> PlannerRoutingPreviewView {
    PlannerRoutingPreviewView {
        working_route_id,
        revision,
        state,
        engine_version,
        map_dataset_id,
        profile_fingerprint,
        distance_m: None,
        duration_s: None,
        geometry: Vec::new(),
        warnings: Vec::new(),
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
    }
    Ok(())
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
fn planner_routing_status(
    state: State<'_, PlannerDesktopService>,
) -> Result<Option<PlannerRoutingStatus>, String> {
    state.routing_status()
}

#[tauri::command]
fn planner_configure_routing(
    input: PlannerRoutingConfigInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRoutingStatus, String> {
    state.configure_routing(input)
}

#[tauri::command]
fn planner_disable_routing(state: State<'_, PlannerDesktopService>) -> Result<(), String> {
    state.disable_routing()
}

#[tauri::command]
async fn planner_test_routing(
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRoutingProbeResult, String> {
    state.test_routing().await
}

#[tauri::command]
async fn planner_preview_route(
    input: PlannerRoutingPreviewInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRoutingPreviewView, String> {
    state.preview_route(input).await
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
            planner_routing_status,
            planner_configure_routing,
            planner_disable_routing,
            planner_test_routing,
            planner_preview_route
        ])
        .run(tauri::generate_context!())
        .expect("Cargo Dispatch Planner failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::State as AxumState,
        routing::{get, post},
    };
    use serde_json::{Value, json};
    use std::{
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::{
        net::TcpListener,
        sync::Notify,
        task::JoinHandle,
    };

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

    #[derive(Clone)]
    struct MockValhalla {
        requests: Arc<AsyncMutex<Vec<Value>>>,
        route_started: Arc<Notify>,
        route_release: Arc<Notify>,
        delay_route: bool,
    }

    fn preview_profile() -> PlannerRoutingProfileInput {
        PlannerRoutingProfileInput {
            height_m: None,
            width_m: Some(2.4),
            length_m: Some(8.5),
            gross_weight_t: Some(12.0),
            axle_load_t: None,
            axle_count: Some(2),
            hazmat: false,
            hgv: true,
            exclude_unpaved: true,
            prefer_truck_routes: true,
            toll_preference: PlannerTollPreference::Allow,
            max_speed_kmh: None,
        }
    }

    async fn mock_status() -> Json<Value> {
        Json(json!({
            "version": "3.9.2",
            "tileset_last_modified": 1_795_550_000_u64
        }))
    }

    async fn mock_route(
        AxumState(mock): AxumState<MockValhalla>,
        Json(payload): Json<Value>,
    ) -> Json<Value> {
        mock.requests.lock().await.push(payload);
        mock.route_started.notify_one();
        if mock.delay_route {
            mock.route_release.notified().await;
        }
        Json(json!({
            "trip": {
                "summary": {"length": 2.75, "time": 240.0},
                "legs": [{"shape": "gk}`~A_ysyl@gfF_oKwvInnT"}]
            }
        }))
    }

    async fn spawn_mock_valhalla(
        delay_route: bool,
    ) -> std::io::Result<(String, MockValhalla, JoinHandle<()>)> {
        let mock = MockValhalla {
            requests: Arc::new(AsyncMutex::new(Vec::new())),
            route_started: Arc::new(Notify::new()),
            route_release: Arc::new(Notify::new()),
            delay_route,
        };
        let app = Router::new()
            .route("/status", get(mock_status))
            .route("/route", post(mock_route))
            .with_state(mock.clone());
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock Valhalla");
        });
        Ok((format!("http://{address}/"), mock, task))
    }

    fn configure_mock_routing(
        service: &PlannerDesktopService,
        base_url: String,
    ) -> PlannerRoutingStatus {
        service
            .configure_routing(PlannerRoutingConfigInput {
                base_url,
                map_dataset_id: "fixture-europe-2026-11".to_owned(),
                connect_timeout_ms: 250,
                request_timeout_ms: 2_000,
            })
            .expect("routing config")
    }

    #[tokio::test]
    async fn real_preview_uses_current_order_and_only_explicit_vehicle_values()
    -> std::io::Result<()> {
        let service = PlannerDesktopService::default();
        service.add_point(point("A", 49.8393, 24.0296)).expect("A");
        service.add_point(point("B", 49.8430, 24.0360)).expect("B");
        let current = service.add_point(point("C", 49.8500, 24.0500)).expect("C");
        let reordered = service
            .reorder(ReorderPointsInput {
                point_ids: vec![
                    current.points[2].point_id.clone(),
                    current.points[0].point_id.clone(),
                    current.points[1].point_id.clone(),
                ],
            })
            .expect("reorder");

        let (base_url, mock, server) = spawn_mock_valhalla(false).await?;
        let routing = configure_mock_routing(&service, base_url);
        assert_eq!(routing.map_dataset_id, "fixture-europe-2026-11");

        let probe = service.test_routing().await.expect("probe");
        assert_eq!(probe.state, PlannerRoutingAvailabilityState::Ok);
        assert_eq!(probe.engine_version.as_deref(), Some("3.9.2"));

        let profile = preview_profile();
        let expected_fingerprint = routing_profile(profile.clone())
            .expect("profile")
            .profile_fingerprint();
        let preview = service
            .preview_route(PlannerRoutingPreviewInput { profile })
            .await
            .expect("preview");

        assert_eq!(preview.state, PlannerRoutingPreviewState::Ok);
        assert_eq!(preview.revision, reordered.revision);
        assert_eq!(preview.profile_fingerprint, expected_fingerprint);
        assert_eq!(preview.engine_version.as_deref(), Some("3.9.2"));
        assert_eq!(
            preview.map_dataset_id.as_deref(),
            Some("fixture-europe-2026-11")
        );
        assert_eq!(preview.distance_m, Some(2_750.0));
        assert_eq!(preview.duration_s, Some(240.0));
        assert!(preview.geometry.len() > 1);
        assert!(preview.warnings.contains(&"missing_height_m".to_owned()));
        assert!(preview.warnings.contains(&"missing_axle_load_t".to_owned()));
        assert_eq!(
            service.status().expect("status").revision,
            reordered.revision,
            "derived preview must not mutate the Planner draft"
        );

        let requests = mock.requests.lock().await;
        assert_eq!(requests.len(), 1);
        let payload = &requests[0];
        assert_eq!(payload["locations"][0]["lat"], json!(49.8500));
        assert_eq!(payload["locations"][0]["lon"], json!(24.0500));
        assert_eq!(payload["locations"][1]["lat"], json!(49.8393));
        assert_eq!(payload["locations"][2]["lat"], json!(49.8430));
        assert!(
            payload["costing_options"]["truck"].get("height").is_none(),
            "unknown vehicle height must not be invented"
        );
        assert_eq!(payload["costing_options"]["truck"]["width"], json!(2.4));
        drop(requests);

        server.abort();
        Ok(())
    }

    #[tokio::test]
    async fn late_valhalla_preview_is_stale_after_draft_edit() -> std::io::Result<()> {
        let service = Arc::new(PlannerDesktopService::default());
        service.add_point(point("A", 49.8393, 24.0296)).expect("A");
        let before = service.add_point(point("B", 49.8430, 24.0360)).expect("B");

        let (base_url, mock, server) = spawn_mock_valhalla(true).await?;
        configure_mock_routing(&service, base_url);

        let running_service = Arc::clone(&service);
        let running = tokio::spawn(async move {
            running_service
                .preview_route(PlannerRoutingPreviewInput {
                    profile: preview_profile(),
                })
                .await
        });

        mock.route_started.notified().await;
        let edited = service
            .add_point(point("Edited while routing", 49.8500, 24.0500))
            .expect("edit");
        assert!(edited.revision > before.revision);
        mock.route_release.notify_one();

        let preview = running.await.expect("preview task").expect("preview result");
        assert_eq!(preview.state, PlannerRoutingPreviewState::StaleDraft);
        assert!(preview.geometry.is_empty());
        assert_eq!(preview.revision, before.revision);
        assert_eq!(service.status().expect("current").revision, edited.revision);

        server.abort();
        Ok(())
    }

    fn routing_profile_input() -> PlannerRoutingProfileInput {
        PlannerRoutingProfileInput {
            height_m: None,
            width_m: Some(2.55),
            length_m: Some(16.5),
            gross_weight_t: Some(40.0),
            axle_load_t: None,
            axle_count: Some(5),
            hazmat: false,
            hgv: true,
            exclude_unpaved: true,
            prefer_truck_routes: true,
            toll_preference: PlannerTollPreference::Allow,
            max_speed_kmh: Some(90.0),
        }
    }

    async fn status_ok() -> Json<Value> {
        Json(json!({
            "version": "3.9.2",
            "tileset_last_modified": 1_795_550_000_u64
        }))
    }

    async fn route_ok(Json(_payload): Json<Value>) -> Json<Value> {
        Json(json!({
            "trip": {
                "summary": {"length": 2.75, "time": 240.0},
                "legs": [
                    {"shape": "gk}`~A_ysyl@gfF_oK"},
                    {"shape": "orda~A_i`zl@wvInnT"}
                ]
            }
        }))
    }

    async fn spawn_mock_valhalla(app: Router) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind mock Valhalla");
        let address = listener.local_addr().expect("mock address");
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock Valhalla");
        });
        (format!("http://{address}/"), task)
    }

    fn configure_mock(service: &PlannerDesktopService, base_url: &str) {
        service
            .configure_routing(PlannerRoutingConfigInput {
                base_url: base_url.to_owned(),
                map_dataset_id: "fixture-europe-2026-11".to_owned(),
                connect_timeout_ms: 250,
                request_timeout_ms: 2_000,
            })
            .expect("configure mock Valhalla");
    }

    #[derive(Debug)]
    struct DelayedRoute {
        started: Notify,
        release: Notify,
    }

    async fn delayed_route(
        AxumState(delay): AxumState<Arc<DelayedRoute>>,
        Json(_payload): Json<Value>,
    ) -> Json<Value> {
        delay.started.notify_one();
        delay.release.notified().await;
        route_ok(Json(json!({}))).await
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

    #[tokio::test]
    async fn real_valhalla_preview_does_not_mutate_planner_draft() {
        let service = PlannerDesktopService::default();
        service
            .add_point(point("Warehouse", 49.8393, 24.0296))
            .expect("first point");
        let before = service
            .add_point(point("Customer", 49.8430, 24.0360))
            .expect("second point");

        let app = Router::new()
            .route("/status", get(status_ok))
            .route("/route", post(route_ok));
        let (base_url, server) = spawn_mock_valhalla(app).await;
        configure_mock(&service, &base_url);

        let probe = service.test_routing().await.expect("routing probe");
        assert_eq!(probe.state, PlannerRoutingAvailabilityState::Ok);
        assert_eq!(probe.engine_version.as_deref(), Some("3.9.2"));

        let preview = service
            .preview_route(PlannerRoutingPreviewInput {
                profile: routing_profile_input(),
            })
            .await
            .expect("road preview");
        assert_eq!(preview.state, PlannerRoutingPreviewState::Ok);
        assert_eq!(preview.working_route_id, before.working_route_id);
        assert_eq!(preview.revision, before.revision);
        assert_eq!(preview.distance_m, Some(2_750.0));
        assert_eq!(preview.duration_s, Some(240.0));
        assert!(!preview.geometry.is_empty());
        assert_eq!(preview.profile_fingerprint.len(), 64);

        let after = service.status().expect("draft after preview");
        assert_eq!(after, before);
        server.abort();
    }

    #[tokio::test]
    async fn missing_valhalla_keeps_draft_unchanged() {
        let service = PlannerDesktopService::default();
        service
            .add_point(point("A", 49.8393, 24.0296))
            .expect("first point");
        let before = service
            .add_point(point("B", 49.8430, 24.0360))
            .expect("second point");

        let preview = service
            .preview_route(PlannerRoutingPreviewInput {
                profile: routing_profile_input(),
            })
            .await
            .expect("unconfigured preview");
        assert_eq!(
            preview.state,
            PlannerRoutingPreviewState::NotConfigured
        );
        assert!(preview.geometry.is_empty());
        assert_eq!(service.status().expect("draft after failure"), before);
    }

    #[tokio::test]
    async fn late_valhalla_preview_is_rejected_after_draft_change() {
        let service = Arc::new(PlannerDesktopService::default());
        service
            .add_point(point("A", 49.8393, 24.0296))
            .expect("first point");
        service
            .add_point(point("B", 49.8430, 24.0360))
            .expect("second point");

        let delay = Arc::new(DelayedRoute {
            started: Notify::new(),
            release: Notify::new(),
        });
        let app = Router::new()
            .route("/status", get(status_ok))
            .route("/route", post(delayed_route))
            .with_state(delay.clone());
        let (base_url, server) = spawn_mock_valhalla(app).await;
        configure_mock(&service, &base_url);

        let running_service = service.clone();
        let running = tokio::spawn(async move {
            running_service
                .preview_route(PlannerRoutingPreviewInput {
                    profile: routing_profile_input(),
                })
                .await
        });

        delay.started.notified().await;
        let changed = service
            .add_point(point("C", 49.8500, 24.0500))
            .expect("draft changes while routing");
        delay.release.notify_one();

        let preview = running
            .await
            .expect("preview task")
            .expect("preview result");
        assert_eq!(preview.state, PlannerRoutingPreviewState::StaleDraft);
        assert!(preview.geometry.is_empty());
        assert!(preview.revision < changed.revision);
        assert_eq!(
            service.status().expect("current draft").revision,
            changed.revision
        );

        server.abort();
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
