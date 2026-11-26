use crate::{
    GeoContractError, GeoPoint, GeoWarning, NormalizedStatus, RouteRequest, RouteResult,
    RoutingIdentity, TollPreference,
};
use core::fmt;
use serde_json::{Map, Value, json};

pub const ENGINE_NAME: &str = "valhalla";

#[must_use]
pub fn route_payload(request: &RouteRequest) -> Value {
    let locations = request
        .points()
        .iter()
        .map(|point| {
            json!({
                "lat": point.lat(),
                "lon": point.lon(),
                "search_cutoff": 100.0,
                "search_filter": {
                    "exclude_restricted": true
                }
            })
        })
        .collect::<Vec<_>>();

    json!({
        "locations": locations,
        "costing": "truck",
        "costing_options": {
            "truck": truck_options(request)
        },
        "directions_options": {
            "units": "kilometers"
        }
    })
}

pub fn normalize_route_response(
    request: &RouteRequest,
    engine_version: &str,
    map_dataset_id: &str,
    http_status: u16,
    body: &str,
) -> Result<RouteResult, GeoContractError> {
    let identity = RoutingIdentity::new(
        ENGINE_NAME,
        engine_version,
        map_dataset_id,
        request.profile().profile_fingerprint(),
    )?;
    let warnings = request.profile().warnings();

    let parsed: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => {
            return Ok(empty_result(
                identity,
                if http_status >= 500 {
                    NormalizedStatus::EngineUnavailable
                } else {
                    NormalizedStatus::EngineError
                },
                warnings,
            ));
        }
    };

    if !(200..300).contains(&http_status) {
        let error_code = parsed.get("error_code").and_then(Value::as_u64);
        return Ok(empty_result(
            identity,
            status_for_error(error_code, http_status),
            warnings,
        ));
    }

    let Some(summary) = parsed.pointer("/trip/summary") else {
        return Ok(empty_result(
            identity,
            NormalizedStatus::EngineError,
            warnings,
        ));
    };
    let distance_m = summary
        .get("length")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|kilometers| kilometers * 1000.0);
    let duration_s = summary
        .get("time")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0);

    let Some(legs) = parsed
        .get("trip")
        .and_then(|trip| trip.get("legs"))
        .and_then(Value::as_array)
    else {
        return Ok(empty_result(
            identity,
            NormalizedStatus::EngineError,
            warnings,
        ));
    };

    if distance_m.is_none() || duration_s.is_none() || legs.is_empty() {
        return Ok(empty_result(
            identity,
            NormalizedStatus::EngineError,
            warnings,
        ));
    }

    let mut geometry = Vec::new();
    for leg in legs {
        let Some(shape) = leg.get("shape").and_then(Value::as_str) else {
            return Ok(empty_result(
                identity,
                NormalizedStatus::EngineError,
                warnings,
            ));
        };
        let Ok(mut leg_geometry) = decode_polyline6(shape) else {
            return Ok(empty_result(
                identity,
                NormalizedStatus::EngineError,
                warnings,
            ));
        };
        if leg_geometry.is_empty() {
            return Ok(empty_result(
                identity,
                NormalizedStatus::EngineError,
                warnings,
            ));
        }
        if geometry
            .last()
            .is_some_and(|last| Some(last) == leg_geometry.first())
        {
            leg_geometry.remove(0);
        }
        geometry.extend(leg_geometry);
    }

    if geometry.len() < 2 {
        return Ok(empty_result(
            identity,
            NormalizedStatus::EngineError,
            warnings,
        ));
    }

    Ok(RouteResult {
        status: NormalizedStatus::Ok,
        identity,
        distance_m,
        duration_s,
        geometry,
        snapped_points: Vec::new(),
        warnings,
    })
}

fn truck_options(request: &RouteRequest) -> Value {
    let profile = request.profile();
    let mut truck = Map::new();

    insert_optional_f64(&mut truck, "height", profile.height_m);
    insert_optional_f64(&mut truck, "width", profile.width_m);
    insert_optional_f64(&mut truck, "length", profile.length_m);
    insert_optional_f64(&mut truck, "weight", profile.gross_weight_t);
    insert_optional_f64(&mut truck, "axle_load", profile.axle_load_t);
    if let Some(axle_count) = profile.axle_count {
        truck.insert("axle_count".to_owned(), json!(axle_count));
    }
    truck.insert("hazmat".to_owned(), json!(profile.hazmat));
    truck.insert("exclude_unpaved".to_owned(), json!(profile.exclude_unpaved));
    truck.insert(
        "use_truck_route".to_owned(),
        json!(if profile.prefer_truck_routes {
            1.0
        } else {
            0.0
        }),
    );
    truck.insert(
        "use_tolls".to_owned(),
        json!(use_tolls(profile.toll_preference)),
    );
    // Without an explicit value the engine keeps its own truck default.
    // Valhalla works with whole km/h and drops any fraction.
    insert_optional_f64(&mut truck, "top_speed", profile.max_speed_kmh);

    Value::Object(truck)
}

