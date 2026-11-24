#![forbid(unsafe_code)]

use cargo_dispatch_domain::{PointId, TripId};
use core::fmt;

pub const TRIP_PACKAGE_SCHEMA_NAME: &str = "cargo-dispatch.trip-package";
pub const CURRENT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0);

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
pub struct TripRoutePoint {
    pub point_id: PointId,
    pub sequence: u32,
    pub kind: TripPointKind,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteContractError {
    DuplicatePointId(PointId),
    SequenceMismatch { expected: u32, actual: u32 },
    TitleRequired(PointId),
    CustomKindRequired(PointId),
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
    }

    Ok(())
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

    #[test]
    fn ordered_route_points_accept_known_and_custom_types() {
        let points = vec![
            TripRoutePoint {
                point_id: point_id(1),
                sequence: 1,
                kind: TripPointKind::Load,
                title: "Warehouse A".to_owned(),
            },
            TripRoutePoint {
                point_id: point_id(2),
                sequence: 2,
                kind: TripPointKind::Custom("ferry".to_owned()),
                title: "Ferry terminal".to_owned(),
            },
            TripRoutePoint {
                point_id: point_id(3),
                sequence: 3,
                kind: TripPointKind::Unload,
                title: "Receiver".to_owned(),
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
            },
            TripRoutePoint {
                point_id: point_id(4),
                sequence: 2,
                kind: TripPointKind::Unload,
                title: "Unload".to_owned(),
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
        }];
        assert_eq!(
            validate_route_points(&empty_custom),
            Err(RouteContractError::CustomKindRequired(point_id(7)))
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
        assert_eq!(header.schema_version, SchemaVersion::new(1, 0));
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
