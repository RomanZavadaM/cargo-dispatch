#![forbid(unsafe_code)]

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
