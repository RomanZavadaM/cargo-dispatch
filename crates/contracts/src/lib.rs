#![forbid(unsafe_code)]

use cargo_dispatch_domain::{PointId, ReplanRequestId, TripId};
use core::fmt;

pub const TRIP_PACKAGE_SCHEMA_NAME: &str = "cargo-dispatch.trip-package";
pub const CURRENT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
}

impl SchemaVersion {
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TripVersion(u64);

impl TripVersion {
    pub fn new(value: u64) -> Result<Self, ContractError> {
        if value == 0 {
            return Err(ContractError::TripVersionMustStartAtOne);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageContentFingerprint([u8; 32]);

impl PackageContentFingerprint {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripPackageHeader {
    pub schema_name: String,
    pub schema_version: SchemaVersion,
    pub trip_id: TripId,
    pub trip_version: TripVersion,
    pub base_version: Option<TripVersion>,
}

impl TripPackageHeader {
    pub fn new(
        trip_id: TripId,
        trip_version: TripVersion,
        base_version: Option<TripVersion>,
    ) -> Result<Self, ContractError> {
        match (trip_version.get(), base_version) {
            (1, None) => {}
            (1, Some(_)) => return Err(ContractError::FirstVersionMustNotHaveBase),
            (_, None) => return Err(ContractError::LaterVersionRequiresBase),
            (current, Some(base)) if base.get() >= current => {
                return Err(ContractError::BaseVersionMustBeOlder);
            }
            _ => {}
        }

        Ok(Self {
            schema_name: TRIP_PACKAGE_SCHEMA_NAME.to_owned(),
            schema_version: CURRENT_SCHEMA_VERSION,
            trip_id,
            trip_version,
            base_version,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TripPointKind {
    Load,
    Unload,
    CombinedLoadUnload,
    Rest,
    Break,
    Service,
    BorderCustoms,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripNavigationTarget {
    pub latitude_e7: i32,
    pub longitude_e7: i32,
    pub address: Option<String>,
}

impl TripNavigationTarget {
    #[must_use]
    pub fn latitude(&self) -> f64 {
        f64::from(self.latitude_e7) / 10_000_000.0
    }

    #[must_use]
    pub fn longitude(&self) -> f64 {
        f64::from(self.longitude_e7) / 10_000_000.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripRoutePoint {
    pub point_id: PointId,
    pub sequence: u32,
    pub kind: TripPointKind,
    pub title: String,
    pub navigation: Option<TripNavigationTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TripRoadGeometryPoint {
    pub latitude_e7: i32,
    pub longitude_e7: i32,
}

impl TripRoadGeometryPoint {
    #[must_use]
    pub fn latitude(self) -> f64 {
        f64::from(self.latitude_e7) / 10_000_000.0
    }

    #[must_use]
    pub fn longitude(self) -> f64 {
        f64::from(self.longitude_e7) / 10_000_000.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripRoutingIdentity {
    pub engine: String,
    pub engine_version: String,
    pub map_dataset_id: String,
    pub profile_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripRoadGeometry {
    pub routing: TripRoutingIdentity,
    pub points: Vec<TripRoadGeometryPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripGeometryContractError {
    TooFewPoints,
    EmptyIdentity(&'static str),
    InvalidLatitude(usize),
    InvalidLongitude(usize),
}

pub fn validate_trip_road_geometry(
    geometry: &TripRoadGeometry,
) -> Result<(), TripGeometryContractError> {
    if geometry.points.len() < 2 {
        return Err(TripGeometryContractError::TooFewPoints);
    }
    for (field, value) in [
        ("engine", geometry.routing.engine.as_str()),
        ("engine_version", geometry.routing.engine_version.as_str()),
        ("map_dataset_id", geometry.routing.map_dataset_id.as_str()),
        (
            "profile_fingerprint",
            geometry.routing.profile_fingerprint.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(TripGeometryContractError::EmptyIdentity(field));
        }
    }
    for (index, point) in geometry.points.iter().enumerate() {
        if !(-900_000_000..=900_000_000).contains(&point.latitude_e7) {
            return Err(TripGeometryContractError::InvalidLatitude(index));
        }
        if !(-1_800_000_000..=1_800_000_000).contains(&point.longitude_e7) {
            return Err(TripGeometryContractError::InvalidLongitude(index));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteContractError {
    DuplicatePointId(PointId),
    SequenceMismatch { expected: u32, actual: u32 },
    TitleRequired(PointId),
    CustomKindRequired(PointId),
    InvalidNavigationLatitude(PointId),
    InvalidNavigationLongitude(PointId),
    NavigationAddressRequired(PointId),
    SequenceOverflow,
}

pub fn validate_route_points(points: &[TripRoutePoint]) -> Result<(), RouteContractError> {
    let mut seen = std::collections::BTreeSet::new();

    for (index, point) in points.iter().enumerate() {
        if !seen.insert(point.point_id) {
            return Err(RouteContractError::DuplicatePointId(point.point_id));
        }

        let expected = u32::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(RouteContractError::SequenceOverflow)?;
        if point.sequence != expected {
            return Err(RouteContractError::SequenceMismatch {
                expected,
                actual: point.sequence,
            });
        }

        if point.title.trim().is_empty() {
            return Err(RouteContractError::TitleRequired(point.point_id));
        }

        if matches!(&point.kind, TripPointKind::Custom(value) if value.trim().is_empty()) {
            return Err(RouteContractError::CustomKindRequired(point.point_id));
        }

        if let Some(navigation) = &point.navigation {
            if !(-900_000_000..=900_000_000).contains(&navigation.latitude_e7) {
                return Err(RouteContractError::InvalidNavigationLatitude(
                    point.point_id,
                ));
            }
            if !(-1_800_000_000..=1_800_000_000).contains(&navigation.longitude_e7) {
                return Err(RouteContractError::InvalidNavigationLongitude(
                    point.point_id,
                ));
            }
            if navigation
                .address
                .as_ref()
                .is_some_and(|address| address.trim().is_empty())
            {
                return Err(RouteContractError::NavigationAddressRequired(
                    point.point_id,
                ));
            }
        }
    }

    Ok(())
}

pub const REPLAN_REQUEST_SCHEMA_NAME: &str = "cargo-dispatch.replan-request";
pub const CURRENT_REPLAN_REQUEST_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplanReasonCode {
    ShipperRefused,
    CargoChanged,
    Delay,
    Breakdown,
    RoadProblem,
    NewPickup,
    PointUnavailable,
    CustomerChange,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplanRequest {
    pub schema_name: String,
    pub schema_version: SchemaVersion,
    pub request_id: ReplanRequestId,
    pub trip_id: TripId,
    pub based_on_trip_version: TripVersion,
    pub created_at_unix_ms: u64,
    pub current_time_unix_ms: u64,
    pub reason_code: ReplanReasonCode,
    pub operator_note: Option<String>,
    pub completed_point_ids: Vec<PointId>,
    pub current_point_id: Option<PointId>,
    pub affected_point_ids: Vec<PointId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplanRequestData {
    pub request_id: ReplanRequestId,
    pub trip_id: TripId,
    pub based_on_trip_version: TripVersion,
    pub created_at_unix_ms: u64,
    pub current_time_unix_ms: u64,
    pub reason_code: ReplanReasonCode,
    pub operator_note: Option<String>,
    pub completed_point_ids: Vec<PointId>,
    pub current_point_id: Option<PointId>,
    pub affected_point_ids: Vec<PointId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplanContractError {
    DuplicateCompletedPoint(PointId),
    DuplicateAffectedPoint(PointId),
    EmptyOperatorNote,
}

impl ReplanRequest {
    pub fn new(data: ReplanRequestData) -> Result<Self, ReplanContractError> {
        let mut completed = std::collections::BTreeSet::new();
        for point_id in &data.completed_point_ids {
            if !completed.insert(*point_id) {
                return Err(ReplanContractError::DuplicateCompletedPoint(*point_id));
            }
        }

        let mut affected = std::collections::BTreeSet::new();
        for point_id in &data.affected_point_ids {
            if !affected.insert(*point_id) {
                return Err(ReplanContractError::DuplicateAffectedPoint(*point_id));
            }
        }

        if data
            .operator_note
            .as_ref()
            .is_some_and(|note| note.trim().is_empty())
        {
            return Err(ReplanContractError::EmptyOperatorNote);
        }

        Ok(Self {
            schema_name: REPLAN_REQUEST_SCHEMA_NAME.to_owned(),
            schema_version: CURRENT_REPLAN_REQUEST_SCHEMA_VERSION,
            request_id: data.request_id,
            trip_id: data.trip_id,
            based_on_trip_version: data.based_on_trip_version,
            created_at_unix_ms: data.created_at_unix_ms,
            current_time_unix_ms: data.current_time_unix_ms,
            reason_code: data.reason_code,
            operator_note: data.operator_note,
            completed_point_ids: data.completed_point_ids,
            current_point_id: data.current_point_id,
            affected_point_ids: data.affected_point_ids,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractError {
    TripVersionMustStartAtOne,
    FirstVersionMustNotHaveBase,
    LaterVersionRequiresBase,
    BaseVersionMustBeOlder,
}

impl fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TripVersionMustStartAtOne => "trip version must start at 1",
            Self::FirstVersionMustNotHaveBase => "trip version 1 must not have a base version",
            Self::LaterVersionRequiresBase => "trip version after 1 requires a base version",
            Self::BaseVersionMustBeOlder => "base version must be older than trip version",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn trip_id() -> TripId {
        TripId::from_bytes([
            0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0,
            0xf0, 0x01,
        ])
        .expect("fixture UUID is non-nil")
    }

    fn point_id(byte: u8) -> PointId {
        PointId::from_bytes([byte; 16]).expect("fixture point id")
    }

    fn replan_request_id(byte: u8) -> ReplanRequestId {
        ReplanRequestId::from_bytes([byte; 16]).expect("fixture replan request id")
    }

    #[test]
    fn replan_request_keeps_plan_version_and_execution_context() {
        let request = ReplanRequest::new(ReplanRequestData {
            request_id: replan_request_id(1),
            trip_id: trip_id(),
            based_on_trip_version: TripVersion::new(3).expect("trip version"),
            created_at_unix_ms: 1_800_000_000_000,
            current_time_unix_ms: 1_800_000_030_000,
            reason_code: ReplanReasonCode::Delay,
            operator_note: Some("Traffic delay".to_owned()),
            completed_point_ids: vec![point_id(1), point_id(2)],
            current_point_id: Some(point_id(3)),
            affected_point_ids: vec![point_id(3), point_id(4)],
        })
        .expect("replan request");

        assert_eq!(request.schema_name, REPLAN_REQUEST_SCHEMA_NAME);
        assert_eq!(
            request.schema_version,
            CURRENT_REPLAN_REQUEST_SCHEMA_VERSION
        );
        assert_eq!(request.based_on_trip_version.get(), 3);
        assert_eq!(request.completed_point_ids, vec![point_id(1), point_id(2)]);
        assert_eq!(request.current_point_id, Some(point_id(3)));
    }

    #[test]
    fn replan_request_rejects_duplicate_point_lists_and_blank_note() {
        assert_eq!(
            ReplanRequest::new(ReplanRequestData {
                request_id: replan_request_id(2),
                trip_id: trip_id(),
                based_on_trip_version: TripVersion::new(1).expect("trip version"),
                created_at_unix_ms: 1,
                current_time_unix_ms: 1,
                reason_code: ReplanReasonCode::RoadProblem,
                operator_note: None,
                completed_point_ids: vec![point_id(5), point_id(5)],
                current_point_id: None,
                affected_point_ids: Vec::new(),
            }),
            Err(ReplanContractError::DuplicateCompletedPoint(point_id(5)))
        );

        assert_eq!(
            ReplanRequest::new(ReplanRequestData {
                request_id: replan_request_id(3),
                trip_id: trip_id(),
                based_on_trip_version: TripVersion::new(1).expect("trip version"),
                created_at_unix_ms: 1,
                current_time_unix_ms: 1,
                reason_code: ReplanReasonCode::NewPickup,
                operator_note: None,
                completed_point_ids: Vec::new(),
                current_point_id: None,
                affected_point_ids: vec![point_id(6), point_id(6)],
            }),
            Err(ReplanContractError::DuplicateAffectedPoint(point_id(6)))
        );

        assert_eq!(
            ReplanRequest::new(ReplanRequestData {
                request_id: replan_request_id(4),
                trip_id: trip_id(),
                based_on_trip_version: TripVersion::new(1).expect("trip version"),
                created_at_unix_ms: 1,
                current_time_unix_ms: 1,
                reason_code: ReplanReasonCode::Other,
                operator_note: Some("   ".to_owned()),
                completed_point_ids: Vec::new(),
                current_point_id: None,
                affected_point_ids: Vec::new(),
            }),
            Err(ReplanContractError::EmptyOperatorNote)
        );
    }

    #[test]
    fn ordered_route_points_accept_known_and_custom_types() {
        let points = vec![
            TripRoutePoint {
                point_id: point_id(1),
                sequence: 1,
                kind: TripPointKind::Load,
                title: "Warehouse A".to_owned(),
                navigation: Some(TripNavigationTarget {
                    latitude_e7: 498_393_000,
                    longitude_e7: 240_296_000,
                    address: Some("Synthetic street 1".to_owned()),
                }),
            },
            TripRoutePoint {
                point_id: point_id(2),
                sequence: 2,
                kind: TripPointKind::Custom("ferry".to_owned()),
                title: "Ferry terminal".to_owned(),
                navigation: None,
            },
            TripRoutePoint {
                point_id: point_id(3),
                sequence: 3,
                kind: TripPointKind::Unload,
                title: "Receiver".to_owned(),
                navigation: None,
            },
        ];

        assert_eq!(validate_route_points(&points), Ok(()));
    }

    #[test]
    fn route_points_reject_duplicate_id_or_wrong_sequence() {
        let duplicate = vec![
            TripRoutePoint {
                point_id: point_id(4),
                sequence: 1,
                kind: TripPointKind::Load,
                title: "Load".to_owned(),
                navigation: None,
            },
            TripRoutePoint {
                point_id: point_id(4),
                sequence: 2,
                kind: TripPointKind::Unload,
                title: "Unload".to_owned(),
                navigation: None,
            },
        ];
        assert_eq!(
            validate_route_points(&duplicate),
            Err(RouteContractError::DuplicatePointId(point_id(4)))
        );

        let wrong_sequence = vec![TripRoutePoint {
            point_id: point_id(5),
            sequence: 2,
            kind: TripPointKind::Service,
            title: "Service".to_owned(),
            navigation: None,
        }];
        assert_eq!(
            validate_route_points(&wrong_sequence),
            Err(RouteContractError::SequenceMismatch {
                expected: 1,
                actual: 2,
            })
        );
    }

    #[test]
    fn route_point_requires_human_title_and_nonempty_custom_kind() {
        let no_title = vec![TripRoutePoint {
            point_id: point_id(6),
            sequence: 1,
            kind: TripPointKind::Rest,
            title: "   ".to_owned(),
            navigation: None,
        }];
        assert_eq!(
            validate_route_points(&no_title),
            Err(RouteContractError::TitleRequired(point_id(6)))
        );

        let empty_custom = vec![TripRoutePoint {
            point_id: point_id(7),
            sequence: 1,
            kind: TripPointKind::Custom(" ".to_owned()),
            title: "Special stop".to_owned(),
            navigation: None,
        }];
        assert_eq!(
            validate_route_points(&empty_custom),
            Err(RouteContractError::CustomKindRequired(point_id(7)))
        );
    }

    #[test]
    fn route_navigation_target_validates_coordinates_and_address() {
        let invalid_latitude = vec![TripRoutePoint {
            point_id: point_id(8),
            sequence: 1,
            kind: TripPointKind::Unload,
            title: "Receiver".to_owned(),
            navigation: Some(TripNavigationTarget {
                latitude_e7: 901_000_000,
                longitude_e7: 240_296_000,
                address: Some("Synthetic receiver".to_owned()),
            }),
        }];
        assert_eq!(
            validate_route_points(&invalid_latitude),
            Err(RouteContractError::InvalidNavigationLatitude(point_id(8)))
        );

        let blank_address = vec![TripRoutePoint {
            point_id: point_id(9),
            sequence: 1,
            kind: TripPointKind::Unload,
            title: "Receiver".to_owned(),
            navigation: Some(TripNavigationTarget {
                latitude_e7: 498_393_000,
                longitude_e7: 240_296_000,
                address: Some("   ".to_owned()),
            }),
        }];
        assert_eq!(
            validate_route_points(&blank_address),
            Err(RouteContractError::NavigationAddressRequired(point_id(9)))
        );
    }

    #[test]
    fn package_content_fingerprint_is_opaque_and_stable() {
        let fingerprint = PackageContentFingerprint::from_bytes([0x5a; 32]);
        assert_eq!(fingerprint.into_bytes(), [0x5a; 32]);
    }

    #[test]
    fn road_geometry_requires_identity_and_bounded_points() {
        let mut geometry = TripRoadGeometry {
            routing: TripRoutingIdentity {
                engine: "valhalla".to_owned(),
                engine_version: "3.9.2".to_owned(),
                map_dataset_id: "dataset-a".to_owned(),
                profile_fingerprint: "abc".to_owned(),
            },
            points: vec![
                TripRoadGeometryPoint {
                    latitude_e7: 498_393_000,
                    longitude_e7: 240_296_000,
                },
                TripRoadGeometryPoint {
                    latitude_e7: 498_430_000,
                    longitude_e7: 240_360_000,
                },
            ],
        };
        assert_eq!(validate_trip_road_geometry(&geometry), Ok(()));

        geometry.points[1].latitude_e7 = 900_000_001;
        assert_eq!(
            validate_trip_road_geometry(&geometry),
            Err(TripGeometryContractError::InvalidLatitude(1))
        );

        geometry.points[1].latitude_e7 = 498_430_000;
        geometry.routing.map_dataset_id.clear();
        assert_eq!(
            validate_trip_road_geometry(&geometry),
            Err(TripGeometryContractError::EmptyIdentity("map_dataset_id"))
        );
    }

    #[test]
    fn schema_version_and_trip_version_are_separate() {
        let header = TripPackageHeader::new(
            trip_id(),
            TripVersion::new(7).expect("valid trip version"),
            Some(TripVersion::new(6).expect("valid base version")),
        )
        .expect("valid trip header");

        assert_eq!(header.schema_name, TRIP_PACKAGE_SCHEMA_NAME);
        assert_eq!(header.schema_version, SchemaVersion::new(1, 3));
        assert_eq!(header.trip_version.get(), 7);
        assert_eq!(header.base_version.map(TripVersion::get), Some(6));
    }

    #[test]
    fn first_trip_version_has_no_base_version() {
        let header = TripPackageHeader::new(
            trip_id(),
            TripVersion::new(1).expect("valid trip version"),
            None,
        )
        .expect("first version is valid");

        assert_eq!(header.base_version, None);
    }

    #[test]
    fn later_trip_version_requires_an_older_base() {
        let later = TripVersion::new(2).expect("valid later version");

        assert_eq!(
            TripPackageHeader::new(trip_id(), later, None),
            Err(ContractError::LaterVersionRequiresBase)
        );
        assert_eq!(
            TripPackageHeader::new(trip_id(), later, Some(later)),
            Err(ContractError::BaseVersionMustBeOlder)
        );
    }

    #[test]
    fn version_zero_and_base_on_first_version_are_rejected() {
        assert_eq!(
            TripVersion::new(0),
            Err(ContractError::TripVersionMustStartAtOne)
        );

        let first = TripVersion::new(1).expect("valid first version");
        assert_eq!(
            TripPackageHeader::new(trip_id(), first, Some(first)),
            Err(ContractError::FirstVersionMustNotHaveBase)
        );
    }
}
