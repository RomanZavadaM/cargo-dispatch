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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerVehicleInput {
    pub name: String,
    pub profile: PlannerVehicleProfileInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerVehicleView {
    pub vehicle_id: String,
    pub name: String,
    pub profile: PlannerVehicleProfileView,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerVehiclePoolView {
    pub vehicles: Vec<PlannerVehicleView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerVehicleProfileView {
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
    pub toll_preference: String,
    pub max_speed_kmh: Option<f64>,
    pub fingerprint: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PointActionInput {
    pub point_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReorderPointsInput {
    pub point_ids: Vec<String>,
}
