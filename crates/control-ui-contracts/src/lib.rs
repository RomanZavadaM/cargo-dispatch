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

impl SessionStatus {
    /// Identity of one rendered session card. The Control UI keys its session list
    /// with it, so a card is rebuilt whenever anything it shows changes, such as
    /// the Driver URL appearing after the listener starts.
    #[must_use]
    pub fn card_key(&self) -> String {
        format!("{self:?}")
    }
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

/// One step of the "what to do next" helper in the Control access area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessStep {
    Connect,
    AddTrip,
    ShareLink,
}

/// Order of the helper steps and which of them are already done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessGuide {
    pub order: [AccessStep; 3],
    pub connected: bool,
    pub has_trip: bool,
}

impl AccessGuide {
    /// The Gateway refuses to start without a trip and refuses new trips while it
    /// runs, so over the internet trips come first. Locally sharing comes first.
    #[must_use]
    pub fn new(
        through_internet: bool,
        has_trip: bool,
        listener_on: bool,
        gateway_on: bool,
    ) -> Self {
        let (order, connected) = if through_internet {
            (
                [
                    AccessStep::AddTrip,
                    AccessStep::Connect,
                    AccessStep::ShareLink,
                ],
                gateway_on,
            )
        } else {
            (
                [
                    AccessStep::Connect,
                    AccessStep::AddTrip,
                    AccessStep::ShareLink,
                ],
                listener_on,
            )
        };
        Self {
            order,
            connected,
            has_trip,
        }
    }

    #[must_use]
    pub fn is_done(&self, step: AccessStep) -> bool {
        match step {
            AccessStep::Connect => self.connected,
            AccessStep::AddTrip => self.has_trip,
            AccessStep::ShareLink => false,
        }
    }

    #[must_use]
    pub fn current(&self) -> AccessStep {
        self.order
            .into_iter()
            .find(|step| !self.is_done(*step))
            .unwrap_or(AccessStep::ShareLink)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> SessionStatus {
        SessionStatus {
            trip_id: "41".repeat(16),
            token: "trip-0000000000000001".to_owned(),
            display_name: "trip-41414141".to_owned(),
            endpoint_id: None,
            endpoint_label: None,
            vehicle_label: None,
            vehicle_type: None,
            routing_profile: None,
            driver_url: None,
            public_driver_url: None,
        }
    }

    #[test]
    fn session_card_is_rebuilt_when_what_it_shows_changes() {
        let before = session();
        assert_eq!(before.card_key(), session().card_key());

        let mut local = session();
        local.driver_url = Some("http://local.invalid/driver/t".to_owned());
        assert_ne!(before.card_key(), local.card_key());

        let mut public = session();
        public.public_driver_url = Some("https://public.invalid/driver/t".to_owned());
        assert_ne!(before.card_key(), public.card_key());

        let mut renamed = session();
        renamed.vehicle_label = Some("synthetic truck".to_owned());
        assert_ne!(before.card_key(), renamed.card_key());
    }

    #[test]
    fn local_access_switches_sharing_on_before_adding_a_trip() {
        let start = AccessGuide::new(false, false, false, false);
        assert_eq!(start.current(), AccessStep::Connect);
        assert_eq!(
            AccessGuide::new(false, false, true, false).current(),
            AccessStep::AddTrip
        );
        assert_eq!(
            AccessGuide::new(false, true, true, false).current(),
            AccessStep::ShareLink
        );
    }

    #[test]
    fn internet_access_adds_trips_before_the_gateway() {
        assert_eq!(
            AccessGuide::new(true, false, false, false).current(),
            AccessStep::AddTrip
        );
        // Local sharing does not count as an internet connection.
        assert_eq!(
            AccessGuide::new(true, true, true, false).current(),
            AccessStep::Connect
        );
        assert_eq!(
            AccessGuide::new(true, true, false, true).current(),
            AccessStep::ShareLink
        );
    }
}
