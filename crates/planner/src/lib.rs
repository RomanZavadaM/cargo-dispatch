#![forbid(unsafe_code)]

use cargo_dispatch_domain::{
    AvailabilityState, CalculationId, CallTaskId, CandidateId, CargoId, CargoSummary, LoadRequest,
    NegotiatedTermsId, OperatorId, PlanningAssignment, PointId, RequestId, RequestRevision,
    ReservationId, WorkingRouteId,
};
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
    AlreadyConfirmedToTrip,
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
    SoftReserved,
    Calling,
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

    match request.request.planning_assignment {
        PlanningAssignment::Free | PlanningAssignment::Released => {}
        PlanningAssignment::SoftReserved => {
            check_reasons.push(NeedsCheckReason::SoftReserved);
        }
        PlanningAssignment::Calling => {
            check_reasons.push(NeedsCheckReason::Calling);
        }
        PlanningAssignment::ConfirmedToTrip => {
            hard_reasons.push(HardRejectReason::AlreadyConfirmedToTrip);
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
        (Some(cargo_pallets), Some(available_pallets)) if cargo_pallets > available_pallets => {
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
pub struct RouteCargoLeg {
    pub cargo_id: CargoId,
    pub pickup_stop: usize,
    pub unload_stop: usize,
    pub weight_kg: Option<u32>,
    pub pallets: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteHardRejectReason {
    InvalidCargoStopOrder {
        cargo_id: CargoId,
        pickup_stop: usize,
        unload_stop: usize,
    },
    PayloadExceededOnSegment {
        segment_index: usize,
        load_kg: u64,
        capacity_kg: u32,
    },
    PalletCapacityExceededOnSegment {
        segment_index: usize,
        load_pallets: u32,
        capacity_pallets: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteNeedsCheckReason {
    CargoWeightUnknown(CargoId),
    CargoPalletsUnknown(CargoId),
    AvailablePayloadUnknown,
    AvailablePalletsUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSegmentFeasibility {
    pub level: FeasibilityLevel,
    pub hard_reasons: Vec<RouteHardRejectReason>,
    pub check_reasons: Vec<RouteNeedsCheckReason>,
}

#[must_use]
pub fn evaluate_route_segments(
    stop_count: usize,
    cargo_legs: &[RouteCargoLeg],
    capacity: AvailableCapacity,
) -> RouteSegmentFeasibility {
    let mut hard_reasons = Vec::new();
    let mut check_reasons = Vec::new();
    let mut valid_legs = Vec::new();

    for leg in cargo_legs {
        if leg.pickup_stop >= leg.unload_stop || leg.unload_stop >= stop_count {
            hard_reasons.push(RouteHardRejectReason::InvalidCargoStopOrder {
                cargo_id: leg.cargo_id,
                pickup_stop: leg.pickup_stop,
                unload_stop: leg.unload_stop,
            });
        } else {
            valid_legs.push(leg);
        }
    }

    for segment_index in 0..stop_count.saturating_sub(1) {
        let active_legs: Vec<_> = valid_legs
            .iter()
            .copied()
            .filter(|leg| leg.pickup_stop <= segment_index && segment_index < leg.unload_stop)
            .collect();

        if active_legs.is_empty() {
            continue;
        }

        let mut known_weight_kg = 0_u64;
        let mut known_pallets = 0_u32;

        for leg in active_legs {
            match leg.weight_kg {
                Some(weight_kg) => known_weight_kg += u64::from(weight_kg),
                None => {
                    let reason = RouteNeedsCheckReason::CargoWeightUnknown(leg.cargo_id);
                    if !check_reasons.contains(&reason) {
                        check_reasons.push(reason);
                    }
                }
            }

            match leg.pallets {
                Some(pallets) => known_pallets += u32::from(pallets),
                None => {
                    let reason = RouteNeedsCheckReason::CargoPalletsUnknown(leg.cargo_id);
                    if !check_reasons.contains(&reason) {
                        check_reasons.push(reason);
                    }
                }
            }
        }

        match capacity.payload_kg {
            Some(capacity_kg) if known_weight_kg > u64::from(capacity_kg) => {
                hard_reasons.push(RouteHardRejectReason::PayloadExceededOnSegment {
                    segment_index,
                    load_kg: known_weight_kg,
                    capacity_kg,
                });
            }
            Some(_) => {}
            None => {
                if !check_reasons.contains(&RouteNeedsCheckReason::AvailablePayloadUnknown) {
                    check_reasons.push(RouteNeedsCheckReason::AvailablePayloadUnknown);
                }
            }
        }

        match capacity.pallets {
            Some(capacity_pallets) if known_pallets > u32::from(capacity_pallets) => {
                hard_reasons.push(RouteHardRejectReason::PalletCapacityExceededOnSegment {
                    segment_index,
                    load_pallets: known_pallets,
                    capacity_pallets,
                });
            }
            Some(_) => {}
            None => {
                if !check_reasons.contains(&RouteNeedsCheckReason::AvailablePalletsUnknown) {
                    check_reasons.push(RouteNeedsCheckReason::AvailablePalletsUnknown);
                }
            }
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

    RouteSegmentFeasibility {
        level,
        hard_reasons,
        check_reasons,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmedTimeWindow {
    pub earliest_unix_ms: u64,
    pub latest_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteTimePoint {
    pub window: Option<ConfirmedTimeWindow>,
    pub handling_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteTimeSegment {
    pub road_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeHardRejectReason {
    SegmentCountMismatch {
        point_count: usize,
        segment_count: usize,
    },
    InvalidTimeWindow {
        point_index: usize,
        earliest_unix_ms: u64,
        latest_unix_ms: u64,
    },
    WindowMissed {
        point_index: usize,
        earliest_possible_arrival_unix_ms: u64,
        latest_unix_ms: u64,
    },
    TimeOverflow {
        point_index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeNeedsCheckReason {
    TimeWindowUnknown(usize),
    HandlingTimeUnknown(usize),
    RoadTimeUnknown(usize),
    TightTimeWindow { point_index: usize, slack_ms: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTimeFeasibility {
    pub level: FeasibilityLevel,
    pub hard_reasons: Vec<TimeHardRejectReason>,
    pub check_reasons: Vec<TimeNeedsCheckReason>,
    pub earliest_arrivals_unix_ms: Vec<u64>,
    pub earliest_departures_unix_ms: Vec<u64>,
    pub minimum_known_slack_ms: Option<u64>,
}

#[must_use]
pub fn evaluate_route_time_windows(
    start_arrival_unix_ms: u64,
    points: &[RouteTimePoint],
    segments: &[RouteTimeSegment],
    tight_slack_ms: u64,
) -> RouteTimeFeasibility {
    if segments.len() != points.len().saturating_sub(1) {
        return RouteTimeFeasibility {
            level: FeasibilityLevel::Impossible,
            hard_reasons: vec![TimeHardRejectReason::SegmentCountMismatch {
                point_count: points.len(),
                segment_count: segments.len(),
            }],
            check_reasons: Vec::new(),
            earliest_arrivals_unix_ms: Vec::new(),
            earliest_departures_unix_ms: Vec::new(),
            minimum_known_slack_ms: None,
        };
    }

    let mut hard_reasons = Vec::new();
    let mut check_reasons = Vec::new();
    let mut earliest_arrivals_unix_ms = Vec::with_capacity(points.len());
    let mut earliest_departures_unix_ms = Vec::with_capacity(points.len());
    let mut minimum_known_slack_ms: Option<u64> = None;
    let mut current_arrival = start_arrival_unix_ms;

    for (point_index, point) in points.iter().enumerate() {
        earliest_arrivals_unix_ms.push(current_arrival);

        let service_start = match point.window {
            Some(window) if window.earliest_unix_ms > window.latest_unix_ms => {
                hard_reasons.push(TimeHardRejectReason::InvalidTimeWindow {
                    point_index,
                    earliest_unix_ms: window.earliest_unix_ms,
                    latest_unix_ms: window.latest_unix_ms,
                });
                current_arrival
            }
            Some(window) => {
                let service_start = current_arrival.max(window.earliest_unix_ms);
                if service_start > window.latest_unix_ms {
                    hard_reasons.push(TimeHardRejectReason::WindowMissed {
                        point_index,
                        earliest_possible_arrival_unix_ms: current_arrival,
                        latest_unix_ms: window.latest_unix_ms,
                    });
                } else {
                    let slack_ms = window.latest_unix_ms - service_start;
                    minimum_known_slack_ms = Some(
                        minimum_known_slack_ms.map_or(slack_ms, |minimum| minimum.min(slack_ms)),
                    );
                    if slack_ms <= tight_slack_ms {
                        check_reasons.push(TimeNeedsCheckReason::TightTimeWindow {
                            point_index,
                            slack_ms,
                        });
                    }
                }
                service_start
            }
            None => {
                check_reasons.push(TimeNeedsCheckReason::TimeWindowUnknown(point_index));
                current_arrival
            }
        };

        let handling_time_ms = match point.handling_time_ms {
            Some(value) => value,
            None => {
                check_reasons.push(TimeNeedsCheckReason::HandlingTimeUnknown(point_index));
                0
            }
        };

        let departure = match service_start.checked_add(handling_time_ms) {
            Some(value) => value,
            None => {
                hard_reasons.push(TimeHardRejectReason::TimeOverflow { point_index });
                u64::MAX
            }
        };
        earliest_departures_unix_ms.push(departure);

        if let Some(segment) = segments.get(point_index) {
            let road_time_ms = match segment.road_time_ms {
                Some(value) => value,
                None => {
                    check_reasons.push(TimeNeedsCheckReason::RoadTimeUnknown(point_index));
                    0
                }
            };
            current_arrival = match departure.checked_add(road_time_ms) {
                Some(value) => value,
                None => {
                    hard_reasons.push(TimeHardRejectReason::TimeOverflow { point_index });
                    u64::MAX
                }
            };
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

    RouteTimeFeasibility {
        level,
        hard_reasons,
        check_reasons,
        earliest_arrivals_unix_ms,
        earliest_departures_unix_ms,
        minimum_known_slack_ms,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceHardRejectReason {
    DrivingBudgetExceeded {
        required_driving_ms: u64,
        available_driving_ms: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceNeedsCheckReason {
    RequiredDrivingUnknown,
    AvailableDrivingUnknown,
    LowDrivingReserve { reserve_ms: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceFeasibility {
    pub level: FeasibilityLevel,
    pub hard_reasons: Vec<ComplianceHardRejectReason>,
    pub check_reasons: Vec<ComplianceNeedsCheckReason>,
    pub known_driving_reserve_ms: Option<u64>,
}

#[must_use]
pub fn evaluate_driving_budget(
    required_driving_ms: Option<u64>,
    available_driving_ms: Option<u64>,
    low_reserve_threshold_ms: u64,
) -> ComplianceFeasibility {
    match (required_driving_ms, available_driving_ms) {
        (Some(required), Some(available)) if required > available => ComplianceFeasibility {
            level: FeasibilityLevel::Impossible,
            hard_reasons: vec![ComplianceHardRejectReason::DrivingBudgetExceeded {
                required_driving_ms: required,
                available_driving_ms: available,
            }],
            check_reasons: Vec::new(),
            known_driving_reserve_ms: Some(0),
        },
        (Some(required), Some(available)) => {
            let reserve_ms = available - required;
            let check_reasons = if reserve_ms <= low_reserve_threshold_ms {
                vec![ComplianceNeedsCheckReason::LowDrivingReserve { reserve_ms }]
            } else {
                Vec::new()
            };
            ComplianceFeasibility {
                level: if check_reasons.is_empty() {
                    FeasibilityLevel::Feasible
                } else {
                    FeasibilityLevel::NeedsCheck
                },
                hard_reasons: Vec::new(),
                check_reasons,
                known_driving_reserve_ms: Some(reserve_ms),
            }
        }
        (None, Some(_)) => ComplianceFeasibility {
            level: FeasibilityLevel::NeedsCheck,
            hard_reasons: Vec::new(),
            check_reasons: vec![ComplianceNeedsCheckReason::RequiredDrivingUnknown],
            known_driving_reserve_ms: None,
        },
        (Some(_), None) => ComplianceFeasibility {
            level: FeasibilityLevel::NeedsCheck,
            hard_reasons: Vec::new(),
            check_reasons: vec![ComplianceNeedsCheckReason::AvailableDrivingUnknown],
            known_driving_reserve_ms: None,
        },
        (None, None) => ComplianceFeasibility {
            level: FeasibilityLevel::NeedsCheck,
            hard_reasons: Vec::new(),
            check_reasons: vec![
                ComplianceNeedsCheckReason::RequiredDrivingUnknown,
                ComplianceNeedsCheckReason::AvailableDrivingUnknown,
            ],
            known_driving_reserve_ms: None,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkingRouteOrigin {
    Candidate(CandidateId),
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkingRouteState {
    Draft,
    Selected,
    Negotiating,
    PartiallyConfirmed,
    Ready,
    TransferredToControl,
    Replanning,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingRoute {
    pub working_route_id: WorkingRouteId,
    pub origin: WorkingRouteOrigin,
    pub points: Vec<WorkingRoutePoint>,
    pub current_revision: u64,
    pub state: WorkingRouteState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkingRouteError {
    CandidateNotFound,
    DuplicatePoint,
}

fn working_route_points(
    point_ids: &[PointId],
) -> Result<Vec<WorkingRoutePoint>, WorkingRouteError> {
    let mut seen = std::collections::BTreeSet::new();
    if point_ids.iter().any(|point_id| !seen.insert(*point_id)) {
        return Err(WorkingRouteError::DuplicatePoint);
    }

    Ok(point_ids
        .iter()
        .map(|point_id| WorkingRoutePoint {
            point_id: *point_id,
            locked: false,
            confirmed: false,
        })
        .collect())
}

pub fn working_route_from_candidate(
    working_route_id: WorkingRouteId,
    candidate_id: CandidateId,
    candidates: &InMemoryCandidateStore,
) -> Result<WorkingRoute, WorkingRouteError> {
    let candidate = candidates
        .get(candidate_id)
        .ok_or(WorkingRouteError::CandidateNotFound)?;

    Ok(WorkingRoute {
        working_route_id,
        origin: WorkingRouteOrigin::Candidate(candidate_id),
        points: working_route_points(&candidate.ordered_point_ids)?,
        current_revision: 1,
        state: WorkingRouteState::Selected,
    })
}

pub fn manual_working_route(
    working_route_id: WorkingRouteId,
    point_ids: &[PointId],
) -> Result<WorkingRoute, WorkingRouteError> {
    Ok(WorkingRoute {
        working_route_id,
        origin: WorkingRouteOrigin::Manual,
        points: working_route_points(point_ids)?,
        current_revision: 1,
        state: WorkingRouteState::Draft,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkingRouteUpdateError {
    RecalculationRejected(Vec<RecalculationHardRejectReason>),
    RevisionOverflow,
}

pub fn recalculate_working_route(
    route: &WorkingRoute,
    proposed_point_ids: &[PointId],
) -> Result<WorkingRoute, WorkingRouteUpdateError> {
    let validation = validate_recalculation(&route.points, proposed_point_ids);
    if !validation.hard_reasons.is_empty() {
        return Err(WorkingRouteUpdateError::RecalculationRejected(
            validation.hard_reasons,
        ));
    }

    let existing: BTreeMap<_, _> = route
        .points
        .iter()
        .map(|point| (point.point_id, *point))
        .collect();
    let points = proposed_point_ids
        .iter()
        .map(|point_id| {
            existing.get(point_id).copied().unwrap_or(WorkingRoutePoint {
                point_id: *point_id,
                locked: false,
                confirmed: false,
            })
        })
        .collect();

    let current_revision = route
        .current_revision
        .checked_add(1)
        .ok_or(WorkingRouteUpdateError::RevisionOverflow)?;

    Ok(WorkingRoute {
        working_route_id: route.working_route_id,
        origin: route.origin,
        points,
        current_revision,
        state: WorkingRouteState::Replanning,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkingRoutePoint {
    pub point_id: PointId,
    pub locked: bool,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecalculationHardRejectReason {
    DuplicateProposedPoint(PointId),
    ProtectedPointRemoved(PointId),
    ProtectedOrderChanged {
        earlier_point_id: PointId,
        later_point_id: PointId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecalculationValidation {
    pub level: FeasibilityLevel,
    pub hard_reasons: Vec<RecalculationHardRejectReason>,
}

#[must_use]
pub fn validate_recalculation(
    current_points: &[WorkingRoutePoint],
    proposed_point_ids: &[PointId],
) -> RecalculationValidation {
    let mut hard_reasons = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for point_id in proposed_point_ids {
        if !seen.insert(*point_id) {
            hard_reasons.push(RecalculationHardRejectReason::DuplicateProposedPoint(
                *point_id,
            ));
        }
    }

    let protected_points: Vec<_> = current_points
        .iter()
        .filter(|point| point.locked || point.confirmed)
        .map(|point| point.point_id)
        .collect();

    for protected_point_id in &protected_points {
        if !seen.contains(protected_point_id) {
            hard_reasons.push(RecalculationHardRejectReason::ProtectedPointRemoved(
                *protected_point_id,
            ));
        }
    }

    let proposed_positions: BTreeMap<_, _> = proposed_point_ids
        .iter()
        .enumerate()
        .map(|(index, point_id)| (*point_id, index))
        .collect();

    for pair in protected_points.windows(2) {
        let [earlier_point_id, later_point_id] = pair else {
            continue;
        };
        let (Some(earlier_position), Some(later_position)) = (
            proposed_positions.get(earlier_point_id),
            proposed_positions.get(later_point_id),
        ) else {
            continue;
        };

        if earlier_position > later_position {
            hard_reasons.push(RecalculationHardRejectReason::ProtectedOrderChanged {
                earlier_point_id: *earlier_point_id,
                later_point_id: *later_point_id,
            });
        }
    }

    RecalculationValidation {
        level: if hard_reasons.is_empty() {
            FeasibilityLevel::Feasible
        } else {
            FeasibilityLevel::Impossible
        },
        hard_reasons,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessLevel {
    Fresh,
    Warning,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessError {
    InvalidThresholds,
    ObservationInFuture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestFreshness {
    pub age_ms: u64,
    pub level: FreshnessLevel,
}

pub fn evaluate_request_freshness(
    request: &PlanningRequest,
    now_unix_ms: u64,
    warning_after_ms: u64,
    high_after_ms: u64,
) -> Result<RequestFreshness, FreshnessError> {
    if warning_after_ms > high_after_ms {
        return Err(FreshnessError::InvalidThresholds);
    }

    let observed_at_unix_ms = request.request.source.observed_at_unix_ms;
    if observed_at_unix_ms > now_unix_ms {
        return Err(FreshnessError::ObservationInFuture);
    }

    let age_ms = now_unix_ms - observed_at_unix_ms;
    let level = if age_ms >= high_after_ms {
        FreshnessLevel::High
    } else if age_ms >= warning_after_ms {
        FreshnessLevel::Warning
    } else {
        FreshnessLevel::Fresh
    };

    Ok(RequestFreshness { age_ms, level })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationState {
    Active,
    Expired,
    Released,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftReservation {
    pub reservation_id: ReservationId,
    pub request_id: RequestId,
    pub operator_id: OperatorId,
    pub working_route_id: WorkingRouteId,
    pub created_at_unix_ms: u64,
    pub expires_at_unix_ms: Option<u64>,
    pub released_at_unix_ms: Option<u64>,
}

impl SoftReservation {
    #[must_use]
    pub fn state_at(&self, now_unix_ms: u64) -> ReservationState {
        if self.released_at_unix_ms.is_some() {
            ReservationState::Released
        } else if self
            .expires_at_unix_ms
            .is_some_and(|expires_at| now_unix_ms >= expires_at)
        {
            ReservationState::Expired
        } else {
            ReservationState::Active
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationError {
    DuplicateReservation,
    ReservationNotFound,
    AlreadyReleased,
    ReleaseBeforeCreation,
}

#[derive(Debug, Default, Clone)]
pub struct InMemoryReservationBook {
    reservations: BTreeMap<ReservationId, SoftReservation>,
}

impl InMemoryReservationBook {
    pub fn reserve(&mut self, reservation: SoftReservation) -> Result<(), ReservationError> {
        if self.reservations.contains_key(&reservation.reservation_id) {
            return Err(ReservationError::DuplicateReservation);
        }
        self.reservations
            .insert(reservation.reservation_id, reservation);
        Ok(())
    }

    pub fn release(
        &mut self,
        reservation_id: ReservationId,
        released_at_unix_ms: u64,
    ) -> Result<(), ReservationError> {
        let reservation = self
            .reservations
            .get_mut(&reservation_id)
            .ok_or(ReservationError::ReservationNotFound)?;

        if reservation.released_at_unix_ms.is_some() {
            return Err(ReservationError::AlreadyReleased);
        }
        if released_at_unix_ms < reservation.created_at_unix_ms {
            return Err(ReservationError::ReleaseBeforeCreation);
        }

        reservation.released_at_unix_ms = Some(released_at_unix_ms);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, reservation_id: ReservationId) -> Option<SoftReservation> {
        self.reservations.get(&reservation_id).cloned()
    }

    #[must_use]
    pub fn for_request(&self, request_id: RequestId) -> Vec<SoftReservation> {
        self.reservations
            .values()
            .filter(|reservation| reservation.request_id == request_id)
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn active_for_request(
        &self,
        request_id: RequestId,
        now_unix_ms: u64,
    ) -> Vec<SoftReservation> {
        self.reservations
            .values()
            .filter(|reservation| reservation.request_id == request_id)
            .filter(|reservation| reservation.state_at(now_unix_ms) == ReservationState::Active)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallResult {
    Confirmed,
    Rejected,
    LostTaken,
    Callback,
    TermsChanged,
    NoAnswer,
    ContactInvalid,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedCallAttempt {
    pub call_task_id: CallTaskId,
    pub request_id: RequestId,
    pub working_route_id: WorkingRouteId,
    pub operator_id: OperatorId,
    pub created_at_unix_ms: u64,
    pub started_at_unix_ms: u64,
    pub finished_at_unix_ms: u64,
    pub result: CallResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallHistoryError {
    DuplicateCallTask,
    InvalidTimeline,
}

#[derive(Debug, Default, Clone)]
pub struct InMemoryCallHistory {
    attempts: Vec<CompletedCallAttempt>,
}

impl InMemoryCallHistory {
    pub fn append(&mut self, attempt: CompletedCallAttempt) -> Result<(), CallHistoryError> {
        if attempt.created_at_unix_ms > attempt.started_at_unix_ms
            || attempt.started_at_unix_ms > attempt.finished_at_unix_ms
        {
            return Err(CallHistoryError::InvalidTimeline);
        }
        if self
            .attempts
            .iter()
            .any(|saved| saved.call_task_id == attempt.call_task_id)
        {
            return Err(CallHistoryError::DuplicateCallTask);
        }

        self.attempts.push(attempt);
        Ok(())
    }

    #[must_use]
    pub fn for_request(&self, request_id: RequestId) -> Vec<CompletedCallAttempt> {
        self.attempts
            .iter()
            .filter(|attempt| attempt.request_id == request_id)
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn latest_for_request(&self, request_id: RequestId) -> Option<CompletedCallAttempt> {
        self.attempts
            .iter()
            .rev()
            .find(|attempt| attempt.request_id == request_id)
            .cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedTerms {
    pub negotiated_terms_id: NegotiatedTermsId,
    pub request_id: RequestId,
    pub working_route_id: WorkingRouteId,
    pub operator_id: OperatorId,
    pub confirmed_by_call_task_id: CallTaskId,
    pub confirmed_at_unix_ms: u64,
    pub cargo_weight_kg: Option<u32>,
    pub cargo_pallets: Option<u16>,
    pub pickup_window: Option<ConfirmedTimeWindow>,
    pub delivery_window: Option<ConfirmedTimeWindow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegotiatedTermsError {
    DuplicateTerms,
    NoConfirmedChanges,
    WeightMustBePositive,
    PalletsMustBePositive,
    InvalidPickupWindow,
    InvalidDeliveryWindow,
}

#[derive(Debug, Default, Clone)]
pub struct InMemoryNegotiatedTermsHistory {
    terms: Vec<NegotiatedTerms>,
}

impl InMemoryNegotiatedTermsHistory {
    pub fn append(&mut self, terms: NegotiatedTerms) -> Result<(), NegotiatedTermsError> {
        if self
            .terms
            .iter()
            .any(|saved| saved.negotiated_terms_id == terms.negotiated_terms_id)
        {
            return Err(NegotiatedTermsError::DuplicateTerms);
        }
        if terms.cargo_weight_kg == Some(0) {
            return Err(NegotiatedTermsError::WeightMustBePositive);
        }
        if terms.cargo_pallets == Some(0) {
            return Err(NegotiatedTermsError::PalletsMustBePositive);
        }
        if terms
            .pickup_window
            .is_some_and(|window| window.earliest_unix_ms > window.latest_unix_ms)
        {
            return Err(NegotiatedTermsError::InvalidPickupWindow);
        }
        if terms
            .delivery_window
            .is_some_and(|window| window.earliest_unix_ms > window.latest_unix_ms)
        {
            return Err(NegotiatedTermsError::InvalidDeliveryWindow);
        }
        if terms.cargo_weight_kg.is_none()
            && terms.cargo_pallets.is_none()
            && terms.pickup_window.is_none()
            && terms.delivery_window.is_none()
        {
            return Err(NegotiatedTermsError::NoConfirmedChanges);
        }

        self.terms.push(terms);
        Ok(())
    }

    #[must_use]
    pub fn for_request(&self, request_id: RequestId) -> Vec<NegotiatedTerms> {
        self.terms
            .iter()
            .filter(|terms| terms.request_id == request_id)
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn latest_for_request(&self, request_id: RequestId) -> Option<NegotiatedTerms> {
        self.terms
            .iter()
            .rev()
            .find(|terms| terms.request_id == request_id)
            .cloned()
    }
}

#[must_use]
pub fn effective_cargo_for_planning(
    request: &PlanningRequest,
    terms: Option<&NegotiatedTerms>,
) -> CargoSummary {
    let mut cargo = request.request.cargo.clone();
    if let Some(terms) = terms {
        if let Some(weight_kg) = terms.cargo_weight_kg {
            cargo.weight_kg = Some(weight_kg);
        }
        if let Some(pallets) = terms.cargo_pallets {
            cargo.pallets = Some(pallets);
        }
    }
    cargo
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateRequestBasis {
    pub request_id: RequestId,
    pub revision: RequestRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCandidateSnapshot {
    pub candidate_id: CandidateId,
    pub calculation_id: CalculationId,
    pub generated_at_unix_ms: u64,
    pub ordered_point_ids: Vec<PointId>,
    pub request_basis: Vec<CandidateRequestBasis>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSnapshotError {
    DuplicateCandidate,
    DuplicatePoint,
    DuplicateRequestBasis,
}

#[derive(Debug, Default, Clone)]
pub struct InMemoryCandidateStore {
    candidates: BTreeMap<CandidateId, RouteCandidateSnapshot>,
}

impl InMemoryCandidateStore {
    pub fn append(
        &mut self,
        candidate: RouteCandidateSnapshot,
    ) -> Result<(), CandidateSnapshotError> {
        if self.candidates.contains_key(&candidate.candidate_id) {
            return Err(CandidateSnapshotError::DuplicateCandidate);
        }

        let mut point_ids = std::collections::BTreeSet::new();
        if candidate
            .ordered_point_ids
            .iter()
            .any(|point_id| !point_ids.insert(*point_id))
        {
            return Err(CandidateSnapshotError::DuplicatePoint);
        }

        let mut request_ids = std::collections::BTreeSet::new();
        if candidate
            .request_basis
            .iter()
            .any(|basis| !request_ids.insert(basis.request_id))
        {
            return Err(CandidateSnapshotError::DuplicateRequestBasis);
        }

        self.candidates.insert(candidate.candidate_id, candidate);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, candidate_id: CandidateId) -> Option<RouteCandidateSnapshot> {
        self.candidates.get(&candidate_id).cloned()
    }

    #[must_use]
    pub fn for_calculation(&self, calculation_id: CalculationId) -> Vec<RouteCandidateSnapshot> {
        self.candidates
            .values()
            .filter(|candidate| candidate.calculation_id == calculation_id)
            .cloned()
            .collect()
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

    fn cargo_id(byte: u8) -> CargoId {
        CargoId::from_bytes([byte; 16]).expect("fixture cargo id")
    }

    fn point_id(byte: u8) -> PointId {
        PointId::from_bytes([byte; 16]).expect("fixture point id")
    }

    fn operator_id(byte: u8) -> OperatorId {
        OperatorId::from_bytes([byte; 16]).expect("fixture operator id")
    }

    fn working_route_id(byte: u8) -> WorkingRouteId {
        WorkingRouteId::from_bytes([byte; 16]).expect("fixture working route id")
    }

    fn reservation_id(byte: u8) -> ReservationId {
        ReservationId::from_bytes([byte; 16]).expect("fixture reservation id")
    }

    fn call_task_id(byte: u8) -> CallTaskId {
        CallTaskId::from_bytes([byte; 16]).expect("fixture call task id")
    }

    fn negotiated_terms_id(byte: u8) -> NegotiatedTermsId {
        NegotiatedTermsId::from_bytes([byte; 16]).expect("fixture negotiated terms id")
    }

    fn candidate_id(byte: u8) -> CandidateId {
        CandidateId::from_bytes([byte; 16]).expect("fixture candidate id")
    }

    fn calculation_id(byte: u8) -> CalculationId {
        CalculationId::from_bytes([byte; 16]).expect("fixture calculation id")
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
        assert!(
            result
                .check_reasons
                .contains(&NeedsCheckReason::RequestAvailabilityUnknown)
        );
        assert!(
            result
                .check_reasons
                .contains(&NeedsCheckReason::CargoPalletsUnknown)
        );
        assert!(
            result
                .check_reasons
                .contains(&NeedsCheckReason::AvailablePalletsUnknown)
        );
    }

    #[test]
    fn confirmed_request_is_not_offered_as_a_fresh_candidate() {
        let mut item = request(
            request_id(12),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        item.request.planning_assignment = PlanningAssignment::ConfirmedToTrip;

        let result = evaluate_request_candidate(
            &item,
            AvailableCapacity {
                payload_kg: Some(3_000),
                pallets: Some(8),
            },
        );

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert!(
            result
                .hard_reasons
                .contains(&HardRejectReason::AlreadyConfirmedToTrip)
        );
    }

    #[test]
    fn soft_reserved_and_calling_requests_stay_visible_as_needs_check() {
        for assignment in [
            PlanningAssignment::SoftReserved,
            PlanningAssignment::Calling,
        ] {
            let mut item = request(
                request_id(13),
                "Lviv",
                "Nuremberg",
                AvailabilityState::Active,
                true,
            );
            item.request.planning_assignment = assignment;

            let result = evaluate_request_candidate(
                &item,
                AvailableCapacity {
                    payload_kg: Some(3_000),
                    pallets: Some(8),
                },
            );

            assert_eq!(result.level, FeasibilityLevel::NeedsCheck);
            assert!(result.hard_reasons.is_empty());
            let expected = match assignment {
                PlanningAssignment::SoftReserved => NeedsCheckReason::SoftReserved,
                PlanningAssignment::Calling => NeedsCheckReason::Calling,
                _ => unreachable!("fixture covers only active planner use"),
            };
            assert!(result.check_reasons.contains(&expected));
        }
    }

    #[test]
    fn route_segment_rejects_unload_before_pickup() {
        let result = evaluate_route_segments(
            3,
            &[RouteCargoLeg {
                cargo_id: cargo_id(1),
                pickup_stop: 2,
                unload_stop: 1,
                weight_kg: Some(1_000),
                pallets: Some(2),
            }],
            AvailableCapacity {
                payload_kg: Some(3_500),
                pallets: Some(8),
            },
        );

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert_eq!(
            result.hard_reasons,
            vec![RouteHardRejectReason::InvalidCargoStopOrder {
                cargo_id: cargo_id(1),
                pickup_stop: 2,
                unload_stop: 1,
            }]
        );
    }

    #[test]
    fn route_capacity_is_checked_on_each_overlapping_segment() {
        let result = evaluate_route_segments(
            4,
            &[
                RouteCargoLeg {
                    cargo_id: cargo_id(2),
                    pickup_stop: 0,
                    unload_stop: 2,
                    weight_kg: Some(2_500),
                    pallets: Some(4),
                },
                RouteCargoLeg {
                    cargo_id: cargo_id(3),
                    pickup_stop: 1,
                    unload_stop: 3,
                    weight_kg: Some(2_000),
                    pallets: Some(3),
                },
            ],
            AvailableCapacity {
                payload_kg: Some(4_000),
                pallets: Some(6),
            },
        );

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert!(
            result
                .hard_reasons
                .contains(&RouteHardRejectReason::PayloadExceededOnSegment {
                    segment_index: 1,
                    load_kg: 4_500,
                    capacity_kg: 4_000,
                })
        );
        assert!(result.hard_reasons.contains(
            &RouteHardRejectReason::PalletCapacityExceededOnSegment {
                segment_index: 1,
                load_pallets: 7,
                capacity_pallets: 6,
            }
        ));
    }

    #[test]
    fn non_overlapping_cargo_is_not_rejected_by_trip_total() {
        let result = evaluate_route_segments(
            3,
            &[
                RouteCargoLeg {
                    cargo_id: cargo_id(4),
                    pickup_stop: 0,
                    unload_stop: 1,
                    weight_kg: Some(3_000),
                    pallets: Some(5),
                },
                RouteCargoLeg {
                    cargo_id: cargo_id(5),
                    pickup_stop: 1,
                    unload_stop: 2,
                    weight_kg: Some(3_000),
                    pallets: Some(5),
                },
            ],
            AvailableCapacity {
                payload_kg: Some(3_500),
                pallets: Some(6),
            },
        );

        assert_eq!(result.level, FeasibilityLevel::Feasible);
        assert!(result.hard_reasons.is_empty());
        assert!(result.check_reasons.is_empty());
    }

    #[test]
    fn unknown_segment_data_stays_needs_check() {
        let result = evaluate_route_segments(
            2,
            &[RouteCargoLeg {
                cargo_id: cargo_id(6),
                pickup_stop: 0,
                unload_stop: 1,
                weight_kg: None,
                pallets: None,
            }],
            AvailableCapacity {
                payload_kg: None,
                pallets: None,
            },
        );

        assert_eq!(result.level, FeasibilityLevel::NeedsCheck);
        assert!(result.hard_reasons.is_empty());
        assert!(
            result
                .check_reasons
                .contains(&RouteNeedsCheckReason::CargoWeightUnknown(cargo_id(6)))
        );
        assert!(
            result
                .check_reasons
                .contains(&RouteNeedsCheckReason::CargoPalletsUnknown(cargo_id(6)))
        );
        assert!(
            result
                .check_reasons
                .contains(&RouteNeedsCheckReason::AvailablePayloadUnknown)
        );
        assert!(
            result
                .check_reasons
                .contains(&RouteNeedsCheckReason::AvailablePalletsUnknown)
        );
    }

    #[test]
    fn route_time_is_feasible_when_known_windows_have_slack() {
        let base = 1_800_000_000_000_u64;
        let result = evaluate_route_time_windows(
            base,
            &[
                RouteTimePoint {
                    window: Some(ConfirmedTimeWindow {
                        earliest_unix_ms: base,
                        latest_unix_ms: base + 60 * 60 * 1_000,
                    }),
                    handling_time_ms: Some(15 * 60 * 1_000),
                },
                RouteTimePoint {
                    window: Some(ConfirmedTimeWindow {
                        earliest_unix_ms: base + 90 * 60 * 1_000,
                        latest_unix_ms: base + 150 * 60 * 1_000,
                    }),
                    handling_time_ms: Some(10 * 60 * 1_000),
                },
            ],
            &[RouteTimeSegment {
                road_time_ms: Some(60 * 60 * 1_000),
            }],
            10 * 60 * 1_000,
        );

        assert_eq!(result.level, FeasibilityLevel::Feasible);
        assert!(result.hard_reasons.is_empty());
        assert!(result.check_reasons.is_empty());
        assert_eq!(result.earliest_arrivals_unix_ms[1], base + 75 * 60 * 1_000);
        assert_eq!(
            result.earliest_departures_unix_ms[1],
            base + 100 * 60 * 1_000
        );
        assert_eq!(result.minimum_known_slack_ms, Some(60 * 60 * 1_000));
    }

    #[test]
    fn confirmed_window_missed_even_with_optimistic_timing_is_impossible() {
        let base = 1_800_000_000_000_u64;
        let result = evaluate_route_time_windows(
            base,
            &[
                RouteTimePoint {
                    window: None,
                    handling_time_ms: None,
                },
                RouteTimePoint {
                    window: Some(ConfirmedTimeWindow {
                        earliest_unix_ms: base,
                        latest_unix_ms: base + 50 * 60 * 1_000,
                    }),
                    handling_time_ms: Some(0),
                },
            ],
            &[RouteTimeSegment {
                road_time_ms: Some(60 * 60 * 1_000),
            }],
            10 * 60 * 1_000,
        );

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert!(
            result
                .hard_reasons
                .contains(&TimeHardRejectReason::WindowMissed {
                    point_index: 1,
                    earliest_possible_arrival_unix_ms: base + 60 * 60 * 1_000,
                    latest_unix_ms: base + 50 * 60 * 1_000,
                })
        );
    }

    #[test]
    fn unknown_road_or_handling_time_stays_needs_check() {
        let base = 1_800_000_000_000_u64;
        let result = evaluate_route_time_windows(
            base,
            &[
                RouteTimePoint {
                    window: Some(ConfirmedTimeWindow {
                        earliest_unix_ms: base,
                        latest_unix_ms: base + 30 * 60 * 1_000,
                    }),
                    handling_time_ms: None,
                },
                RouteTimePoint {
                    window: Some(ConfirmedTimeWindow {
                        earliest_unix_ms: base,
                        latest_unix_ms: base + 120 * 60 * 1_000,
                    }),
                    handling_time_ms: Some(0),
                },
            ],
            &[RouteTimeSegment { road_time_ms: None }],
            10 * 60 * 1_000,
        );

        assert_eq!(result.level, FeasibilityLevel::NeedsCheck);
        assert!(result.hard_reasons.is_empty());
        assert!(
            result
                .check_reasons
                .contains(&TimeNeedsCheckReason::HandlingTimeUnknown(0))
        );
        assert!(
            result
                .check_reasons
                .contains(&TimeNeedsCheckReason::RoadTimeUnknown(0))
        );
    }

    #[test]
    fn tight_but_reachable_window_remains_candidate_with_warning() {
        let base = 1_800_000_000_000_u64;
        let result = evaluate_route_time_windows(
            base,
            &[
                RouteTimePoint {
                    window: Some(ConfirmedTimeWindow {
                        earliest_unix_ms: base,
                        latest_unix_ms: base + 60 * 60 * 1_000,
                    }),
                    handling_time_ms: Some(0),
                },
                RouteTimePoint {
                    window: Some(ConfirmedTimeWindow {
                        earliest_unix_ms: base,
                        latest_unix_ms: base + 65 * 60 * 1_000,
                    }),
                    handling_time_ms: Some(0),
                },
            ],
            &[RouteTimeSegment {
                road_time_ms: Some(60 * 60 * 1_000),
            }],
            10 * 60 * 1_000,
        );

        assert_eq!(result.level, FeasibilityLevel::NeedsCheck);
        assert!(result.hard_reasons.is_empty());
        assert!(
            result
                .check_reasons
                .contains(&TimeNeedsCheckReason::TightTimeWindow {
                    point_index: 1,
                    slack_ms: 5 * 60 * 1_000,
                })
        );
    }

    #[test]
    fn driving_budget_violation_is_hard_reject() {
        let result = evaluate_driving_budget(
            Some(6 * 60 * 60 * 1_000),
            Some(5 * 60 * 60 * 1_000),
            30 * 60 * 1_000,
        );

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert_eq!(
            result.hard_reasons,
            vec![ComplianceHardRejectReason::DrivingBudgetExceeded {
                required_driving_ms: 6 * 60 * 60 * 1_000,
                available_driving_ms: 5 * 60 * 60 * 1_000,
            }]
        );
    }

    #[test]
    fn low_but_legal_driving_reserve_is_warning() {
        let result = evaluate_driving_budget(
            Some(4 * 60 * 60 * 1_000 + 45 * 60 * 1_000),
            Some(5 * 60 * 60 * 1_000),
            30 * 60 * 1_000,
        );

        assert_eq!(result.level, FeasibilityLevel::NeedsCheck);
        assert!(result.hard_reasons.is_empty());
        assert_eq!(result.known_driving_reserve_ms, Some(15 * 60 * 1_000));
        assert_eq!(
            result.check_reasons,
            vec![ComplianceNeedsCheckReason::LowDrivingReserve {
                reserve_ms: 15 * 60 * 1_000,
            }]
        );
    }

    #[test]
    fn sufficient_known_driving_budget_is_feasible() {
        let result = evaluate_driving_budget(
            Some(4 * 60 * 60 * 1_000),
            Some(6 * 60 * 60 * 1_000),
            30 * 60 * 1_000,
        );

        assert_eq!(result.level, FeasibilityLevel::Feasible);
        assert!(result.hard_reasons.is_empty());
        assert!(result.check_reasons.is_empty());
        assert_eq!(result.known_driving_reserve_ms, Some(2 * 60 * 60 * 1_000));
    }

    #[test]
    fn unknown_driving_budget_data_needs_check() {
        let result = evaluate_driving_budget(None, None, 30 * 60 * 1_000);

        assert_eq!(result.level, FeasibilityLevel::NeedsCheck);
        assert!(result.hard_reasons.is_empty());
        assert_eq!(
            result.check_reasons,
            vec![
                ComplianceNeedsCheckReason::RequiredDrivingUnknown,
                ComplianceNeedsCheckReason::AvailableDrivingUnknown,
            ]
        );
        assert_eq!(result.known_driving_reserve_ms, None);
    }

    #[test]
    fn recalculation_cannot_remove_confirmed_or_locked_point() {
        let current = [
            WorkingRoutePoint {
                point_id: point_id(1),
                locked: false,
                confirmed: true,
            },
            WorkingRoutePoint {
                point_id: point_id(2),
                locked: true,
                confirmed: false,
            },
            WorkingRoutePoint {
                point_id: point_id(3),
                locked: false,
                confirmed: false,
            },
        ];

        let result = validate_recalculation(&current, &[point_id(1), point_id(3)]);

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert_eq!(
            result.hard_reasons,
            vec![RecalculationHardRejectReason::ProtectedPointRemoved(
                point_id(2)
            )]
        );
    }

    #[test]
    fn recalculation_cannot_reverse_protected_point_order() {
        let current = [
            WorkingRoutePoint {
                point_id: point_id(4),
                locked: true,
                confirmed: false,
            },
            WorkingRoutePoint {
                point_id: point_id(5),
                locked: false,
                confirmed: false,
            },
            WorkingRoutePoint {
                point_id: point_id(6),
                locked: false,
                confirmed: true,
            },
        ];

        let result = validate_recalculation(&current, &[point_id(6), point_id(5), point_id(4)]);

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert_eq!(
            result.hard_reasons,
            vec![RecalculationHardRejectReason::ProtectedOrderChanged {
                earlier_point_id: point_id(4),
                later_point_id: point_id(6),
            }]
        );
    }

    #[test]
    fn recalculation_may_change_only_unprotected_part() {
        let current = [
            WorkingRoutePoint {
                point_id: point_id(7),
                locked: true,
                confirmed: false,
            },
            WorkingRoutePoint {
                point_id: point_id(8),
                locked: false,
                confirmed: false,
            },
            WorkingRoutePoint {
                point_id: point_id(9),
                locked: false,
                confirmed: true,
            },
            WorkingRoutePoint {
                point_id: point_id(10),
                locked: false,
                confirmed: false,
            },
        ];

        let result = validate_recalculation(
            &current,
            &[point_id(11), point_id(7), point_id(10), point_id(9)],
        );

        assert_eq!(result.level, FeasibilityLevel::Feasible);
        assert!(result.hard_reasons.is_empty());
    }

    #[test]
    fn recalculation_rejects_duplicate_point_ids() {
        let current = [WorkingRoutePoint {
            point_id: point_id(12),
            locked: false,
            confirmed: false,
        }];

        let result = validate_recalculation(&current, &[point_id(12), point_id(13), point_id(13)]);

        assert_eq!(result.level, FeasibilityLevel::Impossible);
        assert_eq!(
            result.hard_reasons,
            vec![RecalculationHardRejectReason::DuplicateProposedPoint(
                point_id(13)
            )]
        );
    }

    #[test]
    fn request_freshness_uses_organization_thresholds_without_changing_availability() {
        let mut item = request(
            request_id(14),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        item.request.source.observed_at_unix_ms = 1_800_000_000_000;

        let freshness = evaluate_request_freshness(&item, 1_800_000_030_000, 20_000, 60_000)
            .expect("freshness");

        assert_eq!(freshness.age_ms, 30_000);
        assert_eq!(freshness.level, FreshnessLevel::Warning);
        assert_eq!(item.request.availability, AvailabilityState::Active);
    }

    #[test]
    fn stale_request_becomes_high_risk_but_is_not_auto_removed() {
        let mut item = request(
            request_id(15),
            "Kyiv",
            "Berlin",
            AvailabilityState::NeedsCheck,
            true,
        );
        item.request.source.observed_at_unix_ms = 1_800_000_000_000;

        let freshness = evaluate_request_freshness(&item, 1_800_000_120_000, 30_000, 90_000)
            .expect("freshness");

        assert_eq!(freshness.level, FreshnessLevel::High);
        assert_eq!(item.request.availability, AvailabilityState::NeedsCheck);
    }

    #[test]
    fn request_freshness_rejects_invalid_threshold_order() {
        let item = request(
            request_id(16),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );

        assert_eq!(
            evaluate_request_freshness(
                &item,
                item.request.source.observed_at_unix_ms,
                60_000,
                30_000,
            ),
            Err(FreshnessError::InvalidThresholds)
        );
    }

    #[test]
    fn request_freshness_rejects_future_observation_time() {
        let mut item = request(
            request_id(17),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        item.request.source.observed_at_unix_ms = 1_800_000_100_000;

        assert_eq!(
            evaluate_request_freshness(&item, 1_800_000_000_000, 30_000, 60_000,),
            Err(FreshnessError::ObservationInFuture)
        );
    }

    #[test]
    fn soft_reservation_expiry_does_not_change_request_availability() {
        let request_id = request_id(18);
        let item = request(
            request_id,
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        let mut book = InMemoryReservationBook::default();
        book.reserve(SoftReservation {
            reservation_id: reservation_id(1),
            request_id,
            operator_id: operator_id(1),
            working_route_id: working_route_id(1),
            created_at_unix_ms: 1_800_000_000_000,
            expires_at_unix_ms: Some(1_800_000_060_000),
            released_at_unix_ms: None,
        })
        .expect("reservation");

        assert_eq!(
            book.active_for_request(request_id, 1_800_000_030_000).len(),
            1
        );
        assert!(
            book.active_for_request(request_id, 1_800_000_060_000)
                .is_empty()
        );
        assert_eq!(
            book.get(reservation_id(1))
                .expect("saved")
                .state_at(1_800_000_060_000),
            ReservationState::Expired
        );
        assert_eq!(item.request.availability, AvailabilityState::Active);
    }

    #[test]
    fn explicit_reservation_release_is_separate_from_source_availability() {
        let request_id = request_id(19);
        let mut service = PlannerService::new(InMemoryRequestRepository::default());
        service
            .add_request(request(
                request_id,
                "Kyiv",
                "Berlin",
                AvailabilityState::Active,
                true,
            ))
            .expect("request");
        service
            .set_planning_assignment(request_id, PlanningAssignment::SoftReserved)
            .expect("soft reserved");

        let mut book = InMemoryReservationBook::default();
        book.reserve(SoftReservation {
            reservation_id: reservation_id(2),
            request_id,
            operator_id: operator_id(2),
            working_route_id: working_route_id(2),
            created_at_unix_ms: 1_800_000_000_000,
            expires_at_unix_ms: None,
            released_at_unix_ms: None,
        })
        .expect("reservation");
        book.release(reservation_id(2), 1_800_000_030_000)
            .expect("release");
        service
            .set_planning_assignment(request_id, PlanningAssignment::Released)
            .expect("planner release");

        let saved = service.pool(&PoolFilter::all());
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].request.availability, AvailabilityState::Active);
        assert_eq!(
            saved[0].request.planning_assignment,
            PlanningAssignment::Released
        );
        assert_eq!(
            book.get(reservation_id(2))
                .expect("saved")
                .state_at(1_800_000_030_000),
            ReservationState::Released
        );
    }

    #[test]
    fn reservation_book_rejects_duplicate_id_and_invalid_release() {
        let request_id = request_id(20);
        let reservation = SoftReservation {
            reservation_id: reservation_id(3),
            request_id,
            operator_id: operator_id(3),
            working_route_id: working_route_id(3),
            created_at_unix_ms: 1_800_000_100_000,
            expires_at_unix_ms: None,
            released_at_unix_ms: None,
        };
        let mut book = InMemoryReservationBook::default();
        book.reserve(reservation.clone())
            .expect("first reservation");

        assert_eq!(
            book.reserve(reservation),
            Err(ReservationError::DuplicateReservation)
        );
        assert_eq!(
            book.release(reservation_id(3), 1_800_000_090_000),
            Err(ReservationError::ReleaseBeforeCreation)
        );
        assert_eq!(
            book.release(reservation_id(99), 1_800_000_120_000),
            Err(ReservationError::ReservationNotFound)
        );
    }

    #[test]
    fn call_history_keeps_all_attempts_and_exposes_latest_separately() {
        let request_id = request_id(21);
        let mut history = InMemoryCallHistory::default();
        history
            .append(CompletedCallAttempt {
                call_task_id: call_task_id(1),
                request_id,
                working_route_id: working_route_id(4),
                operator_id: operator_id(4),
                created_at_unix_ms: 1_800_000_000_000,
                started_at_unix_ms: 1_800_000_010_000,
                finished_at_unix_ms: 1_800_000_020_000,
                result: CallResult::NoAnswer,
            })
            .expect("first call");
        history
            .append(CompletedCallAttempt {
                call_task_id: call_task_id(2),
                request_id,
                working_route_id: working_route_id(4),
                operator_id: operator_id(4),
                created_at_unix_ms: 1_800_000_030_000,
                started_at_unix_ms: 1_800_000_040_000,
                finished_at_unix_ms: 1_800_000_050_000,
                result: CallResult::Confirmed,
            })
            .expect("second call");

        let attempts = history.for_request(request_id);
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].result, CallResult::NoAnswer);
        assert_eq!(attempts[1].result, CallResult::Confirmed);
        assert_eq!(
            history
                .latest_for_request(request_id)
                .expect("latest")
                .result,
            CallResult::Confirmed
        );
    }

    #[test]
    fn call_history_rejects_duplicate_task_id_and_invalid_timeline() {
        let request_id = request_id(22);
        let attempt = CompletedCallAttempt {
            call_task_id: call_task_id(3),
            request_id,
            working_route_id: working_route_id(5),
            operator_id: operator_id(5),
            created_at_unix_ms: 1_800_000_000_000,
            started_at_unix_ms: 1_800_000_010_000,
            finished_at_unix_ms: 1_800_000_020_000,
            result: CallResult::Callback,
        };
        let mut history = InMemoryCallHistory::default();
        history.append(attempt.clone()).expect("first call");

        assert_eq!(
            history.append(attempt),
            Err(CallHistoryError::DuplicateCallTask)
        );
        assert_eq!(
            history.append(CompletedCallAttempt {
                call_task_id: call_task_id(4),
                request_id,
                working_route_id: working_route_id(5),
                operator_id: operator_id(5),
                created_at_unix_ms: 1_800_000_030_000,
                started_at_unix_ms: 1_800_000_020_000,
                finished_at_unix_ms: 1_800_000_040_000,
                result: CallResult::Other,
            }),
            Err(CallHistoryError::InvalidTimeline)
        );
    }

    #[test]
    fn call_result_does_not_silently_change_request_state() {
        let item = request(
            request_id(23),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        let mut history = InMemoryCallHistory::default();
        history
            .append(CompletedCallAttempt {
                call_task_id: call_task_id(5),
                request_id: item.request.request_id,
                working_route_id: working_route_id(6),
                operator_id: operator_id(6),
                created_at_unix_ms: 1_800_000_000_000,
                started_at_unix_ms: 1_800_000_010_000,
                finished_at_unix_ms: 1_800_000_020_000,
                result: CallResult::LostTaken,
            })
            .expect("call");

        assert_eq!(history.for_request(item.request.request_id).len(), 1);
        assert_eq!(item.request.availability, AvailabilityState::Active);
        assert_eq!(item.request.planning_assignment, PlanningAssignment::Free);
    }

    #[test]
    fn negotiated_terms_override_planning_view_without_overwriting_source_request() {
        let item = request(
            request_id(24),
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        let source_cargo = item.request.cargo.clone();
        let terms = NegotiatedTerms {
            negotiated_terms_id: negotiated_terms_id(1),
            request_id: item.request.request_id,
            working_route_id: working_route_id(7),
            operator_id: operator_id(7),
            confirmed_by_call_task_id: call_task_id(6),
            confirmed_at_unix_ms: 1_800_000_060_000,
            cargo_weight_kg: Some(3_200),
            cargo_pallets: Some(8),
            pickup_window: None,
            delivery_window: None,
        };

        let effective = effective_cargo_for_planning(&item, Some(&terms));

        assert_eq!(effective.weight_kg, Some(3_200));
        assert_eq!(effective.pallets, Some(8));
        assert_eq!(item.request.cargo, source_cargo);
    }

    #[test]
    fn negotiated_terms_history_preserves_previous_confirmations() {
        let request_id = request_id(25);
        let mut history = InMemoryNegotiatedTermsHistory::default();
        history
            .append(NegotiatedTerms {
                negotiated_terms_id: negotiated_terms_id(2),
                request_id,
                working_route_id: working_route_id(8),
                operator_id: operator_id(8),
                confirmed_by_call_task_id: call_task_id(7),
                confirmed_at_unix_ms: 1_800_000_060_000,
                cargo_weight_kg: Some(2_800),
                cargo_pallets: None,
                pickup_window: None,
                delivery_window: None,
            })
            .expect("first terms");
        history
            .append(NegotiatedTerms {
                negotiated_terms_id: negotiated_terms_id(3),
                request_id,
                working_route_id: working_route_id(8),
                operator_id: operator_id(8),
                confirmed_by_call_task_id: call_task_id(8),
                confirmed_at_unix_ms: 1_800_000_120_000,
                cargo_weight_kg: Some(3_000),
                cargo_pallets: Some(7),
                pickup_window: Some(ConfirmedTimeWindow {
                    earliest_unix_ms: 1_800_003_600_000,
                    latest_unix_ms: 1_800_007_200_000,
                }),
                delivery_window: None,
            })
            .expect("second terms");

        let saved = history.for_request(request_id);
        assert_eq!(saved.len(), 2);
        assert_eq!(saved[0].cargo_weight_kg, Some(2_800));
        assert_eq!(saved[1].cargo_weight_kg, Some(3_000));
        assert_eq!(
            history
                .latest_for_request(request_id)
                .expect("latest")
                .negotiated_terms_id,
            negotiated_terms_id(3)
        );
    }

    #[test]
    fn negotiated_terms_validate_confirmed_values_and_windows() {
        let request_id = request_id(26);
        let base = NegotiatedTerms {
            negotiated_terms_id: negotiated_terms_id(4),
            request_id,
            working_route_id: working_route_id(9),
            operator_id: operator_id(9),
            confirmed_by_call_task_id: call_task_id(9),
            confirmed_at_unix_ms: 1_800_000_000_000,
            cargo_weight_kg: None,
            cargo_pallets: None,
            pickup_window: None,
            delivery_window: None,
        };
        let mut history = InMemoryNegotiatedTermsHistory::default();

        assert_eq!(
            history.append(base.clone()),
            Err(NegotiatedTermsError::NoConfirmedChanges)
        );

        let mut invalid_weight = base.clone();
        invalid_weight.negotiated_terms_id = negotiated_terms_id(5);
        invalid_weight.cargo_weight_kg = Some(0);
        assert_eq!(
            history.append(invalid_weight),
            Err(NegotiatedTermsError::WeightMustBePositive)
        );

        let mut invalid_window = base;
        invalid_window.negotiated_terms_id = negotiated_terms_id(6);
        invalid_window.pickup_window = Some(ConfirmedTimeWindow {
            earliest_unix_ms: 1_800_000_100_000,
            latest_unix_ms: 1_800_000_050_000,
        });
        assert_eq!(
            history.append(invalid_window),
            Err(NegotiatedTermsError::InvalidPickupWindow)
        );
    }

    #[test]
    fn candidate_snapshot_keeps_request_revision_after_request_refresh() {
        let request_id = request_id(27);
        let mut service = PlannerService::new(InMemoryRequestRepository::default());
        service
            .add_request(request(
                request_id,
                "Lviv",
                "Nuremberg",
                AvailabilityState::Active,
                true,
            ))
            .expect("request");

        let mut candidates = InMemoryCandidateStore::default();
        candidates
            .append(RouteCandidateSnapshot {
                candidate_id: candidate_id(1),
                calculation_id: calculation_id(1),
                generated_at_unix_ms: 1_800_000_010_000,
                ordered_point_ids: vec![point_id(20), point_id(21)],
                request_basis: vec![CandidateRequestBasis {
                    request_id,
                    revision: RequestRevision::new(1).expect("revision"),
                }],
            })
            .expect("candidate");

        let mut refreshed = request(
            request_id,
            "Lviv",
            "Nuremberg",
            AvailabilityState::Active,
            true,
        );
        refreshed.request.revision = RequestRevision::new(2).expect("revision");
        refreshed.request.cargo =
            CargoSummary::new("changed source cargo", Some(3_100), Some(8)).expect("cargo");
        service.refresh_request(refreshed).expect("refresh");

        let snapshot = candidates.get(candidate_id(1)).expect("snapshot");
        assert_eq!(snapshot.request_basis[0].revision.get(), 1);
        assert_eq!(snapshot.ordered_point_ids, vec![point_id(20), point_id(21)]);
        assert_eq!(
            service.pool(&PoolFilter::all())[0].request.revision.get(),
            2
        );
    }

    #[test]
    fn one_calculation_may_keep_multiple_immutable_candidates() {
        let calculation_id = calculation_id(2);
        let mut candidates = InMemoryCandidateStore::default();

        for (candidate_byte, first_point, second_point) in [(2, 22, 23), (3, 22, 24)] {
            candidates
                .append(RouteCandidateSnapshot {
                    candidate_id: candidate_id(candidate_byte),
                    calculation_id,
                    generated_at_unix_ms: 1_800_000_020_000,
                    ordered_point_ids: vec![point_id(first_point), point_id(second_point)],
                    request_basis: Vec::new(),
                })
                .expect("candidate");
        }

        let saved = candidates.for_calculation(calculation_id);
        assert_eq!(saved.len(), 2);
        assert_ne!(saved[0].candidate_id, saved[1].candidate_id);
    }

    #[test]
    fn candidate_store_rejects_duplicate_identity_and_ambiguous_basis() {
        let calculation_id = calculation_id(3);
        let base = RouteCandidateSnapshot {
            candidate_id: candidate_id(4),
            calculation_id,
            generated_at_unix_ms: 1_800_000_030_000,
            ordered_point_ids: vec![point_id(25), point_id(26)],
            request_basis: vec![CandidateRequestBasis {
                request_id: request_id(28),
                revision: RequestRevision::new(1).expect("revision"),
            }],
        };
        let mut candidates = InMemoryCandidateStore::default();
        candidates.append(base.clone()).expect("first candidate");

        assert_eq!(
            candidates.append(base),
            Err(CandidateSnapshotError::DuplicateCandidate)
        );

        assert_eq!(
            candidates.append(RouteCandidateSnapshot {
                candidate_id: candidate_id(5),
                calculation_id,
                generated_at_unix_ms: 1_800_000_040_000,
                ordered_point_ids: vec![point_id(27), point_id(27)],
                request_basis: Vec::new(),
            }),
            Err(CandidateSnapshotError::DuplicatePoint)
        );

        let request_id = request_id(29);
        assert_eq!(
            candidates.append(RouteCandidateSnapshot {
                candidate_id: candidate_id(6),
                calculation_id,
                generated_at_unix_ms: 1_800_000_050_000,
                ordered_point_ids: vec![point_id(28), point_id(29)],
                request_basis: vec![
                    CandidateRequestBasis {
                        request_id,
                        revision: RequestRevision::new(1).expect("revision"),
                    },
                    CandidateRequestBasis {
                        request_id,
                        revision: RequestRevision::new(2).expect("revision"),
                    },
                ],
            }),
            Err(CandidateSnapshotError::DuplicateRequestBasis)
        );
    }

    #[test]
    fn selected_candidate_becomes_independent_working_route_revision_one() {
        let mut candidates = InMemoryCandidateStore::default();
        candidates
            .append(RouteCandidateSnapshot {
                candidate_id: candidate_id(7),
                calculation_id: calculation_id(4),
                generated_at_unix_ms: 1_800_000_000_000,
                ordered_point_ids: vec![point_id(30), point_id(31)],
                request_basis: Vec::new(),
            })
            .expect("candidate");

        let mut route =
            working_route_from_candidate(working_route_id(10), candidate_id(7), &candidates)
                .expect("working route");
        route.points[0].locked = true;

        let candidate = candidates.get(candidate_id(7)).expect("candidate");
        assert_eq!(route.origin, WorkingRouteOrigin::Candidate(candidate_id(7)));
        assert_eq!(route.current_revision, 1);
        assert_eq!(route.state, WorkingRouteState::Selected);
        assert!(route.points[0].locked);
        assert_eq!(
            candidate.ordered_point_ids,
            vec![point_id(30), point_id(31)]
        );
    }

    #[test]
    fn manual_working_route_needs_no_candidate_and_starts_as_draft() {
        let route = manual_working_route(working_route_id(11), &[point_id(32), point_id(33)])
            .expect("manual route");

        assert_eq!(route.origin, WorkingRouteOrigin::Manual);
        assert_eq!(route.current_revision, 1);
        assert_eq!(route.state, WorkingRouteState::Draft);
        assert_eq!(
            route
                .points
                .iter()
                .map(|point| point.point_id)
                .collect::<Vec<_>>(),
            vec![point_id(32), point_id(33)]
        );
    }

    #[test]
    fn working_route_creation_rejects_missing_candidate_or_duplicate_points() {
        let candidates = InMemoryCandidateStore::default();
        assert_eq!(
            working_route_from_candidate(working_route_id(12), candidate_id(8), &candidates,),
            Err(WorkingRouteError::CandidateNotFound)
        );
        assert_eq!(
            manual_working_route(working_route_id(13), &[point_id(34), point_id(34)],),
            Err(WorkingRouteError::DuplicatePoint)
        );
    }

    #[test]
    fn safe_recalculation_creates_new_revision_and_preserves_protected_flags() {
        let mut route = manual_working_route(
            working_route_id(14),
            &[point_id(35), point_id(36), point_id(37)],
        )
        .expect("route");
        route.points[0].locked = true;
        route.points[2].confirmed = true;

        let recalculated = recalculate_working_route(
            &route,
            &[point_id(35), point_id(38), point_id(37)],
        )
        .expect("recalculation");

        assert_eq!(route.current_revision, 1);
        assert_eq!(recalculated.current_revision, 2);
        assert_eq!(recalculated.state, WorkingRouteState::Replanning);
        assert!(recalculated.points[0].locked);
        assert!(!recalculated.points[1].locked);
        assert!(!recalculated.points[1].confirmed);
        assert!(recalculated.points[2].confirmed);
    }

    #[test]
    fn rejected_recalculation_leaves_original_working_route_unchanged() {
        let mut route = manual_working_route(
            working_route_id(15),
            &[point_id(39), point_id(40), point_id(41)],
        )
        .expect("route");
        route.points[0].locked = true;
        route.points[2].confirmed = true;
        let before = route.clone();

        let result = recalculate_working_route(
            &route,
            &[point_id(41), point_id(40), point_id(39)],
        );

        assert!(matches!(
            result,
            Err(WorkingRouteUpdateError::RecalculationRejected(_))
        ));
        assert_eq!(route, before);
    }

    #[test]
    fn working_route_revision_overflow_is_explicit() {
        let mut route =
            manual_working_route(working_route_id(16), &[point_id(42), point_id(43)])
                .expect("route");
        route.current_revision = u64::MAX;

        assert_eq!(
            recalculate_working_route(&route, &[point_id(42), point_id(43)]),
            Err(WorkingRouteUpdateError::RevisionOverflow)
        );
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
