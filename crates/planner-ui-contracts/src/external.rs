//! Checking the route in third-party map services, and the figures typed by hand.

use serde::{Deserialize, Serialize};

/// A service whose route page accepts the points in its address.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MapService {
    /// Car routes with traffic.
    Google,
    /// The truck profile of the service; it does not know this vehicle.
    GraphHopper,
}

/// Google documents an origin, a destination and nine points between them.
pub const GOOGLE_POINT_LIMIT: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapLinkError {
    TooFewPoints,
    TooManyPoints,
}

impl MapService {
    /// Every address of the service starts with this.
    #[must_use]
    pub const fn address_start(self) -> &'static str {
        match self {
            Self::Google => "https://www.google.com/maps/dir/?api=1&",
            Self::GraphHopper => "https://graphhopper.com/maps/?",
        }
    }

    /// The address of the route through `points`, given as latitude and longitude.
    /// A route is never shortened to fit: too many points are an error.
    pub fn route_address(self, points: &[(f64, f64)]) -> Result<String, MapLinkError> {
        let [first, .., last] = points else {
            return Err(MapLinkError::TooFewPoints);
        };
        let rest = match self {
            Self::Google => {
                if points.len() > GOOGLE_POINT_LIMIT {
                    return Err(MapLinkError::TooManyPoints);
                }
                let place = |point: &(f64, f64)| format!("{:.6},{:.6}", point.0, point.1);
                let mut query = format!(
                    "origin={}&destination={}&travelmode=driving",
                    place(first),
                    place(last)
                );
                let between: Vec<String> =
                    points[1..points.len() - 1].iter().map(place).collect();
                if !between.is_empty() {
                    query.push_str("&waypoints=");
                    query.push_str(&between.join("%7C"));
                }
                query
            }
            Self::GraphHopper => {
                let mut query: String = points
                    .iter()
                    .map(|point| format!("point={:.6}%2C{:.6}&", point.0, point.1))
                    .collect();
                query.push_str("profile=truck");
                query
            }
        };
        Ok(format!("{}{rest}", self.address_start()))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenRouteInput {
    pub service: MapService,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenedRoute {
    /// Shown so that it can be copied.
    pub url: String,
    /// Whether the system was asked to open the browser.
    pub opened: bool,
}

/// One value typed by the operator and the route it was typed for.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FigureDraft {
    pub value: String,
    pub route_id: String,
    pub revision: u64,
}

impl FigureDraft {
    /// A value typed for another route or an earlier order of its points.
    #[must_use]
    pub fn is_outdated(&self, route_id: &str, revision: u64) -> bool {
        !self.value.trim().is_empty() && (self.route_id != route_id || self.revision != revision)
    }
}

/// Kilometres and driving time the operator found for the route.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FiguresDraft {
    pub distance_km: FigureDraft,
    pub drive_time: FigureDraft,
}

/// A positive number of kilometres; a comma or a point separates the fraction.
#[must_use]
pub fn parse_distance_km(text: &str) -> Option<f64> {
    let plain: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    let value: f64 = plain.replace(',', ".").parse().ok()?;
    (value.is_finite() && value > 0.0 && value <= 100_000.0).then_some(value)
}

/// Driving time written as hours and minutes, for example `9:15`, in minutes.
#[must_use]
pub fn parse_drive_minutes(text: &str) -> Option<u32> {
    let (hours, minutes) = text.trim().split_once(':')?;
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(hours) || !digits(minutes) || hours.len() > 4 || minutes.len() != 2 {
        return None;
    }
    let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    let total = hours * 60 + minutes;
    (minutes < 60 && total > 0).then_some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTE: [(f64, f64); 3] = [(50.0, 30.0), (50.1, 30.1), (50.4, 31.5)];

    #[test]
    fn google_takes_latitude_first_and_names_the_ends() {
        assert_eq!(
            MapService::Google.route_address(&ROUTE).expect("route"),
            "https://www.google.com/maps/dir/?api=1&origin=50.000000,30.000000\
             &destination=50.400000,31.500000&travelmode=driving&waypoints=50.100000,30.100000"
        );
        assert_eq!(
            MapService::Google.route_address(&ROUTE[..2]).expect("route"),
            "https://www.google.com/maps/dir/?api=1&origin=50.000000,30.000000\
             &destination=50.100000,30.100000&travelmode=driving"
        );
        let two_between = [ROUTE[0], ROUTE[1], ROUTE[1], ROUTE[2]];
        let address = MapService::Google.route_address(&two_between).expect("route");
        assert!(address.ends_with("&waypoints=50.100000,30.100000%7C50.100000,30.100000"));
    }

    #[test]
    fn graphhopper_lists_every_point_for_a_truck() {
        assert_eq!(
            MapService::GraphHopper.route_address(&ROUTE).expect("route"),
            "https://graphhopper.com/maps/?point=50.000000%2C30.000000\
             &point=50.100000%2C30.100000&point=50.400000%2C31.500000&profile=truck"
        );
    }

    #[test]
    fn a_route_is_never_cut_short() {
        let many = vec![(50.0, 30.0); GOOGLE_POINT_LIMIT + 1];
        assert_eq!(
            MapService::Google.route_address(&many),
            Err(MapLinkError::TooManyPoints)
        );
        assert!(MapService::Google.route_address(&many[1..]).is_ok());
        assert!(MapService::GraphHopper.route_address(&many).is_ok());
        assert_eq!(
            MapService::Google.route_address(&ROUTE[..1]),
            Err(MapLinkError::TooFewPoints)
        );
    }

    #[test]
    fn typed_figures_are_read_strictly() {
        assert_eq!(parse_distance_km(" 1 640,5 "), Some(1640.5));
        for bad in ["", "0", "-5", "12 km", "inf"] {
            assert_eq!(parse_distance_km(bad), None, "{bad}");
        }
        assert_eq!(parse_drive_minutes("9:15"), Some(555));
        assert_eq!(parse_drive_minutes("112:00"), Some(6720));
        for bad in ["", "9", "9,5", "9:5", "9:75", "0:00", "-1:00", ":30"] {
            assert_eq!(parse_drive_minutes(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_figure_remembers_its_route() {
        let figure = FigureDraft {
            value: "640".to_owned(),
            route_id: "a".to_owned(),
            revision: 3,
        };
        assert!(!figure.is_outdated("a", 3));
        assert!(figure.is_outdated("a", 4));
        assert!(figure.is_outdated("b", 3));
        assert!(!FigureDraft::default().is_outdated("b", 3));
    }
}
