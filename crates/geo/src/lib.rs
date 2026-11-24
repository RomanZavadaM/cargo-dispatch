#![forbid(unsafe_code)]

use core::fmt;

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoValidationError {
    LatitudeOutOfRange,
    LongitudeOutOfRange,
    NonFiniteCoordinate,
    InvalidVehicleDimension,
    InvalidAxleCount,
    EmptyDatasetId,
    EmptyRoute,
    EmptyMatrix,
}

impl fmt::Display for GeoValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::LatitudeOutOfRange => "latitude must be between -90 and 90",
            Self::LongitudeOutOfRange => "longitude must be between -180 and 180",
            Self::NonFiniteCoordinate => "coordinate must be finite",
            Self::InvalidVehicleDimension => "vehicle dimensions and weights must be positive",
            Self::InvalidAxleCount => "axle count must be greater than zero when known",
            Self::EmptyDatasetId => "map dataset id must not be empty",
            Self::EmptyRoute => "route requires at least origin and destination",
            Self::EmptyMatrix => "matrix requires at least one source and one target",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for GeoValidationError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
}

impl GeoPoint {
    pub fn new(lat: f64, lon: f64) -> Result<Self, GeoValidationError> {
        if !lat.is_finite() || !lon.is_finite() {
            return Err(GeoValidationError::NonFiniteCoordinate);
        }
        if !(-90.0..=90.0).contains(&lat) {
            return Err(GeoValidationError::LatitudeOutOfRange);
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err(GeoValidationError::LongitudeOutOfRange);
        }
        Ok(Self { lat, lon })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoWarning {
    VehicleDimensionUnknown,
    ApproximateEstimate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoStatus {
    Ok,
    NoRoute,
    InvalidPoint,
    InvalidProfile,
    EngineUnavailable,
    Timeout,
    DatasetMissing,
    EngineError,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleRoutingProfile {
    pub height_m: Option<f64>,
    pub width_m: Option<f64>,
    pub length_m: Option<f64>,
    pub weight_t: Option<f64>,
    pub axle_load_t: Option<f64>,
    pub axle_count: Option<u8>,
    pub hazmat: bool,
    pub hgv: bool,
    pub exclude_unpaved: bool,
    pub prefer_truck_routes: bool,
}

impl VehicleRoutingProfile {
    pub fn validate(&self) -> Result<(), GeoValidationError> {
        for value in [
            self.height_m,
            self.width_m,
            self.length_m,
            self.weight_t,
            self.axle_load_t,
        ]
        .into_iter()
        .flatten()
        {
            if !value.is_finite() || value <= 0.0 {
                return Err(GeoValidationError::InvalidVehicleDimension);
            }
        }

        if self.axle_count == Some(0) {
            return Err(GeoValidationError::InvalidAxleCount);
        }
        Ok(())
    }

    #[must_use]
    pub fn warnings(&self) -> Vec<GeoWarning> {
        let missing = self.height_m.is_none()
            || self.width_m.is_none()
            || self.length_m.is_none()
            || self.weight_t.is_none()
            || self.axle_load_t.is_none()
            || self.axle_count.is_none();

        if missing {
            vec![GeoWarning::VehicleDimensionUnknown]
        } else {
            Vec::new()
        }
    }

    pub fn profile_hash(&self) -> Result<RoutingProfileHash, GeoValidationError> {
        self.validate()?;
        let mut hash = FNV_OFFSET;

        for value in [
            self.height_m,
            self.width_m,
            self.length_m,
            self.weight_t,
            self.axle_load_t,
        ] {
            hash = hash_option_f64(hash, value);
        }
        hash = hash_option_u8(hash, self.axle_count);

        for flag in [
            self.hazmat,
            self.hgv,
            self.exclude_unpaved,
            self.prefer_truck_routes,
        ] {
            hash = fnv_update(hash, &[u8::from(flag)]);
        }

        Ok(RoutingProfileHash(hash))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutingProfileHash(u64);

impl RoutingProfileHash {
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn to_hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MapDatasetId(String);

impl MapDatasetId {
    pub fn new(value: impl Into<String>) -> Result<Self, GeoValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(GeoValidationError::EmptyDatasetId);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteRequest {
    pub from: GeoPoint,
    pub to: GeoPoint,
    pub via: Vec<GeoPoint>,
    pub profile: VehicleRoutingProfile,
}

impl RouteRequest {
    pub fn validate(&self) -> Result<(), GeoValidationError> {
        self.profile.validate()
    }

    pub fn cache_key(&self, dataset_id: MapDatasetId) -> Result<RouteCacheKey, GeoValidationError> {
        self.validate()?;

        let mut coordinates_hash = FNV_OFFSET;
        coordinates_hash = hash_point(coordinates_hash, self.from);
        for point in &self.via {
            coordinates_hash = hash_point(coordinates_hash, *point);
        }
        coordinates_hash = hash_point(coordinates_hash, self.to);

        Ok(RouteCacheKey {
            dataset_id,
            profile_hash: self.profile.profile_hash()?,
            coordinates_hash,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteResult {
    pub status: GeoStatus,
    pub profile_hash: RoutingProfileHash,
    pub map_dataset_id: MapDatasetId,
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
    pub warnings: Vec<GeoWarning>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatrixRequest {
    pub sources: Vec<GeoPoint>,
    pub targets: Vec<GeoPoint>,
    pub profile: VehicleRoutingProfile,
}

impl MatrixRequest {
    pub fn validate(&self) -> Result<(), GeoValidationError> {
        if self.sources.is_empty() || self.targets.is_empty() {
            return Err(GeoValidationError::EmptyMatrix);
        }
        self.profile.validate()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatrixCell {
    pub source_index: usize,
    pub target_index: usize,
    pub status: GeoStatus,
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatrixResult {
    pub status: GeoStatus,
    pub profile_hash: RoutingProfileHash,
    pub map_dataset_id: MapDatasetId,
    pub source_count: usize,
    pub target_count: usize,
    pub cells: Vec<MatrixCell>,
    pub warnings: Vec<GeoWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCacheKey {
    pub dataset_id: MapDatasetId,
    pub profile_hash: RoutingProfileHash,
    pub coordinates_hash: u64,
}

fn canonical_f64_bits(value: f64) -> u64 {
    if value == 0.0 {
        0.0f64.to_bits()
    } else {
        value.to_bits()
    }
}

fn hash_option_f64(mut hash: u64, value: Option<f64>) -> u64 {
    match value {
        Some(value) => {
            hash = fnv_update(hash, &[1]);
            fnv_update(hash, &canonical_f64_bits(value).to_be_bytes())
        }
        None => fnv_update(hash, &[0]),
    }
}

fn hash_option_u8(mut hash: u64, value: Option<u8>) -> u64 {
    match value {
        Some(value) => {
            hash = fnv_update(hash, &[1]);
            fnv_update(hash, &[value])
        }
        None => fnv_update(hash, &[0]),
    }
}

fn hash_point(mut hash: u64, point: GeoPoint) -> u64 {
    hash = fnv_update(hash, &canonical_f64_bits(point.lat).to_be_bytes());
    fnv_update(hash, &canonical_f64_bits(point.lon).to_be_bytes())
}

fn fnv_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(lat: f64, lon: f64) -> GeoPoint {
        GeoPoint::new(lat, lon).expect("fixture point is valid")
    }

    fn profile(height_m: Option<f64>) -> VehicleRoutingProfile {
        VehicleRoutingProfile {
            height_m,
            width_m: Some(2.2),
            length_m: Some(7.0),
            weight_t: Some(7.5),
            axle_load_t: Some(4.0),
            axle_count: Some(2),
            hazmat: false,
            hgv: true,
            exclude_unpaved: false,
            prefer_truck_routes: true,
        }
    }

    #[test]
    fn coordinates_are_validated_before_an_adapter_sees_them() {
        assert_eq!(
            GeoPoint::new(91.0, 24.0),
            Err(GeoValidationError::LatitudeOutOfRange)
        );
        assert_eq!(
            GeoPoint::new(49.0, 181.0),
            Err(GeoValidationError::LongitudeOutOfRange)
        );
        assert_eq!(
            GeoPoint::new(f64::NAN, 24.0),
            Err(GeoValidationError::NonFiniteCoordinate)
        );
    }

    #[test]
    fn unknown_vehicle_dimensions_remain_explicit() {
        assert_eq!(
            profile(None).warnings(),
            vec![GeoWarning::VehicleDimensionUnknown]
        );
    }

    #[test]
    fn invalid_vehicle_profile_is_rejected() {
        let mut invalid = profile(Some(3.2));
        invalid.weight_t = Some(0.0);
        assert_eq!(
            invalid.validate(),
            Err(GeoValidationError::InvalidVehicleDimension)
        );

        invalid.weight_t = Some(7.5);
        invalid.axle_count = Some(0);
        assert_eq!(
            invalid.validate(),
            Err(GeoValidationError::InvalidAxleCount)
        );
    }

    #[test]
    fn profile_hash_changes_when_truck_constraints_change() {
        let low = profile(Some(3.0)).profile_hash().expect("valid profile");
        let high = profile(Some(4.0)).profile_hash().expect("valid profile");
        assert_ne!(low, high);

        let mut hazmat = profile(Some(3.0));
        hazmat.hazmat = true;
        assert_ne!(low, hazmat.profile_hash().expect("valid profile"));
    }

    #[test]
    fn cache_identity_separates_profile_from_coordinates_and_dataset() {
        let request_a = RouteRequest {
            from: point(49.0, 24.0),
            to: point(49.0, 24.01),
            via: Vec::new(),
            profile: profile(Some(3.0)),
        };
        let request_b = RouteRequest {
            profile: profile(Some(4.0)),
            ..request_a.clone()
        };

        let key_a = request_a
            .cache_key(MapDatasetId::new("fixture-v1").expect("dataset id"))
            .expect("cache key");
        let key_b = request_b
            .cache_key(MapDatasetId::new("fixture-v1").expect("dataset id"))
            .expect("cache key");

        assert_ne!(key_a.profile_hash, key_b.profile_hash);
        assert_eq!(key_a.coordinates_hash, key_b.coordinates_hash);
        assert_eq!(key_a.dataset_id, key_b.dataset_id);
    }

    #[test]
    fn route_and_matrix_share_the_same_vehicle_profile_identity() {
        let profile = profile(Some(3.2));
        let route_hash = profile.profile_hash().expect("route profile hash");

        let matrix = MatrixRequest {
            sources: vec![point(49.0, 24.0)],
            targets: vec![point(49.0, 24.01)],
            profile,
        };
        matrix.validate().expect("matrix request is valid");
        assert_eq!(
            route_hash,
            matrix.profile.profile_hash().expect("matrix profile hash")
        );
    }

    #[test]
    fn matrix_requires_sources_and_targets() {
        let empty = MatrixRequest {
            sources: Vec::new(),
            targets: vec![point(49.0, 24.01)],
            profile: profile(Some(3.2)),
        };
        assert_eq!(empty.validate(), Err(GeoValidationError::EmptyMatrix));
    }

    #[test]
    fn map_dataset_identity_is_never_implicit() {
        assert_eq!(
            MapDatasetId::new("   "),
            Err(GeoValidationError::EmptyDatasetId)
        );
        assert_eq!(
            MapDatasetId::new("fixture-v2")
                .expect("dataset id")
                .as_str(),
            "fixture-v2"
        );
    }
}
