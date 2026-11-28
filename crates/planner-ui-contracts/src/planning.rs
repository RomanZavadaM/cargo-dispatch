//! What the Planner window and its service exchange while planning from orders.

use crate::input::bad_volume;
use crate::{
    InputProblem, InputVehicle, Money, PlannerPackageCargoInput, PlannerRouteView,
    PlannerRoutingConfigInput, PlannerRoutingProfileInput, PlanningInput,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanningStateView {
    pub input: PlanningInput,
    pub problems: Vec<InputProblem>,
    /// Remembered between starts so the operator does not type it again.
    pub routing: Option<PlannerRoutingConfigInput>,
    pub routing_profile: Option<PlannerRoutingProfileInput>,
    /// Cargo rows of the route chosen last.
    pub cargos: Vec<PlannerPackageCargoInput>,
    /// The trip package section as the operator left it; wins over `cargos`.
    pub package: Option<PackageDraft>,
    /// Saved vehicle types, the one used last comes first.
    pub vehicle_types: Vec<VehicleType>,
}

/// A kind of vehicle the operator chooses from a list instead of typing it again.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VehicleType {
    pub name: String,
    pub payload_kg: Option<u32>,
    pub volume_m3: Option<f64>,
    pub pallets: Option<u16>,
    pub features: Option<String>,
}

impl VehicleType {
    /// The type described by the vehicle of the document, once it has a name.
    #[must_use]
    pub fn of(vehicle: &InputVehicle) -> Option<Self> {
        let name = vehicle.vehicle_type.as_deref()?.trim();
        (!name.is_empty()).then(|| Self {
            name: name.to_owned(),
            payload_kg: vehicle.payload_kg,
            volume_m3: vehicle.volume_m3,
            pallets: vehicle.pallets,
            features: vehicle.features.clone(),
        })
    }

    /// Copies the type into the vehicle; where it stands and its cost stay.
    pub fn apply_to(&self, vehicle: &mut InputVehicle) {
        vehicle.vehicle_type = Some(self.name.clone());
        vehicle.payload_kg = self.payload_kg;
        vehicle.volume_m3 = self.volume_m3;
        vehicle.pallets = self.pallets;
        vehicle.features.clone_from(&self.features);
    }

    /// Names differing only in letter case or outer spaces mean one type.
    #[must_use]
    pub fn is_named(&self, name: &str) -> bool {
        self.name.trim().to_lowercase() == name.trim().to_lowercase()
    }

    /// A name and, where stated, positive limits.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        !self.name.trim().is_empty()
            && self.payload_kg != Some(0)
            && self.pallets != Some(0)
            && !bad_volume(self.volume_m3)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VehicleTypeName {
    pub name: String,
}

/// The trip package section exactly as typed, so that a half-filled form comes back.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PackageDraft {
    pub vehicle_type: String,
    pub cargos: Vec<CargoRowDraft>,
    /// Row numbers give cargo its identity in a package, so none is used twice.
    pub next_cargo_row: u64,
    pub profile: ProfileDraft,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CargoRowDraft {
    pub row_id: u64,
    pub title: String,
    pub weight_kg: String,
    pub pallets: String,
    pub pickup_point_id: String,
    pub unload_point_id: String,
}

/// Vehicle parameters of the calculation as typed or chosen.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ProfileDraft {
    pub height_m: String,
    pub width_m: String,
    pub length_m: String,
    pub gross_weight_t: String,
    pub axle_load_t: String,
    pub axle_count: String,
    pub hazmat: String,
    pub hgv: String,
    pub exclude_unpaved: String,
    pub prefer_truck_routes: String,
    pub toll_preference: String,
    pub max_speed_kmh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdatePlanningInput {
    pub input: PlanningInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposeInput {
    pub order_ids: Vec<String>,
    /// Vehicle parameters for the road calculation; without them there is none.
    pub routing_profile: Option<PlannerRoutingProfileInput>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStopAction {
    Pickup,
    Delivery,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanStopRef {
    pub order_id: String,
    pub action: PlanStopAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanStopView {
    pub order_id: String,
    pub action: PlanStopAction,
    pub place: String,
    pub cargo: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanKind {
    Shortest,
    OneAtATime,
    LoadAllFirst,
    Alternative,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoadState {
    /// The road was calculated.
    Ok,
    /// No routing service or vehicle parameters.
    NotRequested,
    /// The routing service did not deliver a road.
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanProposalView {
    pub kind: PlanKind,
    pub stops: Vec<PlanStopView>,
    pub straight_km: f64,
    pub empty_km: f64,
    pub road_state: RoadState,
    pub road_km: Option<f64>,
    pub road_hours: Option<f64>,
    pub peak_weight_kg: u64,
    pub peak_pallets: u32,
    /// Zero when no volume is known.
    pub peak_volume_m3: f64,
    pub needs_check: bool,
    /// Dates of the orders contradict this order of stops.
    pub date_conflict: bool,
    /// Rates of the included orders, one sum per currency.
    pub revenue: Vec<Money>,
    /// Only when every order has a rate in the currency of the cost per km.
    pub result_after_distance_cost: Option<Money>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    Invalid,
    MissingCoordinates,
    TooLarge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkippedOrder {
    pub order_id: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanProposalSetView {
    pub proposals: Vec<PlanProposalView>,
    pub skipped: Vec<SkippedOrder>,
    /// Every order of stops was examined.
    pub exhaustive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplyPlanInput {
    pub stops: Vec<PlanStopRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppliedPlanView {
    pub route: PlannerRouteView,
    pub cargos: Vec<PlannerPackageCargoInput>,
    pub vehicle_type: String,
}
