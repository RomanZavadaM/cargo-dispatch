#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterTripInput {
    pub folder_path: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateManualTripPointInput {
    pub point_id: String,
    pub title: String,
    pub latitude: f64,
    pub longitude: f64,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateManualTripInput {
    pub folder_path: String,
    pub token: String,
    pub trip_id: String,
    pub display_name: Option<String>,
    pub points: Vec<CreateManualTripPointInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StartLanInput {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StartGatewayInput {
    pub gateway_base_url: String,
    pub control_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayStatus {
    pub base_url: String,
    pub active_sessions: usize,
    pub total_sessions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigureRoutingInput {
    pub base_url: String,
    pub map_dataset_id: String,
    pub connect_timeout_ms: u64,
    pub request_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingStatus {
    pub base_url: String,
    pub map_dataset_id: String,
    pub connect_timeout_ms: u64,
    pub request_timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingAvailabilityState {
    NotConfigured,
    Ok,
    Timeout,
    EngineUnavailable,
    DatasetMissing,
    EngineError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingProbeResult {
    pub state: RoutingAvailabilityState,
    pub engine_version: Option<String>,
    pub tileset_last_modified: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListenerMode {
    Loopback,
    Lan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListenerStatus {
    pub mode: ListenerMode,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionStatus {
    pub trip_id: String,
    pub token: String,
    pub display_name: String,
    pub endpoint_id: Option<String>,
    pub endpoint_label: Option<String>,
    pub vehicle_label: Option<String>,
    pub vehicle_type: Option<String>,
    pub routing_profile: Option<DashboardVehicleRoutingProfile>,
    pub driver_url: Option<String>,
    pub public_driver_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateSessionMetadataInput {
    pub token: String,
    pub display_name: String,
    pub endpoint_id: Option<String>,
    pub endpoint_label: Option<String>,
    pub vehicle_label: Option<String>,
    pub vehicle_type: Option<String>,
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
pub enum RoutingTollPreference {
    Allow,
    PreferAvoid,
    Avoid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardVehicleRoutingProfile {
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
    pub toll_preference: RoutingTollPreference,
    pub max_speed_kmh: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateRoutingContextInput {
    pub token: String,
    pub profile: Option<DashboardVehicleRoutingProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlUiStatus {
    pub listener: Option<ListenerStatus>,
    pub gateway: Option<GatewayStatus>,
    pub routing: Option<RoutingStatus>,
    pub sessions: Vec<SessionStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardNavigation {
    pub latitude: f64,
    pub longitude: f64,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardPoint {
    pub point_id: String,
    pub sequence: u32,
    pub title: String,
    pub completed: bool,
    pub navigation: Option<DashboardNavigation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: Option<f64>,
    pub observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DashboardDocumentDirection {
    FromOperator,
    FromDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardDocument {
    pub document_id: String,
    pub direction: DashboardDocumentDirection,
    pub original_name: String,
    pub mime_type: String,
    pub size: u64,
    pub point_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardRoadGeometryPoint {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardRoutingIdentity {
    pub engine: String,
    pub engine_version: String,
    pub map_dataset_id: String,
    pub profile_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardRoadGeometry {
    pub routing: DashboardRoutingIdentity,
    pub points: Vec<DashboardRoadGeometryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardPlanningCargo {
    pub cargo_id: String,
    pub title: String,
    pub weight_kg: Option<u32>,
    pub pallets: Option<u16>,
    pub pickup_point_id: Option<String>,
    pub unload_point_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardPlanningBasis {
    pub vehicle_type: String,
    pub cargos: Vec<DashboardPlanningCargo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardActualContext {
    pub display_name: Option<String>,
    pub endpoint_id: Option<String>,
    pub endpoint_label: Option<String>,
    pub vehicle_label: Option<String>,
    pub vehicle_type: Option<String>,
    pub routing_profile: Option<DashboardVehicleRoutingProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TripDashboard {
    pub trip_id: String,
    pub trip_version: u64,
    pub local_route_revision: u64,
    pub planning_basis: Option<DashboardPlanningBasis>,
    pub actual_context: DashboardActualContext,
    pub points: Vec<DashboardPoint>,
    pub road_geometry: Option<DashboardRoadGeometry>,
    pub documents: Vec<DashboardDocument>,
    pub latest_location: Option<DashboardLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInput {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveDocumentInput {
    pub token: String,
    pub document_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReorderRouteInput {
    pub token: String,
    pub based_on_revision: u64,
    pub open_point_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorDocumentInput {
    pub token: String,
    pub document_id: String,
    pub source_path: String,
    pub original_name: String,
    pub mime_type: String,
    pub point_id: Option<String>,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PickedFile {
    pub path: String,
    pub original_name: String,
    pub mime_type: String,
}