/// Valhalla `use_tolls` is a willingness factor within 0..=1, where 0.5 means no
/// preference. It only changes route cost: even 0 does not guarantee a route
/// without toll roads. The hard exclusion `exclude_tolls` is experimental and
/// ignored unless the server allows it, so it is not used.
const fn use_tolls(preference: TollPreference) -> f64 {
    match preference {
        TollPreference::Allow => 0.5,
        TollPreference::PreferAvoid => 0.25,
        TollPreference::Avoid => 0.0,
    }
}

fn insert_optional_f64(target: &mut Map<String, Value>, key: &str, value: Option<f64>) {
    if let Some(value) = value {
        target.insert(key.to_owned(), json!(value));
    }
}

fn status_for_error(error_code: Option<u64>, http_status: u16) -> NormalizedStatus {
    match error_code {
        Some(170 | 441 | 442 | 443) => NormalizedStatus::NoRoute,
        Some(130 | 131 | 132 | 171 | 420 | 421 | 422 | 423 | 424) => NormalizedStatus::InvalidPoint,
        Some(124 | 125 | 412) => NormalizedStatus::InvalidProfile,
        _ if http_status >= 500 => NormalizedStatus::EngineUnavailable,
        _ => NormalizedStatus::EngineError,
    }
}

fn empty_result(
    identity: RoutingIdentity,
    status: NormalizedStatus,
    warnings: Vec<GeoWarning>,
) -> RouteResult {
    RouteResult {
        status,
        identity,
        distance_m: None,
        duration_s: None,
        geometry: Vec::new(),
        snapped_points: Vec::new(),
        warnings,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolylineDecodeError {
    InvalidByte,
    Truncated,
    Overflow,
    InvalidCoordinate,
}

impl fmt::Display for PolylineDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidByte => formatter.write_str("polyline contains an invalid byte"),
            Self::Truncated => formatter.write_str("polyline is truncated"),
            Self::Overflow => formatter.write_str("polyline delta overflows"),
            Self::InvalidCoordinate => {
                formatter.write_str("polyline decodes outside coordinate bounds")
            }
        }
    }
}

impl std::error::Error for PolylineDecodeError {}

pub fn decode_polyline6(encoded: &str) -> Result<Vec<GeoPoint>, PolylineDecodeError> {
    let bytes = encoded.as_bytes();
    let mut index = 0_usize;
    let mut latitude = 0_i64;
    let mut longitude = 0_i64;
    let mut points = Vec::new();

    while index < bytes.len() {
        let lat_delta = decode_component(bytes, &mut index)?;
        if index >= bytes.len() {
            return Err(PolylineDecodeError::Truncated);
        }
        let lon_delta = decode_component(bytes, &mut index)?;
        latitude = latitude
            .checked_add(lat_delta)
            .ok_or(PolylineDecodeError::Overflow)?;
        longitude = longitude
            .checked_add(lon_delta)
            .ok_or(PolylineDecodeError::Overflow)?;

        let point = GeoPoint::new(
            latitude as f64 / 1_000_000.0,
            longitude as f64 / 1_000_000.0,
        )
        .map_err(|_| PolylineDecodeError::InvalidCoordinate)?;
        points.push(point);
    }

    Ok(points)
}

