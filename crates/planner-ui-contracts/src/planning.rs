//! What the Planner window and its service exchange while planning from orders.

use crate::{
    InputProblem, Money, PlannerPackageCargoInput, PlannerRouteView, PlannerRoutingConfigInput,
    PlannerRoutingProfileInput, PlanningInput,
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
