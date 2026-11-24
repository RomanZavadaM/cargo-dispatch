#![forbid(unsafe_code)]

use cargo_dispatch_contracts::{
    PackageContentFingerprint, RouteContractError, TripPackageHeader, TripRoutePoint, TripVersion,
    validate_route_points,
};
use cargo_dispatch_domain::{PointId, TripId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripImportCandidate {
    pub header: TripPackageHeader,
    pub content_fingerprint: PackageContentFingerprint,
    pub route_points: Vec<TripRoutePoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTripPlan {
    pub trip_id: TripId,
    pub current_version: TripVersion,
    pub content_fingerprint: PackageContentFingerprint,
    pub route_points: Vec<TripRoutePoint>,
    pub completed_point_ids: BTreeSet<PointId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportResult {
    AppliedFirstVersion,
    AppliedNewerVersion { previous_version: TripVersion },
    IdempotentSameVersion,
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
    SameVersionContentConflict {
        version: TripVersion,
    },
    CompletedPointRemoved(PointId),
    CompletedPointChanged(PointId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionResult {
    Marked,
    AlreadyCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionError {
    TripNotFound,
    PointNotFound,
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
                    content_fingerprint: candidate.content_fingerprint,
                    route_points: candidate.route_points,
                    completed_point_ids: BTreeSet::new(),
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
            if candidate.content_fingerprint == current.content_fingerprint {
                return Ok(ImportResult::IdempotentSameVersion);
            }
            return Err(ImportError::SameVersionContentConflict {
                version: current.current_version,
            });
        }

        if candidate.header.base_version != Some(current.current_version) {
            return Err(ImportError::BaseVersionConflict {
                current_version: current.current_version,
                provided_base_version: candidate.header.base_version,
            });
        }

        for completed_point_id in &current.completed_point_ids {
            let current_point = current
                .route_points
                .iter()
                .find(|point| point.point_id == *completed_point_id)
                .expect("completed point must belong to stored route");
            let Some(incoming_point) = candidate
                .route_points
                .iter()
                .find(|point| point.point_id == *completed_point_id)
            else {
                return Err(ImportError::CompletedPointRemoved(*completed_point_id));
            };

            if incoming_point != current_point {
                return Err(ImportError::CompletedPointChanged(*completed_point_id));
            }
        }

        let completed_point_ids = current.completed_point_ids.clone();
        let previous_version = current.current_version;
        self.trips.insert(
            trip_id,
            StoredTripPlan {
                trip_id,
                current_version: incoming_version,
                content_fingerprint: candidate.content_fingerprint,
                route_points: candidate.route_points,
                completed_point_ids,
            },
        );

        Ok(ImportResult::AppliedNewerVersion { previous_version })
    }

    pub fn mark_point_completed(
        &mut self,
        trip_id: TripId,
        point_id: PointId,
    ) -> Result<CompletionResult, CompletionError> {
        let trip = self
            .trips
            .get_mut(&trip_id)
            .ok_or(CompletionError::TripNotFound)?;

        if !trip
            .route_points
            .iter()
            .any(|point| point.point_id == point_id)
        {
            return Err(CompletionError::PointNotFound);
        }

        if !trip.completed_point_ids.insert(point_id) {
            return Ok(CompletionResult::AlreadyCompleted);
        }

        Ok(CompletionResult::Marked)
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

    fn trip_id(byte: u8) -> TripId {
        TripId::from_bytes([byte; 16]).expect("fixture trip id")
    }

    fn point_id(byte: u8) -> PointId {
        PointId::from_bytes([byte; 16]).expect("fixture point id")
    }

    fn fingerprint(byte: u8) -> PackageContentFingerprint {
        PackageContentFingerprint::from_bytes([byte; 32])
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
        fingerprint_byte: u8,
        route_points: Vec<TripRoutePoint>,
    ) -> TripImportCandidate {
        TripImportCandidate {
            header: TripPackageHeader::new(
                trip_id,
                TripVersion::new(version).expect("trip version"),
                base_version.map(|value| TripVersion::new(value).expect("base version")),
            )
            .expect("header"),
            content_fingerprint: fingerprint(fingerprint_byte),
            route_points,
        }
    }

    #[test]
    fn first_version_is_applied_and_stored() {
        let id = trip_id(1);
        let mut registry = InMemoryTripRegistry::default();

        assert_eq!(
            registry.import(candidate(id, 1, None, 1, points(1, 2))),
            Ok(ImportResult::AppliedFirstVersion)
        );

        let stored = registry.get(id).expect("stored trip");
        assert_eq!(stored.current_version.get(), 1);
        assert_eq!(stored.route_points, points(1, 2));
    }

    #[test]
    fn same_version_and_same_fingerprint_is_idempotent() {
        let id = trip_id(2);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, 2, points(3, 4)))
            .expect("first import");

        assert_eq!(
            registry.import(candidate(id, 1, None, 2, points(3, 4))),
            Ok(ImportResult::IdempotentSameVersion)
        );
        assert_eq!(
            registry.get(id).expect("stored trip").route_points,
            points(3, 4)
        );
    }

    #[test]
    fn same_version_with_different_fingerprint_is_conflict() {
        let id = trip_id(8);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, 14, points(24, 25)))
            .expect("first import");

        assert_eq!(
            registry.import(candidate(id, 1, None, 15, points(26, 27))),
            Err(ImportError::SameVersionContentConflict {
                version: TripVersion::new(1).expect("version"),
            })
        );
        let stored = registry.get(id).expect("stored trip");
        assert_eq!(stored.content_fingerprint, fingerprint(14));
        assert_eq!(stored.route_points, points(24, 25));
    }

    #[test]
    fn newer_version_with_matching_base_is_applied() {
        let id = trip_id(3);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, 4, points(7, 8)))
            .expect("first import");

        assert_eq!(
            registry.import(candidate(id, 2, Some(1), 5, points(7, 9))),
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
            .import(candidate(id, 1, None, 6, points(10, 11)))
            .expect("first import");
        registry
            .import(candidate(id, 2, Some(1), 7, points(10, 12)))
            .expect("second import");

        assert_eq!(
            registry.import(candidate(id, 1, None, 8, points(13, 14))),
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
            .import(candidate(id, 1, None, 9, points(15, 16)))
            .expect("first import");
        registry
            .import(candidate(id, 2, Some(1), 10, points(15, 17)))
            .expect("second import");

        assert_eq!(
            registry.import(candidate(id, 4, Some(1), 11, points(18, 19))),
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
            registry.import(candidate(id, 2, Some(1), 12, points(20, 21))),
            Err(ImportError::UnknownTripRequiresFirstVersion {
                incoming_version: TripVersion::new(2).expect("version"),
            })
        );
        assert_eq!(registry.get(id), None);
    }

    #[test]
    fn completion_is_control_owned_and_idempotent() {
        let id = trip_id(9);
        let completed = point_id(28);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, 16, points(28, 29)))
            .expect("first import");

        assert_eq!(
            registry.mark_point_completed(id, completed),
            Ok(CompletionResult::Marked)
        );
        assert_eq!(
            registry.mark_point_completed(id, completed),
            Ok(CompletionResult::AlreadyCompleted)
        );
        assert!(
            registry
                .get(id)
                .expect("stored trip")
                .completed_point_ids
                .contains(&completed)
        );
    }

    #[test]
    fn completed_point_survives_newer_plan_while_future_point_may_change() {
        let id = trip_id(10);
        let completed = point_id(30);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, 17, points(30, 31)))
            .expect("first import");
        registry
            .mark_point_completed(id, completed)
            .expect("completion");

        assert_eq!(
            registry.import(candidate(id, 2, Some(1), 18, points(30, 32))),
            Ok(ImportResult::AppliedNewerVersion {
                previous_version: TripVersion::new(1).expect("version"),
            })
        );

        let stored = registry.get(id).expect("stored trip");
        assert_eq!(stored.current_version.get(), 2);
        assert!(stored.completed_point_ids.contains(&completed));
        assert_eq!(stored.route_points, points(30, 32));
    }

    #[test]
    fn newer_plan_cannot_change_completed_point() {
        let id = trip_id(11);
        let completed = point_id(33);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, 19, points(33, 34)))
            .expect("first import");
        registry
            .mark_point_completed(id, completed)
            .expect("completion");

        let mut changed = points(33, 35);
        changed[0].title = "Changed completed stop".to_owned();

        assert_eq!(
            registry.import(candidate(id, 2, Some(1), 20, changed)),
            Err(ImportError::CompletedPointChanged(completed))
        );
        assert_eq!(
            registry.get(id).expect("stored trip").current_version.get(),
            1
        );
    }

    #[test]
    fn newer_plan_cannot_remove_completed_point() {
        let id = trip_id(12);
        let completed = point_id(36);
        let mut registry = InMemoryTripRegistry::default();
        registry
            .import(candidate(id, 1, None, 21, points(36, 37)))
            .expect("first import");
        registry
            .mark_point_completed(id, completed)
            .expect("completion");

        assert_eq!(
            registry.import(candidate(id, 2, Some(1), 22, points(38, 37))),
            Err(ImportError::CompletedPointRemoved(completed))
        );
        assert!(
            registry
                .get(id)
                .expect("stored trip")
                .completed_point_ids
                .contains(&completed)
        );
    }

    #[test]
    fn completion_rejects_unknown_trip_or_point() {
        let id = trip_id(13);
        let mut registry = InMemoryTripRegistry::default();

        assert_eq!(
            registry.mark_point_completed(id, point_id(39)),
            Err(CompletionError::TripNotFound)
        );

        registry
            .import(candidate(id, 1, None, 23, points(40, 41)))
            .expect("first import");
        assert_eq!(
            registry.mark_point_completed(id, point_id(42)),
            Err(CompletionError::PointNotFound)
        );
    }

    #[test]
    fn invalid_route_is_rejected_before_registry_changes() {
        let id = trip_id(7);
        let mut bad_points = points(22, 23);
        bad_points[1].sequence = 3;
        let mut registry = InMemoryTripRegistry::default();

        assert_eq!(
            registry.import(candidate(id, 1, None, 13, bad_points)),
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
