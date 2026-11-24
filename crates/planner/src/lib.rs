#![forbid(unsafe_code)]

use cargo_dispatch_domain::{AvailabilityState, LoadRequest, PlanningAssignment, RequestId};
use cargo_dispatch_geo::{GeoPoint, RouteRequest, RouteResult, VehicleRoutingProfile};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct PlanningRequest {
    pub request: LoadRequest,
    pub pickup_coordinates: Option<GeoPoint>,
    pub delivery_coordinates: Option<GeoPoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolFilter {
    pub availability: Option<AvailabilityState>,
    pub text: Option<String>,
}

impl PoolFilter {
    #[must_use]
    pub const fn all() -> Self {
        Self {
            availability: None,
            text: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvailableCapacity {
    pub payload_kg: Option<u32>,
    pub pallets: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeasibilityLevel {
    Impossible,
    NeedsCheck,
    Feasible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardRejectReason {
    RequestUnavailable(AvailabilityState),
    PayloadExceeded {
        cargo_kg: u32,
        available_kg: u32,
    },
    PalletCapacityExceeded {
        cargo_pallets: u16,
        available_pallets: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedsCheckReason {
    RequestAvailabilityNeedsCheck,
    RequestAvailabilityUnknown,
    CargoWeightUnknown,
    AvailablePayloadUnknown,
    CargoPalletsUnknown,
    AvailablePalletsUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateFeasibility {
    pub level: FeasibilityLevel,
    pub hard_reasons: Vec<HardRejectReason>,
    pub check_reasons: Vec<NeedsCheckReason>,
}

#[must_use]
pub fn evaluate_request_candidate(
    request: &PlanningRequest,
    capacity: AvailableCapacity,
) -> CandidateFeasibility {
    let mut hard_reasons = Vec::new();
    let mut check_reasons = Vec::new();

    match request.request.availability {
        AvailabilityState::Active => {}
        AvailabilityState::NeedsCheck => {
            check_reasons.push(NeedsCheckReason::RequestAvailabilityNeedsCheck);
        }
        AvailabilityState::Unknown => {
            check_reasons.push(NeedsCheckReason::RequestAvailabilityUnknown);
        }
        unavailable @ (AvailabilityState::Taken
        | AvailabilityState::Rejected
        | AvailabilityState::Removed) => {
            hard_reasons.push(HardRejectReason::RequestUnavailable(unavailable));
        }
    }

    match (request.request.cargo.weight_kg, capacity.payload_kg) {
        (Some(cargo_kg), Some(available_kg)) if cargo_kg > available_kg => {
            hard_reasons.push(HardRejectReason::PayloadExceeded {
                cargo_kg,
                available_kg,
            });
        }
        (Some(_), Some(_)) => {}
        (None, Some(_)) => check_reasons.push(NeedsCheckReason::CargoWeightUnknown),
        (Some(_), None) => check_reasons.push(NeedsCheckReason::AvailablePayloadUnknown),
        (None, None) => {
            check_reasons.push(NeedsCheckReason::CargoWeightUnknown);
            check_reasons.push(NeedsCheckReason::AvailablePayloadUnknown);
        }
    }

    match (request.request.cargo.pallets, capacity.pallets) {
        (Some(cargo_pallets), Some(available_pallets))
            if cargo_pallets > available_pallets =>
        {
            hard_reasons.push(HardRejectReason::PalletCapacityExceeded {
                cargo_pallets,
                available_pallets,
            });
        }
        (Some(_), Some(_)) => {}
        (None, Some(_)) => check_reasons.push(NeedsCheckReason::CargoPalletsUnknown),
        (Some(_), None) => check_reasons.push(NeedsCheckReason::AvailablePalletsUnknown),
        (None, None) => {
            check_reasons.push(NeedsCheckReason::CargoPalletsUnknown);
            check_reasons.push(NeedsCheckReason::AvailablePalletsUnknown);
        }
    }

    let level = if hard_reasons.is_empty() {
        if check_reasons.is_empty() {
            FeasibilityLevel::Feasible
        } else {
            FeasibilityLevel::NeedsCheck
        }
    } else {
        FeasibilityLevel::Impossible
    };

    CandidateFeasibility {
        level,
        hard_reasons,
        check_reasons,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryError {
    DuplicateRequest,
    RequestNotFound,
    StaleRevision,
}

pub trait RequestRepository {
    fn insert(&mut self, request: PlanningRequest) -> Result<(), RepositoryError>;
    fn refresh(&mut self, request: PlanningRequest) -> Result<(), RepositoryError>;
    fn set_planning_assignment(
        &mut self,
        request_id: RequestId,
        assignment: PlanningAssignment,
    ) -> Result<(), RepositoryError>;
    fn get(&self, request_id: RequestId) -> Option<PlanningRequest>;
    fn list(&self) -> Vec<PlanningRequest>;
}

#[derive(Debug, Default, Clone)]
pub struct InMemoryRequestRepository {
    requests: BTreeMap<RequestId, PlanningRequest>,
}

impl RequestRepository for InMemoryRequestRepository {
    fn insert(&mut self, request: PlanningRequest) -> Result<(), RepositoryError> {
        if self.requests.contains_key(&request.request.request_id) {
            return Err(RepositoryError::DuplicateRequest);
        }
        self.requests.insert(request.request.request_id, request);
        Ok(())
    }

    fn refresh(&mut self, mut request: PlanningRequest) -> Result<(), RepositoryError> {
        let existing = self
            .requests
            .get_mut(&request.request.request_id)
            .ok_or(RepositoryError::RequestNotFound)?;

        if request.request.revision <= existing.request.revision {
            return Err(RepositoryError::StaleRevision);
        }

        request.request.planning_assignment = existing.request.planning_assignment;
        *existing = request;
        Ok(())
    }

    fn set_planning_assignment(
        &mut self,
        request_id: RequestId,
        assignment: PlanningAssignment,
    ) -> Result<(), RepositoryError> {
        let existing = self
            .requests
            .get_mut(&request_id)
            .ok_or(RepositoryError::RequestNotFound)?;
        existing.request.planning_assignment = assignment;
        Ok(())
    }

    fn get(&self, request_id: RequestId) -> Option<PlanningRequest> {
        self.requests.get(&request_id).cloned()
    }

    fn list(&self) -> Vec<PlanningRequest> {
        self.requests.values().cloned().collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoRouterError {
    Unavailable,
    Rejected,
}

pub trait GeoRouter {
    fn route(&self, request: &RouteRequest) -> Result<RouteResult, GeoRouterError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannerError {
    DuplicateRequest,
    RequestNotFound,
    StaleRequestRevision,
    MissingConfirmedCoordinates,
    InvalidGeoProfile,
    GeoRouterUnavailable,
    GeoRouterRejected,
}

pub struct PlannerService<R> {
    repository: R,
}

impl<R: RequestRepository> PlannerService<R> {
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn add_request(&mut self, request: PlanningRequest) -> Result<(), PlannerError> {
        self.repository
            .insert(request)
            .map_err(|error| match error {
                RepositoryError::DuplicateRequest => PlannerError::DuplicateRequest,
                RepositoryError::RequestNotFound => PlannerError::RequestNotFound,
                RepositoryError::StaleRevision => PlannerError::StaleRequestRevision,
            })
    }

    pub fn refresh_request(&mut self, request: PlanningRequest) -> Result<(), PlannerError> {
        self.repository
            .refresh(request)
            .map_err(|error| match error {
                RepositoryError::DuplicateRequest => PlannerError::DuplicateRequest,
                RepositoryError::RequestNotFound => PlannerError::RequestNotFound,
                RepositoryError::StaleRevision => PlannerError::StaleRequestRevision,
            })
    }

    pub fn set_planning_assignment(
        &mut self,
        request_id: RequestId,
        assignment: PlanningAssignment,
    ) -> Result<(), PlannerError> {
        self.repository
            .set_planning_assignment(request_id, assignment)
            .map_err(|error| match error {
                RepositoryError::DuplicateRequest => PlannerError::DuplicateRequest,
                RepositoryError::RequestNotFound => PlannerError::RequestNotFound,
                RepositoryError::StaleRevision => PlannerError::StaleRequestRevision,
            })
    }

    #[must_use]
    pub fn pool(&self, filter: &PoolFilter) -> Vec<PlanningRequest> {
        let query = filter
            .text
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_lowercase);

        self.repository
            .list()
            .into_iter()
            .filter(|item| {
                filter
                    .availability
                    .is_none_or(|wanted| item.request.availability == wanted)
            })
            .filter(|item| {
                query.as_ref().is_none_or(|query| {
                    let request = &item.request;
                    [
                        request.origin.locality.as_str(),
                        request.destination.locality.as_str(),
                        request.cargo.title.as_str(),
                        request.source.source_name.as_deref().unwrap_or_default(),
                    ]
                    .iter()
                    .any(|value| value.to_lowercase().contains(query))
                })
            })
            .collect()
    }

    pub fn build_route_request(
        &self,
        request_id: RequestId,
        profile: VehicleRoutingProfile,
    ) -> Result<RouteRequest, PlannerError> {
        let item = self
            .repository
            .get(request_id)
            .ok_or(PlannerError::RequestNotFound)?;
        let from = item
            .pickup_coordinates
            .ok_or(PlannerError::MissingConfirmedCoordinates)?;
        let to = item
            .delivery_coordinates
            .ok_or(PlannerError::MissingConfirmedCoordinates)?;

        RouteRequest::new(vec![from, to], profile).map_err(|_| PlannerError::InvalidGeoProfile)
    }

    pub fn preview_route<G: GeoRouter>(
        &self,
        request_id: RequestId,
        profile: VehicleRoutingProfile,
        router: &G,
    ) -> Result<RouteResult, PlannerError> {
        let request = self.build_route_request(request_id, profile)?;
        router.route(&request).map_err(|error| match error {
            GeoRouterError::Unavailable => PlannerError::GeoRouterUnavailable,
            GeoRouterError::Rejected => PlannerError::GeoRouterRejected,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_domain::{
        CargoSummary, Place, PlanningAssignment, RequestRevision, SourceKind, SourceMetadata,
    };
    use cargo_dispatch_geo::{NormalizedStatus, RoutingIdentity, TollPreference};

    fn request_id(byte: u8) -> RequestId {
        RequestId::from_bytes([byte; 16]).expect("fixture id")
    }

    fn request(
        id: RequestId,
        origin: &str,
        destination: &str,
        availability: AvailabilityState,
        coordinates: bool,
    ) -> PlanningRequest {
        PlanningRequest {
            request: LoadRequest {
                request_id: id,
                revision: RequestRevision::new(1).expect("revision"),
                source: SourceMetadata {
                    kind: SourceKind::Exchange,
                    source_name: Some("synthetic exchange".to_owned()),
                    source_order_id: Some(format!("SYN-{}", id.as_bytes()[0])),
                    observed_at_unix_ms: 1_800_000_000_000,
                },
                availability,
                planning_assignment: PlanningAssignment::Free,
                origin: Place::new("UA", origin, None).expect("origin"),
                destination: Place::new("DE", destination, None).expect("destination"),
                cargo: CargoSummary::new("palletized cargo", Some(2_500), Some(6)).expect("cargo"),
            },
            pickup_coordinates: coordinates
                .then(|| GeoPoint::new(49.8397, 24.0297).expect("pickup")),
            delivery_coordinates: coordinates
                .then(|| GeoPoint::new(49.4521, 11.0767).expect("delivery")),
        }
    }

    fn profile() -> VehicleRoutingProfile {
        VehicleRoutingProfile {
            height_m: Some(3.2),
            width_m: Some(2.45),
            length_m: Some(7.5),
            gross_weight_t: Some(7.5),
            axle_load_t: Some(4.0),
            axle_count: Some(2),
            hazmat: false,
            hgv: true,
            exclude_unpaved: true,
            prefer_truck_routes: true,
            toll_preference: TollPreference::Allow,
            max_speed_kmh: None,
        }
    }

    struct FakeRouter;

    impl GeoRouter for FakeRouter {
        fn route(&self, request: &RouteRequest) -> Result<RouteResult, GeoRouterError> {
            Ok(RouteResult {
                status: NormalizedStatus::Ok,
                identity: RoutingIdentity::new(
                    "fake-router",
                    "1",
                    "fake-dataset-v1",
                    format!("{:?}", request.profile()),
                )
                .expect("identity"),
                distance_m: Some(1_020_000.0),
                duration_s: Some(43_200.0),
                geometry: request.points().to_vec(),
                snapped_points: request.points().to_vec(),
                warnings: request.profile().warnings(),
            })
        }
    }

    #[test]
    fn add_list_and_filter_request_pool_without_auto_expiry() {
        let mut service = PlannerService::new(InMemoryRequestRepository::default());
        service
            .add_request(request(
                request_id(1),
                "Lviv",
                "Nuremberg",
                AvailabilityState::Active,
                true,
            ))
            .expect("first request");
        service
            .add_request(request(
                request_id(2),
                "Kyiv",
                "Berlin",
                AvailabilityState::NeedsCheck,
                true,
            ))
            .expect("second request");

        assert_eq!(service.pool(&PoolFilter::all()).len(), 2);

        let active = service.pool(&PoolFilter {
            availability: Some(AvailabilityState::Active),
            text: None,
        });
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].request.origin.locality, "Lviv");

        let berlin = service.pool(&PoolFilter {
            availability: None,
            text: Some("berlin".to_owned()),
        });
        assert_eq!(berlin.len(), 1);
        assert_eq!(berlin[0].request.request_id, request_id(2));
    }

    #[test]
    fn duplicate_request_id_is_rejected() {
        let item = request(
            request_id(3),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        let mut service = PlannerService::new(InMemoryRequestRepository::default());
        service.add_request(item.clone()).expect("first insert");
        assert_eq!(
            service.add_request(item),
            Err(PlannerError::DuplicateRequest)
        );
    }

    #[test]
    fn basic_candidate_is_feasible_when_known_capacity_is_sufficient() {
        let item = request(
            request_id(8),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        let result = evaluate_request_candidate(
            &item,
            AvailableCapacity {
                payload_kg: Some(3_000),
                pallets: Some(8),
            },
        );

        assert_eq!(result.level, FeasibilityLevel::Feasible);
        assert!(result.hard_reasons.is_empty());
        assert!(result.check_reasons.is_empty());
    }

    #[test]
    fn known_capacity_violation_is_impossible_and_explained() {
        let item = request(
            request_id(9),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        let result = evaluate_request_candidate(
            &item,
            AvailableCapacity {
                payload_kg: Some(2_000),
                pallets: Some(5),
            },
        );

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert_eq!(
            result.hard_reasons,
            vec![
                HardRejectReason::PayloadExceeded {
                    cargo_kg: 2_500,
                    available_kg: 2_000,
                },
                HardRejectReason::PalletCapacityExceeded {
                    cargo_pallets: 6,
                    available_pallets: 5,
                },
            ]
        );
    }

    #[test]
    fn unavailable_request_is_rejected_before_expensive_routing() {
        let item = request(
            request_id(10),
            "Kyiv",
            "Berlin",
            AvailabilityState::Taken,
            true,
        );
        let result = evaluate_request_candidate(
            &item,
            AvailableCapacity {
                payload_kg: Some(5_000),
                pallets: Some(10),
            },
        );

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert_eq!(
            result.hard_reasons,
            vec![HardRejectReason::RequestUnavailable(
                AvailabilityState::Taken
            )]
        );
    }

    #[test]
    fn unknown_capacity_data_needs_check_instead_of_becoming_confirmed_fact() {
        let mut item = request(
            request_id(11),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Unknown,
            true,
        );
        item.request.cargo.pallets = None;

        let result = evaluate_request_candidate(
            &item,
            AvailableCapacity {
                payload_kg: Some(3_000),
                pallets: None,
            },
        );

        assert_eq!(result.level, FeasibilityLevel::NeedsCheck);
        assert!(result.hard_reasons.is_empty());
        assert!(result
            .check_reasons
            .contains(&NeedsCheckReason::RequestAvailabilityUnknown));
        assert!(result
            .check_reasons
            .contains(&NeedsCheckReason::CargoPalletsUnknown));
        assert!(result
            .check_reasons
            .contains(&NeedsCheckReason::AvailablePalletsUnknown));
    }

    #[test]
    fn refresh_request_requires_newer_revision_and_preserves_planning_assignment() {
        let id = request_id(6);
        let mut service = PlannerService::new(InMemoryRequestRepository::default());
        service
            .add_request(request(
                id,
                "Lviv",
                "Nuremberg",
                AvailabilityState::Active,
                true,
            ))
            .expect("request");
        service
            .set_planning_assignment(id, PlanningAssignment::Calling)
            .expect("assignment");

        let mut refreshed = request(id, "Lviv", "Nuremberg", AvailabilityState::NeedsCheck, true);
        refreshed.request.revision = RequestRevision::new(2).expect("revision");
        refreshed.request.source.observed_at_unix_ms = 1_800_000_060_000;
        refreshed.request.cargo =
            CargoSummary::new("updated cargo", Some(2_800), Some(7)).expect("cargo");

        service.refresh_request(refreshed).expect("refresh");

        let pool = service.pool(&PoolFilter::all());
        assert_eq!(pool.len(), 1);
        assert_eq!(pool[0].request.revision.get(), 2);
        assert_eq!(pool[0].request.availability, AvailabilityState::NeedsCheck);
        assert_eq!(
            pool[0].request.planning_assignment,
            PlanningAssignment::Calling
        );
        assert_eq!(pool[0].request.cargo.title, "updated cargo");
        assert_eq!(
            pool[0].request.source.observed_at_unix_ms,
            1_800_000_060_000
        );
    }

    #[test]
    fn stale_refresh_is_rejected_without_changing_saved_request() {
        let id = request_id(7);
        let mut service = PlannerService::new(InMemoryRequestRepository::default());
        service
            .add_request(request(
                id,
                "Kyiv",
                "Berlin",
                AvailabilityState::Active,
                true,
            ))
            .expect("request");

        let stale = request(id, "Kyiv", "Hamburg", AvailabilityState::Taken, true);
        assert_eq!(
            service.refresh_request(stale),
            Err(PlannerError::StaleRequestRevision)
        );

        let pool = service.pool(&PoolFilter::all());
        assert_eq!(pool.len(), 1);
        assert_eq!(pool[0].request.revision.get(), 1);
        assert_eq!(pool[0].request.availability, AvailabilityState::Active);
        assert_eq!(pool[0].request.destination.locality, "Berlin");
    }

    #[test]
    fn route_preview_uses_confirmed_coordinates_and_geo_port() {
        let id = request_id(4);
        let mut service = PlannerService::new(InMemoryRequestRepository::default());
        service
            .add_request(request(
                id,
                "Lviv",
                "Nuremberg",
                AvailabilityState::Active,
                true,
            ))
            .expect("request");

        let result = service
            .preview_route(id, profile(), &FakeRouter)
            .expect("fake route");
        assert_eq!(result.status, NormalizedStatus::Ok);
        assert_eq!(result.identity.map_dataset_id(), "fake-dataset-v1");
        assert_eq!(result.distance_m, Some(1_020_000.0));
    }

    #[test]
    fn route_preview_refuses_request_without_confirmed_coordinates() {
        let id = request_id(5);
        let mut service = PlannerService::new(InMemoryRequestRepository::default());
        service
            .add_request(request(
                id,
                "Lviv",
                "Nuremberg",
                AvailabilityState::Active,
                false,
            ))
            .expect("request");

        assert_eq!(
            service.preview_route(id, profile(), &FakeRouter),
            Err(PlannerError::MissingConfirmedCoordinates)
        );
    }
}
