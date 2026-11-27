//! The planning input file: orders and the vehicle they are planned for.
//!
//! One document for a person and for an AI that reads exchange screenshots.
//! A value that is not known is left out; it is never written as zero.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const INPUT_SCHEMA: &str = "cargo-dispatch.planning-input";
pub const INPUT_SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Money {
    pub amount: f64,
    /// Three letters, for example UAH or EUR.
    pub currency: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InputPlace {
    pub place: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
    /// `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`, optionally with `Z` or `+02:00`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub earliest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InputCargo {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_kg: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pallets: Option<u16>,
    /// Names of the values above that are estimates, not facts from the source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub estimated: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InputOrder {
    /// Unique within the file.
    pub id: String,
    /// Where the order came from: an exchange, a phone call, a message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_order_id: Option<String>,
    pub cargo: InputCargo,
    pub pickup: InputPlace,
    pub delivery: InputPlace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<Money>,
    /// Stays in Planner; it is not copied into the trip package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InputVehicle {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub vehicle_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_kg: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pallets: Option<u16>,
    /// Where the vehicle is before the first pickup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<InputPlace>,
    #[serde(default)]
    pub return_to_start: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_km: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlanningInput {
    pub schema: String,
    pub schema_version: String,
    #[serde(default)]
    pub vehicle: InputVehicle,
    #[serde(default)]
    pub orders: Vec<InputOrder>,
}

impl Default for PlanningInput {
    fn default() -> Self {
        Self {
            schema: INPUT_SCHEMA.to_owned(),
            schema_version: INPUT_SCHEMA_VERSION.to_owned(),
            vehicle: InputVehicle::default(),
            orders: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputProblemKind {
    Schema,
    Required,
    Duplicate,
    NotPositive,
    Coordinate,
    CoordinatePair,
    DateFormat,
    DateOrder,
    Currency,
    EstimatedName,
}

/// One thing to correct, named by its place in the document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputProblem {
    /// For example `orders[2].pickup.lat`.
    pub field: String,
    pub kind: InputProblemKind,
}

/// Minutes since 1970 for the two accepted date forms; `None` when malformed.
#[must_use]
pub fn parse_moment(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if !text.is_ascii() || bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let number = |from: usize, to: usize| -> Option<i64> {
        let part = text.get(from..to)?;
        part.bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| part.parse().ok())?
    };
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let length = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=length).contains(&day) {
        return None;
    }
    // Days from the civil date (the proleptic Gregorian calendar).
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let of_era = shifted - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let days = era * 146_097 + of_era * 365 + of_era / 4 - of_era / 100 + of_year - 719_468;
    let mut minutes = days * 1440;
    if bytes.len() == 10 {
        return Some(minutes);
    }
    if bytes.len() < 16 || bytes[10] != b'T' || bytes[13] != b':' {
        return None;
    }
    let (hour, minute) = (number(11, 13)?, number(14, 16)?);
    if hour > 23 || minute > 59 {
        return None;
    }
    minutes += hour * 60 + minute;
    match &text[16..] {
        "" | "Z" => Some(minutes),
        zone => {
            let sign = match zone.as_bytes() {
                [b'+', _, _, b':', _, _] => 1,
                [b'-', _, _, b':', _, _] => -1,
                _ => return None,
            };
            let (hours, rest) = (number(17, 19)?, number(20, 22)?);
            (hours <= 14 && rest <= 59).then_some(minutes - sign * (hours * 60 + rest))
        }
    }
}

fn check_place(place: &InputPlace, path: &str, found: &mut Vec<InputProblem>) {
    let mut add = |name: &str, kind| {
        found.push(InputProblem {
            field: format!("{path}.{name}"),
            kind,
        });
    };
    if place.place.trim().is_empty() {
        add("place", InputProblemKind::Required);
    }
    if place.lat.is_some() != place.lon.is_some() {
        add("lat", InputProblemKind::CoordinatePair);
    }
    if place.lat.is_some_and(|lat| !(-90.0..=90.0).contains(&lat)) {
        add("lat", InputProblemKind::Coordinate);
    }
    if place
        .lon
        .is_some_and(|lon| !(-180.0..=180.0).contains(&lon))
    {
        add("lon", InputProblemKind::Coordinate);
    }
    let mut moments = [None, None];
    for (slot, name, value) in [
        (0, "earliest", &place.earliest),
        (1, "latest", &place.latest),
    ] {
        if let Some(text) = value {
            moments[slot] = parse_moment(text);
            if moments[slot].is_none() {
                add(name, InputProblemKind::DateFormat);
            }
        }
    }
    if let [Some(earliest), Some(latest)] = moments
        && latest < earliest
    {
        add("latest", InputProblemKind::DateOrder);
    }
}

fn check_money(money: &Money, path: &str, found: &mut Vec<InputProblem>) {
    if !(money.amount.is_finite() && money.amount > 0.0) {
        found.push(InputProblem {
            field: format!("{path}.amount"),
            kind: InputProblemKind::NotPositive,
        });
    }
    if money.currency.len() != 3 || !money.currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
        found.push(InputProblem {
            field: format!("{path}.currency"),
            kind: InputProblemKind::Currency,
        });
    }
}

/// Everything that has to be corrected before the document can be used.
#[must_use]
pub fn validate_input(input: &PlanningInput) -> Vec<InputProblem> {
    let mut found = Vec::new();
    let problem = |field: String, kind| InputProblem { field, kind };
    if input.schema != INPUT_SCHEMA {
        found.push(problem("schema".to_owned(), InputProblemKind::Schema));
    }
    if input.schema_version != INPUT_SCHEMA_VERSION {
        found.push(problem(
            "schema_version".to_owned(),
            InputProblemKind::Schema,
        ));
    }
    let vehicle = &input.vehicle;
    if vehicle.payload_kg == Some(0) {
        let field = "vehicle.payload_kg".to_owned();
        found.push(problem(field, InputProblemKind::NotPositive));
    }
    if vehicle.pallets == Some(0) {
        let field = "vehicle.pallets".to_owned();
        found.push(problem(field, InputProblemKind::NotPositive));
    }
    if let Some(start) = &vehicle.start {
        check_place(start, "vehicle.start", &mut found);
    }
    if let Some(cost) = &vehicle.cost_per_km {
        check_money(cost, "vehicle.cost_per_km", &mut found);
    }
    let mut ids = BTreeSet::new();
    for (index, order) in input.orders.iter().enumerate() {
        let path = format!("orders[{index}]");
        if order.id.trim().is_empty() {
            found.push(problem(format!("{path}.id"), InputProblemKind::Required));
        } else if !ids.insert(order.id.trim()) {
            found.push(problem(format!("{path}.id"), InputProblemKind::Duplicate));
        }
        if order.cargo.title.trim().is_empty() {
            let field = format!("{path}.cargo.title");
            found.push(problem(field, InputProblemKind::Required));
        }
        if order.cargo.weight_kg == Some(0) {
            let field = format!("{path}.cargo.weight_kg");
            found.push(problem(field, InputProblemKind::NotPositive));
        }
        if order.cargo.pallets == Some(0) {
            let field = format!("{path}.cargo.pallets");
            found.push(problem(field, InputProblemKind::NotPositive));
        }
        let known = ["weight_kg", "pallets"];
        if order
            .cargo
            .estimated
            .iter()
            .any(|name| !known.contains(&name.as_str()))
        {
            let field = format!("{path}.cargo.estimated");
            found.push(problem(field, InputProblemKind::EstimatedName));
        }
        check_place(&order.pickup, &format!("{path}.pickup"), &mut found);
        check_place(&order.delivery, &format!("{path}.delivery"), &mut found);
        if let Some(rate) = &order.rate {
            check_money(rate, &format!("{path}.rate"), &mut found);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(id: &str) -> InputOrder {
        let place = |name: &str| InputPlace {
            place: name.to_owned(),
            lat: Some(50.0),
            lon: Some(30.0),
            ..InputPlace::default()
        };
        InputOrder {
            id: id.to_owned(),
            cargo: InputCargo {
                title: "Synthetic cargo".to_owned(),
                ..InputCargo::default()
            },
            pickup: place("Alpha"),
            delivery: place("Beta"),
            ..InputOrder::default()
        }
    }

    #[test]
    fn a_complete_document_has_no_problems() {
        let input = PlanningInput {
            orders: vec![order("A1"), order("A2")],
            ..PlanningInput::default()
        };
        assert_eq!(validate_input(&input), []);
    }

    #[test]
    fn every_problem_names_its_field() {
        let mut input = PlanningInput::default();
        let mut broken = order("A1");
        broken.pickup.lon = None;
        broken.delivery.earliest = Some("2026-12-02".to_owned());
        broken.delivery.latest = Some("2026-12-01".to_owned());
        broken.cargo.weight_kg = Some(0);
        broken.rate = Some(Money {
            amount: 10.0,
            currency: "грн".to_owned(),
        });
        input.orders = vec![broken, order("A1")];
        let fields: Vec<_> = validate_input(&input)
            .into_iter()
            .map(|problem| problem.field)
            .collect();
        for expected in [
            "orders[0].pickup.lat",
            "orders[0].delivery.latest",
            "orders[0].cargo.weight_kg",
            "orders[0].rate.currency",
            "orders[1].id",
        ] {
            assert!(fields.iter().any(|field| field == expected), "{expected}");
        }
    }

    #[test]
    fn dates_are_compared_across_time_zones() {
        assert_eq!(parse_moment("1970-01-02"), Some(1440));
        assert_eq!(
            parse_moment("2026-12-01T12:00+02:00"),
            parse_moment("2026-12-01T10:00Z")
        );
        assert!(parse_moment("2028-02-29").is_some());
        for bad in ["2026-02-29", "01.12.2026", "2026-12-01 10:00", "2026-13-01"] {
            assert_eq!(parse_moment(bad), None, "{bad}");
        }
    }
}
