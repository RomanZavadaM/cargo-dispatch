#![forbid(unsafe_code)]

use cargo_dispatch_contracts::{
    RouteContractError, TripPackageHeader, TripRoutePoint, TripVersion, validate_route_points,
};
use cargo_dispatch_domain::TripId;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripImportCandidate {
    pub header: TripPackageHeader,
    pub route_points: Vec<TripRoutePoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTripPlan {
    pub trip_id: TripId,
    pub current_version: TripVersion,
    pub route_points: Vec<TripRoutePoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportResult {
    AppliedFirstVersion,
    AppliedNewerVersion { previous_version: TripVersion },
    SameVersionNeedsContentCheck,
    IgnoredStaleVersion { current_version: TripVersion },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    InvalidRoute(RouteContractError),
    UnknownTripRequiresFirstVersion {
        incoming_version: TripVersion,
    },
    BaseVersionConflict {
        current_version: TripVersion,
        provided_base_version: Option<TripVersion>,
    },
}

#[derive(Debug, Default, Clone)]
pub struct InMemoryTripRegistry {
    trips: BTreeMap<TripId, StoredTripPlan>,
}

impl InMemoryTripRegistry {
    pub fn import(&mut self, candidate: TripImportCandidate) -> Result<ImportResult, ImportError> {
        validate_route_points(&candidate.route_points).map_err(ImportError::InvalidRoute)?;

        let trip_id = candidate.header.trip_id;
        let incoming_version = candidate.header.trip_version;

        let Some(current) = self.trips.get(&trip_id) else {
            if incoming_version.get() != 1 {
                return Err(ImportError::UnknownTripRequiresFirstVersion { incoming_version });
            }

            self.trips.insert(
                trip_id,
                StoredTripPlan {
                    trip_id,
                    current_version: incoming_version,
                    route_points: candidate.route_points,
                },
            );
            return Ok(ImportResult::AppliedFirstVersion);
        };

        if incoming_version < current.current_version {
            return Ok(ImportResult::IgnoredStaleVersion {
                current_version: current.current_version,
            });
        }

        if incoming_version == current.current_version {
            return Ok(ImportResult::SameVersionNeedsContentCheck);
        }

        if candidate.header.base_version != Some(current.current_version) {
            return Err(ImportError::BaseVersionConflict {
                current_version: current.current_version,
                provided_base_version: candidate.header.base_version,
            });
        }

        let previous_version = current.current_version;
        self.trips.insert(
            trip_id,
            StoredTripPlan {
                trip_id,
                current_version: incoming_version,
                route_points: candidate.route_points,
            },
        );

        Ok(ImportResult::AppliedNewerVersion { previous_version })
    }

    #[must_use]
    pub fn get(&self, trip_id: TripId) -> Option<StoredTripPlan> {
        self.trips.get(&trip_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_contracts::{TripPointKind, TripRoutePoint};
    use cargo_dispatch_domain::PointId;

    fn trip_id(byte: u8) -> TripId {
        TripId::from_bytes([byte; 16]).expect("fixture trip id")
    }

    fn point_id(byte: u8) -> PointId {
        PointId::from_bytes([byte; 16]).expect("fixture point id")
    }

    fn points(first: u8, second: u8) -> Vec<TripRoutePoint> {
        vec![
            TripRoutePoint {
                point_id: point_id(first),
                sequence: 1,
                kind: TripPointKind::Load,
                title: "Load".to_owned(),
            },
            TripRoutePoint {
                point_id: point_id(second),
                sequence: 2,
                kind: TripPointKind::Unload,
                title: "Unload".to_owned(),
            },
        ]
    }

    fn candidate(
        trip_id: TripId,
        version: u64,
        base_version: Option<u64>,
        route_points: Vec<TripRoutePoint>,
    ) -> TripImportCandidate {
        TripImportCandidate {
            header: TripPackageHeader::new(
                trip_id,
                TripVersion::new(version).expect("trip version"),
                base_version.map(|value| TripVersion::new(value).expect("base version")),
            )
            .expect("header"),
            route_points,
        }
    }

    #[test]
    fn first_version_is_applied_and_stored() {
        let id = trip_id(1);
        let mut registry = InMemoryTripRegistry::default();

        assert_eq!(
            registry.import(candidate(id, 1, None, points(1, 2))),
            Ok(ImportResult::AppliedFirstVersion)
        );

        let stored = registry.get(id).expect("stored trip");
        assert_eq!(stored.current_version.get(), 1);
        assert_eq!(stored.route_points, points(1, 2));
    }

    #[test]
    fn same_version_requires_content_check_and_does_not_overwrite() {
        let id = trip_id(2);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, points(3, 4)))
            .expect("first import");

        assert_eq!(
            registry.import(candidate(id, 1, None, points(5, 6))),
            Ok(ImportResult::SameVersionNeedsContentCheck)
        );

        assert_eq!(
            registry.get(id).expect("stored trip").route_points,
            points(3, 4)
        );
    }

    #[test]
    fn newer_version_with_matching_base_is_applied() {
        let id = trip_id(3);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, points(7, 8)))
            .expect("first import");

        assert_eq!(
            registry.import(candidate(id, 2, Some(1), points(7, 9))),
            Ok(ImportResult::AppliedNewerVersion {
                previous_version: TripVersion::new(1).expect("version"),
            })
        );

        let stored = registry.get(id).expect("stored trip");
        assert_eq!(stored.current_version.get(), 2);
        assert_eq!(stored.route_points, points(7, 9));
    }

    #[test]
    fn stale_version_is_ignored_without_overwriting_current_plan() {
        let id = trip_id(4);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, points(10, 11)))
            .expect("first import");
        registry
            .import(candidate(id, 2, Some(1), points(10, 12)))
            .expect("second import");

        assert_eq!(
            registry.import(candidate(id, 1, None, points(13, 14))),
            Ok(ImportResult::IgnoredStaleVersion {
                current_version: TripVersion::new(2).expect("version"),
            })
        );
        assert_eq!(
            registry.get(id).expect("stored trip").route_points,
            points(10, 12)
        );
    }

    #[test]
    fn base_version_conflict_is_rejected_without_overwriting() {
        let id = trip_id(5);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, points(15, 16)))
            .expect("first import");
        registry
            .import(candidate(id, 2, Some(1), points(15, 17)))
            .expect("second import");

        assert_eq!(
            registry.import(candidate(id, 4, Some(1), points(18, 19))),
            Err(ImportError::BaseVersionConflict {
                current_version: TripVersion::new(2).expect("version"),
                provided_base_version: Some(TripVersion::new(1).expect("version")),
            })
        );
        assert_eq!(
            registry.get(id).expect("stored trip").current_version.get(),
            2
        );
    }

    #[test]
    fn unknown_trip_must_begin_with_version_one() {
        let id = trip_id(6);
        let mut registry = InMemoryTripRegistry::default();

        assert_eq!(
            registry.import(candidate(id, 2, Some(1), points(20, 21))),
            Err(ImportError::UnknownTripRequiresFirstVersion {
                incoming_version: TripVersion::new(2).expect("version"),
            })
        );
        assert_eq!(registry.get(id), None);
    }

    #[test]
    fn invalid_route_is_rejected_before_registry_changes() {
        let id = trip_id(7);
        let mut bad_points = points(22, 23);
        bad_points[1].sequence = 3;
        let mut registry = InMemoryTripRegistry::default();

        assert_eq!(
            registry.import(candidate(id, 1, None, bad_points)),
            Err(ImportError::InvalidRoute(
                RouteContractError::SequenceMismatch {
                    expected: 2,
                    actual: 3,
                }
            ))
        );
        assert_eq!(registry.get(id), None);
    }
}
