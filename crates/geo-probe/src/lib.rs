use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod valhalla;

pub const GEO_CONTRACT_VERSION: &str = "routing-probe-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    VehicleDimensionDefaulted,
    ApproximateEstimate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportFailure {
    Connection,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleProfile {
    pub height_m: Option<f64>,
    pub width_m: Option<f64>,
    pub length_m: Option<f64>,
    pub weight_t: Option<f64>,
    pub axle_load_t: Option<f64>,
    pub axle_count: Option<u8>,
    pub hazmat: bool,
    pub hgv: bool,
    pub exclude_unpaved: bool,
    pub use_truck_route: bool,
}

impl VehicleProfile {
    pub fn warnings(&self) -> Vec<WarningCode> {
        let missing_critical = self.height_m.is_none()
            || self.width_m.is_none()
            || self.length_m.is_none()
            || self.weight_t.is_none()
            || self.axle_load_t.is_none()
            || self.axle_count.is_none();

        if missing_critical {
            vec![WarningCode::VehicleDimensionDefaulted]
        } else {
            Vec::new()
        }
    }

    pub fn profile_hash(&self) -> String {
        let canonical = serde_json::to_vec(self).expect("vehicle profile serializes");
        let digest = Sha256::digest(canonical);
        format!("{digest:x}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteRequest {
    pub from: GeoPoint,
    pub to: GeoPoint,
    #[serde(default)]
    pub via: Vec<GeoPoint>,
    pub profile: VehicleProfile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteResult {
    pub status: NormalizedStatus,
    pub profile_hash: String,
    pub map_dataset_id: String,
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
    #[serde(default)]
    pub path_names: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<WarningCode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatrixRequest {
    pub sources: Vec<GeoPoint>,
    pub targets: Vec<GeoPoint>,
    pub profile: VehicleProfile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatrixCell {
    pub source_index: usize,
    pub target_index: usize,
    pub status: NormalizedStatus,
    pub distance_m: Option<f64>,
    pub duration_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatrixResult {
    pub status: NormalizedStatus,
    pub profile_hash: String,
    pub map_dataset_id: String,
    pub source_count: usize,
    pub target_count: usize,
    pub cells: Vec<MatrixCell>,
    #[serde(default)]
    pub warnings: Vec<WarningCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    pub engine: String,
    pub engine_version: String,
    pub map_dataset_id: String,
    pub profile_hash: String,
    pub coordinates_hash: String,
    pub options_hash: String,
}

impl CacheKey {
    pub fn for_route(
        engine: impl Into<String>,
        engine_version: impl Into<String>,
        map_dataset_id: impl Into<String>,
        request: &RouteRequest,
    ) -> Self {
        let coordinates = serde_json::to_vec(&(request.from, &request.via, request.to))
            .expect("route coordinates serialize");
        let options = serde_json::to_vec(&(
            request.profile.exclude_unpaved,
            request.profile.use_truck_route,
        ))
        .expect("route options serialize");

        Self {
            engine: engine.into(),
            engine_version: engine_version.into(),
            map_dataset_id: map_dataset_id.into(),
            profile_hash: request.profile.profile_hash(),
            coordinates_hash: sha256_hex(&coordinates),
            options_hash: sha256_hex(&options),
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(height_m: Option<f64>) -> VehicleProfile {
        VehicleProfile {
            height_m,
            width_m: Some(2.2),
            length_m: Some(7.0),
            weight_t: Some(7.5),
            axle_load_t: Some(4.0),
            axle_count: Some(2),
            hazmat: false,
            hgv: true,
            exclude_unpaved: false,
            use_truck_route: true,
        }
    }

    #[test]
    fn profile_hash_is_vehicle_sensitive() {
        let small = profile(Some(3.0));
        let tall = profile(Some(4.0));
        assert_ne!(small.profile_hash(), tall.profile_hash());
    }

    #[test]
    fn unknown_critical_dimension_is_explicit_warning() {
        let unknown = profile(None);
        assert_eq!(
            unknown.warnings(),
            vec![WarningCode::VehicleDimensionDefaulted]
        );
    }

    #[test]
    fn cache_key_includes_profile_hash() {
        let point_a = GeoPoint {
            lat: 49.0,
            lon: 24.0,
        };
        let point_b = GeoPoint {
            lat: 49.0,
            lon: 24.01,
        };

        let a = RouteRequest {
            from: point_a,
            to: point_b,
            via: vec![],
            profile: profile(Some(3.0)),
        };
        let b = RouteRequest {
            profile: profile(Some(4.0)),
            ..a.clone()
        };

        let key_a = CacheKey::for_route("valhalla", "3.9.3", "fixture-v1", &a);
        let key_b = CacheKey::for_route("valhalla", "3.9.3", "fixture-v1", &b);

        assert_ne!(key_a.profile_hash, key_b.profile_hash);
        assert_eq!(key_a.coordinates_hash, key_b.coordinates_hash);
        assert_eq!(key_a.map_dataset_id, key_b.map_dataset_id);
    }
}
