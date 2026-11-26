#![forbid(unsafe_code)]

use cargo_dispatch_domain::{PointId, WorkingRouteId};
use cargo_dispatch_geo::{
    GeoPoint, NormalizedStatus, RouteRequest, TollPreference, VehicleRoutingProfile,
    valhalla_http::{ValhallaHttpClient, ValhallaHttpError, ValhallaRuntimeConfig},
};
use cargo_dispatch_planner::{
    WorkingRoute, WorkingRouteState, edit_manual_working_route, manual_working_route,
};
use cargo_dispatch_planner_ui_contracts::{
    PlannerGeometryPointView, PlannerPointInput, PlannerPointView, PlannerRoutePreviewInput,
    PlannerRoutePreviewView, PlannerRouteView, PlannerRoutingConfigInput, PlannerRoutingConfigView,
    PlannerRoutingProbeView, PlannerVehicleProfileInput, PointActionInput, ReorderPointsInput,
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

    fn routing_status(&self) -> Result<PlannerRoutingConfigView, String> {
        let routing = self
            .routing
            .lock()
            .map_err(|_| "Налаштування маршрутизації тимчасово недоступні".to_owned())?;
        Ok(match routing.as_ref() {
            Some(config) => PlannerRoutingConfigView {
                configured: true,
                base_url: Some(config.base_url().to_string()),
                map_dataset_id: Some(config.map_dataset_id().to_owned()),
                connect_timeout_ms: Some(config.connect_timeout().as_millis() as u64),
                request_timeout_ms: Some(config.request_timeout().as_millis() as u64),
            },
            None => PlannerRoutingConfigView {
                configured: false,
                base_url: None,
                map_dataset_id: None,
                connect_timeout_ms: None,
                request_timeout_ms: None,
            },
        })
    }

    fn configure_routing(
        &self,
        input: PlannerRoutingConfigInput,
    ) -> Result<PlannerRoutingConfigView, String> {
        let base_url = input.base_url.trim();
        let map_dataset_id = input.map_dataset_id.trim();
        if base_url.is_empty() {
            return Err("Потрібно вказати адресу Valhalla".to_owned());
        }
        if map_dataset_id.is_empty() {
            return Err("Потрібно вказати ідентифікатор набору карт".to_owned());
        }
        if !(100..=30_000).contains(&input.connect_timeout_ms) {
            return Err("Таймаут підключення має бути 100–30000 мс".to_owned());
        }
        if !(250..=120_000).contains(&input.request_timeout_ms)
            || input.request_timeout_ms < input.connect_timeout_ms
        {
            return Err(
                "Таймаут запиту має бути 250–120000 мс і не меншим за таймаут підключення"
                    .to_owned(),
            );
        }

        let config = ValhallaRuntimeConfig::new(
            base_url,
            map_dataset_id,
            Duration::from_millis(input.connect_timeout_ms),
            Duration::from_millis(input.request_timeout_ms),
        )
        .map_err(|error| format!("Некоректні налаштування Valhalla: {error}"))?;

        *self
            .routing
            .lock()
            .map_err(|_| "Налаштування маршрутизації тимчасово недоступні".to_owned())? =
            Some(config);
        self.routing_status()
    }

    fn disable_routing(&self) -> Result<PlannerRoutingConfigView, String> {
        *self
            .routing
            .lock()
            .map_err(|_| "Налаштування маршрутизації тимчасово недоступні".to_owned())? = None;
        self.routing_status()
    }

    fn routing_client(&self) -> Result<Option<ValhallaHttpClient>, String> {
        let config = self
            .routing
            .lock()
            .map_err(|_| "Налаштування маршрутизації тимчасово недоступні".to_owned())?
            .clone();
        config
            .map(ValhallaHttpClient::new)
            .transpose()
            .map_err(|error| format!("Не вдалося підготувати клієнт Valhalla: {error}"))
    }

    async fn test_routing(&self) -> Result<PlannerRoutingProbeView, String> {
        let Some(client) = self.routing_client()? else {
            return Ok(PlannerRoutingProbeView {
                state: "not_configured".to_owned(),
                engine_version: None,
                tileset_last_modified: None,
            });
        };
        let availability = client
            .availability()
            .await
            .map_err(|error| format!("Не вдалося перевірити Valhalla: {error}"))?;
        Ok(PlannerRoutingProbeView {
            state: normalized_status_name(availability.status).to_owned(),
            engine_version: availability.engine_version,
            tileset_last_modified: availability.tileset_last_modified,
        })
    }

    fn route_snapshot(
        &self,
        based_on_revision: u64,
    ) -> Result<(u64, Vec<GeoPoint>), String> {
        let state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        if state.route.current_revision != based_on_revision {
            return Err("Чернетка маршруту вже змінилася; оновіть перегляд".to_owned());
        }
        if state.route.points.len() < 2 {
            return Err("Для дорожнього маршруту потрібно щонайменше дві точки".to_owned());
        }

        let points = state
            .route
            .points
            .iter()
            .map(|route_point| {
                let point = state
                    .points
                    .get(&route_point.point_id)
                    .ok_or_else(|| "У Planner відсутні координати точки".to_owned())?;
                GeoPoint::new(point.latitude, point.longitude)
                    .map_err(|error| format!("Некоректні координати точки: {error}"))
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok((state.route.current_revision, points))
    }

    async fn preview_route(
        &self,
        input: PlannerRoutePreviewInput,
    ) -> Result<PlannerRoutePreviewView, String> {
        let Some(client) = self.routing_client()? else {
            return Ok(empty_preview(
                input.based_on_revision,
                "not_configured",
                None,
                None,
            ));
        };

        let profile = vehicle_profile(input.profile)?;
        let (revision, points) = self.route_snapshot(input.based_on_revision)?;
        let request = RouteRequest::new(points, profile)
            .map_err(|error| format!("Не вдалося підготувати запит маршруту: {error}"))?;

        let result = match client.route(&request).await {
            Ok(result) => result,
            Err(ValhallaHttpError::Unavailable(status)) => {
                return Ok(empty_preview(
                    revision,
                    normalized_status_name(status),
                    None,
                    Some(client.config().map_dataset_id().to_owned()),
                ));
            }
            Err(error) => return Err(format!("Не вдалося отримати маршрут Valhalla: {error}")),
        };

        let current_revision = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?
            .route
            .current_revision;
        if current_revision != revision {
            return Ok(empty_preview(revision, "stale", None, None));
        }

        let engine_version = Some(result.identity.engine_version().to_owned());
        let map_dataset_id = Some(result.identity.map_dataset_id().to_owned());
        if result.status != NormalizedStatus::Ok {
            return Ok(empty_preview(
                revision,
                normalized_status_name(result.status),
                engine_version,
                map_dataset_id,
            ));
        }

        Ok(PlannerRoutePreviewView {
            based_on_revision: revision,
            state: "ok".to_owned(),
            engine_version,
            map_dataset_id,
            distance_m: result.distance_m,
            duration_s: result.duration_s,
            geometry: result
                .geometry
                .into_iter()
                .map(|point| PlannerGeometryPointView {
                    latitude: point.lat(),
                    longitude: point.lon(),
                })
                .collect(),
        })
    }
}

fn normalized_status_name(status: NormalizedStatus) -> &'static str {
    match status {
        NormalizedStatus::Ok => "ok",
        NormalizedStatus::NoRoute => "no_route",
        NormalizedStatus::InvalidPoint => "invalid_point",
        NormalizedStatus::InvalidProfile => "invalid_profile",
        NormalizedStatus::EngineUnavailable => "engine_unavailable",
        NormalizedStatus::Timeout => "timeout",
        NormalizedStatus::DatasetMissing => "dataset_missing",
        NormalizedStatus::EngineError => "engine_error",
    }
}

fn empty_preview(
    revision: u64,
    state: &str,
    engine_version: Option<String>,
    map_dataset_id: Option<String>,
) -> PlannerRoutePreviewView {
    PlannerRoutePreviewView {
        based_on_revision: revision,
        state: state.to_owned(),
        engine_version,
        map_dataset_id,
        distance_m: None,
        duration_s: None,
        geometry: Vec::new(),
    }
}

fn vehicle_profile(input: PlannerVehicleProfileInput) -> Result<VehicleRoutingProfile, String> {
    if !input.confirmed {
        return Err(
            "Підтвердіть параметри автомобіля перед розрахунком дорожнього маршруту".to_owned(),
        );
    }

    let hazmat = input
        .hazmat
        .ok_or_else(|| "Вкажіть, чи перевозиться небезпечний вантаж".to_owned())?;
    let hgv = input
        .hgv
        .ok_or_else(|| "Вкажіть, чи є автомобіль вантажним HGV".to_owned())?;
    let exclude_unpaved = input
        .exclude_unpaved
        .ok_or_else(|| "Вкажіть політику щодо ґрунтових доріг".to_owned())?;
    let prefer_truck_routes = input
        .prefer_truck_routes
        .ok_or_else(|| "Вкажіть, чи надавати перевагу вантажним маршрутам".to_owned())?;
    let toll_preference = match input.toll_preference.as_deref() {
        Some("allow") => TollPreference::Allow,
        Some("prefer_avoid") => TollPreference::PreferAvoid,
        Some("avoid") => TollPreference::Avoid,
        _ => return Err("Вкажіть політику щодо платних доріг".to_owned()),
    };

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
        .map_err(|error| format!("Некоректні параметри автомобіля: {error}"))?;
    Ok(profile)
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
) -> Result<PlannerRoutingConfigView, String> {
    state.routing_status()
}

#[tauri::command]
fn planner_configure_routing(
    input: PlannerRoutingConfigInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRoutingConfigView, String> {
    state.configure_routing(input)
}

#[tauri::command]
fn planner_disable_routing(
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRoutingConfigView, String> {
    state.disable_routing()
}

#[tauri::command]
async fn planner_test_routing(
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRoutingProbeView, String> {
    state.test_routing().await
}

#[tauri::command]
async fn planner_preview_route(
    input: PlannerRoutePreviewInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRoutePreviewView, String> {
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

    fn routing_profile_input() -> PlannerVehicleProfileInput {
        PlannerVehicleProfileInput {
            height_m: None,
            width_m: Some(2.4),
            length_m: Some(8.5),
            gross_weight_t: Some(12.0),
            axle_load_t: None,
            axle_count: Some(2),
            hazmat: Some(false),
            hgv: Some(true),
            exclude_unpaved: Some(true),
            prefer_truck_routes: Some(true),
            toll_preference: Some("allow".to_owned()),
            max_speed_kmh: None,
            confirmed: true,
        }
    }

    #[test]
    fn routing_profile_preserves_unknown_dimensions_without_defaults() {
        let profile = vehicle_profile(routing_profile_input()).expect("profile");
        assert_eq!(profile.height_m, None);
        assert_eq!(profile.axle_load_t, None);
        assert_eq!(profile.width_m, Some(2.4));
        assert!(profile.hgv);
        assert!(profile.exclude_unpaved);
    }

    #[test]
    fn routing_profile_requires_explicit_confirmation_and_boolean_choices() {
        let mut unconfirmed = routing_profile_input();
        unconfirmed.confirmed = false;
        assert!(vehicle_profile(unconfirmed).is_err());

        let mut unknown_hgv = routing_profile_input();
        unknown_hgv.hgv = None;
        assert!(vehicle_profile(unknown_hgv).is_err());
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
