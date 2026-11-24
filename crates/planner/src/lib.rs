#![forbid(unsafe_code)]

use cargo_dispatch_domain::{AvailabilityState, LoadRequest, RequestId};
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
pub enum RepositoryError {
    DuplicateRequest,
}

pub trait RequestRepository {
    fn insert(&mut self, request: PlanningRequest) -> Result<(), RepositoryError>;
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
            .map_err(|_| PlannerError::DuplicateRequest)
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
        CargoSummary, PlanningAssignment, Place, RequestRevision, SourceKind, SourceMetadata,
    };
    use cargo_dispatch_geo::{
        NormalizedStatus, RoutingIdentity, TollPreference,
    };

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
                cargo: CargoSummary::new("palletized cargo", Some(2_500), Some(6))
                    .expect("cargo"),
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