fn decode_component(bytes: &[u8], index: &mut usize) -> Result<i64, PolylineDecodeError> {
    let mut result = 0_i64;
    let mut shift = 0_u32;

    loop {
        let Some(raw) = bytes.get(*index).copied() else {
            return Err(PolylineDecodeError::Truncated);
        };
        *index += 1;
        if raw < 63 {
            return Err(PolylineDecodeError::InvalidByte);
        }
        let value = i64::from(raw - 63);
        result |= (value & 0x1f)
            .checked_shl(shift)
            .ok_or(PolylineDecodeError::Overflow)?;
        if value < 0x20 {
            break;
        }
        shift += 5;
        if shift > 60 {
            return Err(PolylineDecodeError::Overflow);
        }
    }

    Ok(if result & 1 == 1 {
        !(result >> 1)
    } else {
        result >> 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VehicleRoutingProfile;

    fn profile() -> VehicleRoutingProfile {
        VehicleRoutingProfile {
            height_m: Some(3.2),
            width_m: Some(2.4),
            length_m: Some(8.5),
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

    fn request() -> RouteRequest {
        RouteRequest::new(
            vec![
                GeoPoint::new(49.8393, 24.0296).expect("valid point"),
                GeoPoint::new(49.8430, 24.0360).expect("valid point"),
                GeoPoint::new(49.8485, 24.0250).expect("valid point"),
            ],
            profile(),
        )
        .expect("valid request")
    }

    #[test]
    fn truck_payload_preserves_operator_constraints_without_defaults() {
        let payload = route_payload(&request());
        assert_eq!(payload["costing"], "truck");
        assert_eq!(payload["costing_options"]["truck"]["height"], 3.2);
        assert_eq!(payload["costing_options"]["truck"]["weight"], 7.5);
        assert_eq!(payload["costing_options"]["truck"]["axle_count"], 2);
        assert_eq!(payload["costing_options"]["truck"]["exclude_unpaved"], true);
        assert_eq!(payload["costing_options"]["truck"]["use_truck_route"], 1.0);
        assert_eq!(
            payload["locations"][0]["search_filter"]["exclude_restricted"],
            true
        );

        let mut incomplete = profile();
        incomplete.height_m = None;
        let incomplete_request = RouteRequest::new(request().points().to_vec(), incomplete)
            .expect("missing dimensions warn instead of inventing defaults");
        let incomplete_payload = route_payload(&incomplete_request);
        assert!(
            incomplete_payload["costing_options"]["truck"]
                .get("height")
                .is_none()
        );
    }

    fn payload_for(profile: VehicleRoutingProfile) -> Value {
        let request =
            RouteRequest::new(request().points().to_vec(), profile).expect("routable profile");
        route_payload(&request)
    }

    #[test]
    fn toll_preference_is_sent_as_use_tolls() {
        for (preference, expected) in [
            (TollPreference::Allow, 0.5),
            (TollPreference::PreferAvoid, 0.25),
            (TollPreference::Avoid, 0.0),
        ] {
            let mut chosen = profile();
            chosen.toll_preference = preference;
            let payload = payload_for(chosen);

            assert_eq!(payload["costing_options"]["truck"]["use_tolls"], expected);
        }
    }

    #[test]
    fn max_speed_is_sent_as_top_speed_only_when_given() {
        let mut limited = profile();
        limited.max_speed_kmh = Some(85.0);
        assert_eq!(
            payload_for(limited)["costing_options"]["truck"]["top_speed"],
            85.0
        );

        assert!(
            payload_for(profile())["costing_options"]["truck"]
                .get("top_speed")
                .is_none()
        );
    }

    #[test]
    fn polyline6_decodes_real_precision_route_shape() {
        let points = decode_polyline6("gk}`~A_ysyl@gfF_oKwvInnT").expect("valid polyline6");
        assert_eq!(points.len(), 3);
        assert!((points[0].lat() - 49.8393).abs() < 0.000_001);
        assert!((points[0].lon() - 24.0296).abs() < 0.000_001);
        assert!((points[2].lat() - 49.8485).abs() < 0.000_001);
        assert!((points[2].lon() - 24.0250).abs() < 0.000_001);
    }

    #[test]
    fn successful_multileg_response_returns_geometry_without_duplicate_joint() {
        let body = r#"{
          "trip": {
            "summary": {"length": 2.75, "time": 240.0},
            "legs": [
              {"shape": "gk}`~A_ysyl@gfF_oK"},
              {"shape": "orda~A_i`zl@wvInnT"}
            ]
          }
        }"#;

        let result = normalize_route_response(&request(), "3.9.2", "fixture-2026-11", 200, body)
            .expect("valid identity");
        assert_eq!(result.status, NormalizedStatus::Ok);
        assert_eq!(result.distance_m, Some(2750.0));
        assert_eq!(result.duration_s, Some(240.0));
        assert_eq!(result.geometry.len(), 3);
        assert!(result.snapped_points.is_empty());
        assert_eq!(result.identity.engine(), "valhalla");
        assert_eq!(result.identity.engine_version(), "3.9.2");
    }

    #[test]
    fn malformed_success_never_returns_partial_verified_geometry() {
        let body = r#"{
          "trip": {
            "summary": {"length": 2.75, "time": 240.0},
            "legs": [
              {"shape": "gk}`~A_ysyl@gfF_oK"},
              {"shape": "!"}
            ]
          }
        }"#;

        let result = normalize_route_response(&request(), "3.9.2", "fixture-2026-11", 200, body)
            .expect("valid identity");
        assert_eq!(result.status, NormalizedStatus::EngineError);
        assert!(result.geometry.is_empty());
        assert_eq!(result.distance_m, None);
    }

    #[test]
    fn valhalla_error_codes_map_to_product_statuses() {
        let no_route = normalize_route_response(
            &request(),
            "3.9.2",
            "fixture-2026-11",
            400,
            r#"{"error_code":442,"error":"No path could be found for input"}"#,
        )
        .expect("valid identity");
        assert_eq!(no_route.status, NormalizedStatus::NoRoute);

        let bad_point = normalize_route_response(
            &request(),
            "3.9.2",
            "fixture-2026-11",
            400,
            r#"{"error_code":171,"error":"No suitable edges near location"}"#,
        )
        .expect("valid identity");
        assert_eq!(bad_point.status, NormalizedStatus::InvalidPoint);

        let unavailable =
            normalize_route_response(&request(), "3.9.2", "fixture-2026-11", 503, "not json")
                .expect("valid identity");
        assert_eq!(unavailable.status, NormalizedStatus::EngineUnavailable);
    }
}
