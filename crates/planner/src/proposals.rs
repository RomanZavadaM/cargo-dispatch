//! Route variants for a set of orders: the order of stops, not the road itself.
//!
//! A variant visits every pickup before its delivery and never exceeds the
//! stated capacity. Variants are ranked by straight-line distance, which is
//! local arithmetic; the road is calculated afterwards by the router. No
//! distance matrix is used (ADR-0020).

use crate::{AvailableCapacity, FeasibilityLevel, RouteCargoLeg, evaluate_route_segments};
use cargo_dispatch_domain::CargoId;
use cargo_dispatch_geo::GeoPoint;

/// Up to this many orders every possible order of stops is examined.
pub const EXACT_ORDER_LIMIT: usize = 4;
/// More orders than this are not planned in one calculation.
pub const MAX_ORDERS: usize = 12;
/// How many variants are offered at most.
pub const MAX_PROPOSALS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProposalOrder {
    pub pickup: GeoPoint,
    pub delivery: GeoPoint,
    pub weight_kg: Option<u32>,
    pub pallets: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProposalSettings {
    /// Where the vehicle is before the first pickup, when the operator knows it.
    pub start: Option<GeoPoint>,
    /// Whether the vehicle has to come back to `start` after the last delivery.
    pub return_to_start: bool,
    pub capacity: AvailableCapacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopAction {
    Pickup,
    Delivery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposalStop {
    pub order: usize,
    pub action: StopAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalKind {
    /// The shortest order of stops found.
    Shortest,
    /// Each cargo is delivered before the next one is loaded.
    OneAtATime,
    /// Everything is loaded first, then delivered.
    LoadAllFirst,
    /// Another short order of stops.
    Alternative,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    pub kind: ProposalKind,
    pub stops: Vec<ProposalStop>,
    /// Straight-line length including the approach and, if asked, the return.
    pub straight_km: f64,
    /// The part of `straight_km` driven with no cargo on board.
    pub empty_km: f64,
    /// Largest known weight on board between two stops.
    pub peak_weight_kg: u64,
    pub peak_pallets: u32,
    /// Some weight, pallet count or capacity is unknown.
    pub needs_check: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProposalSet {
    pub proposals: Vec<Proposal>,
    /// Orders that cannot be carried even alone: their position in the input.
    pub too_large: Vec<usize>,
    /// Every order of stops was examined; otherwise a heuristic chose them.
    pub exhaustive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalError {
    NoOrders,
    TooManyOrders,
}

/// Great-circle distance in kilometres.
#[must_use]
pub fn straight_line_km(a: GeoPoint, b: GeoPoint) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    let (lat_a, lat_b) = (a.lat().to_radians(), b.lat().to_radians());
    let half_lat = (lat_b - lat_a) / 2.0;
    let half_lon = (b.lon() - a.lon()).to_radians() / 2.0;
    let h = half_lat.sin().powi(2) + lat_a.cos() * lat_b.cos() * half_lon.sin().powi(2);
    2.0 * EARTH_RADIUS_KM * h.sqrt().min(1.0).asin()
}

fn cargo_id(order: usize) -> CargoId {
    let mut bytes = [0_u8; 16];
    bytes[15] = u8::try_from(order + 1).unwrap_or(u8::MAX);
    CargoId::from_bytes(bytes).expect("a non-zero identifier")
}

fn measure(
    orders: &[ProposalOrder],
    settings: ProposalSettings,
    stops: &[ProposalStop],
    kind: ProposalKind,
) -> Option<Proposal> {
    let legs: Vec<RouteCargoLeg> = (0..orders.len())
        .filter_map(|order| {
            let position = |action| {
                stops
                    .iter()
                    .position(|stop| stop.order == order && stop.action == action)
            };
            Some(RouteCargoLeg {
                cargo_id: cargo_id(order),
                pickup_stop: position(StopAction::Pickup)?,
                unload_stop: position(StopAction::Delivery)?,
                weight_kg: orders[order].weight_kg,
                pallets: orders[order].pallets,
            })
        })
        .collect();
    let feasibility = evaluate_route_segments(stops.len(), &legs, settings.capacity);
    if feasibility.level == FeasibilityLevel::Impossible {
        return None;
    }

    let place = |stop: &ProposalStop| match stop.action {
        StopAction::Pickup => orders[stop.order].pickup,
        StopAction::Delivery => orders[stop.order].delivery,
    };
    let mut straight_km = 0.0;
    let mut empty_km = 0.0;
    if let (Some(start), Some(first)) = (settings.start, stops.first()) {
        let approach = straight_line_km(start, place(first));
        straight_km += approach;
        empty_km += approach;
    }
    let mut on_board = 0_usize;
    let mut weight = 0_u64;
    let mut pallets = 0_u32;
    let mut peak_weight_kg = 0_u64;
    let mut peak_pallets = 0_u32;
    for (index, stop) in stops.iter().enumerate() {
        let order = orders[stop.order];
        let cargo_weight = u64::from(order.weight_kg.unwrap_or(0));
        let cargo_pallets = u32::from(order.pallets.unwrap_or(0));
        match stop.action {
            StopAction::Pickup => {
                on_board += 1;
                weight += cargo_weight;
                pallets += cargo_pallets;
            }
            StopAction::Delivery => {
                on_board = on_board.saturating_sub(1);
                weight = weight.saturating_sub(cargo_weight);
                pallets = pallets.saturating_sub(cargo_pallets);
            }
        }
        peak_weight_kg = peak_weight_kg.max(weight);
        peak_pallets = peak_pallets.max(pallets);
        if let Some(next) = stops.get(index + 1) {
            let distance = straight_line_km(place(stop), place(next));
            straight_km += distance;
            if on_board == 0 {
                empty_km += distance;
            }
        }
    }
    if settings.return_to_start
        && let (Some(start), Some(last)) = (settings.start, stops.last())
    {
        let back = straight_line_km(place(last), start);
        straight_km += back;
        empty_km += back;
    }

    Some(Proposal {
        kind,
        stops: stops.to_vec(),
        straight_km,
        empty_km,
        peak_weight_kg,
        peak_pallets,
        needs_check: feasibility.level == FeasibilityLevel::NeedsCheck,
    })
}

/// Every order of stops in which each pickup precedes its delivery.
fn every_order(count: usize, prefix: &mut Vec<ProposalStop>, found: &mut Vec<Vec<ProposalStop>>) {
    if prefix.len() == count * 2 {
        found.push(prefix.clone());
        return;
    }
    for order in 0..count {
        let loaded = prefix.iter().any(|stop| stop.order == order);
        let delivered = prefix
            .iter()
            .any(|stop| stop.order == order && stop.action == StopAction::Delivery);
        if delivered {
            continue;
        }
        let action = if loaded {
            StopAction::Delivery
        } else {
            StopAction::Pickup
        };
        prefix.push(ProposalStop { order, action });
        every_order(count, prefix, found);
        prefix.pop();
    }
}

/// Builds a sequence by adding orders one by one where they lengthen it least.
fn insert_orders(
    orders: &[ProposalOrder],
    settings: ProposalSettings,
    sequence: &[usize],
) -> Option<Vec<ProposalStop>> {
    let mut stops: Vec<ProposalStop> = Vec::new();
    for &order in sequence {
        let mut best: Option<(f64, Vec<ProposalStop>)> = None;
        for pickup_at in 0..=stops.len() {
            for delivery_at in pickup_at + 1..=stops.len() + 1 {
                let mut candidate = stops.clone();
                candidate.insert(
                    pickup_at,
                    ProposalStop {
                        order,
                        action: StopAction::Pickup,
                    },
                );
                candidate.insert(
                    delivery_at,
                    ProposalStop {
                        order,
                        action: StopAction::Delivery,
                    },
                );
                let Some(measured) =
                    measure(orders, settings, &candidate, ProposalKind::Alternative)
                else {
                    continue;
                };
                if best
                    .as_ref()
                    .is_none_or(|(length, _)| measured.straight_km < *length)
                {
                    best = Some((measured.straight_km, candidate));
                }
            }
        }
        stops = best?.1;
    }
    Some(stops)
}

fn heuristic_orders(
    orders: &[ProposalOrder],
    settings: ProposalSettings,
    usable: &[usize],
) -> Vec<Vec<ProposalStop>> {
    let mut found = Vec::new();
    for turn in 0..usable.len() {
        let mut sequence = usable.to_vec();
        sequence.rotate_left(turn);
        if let Some(stops) = insert_orders(orders, settings, &sequence) {
            found.push(stops);
        }
        // The same orders, one at a time.
        found.push(
            sequence
                .iter()
                .flat_map(|&order| {
                    [StopAction::Pickup, StopAction::Delivery]
                        .map(|action| ProposalStop { order, action })
                })
                .collect(),
        );
    }
    found
}

fn one_at_a_time(stops: &[ProposalStop]) -> bool {
    stops
        .chunks(2)
        .all(|pair| pair.len() == 2 && pair[0].order == pair[1].order)
}

fn load_all_first(stops: &[ProposalStop]) -> bool {
    stops[..stops.len() / 2]
        .iter()
        .all(|stop| stop.action == StopAction::Pickup)
}

/// Offers up to [`MAX_PROPOSALS`] different variants for the given orders.
pub fn propose_routes(
    orders: &[ProposalOrder],
    settings: ProposalSettings,
) -> Result<ProposalSet, ProposalError> {
    if orders.is_empty() {
        return Err(ProposalError::NoOrders);
    }
    if orders.len() > MAX_ORDERS {
        return Err(ProposalError::TooManyOrders);
    }
    let capacity = settings.capacity;
    let fits = |order: &ProposalOrder| {
        let weight = order.weight_kg.zip(capacity.payload_kg);
        let pallets = order.pallets.zip(capacity.pallets);
        weight.is_none_or(|(cargo, limit)| cargo <= limit)
            && pallets.is_none_or(|(cargo, limit)| cargo <= limit)
    };
    let usable: Vec<usize> = (0..orders.len())
        .filter(|&order| fits(&orders[order]))
        .collect();
    let too_large: Vec<usize> = (0..orders.len())
        .filter(|order| !usable.contains(order))
        .collect();
    let exhaustive = usable.len() <= EXACT_ORDER_LIMIT;

    let sequences = if usable.is_empty() {
        Vec::new()
    } else if exhaustive {
        let mut found = Vec::new();
        every_order(usable.len(), &mut Vec::new(), &mut found);
        for stops in &mut found {
            for stop in stops {
                stop.order = usable[stop.order];
            }
        }
        found
    } else {
        heuristic_orders(orders, settings, &usable)
    };

    let mut measured: Vec<Proposal> = sequences
        .iter()
        .filter_map(|stops| measure(orders, settings, stops, ProposalKind::Alternative))
        .collect();
    measured.sort_by(|a, b| {
        a.straight_km
            .total_cmp(&b.straight_km)
            .then(a.empty_km.total_cmp(&b.empty_km))
    });

    let mut proposals: Vec<Proposal> = Vec::new();
    let mut offer = |kind: ProposalKind, candidate: Option<&Proposal>| {
        if proposals.len() < MAX_PROPOSALS
            && let Some(candidate) = candidate
            && proposals.iter().all(|known| known.stops != candidate.stops)
        {
            let mut chosen = candidate.clone();
            chosen.kind = kind;
            proposals.push(chosen);
        }
    };
    offer(ProposalKind::Shortest, measured.first());
    offer(
        ProposalKind::OneAtATime,
        measured.iter().find(|item| one_at_a_time(&item.stops)),
    );
    offer(
        ProposalKind::LoadAllFirst,
        measured.iter().find(|item| load_all_first(&item.stops)),
    );
    for candidate in &measured {
        offer(ProposalKind::Alternative, Some(candidate));
    }

    Ok(ProposalSet {
        proposals,
        too_large,
        exhaustive,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(lon: f64) -> GeoPoint {
        GeoPoint::new(50.0, lon).expect("valid synthetic point")
    }

    fn order(pickup: f64, delivery: f64, weight_kg: u32) -> ProposalOrder {
        ProposalOrder {
            pickup: at(pickup),
            delivery: at(delivery),
            weight_kg: Some(weight_kg),
            pallets: Some(1),
        }
    }

    fn settings(payload_kg: u32) -> ProposalSettings {
        ProposalSettings {
            start: None,
            return_to_start: false,
            capacity: AvailableCapacity {
                payload_kg: Some(payload_kg),
                pallets: Some(33),
            },
        }
    }

    fn names(proposal: &Proposal) -> Vec<(usize, StopAction)> {
        proposal
            .stops
            .iter()
            .map(|stop| (stop.order, stop.action))
            .collect()
    }

    #[test]
    fn nested_cargo_is_picked_up_on_the_way() {
        use StopAction::{Delivery, Pickup};
        let set = propose_routes(
            &[order(30.0, 33.0, 5_000), order(31.0, 32.0, 5_000)],
            settings(20_000),
        )
        .expect("two orders");
        assert!(set.exhaustive);
        let best = &set.proposals[0];
        assert_eq!(best.kind, ProposalKind::Shortest);
        assert_eq!(
            names(best),
            [(0, Pickup), (1, Pickup), (1, Delivery), (0, Delivery)]
        );
        assert!(best.empty_km < 0.001);
        assert_eq!(best.peak_weight_kg, 10_000);
        assert!(set.proposals.len() > 1);
        assert!(set.proposals[1].straight_km >= best.straight_km);
    }

    #[test]
    fn capacity_forbids_carrying_both_at_once() {
        let set = propose_routes(
            &[order(30.0, 33.0, 15_000), order(31.0, 32.0, 15_000)],
            settings(20_000),
        )
        .expect("two orders");
        assert!(!set.proposals.is_empty());
        for proposal in &set.proposals {
            assert!(proposal.peak_weight_kg <= 20_000);
            assert!(one_at_a_time(&proposal.stops));
        }
    }

    #[test]
    fn an_order_heavier_than_the_vehicle_is_set_aside() {
        let set = propose_routes(
            &[order(30.0, 31.0, 25_000), order(31.0, 32.0, 1_000)],
            settings(20_000),
        )
        .expect("two orders");
        assert_eq!(set.too_large, [0]);
        assert!(set.proposals[0].stops.iter().all(|stop| stop.order == 1));
    }

    #[test]
    fn approach_and_return_count_as_empty_distance() {
        let mut with_base = settings(20_000);
        with_base.start = Some(at(29.0));
        with_base.return_to_start = true;
        let set = propose_routes(&[order(30.0, 31.0, 1_000)], with_base).expect("one order");
        let only = &set.proposals[0];
        let leg = straight_line_km(at(29.0), at(30.0));
        assert!((only.empty_km - 3.0 * leg).abs() < 0.5);
        assert!((only.straight_km - 4.0 * leg).abs() < 0.5);
    }

    #[test]
    fn unknown_weight_asks_for_a_check_and_many_orders_use_the_heuristic() {
        let mut unknown = order(30.0, 31.0, 1);
        unknown.weight_kg = None;
        let set = propose_routes(&[unknown], settings(20_000)).expect("one order");
        assert!(set.proposals[0].needs_check);

        let many: Vec<_> = (0..6)
            .map(|index| {
                let from = 24.0 + f64::from(index);
                order(from, from + 0.5, 1_000)
            })
            .collect();
        let set = propose_routes(&many, settings(20_000)).expect("six orders");
        assert!(!set.exhaustive);
        assert_eq!(set.proposals[0].stops.len(), 12);
    }

    #[test]
    fn empty_and_oversized_requests_are_refused() {
        assert_eq!(
            propose_routes(&[], settings(1)),
            Err(ProposalError::NoOrders)
        );
        let many = vec![order(30.0, 31.0, 1); MAX_ORDERS + 1];
        assert_eq!(
            propose_routes(&many, settings(20_000)),
            Err(ProposalError::TooManyOrders)
        );
    }
}
