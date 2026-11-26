#![forbid(unsafe_code)]

use core::fmt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod valhalla;
pub mod valhalla_http;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoPoint {
    lat: f64,
    lon: f64,
}

impl GeoPoint {
    pub fn new(lat: f64, lon: f64) -> Result<Self, GeoContractError> {
        if !lat.is_finite() || !(-90.0..=90.0).contains(&lat) {
            return Err(GeoContractError::InvalidLatitude);
        }
        if !lon.is_finite() || !(-180.0..=180.0).contains(&lon) {
            return Err(GeoContractError::InvalidLongitude);
        }
        Ok(Self { lat, lon })
    }

    #[must_use]
    pub const fn lat(self) -> f64 {
        self.lat
    }

    #[must_use]
    pub const fn lon(self) -> f64 {
        self.lon
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TollPreference {
    Allow,
    PreferAvoid,
    Avoid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleRoutingProfile {
    pub height_m: Option<f64>,
    pub width_m: Option<f64>,
    pub length_m: Option<f64>,
    pub gross_weight_t: Option<f64>,
    pub axle_load_t: Option<f64>,
    pub axle_count: Option<u8>,
    pub hazmat: bool,
    pub hgv: bool,
    pub exclude_unpaved: bool,
    pub prefer_truck_routes: bool,
    pub toll_preference: TollPreference,
    pub max_speed_kmh: Option<f64>,
}

impl VehicleRoutingProfile {
    pub fn validate(&self) -> Result<(), GeoContractError> {
        validate_optional_positive("height_m", self.height_m)?;
        validate_optional_positive("width_m", self.width_m)?;
        validate_optional_positive("length_m", self.length_m)?;
        validate_optional_positive("gross_weight_t", self.gross_weight_t)?;
        validate_optional_positive("axle_load_t", self.axle_load_t)?;
        validate_optional_positive("max_speed_kmh", self.max_speed_kmh)?;

        if self.axle_count == Some(0) {
            return Err(GeoContractError::InvalidVehicleValue("axle_count"));
        }

        Ok(())
    }

    #[must_use]
    pub fn profile_fingerprint(&self) -> String {
        let canonical = serde_json::to_vec(self).expect("vehicle routing profile serializes");
        let digest = Sha256::digest(canonical);
        format!("{digest:x}")
    }

    #[must_use]
    pub fn warnings(&self) -> Vec<GeoWarning> {
        let mut warnings = Vec::new();

        if self.height_m.is_none() {
            warnings.push(GeoWarning::MissingVehicleDimension(
                VehicleDimension::Height,
            ));
        }
        if self.width_m.is_none() {
            warnings.push(GeoWarning::MissingVehicleDimension(VehicleDimension::Width));
        }
        if self.length_m.is_none() {
            warnings.push(GeoWarning::MissingVehicleDimension(
                VehicleDimension::Length,
            ));
        }
        if self.gross_weight_t.is_none() {
            warnings.push(GeoWarning::MissingVehicleDimension(
                VehicleDimension::GrossWeight,
            ));
        }
        if self.axle_load_t.is_none() {
            warnings.push(GeoWarning::MissingVehicleDimension(
                VehicleDimension::AxleLoad,
            ));
        }
        if self.axle_count.is_none() {
            warnings.push(GeoWarning::MissingVehicleDimension(
                VehicleDimension::AxleCount,
            ));
        }

        warnings
    }
}

fn validate_optional_positive(
    field: &'static str,
    value: Option<f64>,
) -> Result<(), GeoContractError> {
    if value.is_some_and(|value| !value.is_finite() || value <= 0.0) {
        return Err(GeoContractError::InvalidVehicleValue(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleDimension {
    Height,
    Width,
    Length,
    GrossWeight,
    AxleLoad,
    AxleCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoWarning {
    MissingVehicleDimension(VehicleDimension),
    ApproximateEstimate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedStatus {
    Ok,
    NoRoute,
    InvalidPoint,
    InvalidProfile,
    EngineUnavailable,
    Timeout,
    DatasetMissing,
    EngineError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingIdentity {
    engine: String,
    engine_version: String,
    map_dataset_id: String,
    profile_fingerprint: String,
}

impl RoutingIdentity {
    pub fn new(
        engine: impl Into<String>,
        engine_version: impl Into<String>,
        map_dataset_id: impl Into<String>,
        profile_fingerprint: impl Into<String>,
    ) -> Result<Self, GeoContractError> {
        let identity = Self {
            engine: engine.into(),
            engine_version: engine_version.into(),
            map_dataset_id: map_dataset_id.into(),
            profile_fingerprint: profile_fingerprint.into(),
        };

        if identity.engine.trim().is_empty() {
            return Err(GeoContractError::EmptyIdentity("engine"));
        }
        if identity.engine_version.trim().is_empty() {
            return Err(GeoContractError::EmptyIdentity("engine_version"));
        }
        if identity.map_dataset_id.trim().is_empty() {
            return Err(GeoContractError::EmptyIdentity("map_dataset_id"));
        }
        if identity.profile_fingerprint.trim().is_empty() {
            return Err(GeoContractError::EmptyIdentity("profile_fingerprint"));
        }

        Ok(identity)
    }

    #[must_use]
    pub fn engine(&self) -> &str {
        &self.engine
    }

    #[must_use]
    pub fn engine_version(&self) -> &str {
        &self.engine_version
    }

    #[must_use]
    pub fn map_dataset_id(&self) -> &str {
        &self.map_dataset_id
    }

    #[must_use]
    pub fn profile_fingerprint(&self) -> &str {
        &self.profile_fingerprint
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteRequest {
    points: Vec<GeoPoint>,
    profile: VehicleRoutingProfile,
}

impl RouteRequest {
    pub fn new(
        points: Vec<GeoPoint>,
        profile: VehicleRoutingProfile,
    ) -> Result<Self, GeoContractError> {
        if points.len() < 2 {
            return Err(GeoContractError::RouteNeedsAtLeastTwoPoints);
        }
        profile.validate()?;
        Ok(Self { points, profile })
    }

    #[must_use]
    pub fn points(&self) -> &[GeoPoint] {
        &self.points
    }

    #[must_use]
    pub const fn profile(&self) -> &VehicleRoutingProfile {
        &self.profile
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteResult {
    pub status: NormalizedStatus,
    pub identity: RoutingIdentity,
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
    pub geometry: Vec<GeoPoint>,
    pub snapped_points: Vec<GeoPoint>,
    pub warnings: Vec<GeoWarning>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatrixRequest {
    sources: Vec<GeoPoint>,
    targets: Vec<GeoPoint>,
    profile: VehicleRoutingProfile,
}

impl MatrixRequest {
    pub fn new(
        sources: Vec<GeoPoint>,
        targets: Vec<GeoPoint>,
        profile: VehicleRoutingProfile,
    ) -> Result<Self, GeoContractError> {
        if sources.is_empty() {
            return Err(GeoContractError::MatrixNeedsSource);
        }
        if targets.is_empty() {
            return Err(GeoContractError::MatrixNeedsTarget);
        }
        profile.validate()?;
        Ok(Self {
            sources,
            targets,
            profile,
        })
    }

    #[must_use]
    pub fn sources(&self) -> &[GeoPoint] {
        &self.sources
    }

    #[must_use]
    pub fn targets(&self) -> &[GeoPoint] {
        &self.targets
    }

    #[must_use]
    pub const fn profile(&self) -> &VehicleRoutingProfile {
        &self.profile
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatrixCell {
    pub source_index: usize,
    pub target_index: usize,
    pub status: NormalizedStatus,
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatrixResult {
    pub status: NormalizedStatus,
    pub identity: RoutingIdentity,
    pub source_count: usize,
    pub target_count: usize,
    pub cells: Vec<MatrixCell>,
    pub warnings: Vec<GeoWarning>,
}

impl MatrixResult {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.cells.len() == self.source_count.saturating_mul(self.target_count)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoContractError {
    InvalidLatitude,
    InvalidLongitude,
    InvalidVehicleValue(&'static str),
    EmptyIdentity(&'static str),
    RouteNeedsAtLeastTwoPoints,
    MatrixNeedsSource,
    MatrixNeedsTarget,
}

impl fmt::Display for GeoContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLatitude => {
                formatter.write_str("latitude must be finite and within -90..=90")
            }
            Self::InvalidLongitude => {
                formatter.write_str("longitude must be finite and within -180..=180")
            }
            Self::InvalidVehicleValue(field) => {
                write!(formatter, "{field} must be positive and finite")
            }
            Self::EmptyIdentity(field) => write!(formatter, "{field} must not be empty"),
            Self::RouteNeedsAtLeastTwoPoints => {
                formatter.write_str("route request needs at least two ordered points")
            }
            Self::MatrixNeedsSource => {
                formatter.write_str("matrix request needs at least one source")
            }
            Self::MatrixNeedsTarget => {
                formatter.write_str("matrix request needs at least one target")
            }
        }
    }
}

impl std::error::Error for GeoContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> VehicleRoutingProfile {
        VehicleRoutingProfile {
            height_m: Some(3.2),
            width_m: Some(2.4),
            length_m: Some(8.5),
            gross_weight_t: Some(12.0),
            axle_load_t: Some(5.0),
            axle_count: Some(2),
            hazmat: false,
            hgv: true,
            exclude_unpaved: false,
            prefer_truck_routes: true,
            toll_preference: TollPreference::Allow,
            max_speed_kmh: None,
        }
    }

    #[test]
    fn coordinates_are_validated_before_entering_requests() {
        assert!(GeoPoint::new(49.0, 24.0).is_ok());
        assert_eq!(
            GeoPoint::new(91.0, 24.0),
            Err(GeoContractError::InvalidLatitude)
        );
        assert_eq!(
            GeoPoint::new(49.0, f64::NAN),
            Err(GeoContractError::InvalidLongitude)
        );
    }

    #[test]
    fn profile_fingerprint_changes_with_vehicle_constraints() {
        let baseline = profile();
        let mut taller = baseline.clone();
        taller.height_m = Some(3.8);

        assert_ne!(baseline.profile_fingerprint(), taller.profile_fingerprint());
        assert_eq!(baseline.profile_fingerprint().len(), 64);
    }

    #[test]
    fn unknown_vehicle_dimensions_warn_but_do_not_invent_defaults() {
        let mut incomplete = profile();
        incomplete.height_m = None;
        incomplete.axle_count = None;

        assert_eq!(
            incomplete.warnings(),
            vec![
                GeoWarning::MissingVehicleDimension(VehicleDimension::Height),
                GeoWarning::MissingVehicleDimension(VehicleDimension::AxleCount),
            ]
        );
        assert!(incomplete.validate().is_ok());
    }

    #[test]
    fn invalid_provided_vehicle_value_is_rejected() {
        let mut invalid = profile();
        invalid.gross_weight_t = Some(0.0);

        assert_eq!(
            invalid.validate(),
            Err(GeoContractError::InvalidVehicleValue("gross_weight_t"))
        );
    }

    #[test]
    fn route_keeps_operator_ordered_points() {
        let a = GeoPoint::new(49.0, 24.0).expect("valid point");
        let via = GeoPoint::new(49.01, 24.01).expect("valid point");
        let b = GeoPoint::new(49.02, 24.02).expect("valid point");
        let request = RouteRequest::new(vec![a, via, b], profile()).expect("valid route request");

        assert_eq!(request.points(), &[a, via, b]);
    }

    #[test]
    fn matrix_requires_both_sides_and_exposes_expected_shape() {
        let a = GeoPoint::new(49.0, 24.0).expect("valid point");
        let b = GeoPoint::new(49.01, 24.01).expect("valid point");

        assert_eq!(
            MatrixRequest::new(Vec::new(), vec![b], profile()),
            Err(GeoContractError::MatrixNeedsSource)
        );

        let identity =
            RoutingIdentity::new("engine", "1", "dataset", "profile").expect("valid identity");
        let result = MatrixResult {
            status: NormalizedStatus::Ok,
            identity,
            source_count: 1,
            target_count: 2,
            cells: vec![
                MatrixCell {
                    source_index: 0,
                    target_index: 0,
                    status: NormalizedStatus::Ok,
                    distance_m: Some(0.0),
                    duration_s: Some(0.0),
                },
                MatrixCell {
                    source_index: 0,
                    target_index: 1,
                    status: NormalizedStatus::Ok,
                    distance_m: Some(1000.0),
                    duration_s: Some(90.0),
                },
            ],
            warnings: Vec::new(),
        };

        assert!(result.is_complete());
        assert!(MatrixRequest::new(vec![a], vec![a, b], profile()).is_ok());
    }
}
