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
pub struct PlannerRoutingConfigView {
    pub configured: bool,
    pub base_url: Option<String>,
    pub map_dataset_id: Option<String>,
    pub connect_timeout_ms: Option<u64>,
    pub request_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannerRoutingProbeView {
    pub state: String,
    pub engine_version: Option<String>,
    pub tileset_last_modified: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerVehicleProfileInput {
    pub height_m: Option<f64>,
    pub width_m: Option<f64>,
    pub length_m: Option<f64>,
    pub gross_weight_t: Option<f64>,
    pub axle_load_t: Option<f64>,
    pub axle_count: Option<u8>,
    pub hazmat: Option<bool>,
    pub hgv: Option<bool>,
    pub exclude_unpaved: Option<bool>,
    pub prefer_truck_routes: Option<bool>,
    pub toll_preference: Option<String>,
    pub max_speed_kmh: Option<f64>,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerRoutePreviewInput {
    pub based_on_revision: u64,
    pub profile: PlannerVehicleProfileInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerGeometryPointView {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerRoutePreviewView {
    pub based_on_revision: u64,
    pub state: String,
    pub engine_version: Option<String>,
    pub map_dataset_id: Option<String>,
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
    pub geometry: Vec<PlannerGeometryPointView>,
}

