#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterTripInput {
    pub folder_path: String,
    pub token: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionStatus {
    pub trip_id: String,
    pub token: String,
    pub display_name: String,
    pub endpoint_id: Option<String>,
    pub endpoint_label: Option<String>,
    pub vehicle_label: Option<String>,
    pub vehicle_type: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardActualContext {
    pub display_name: Option<String>,
    pub endpoint_id: Option<String>,
    pub endpoint_label: Option<String>,
    pub vehicle_label: Option<String>,
    pub vehicle_type: Option<String>,
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
