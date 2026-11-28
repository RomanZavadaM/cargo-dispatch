#![forbid(unsafe_code)]

mod input;
mod planning;
mod problems;
pub use input::{
    INPUT_SCHEMA, INPUT_SCHEMA_VERSION, InputCargo, InputOrder, InputPlace, InputProblem,
    InputProblemKind, InputVehicle, Money, OLDER_INPUT_SCHEMA_VERSION, PlanningInput, parse_moment,
    validate_input,
};
pub use planning::{
    AppliedPlanView, ApplyPlanInput, CargoRowDraft, PackageDraft, PlanKind, PlanProposalSetView,
    PlanProposalView, PlanStopAction, PlanStopRef, PlanStopView, PlanningStateView, ProfileDraft,
    ProposeInput, RoadState, SkipReason, SkippedOrder, UpdatePlanningInput, VehicleType,
    VehicleTypeName,
};
pub use problems::{ProblemKind, classify_problem, localize_problem};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerPointInput {
    pub title: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerPointView {
    pub point_id: String,
    pub sequence: usize,
    pub title: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerRouteView {
    pub working_route_id: String,
    pub revision: u64,
    pub state: String,
    pub points: Vec<PlannerPointView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PointActionInput {
    pub point_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReorderPointsInput {
    pub point_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannerRoutingConfigInput {
    pub base_url: String,
    pub map_dataset_id: String,
    pub connect_timeout_ms: u64,
    pub request_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannerRoutingStatus {
    pub base_url: String,
    pub map_dataset_id: String,
    pub connect_timeout_ms: u64,
    pub request_timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlannerRoutingAvailabilityState {
    NotConfigured,
    Ok,
    Timeout,
    EngineUnavailable,
    DatasetMissing,
    EngineError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannerRoutingProbeResult {
    pub state: PlannerRoutingAvailabilityState,
    pub engine_version: Option<String>,
    pub tileset_last_modified: Option<u64>,
}

/// Range of `max_speed_kmh` accepted for a new routing calculation. The desktop
/// backend enforces the same bounds through the geo contract.
pub const MIN_ROUTING_MAX_SPEED_KMH: f64 = 10.0;
pub const MAX_ROUTING_MAX_SPEED_KMH: f64 = 252.0;

#[must_use]
pub fn is_supported_max_speed_kmh(value: f64) -> bool {
    (MIN_ROUTING_MAX_SPEED_KMH..=MAX_ROUTING_MAX_SPEED_KMH).contains(&value)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlannerTollPreference {
    Allow,
    PreferAvoid,
    Avoid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerRoutingProfileInput {
    pub height_m: Option<f64>,
    pub width_m: Option<f64>,
    pub length_m: Option<f64>,
    pub gross_weight_t: Option<f64>,
    pub axle_load_t: Option<f64>,
    pub axle_count: Option<u8>,
    pub hazmat: bool,
    pub hgv: bool,
    pub exclude_unpaved: bool,
    pub prefer_truck_routes: bool,
    pub toll_preference: PlannerTollPreference,
    pub max_speed_kmh: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerRoutingPreviewInput {
    pub profile: PlannerRoutingProfileInput,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlannerRoutingPreviewState {
    Ok,
    NotConfigured,
    NoRoute,
    Timeout,
    EngineUnavailable,
    DatasetMissing,
    EngineError,
    StaleDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerRoadGeometryPoint {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerRoutingPreviewView {
    pub working_route_id: String,
    pub revision: u64,
    pub state: PlannerRoutingPreviewState,
    pub engine_version: Option<String>,
    pub map_dataset_id: Option<String>,
    pub profile_fingerprint: String,
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
    pub geometry: Vec<PlannerRoadGeometryPoint>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannerPackageCargoInput {
    pub row_id: u64,
    pub title: String,
    pub weight_kg: Option<u32>,
    pub pallets: Option<u16>,
    pub pickup_point_id: Option<String>,
    pub unload_point_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerPackageExportInput {
    pub path: String,
    pub vehicle_type: String,
    pub cargos: Vec<PlannerPackageCargoInput>,
    pub routing_profile: Option<PlannerRoutingProfileInput>,
    pub routing_preview: Option<PlannerRoutingPreviewView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannerPackageExportResult {
    pub path: String,
    pub trip_id: String,
    pub trip_version: u64,
    pub schema_major: u16,
    pub schema_minor: u16,
    pub point_count: usize,
    pub cargo_count: usize,
    pub geometry_included: bool,
}
