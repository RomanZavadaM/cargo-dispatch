use crate::{NormalizedStatus, RouteRequest, RouteResult, VehicleProfile};
use serde_json::{Map, Value, json};

pub const ENGINE_NAME: &str = "valhalla";
pub const ENGINE_VERSION: &str = "3.9.3";

pub fn route_payload(request: &RouteRequest) -> Value {
    let mut locations = Vec::with_capacity(request.via.len() + 2);
    locations.push(point_json(request.from.lat, request.from.lon));
    locations.extend(
        request
            .via
            .iter()
            .map(|point| point_json(point.lat, point.lon)),
    );
    locations.push(point_json(request.to.lat, request.to.lon));

    let profile = &request.profile;
    let mut truck = Map::new();
    insert_optional_f64(&mut truck, "height", profile.height_m);
    insert_optional_f64(&mut truck, "width", profile.width_m);
    insert_optional_f64(&mut truck, "length", profile.length_m);
    insert_optional_f64(&mut truck, "weight", profile.weight_t);
    insert_optional_f64(&mut truck, "axle_load", profile.axle_load_t);
    if let Some(axle_count) = profile.axle_count {
        truck.insert("axle_count".into(), json!(axle_count));
    }
    truck.insert("hazmat".into(), json!(profile.hazmat));
    truck.insert("exclude_unpaved".into(), json!(profile.exclude_unpaved));
    truck.insert("exclude_restricted".into(), json!(true));
    truck.insert(
        "use_truck_route".into(),
        json!(if profile.use_truck_route { 1.0 } else { 0.0 }),
    );

    json!({
        "locations": locations,
        "costing": "truck",
        "costing_options": {
            "truck": Value::Object(truck)
        },
        "directions_options": {
            "units": "kilometers"
        }
    })
}

pub fn normalize_route_response(
    profile: &VehicleProfile,
    map_dataset_id: &str,
    http_status: u16,
    body: &str,
) -> RouteResult {
    let warnings = profile.warnings();
    let profile_hash = profile.profile_hash();

    let parsed: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => {
            return RouteResult {
                status: if http_status >= 500 {
                    NormalizedStatus::EngineUnavailable
                } else {
                    NormalizedStatus::EngineError
                },
                profile_hash,
                map_dataset_id: map_dataset_id.to_owned(),
                distance_m: None,
                duration_s: None,
                path_names: Vec::new(),
                warnings,
            };
        }
    };

    if (200..300).contains(&http_status) {
        let summary = parsed.pointer("/trip/summary");
        let distance_m = summary
            .and_then(|value| value.get("length"))
            .and_then(Value::as_f64)
            .map(|km| km * 1000.0);
        let duration_s = summary
            .and_then(|value| value.get("time"))
            .and_then(Value::as_f64);

        if distance_m.is_some() && duration_s.is_some() {
            return RouteResult {
                status: NormalizedStatus::Ok,
                profile_hash,
                map_dataset_id: map_dataset_id.to_owned(),
                distance_m,
                duration_s,
                path_names: extract_path_names(&parsed),
                warnings,
            };
        }

        return RouteResult {
            status: NormalizedStatus::EngineError,
            profile_hash,
            map_dataset_id: map_dataset_id.to_owned(),
            distance_m: None,
            duration_s: None,
            path_names: Vec::new(),
            warnings,
        };
    }

    let code = parsed.get("error_code").and_then(Value::as_u64);
    let status = match code {
        Some(170 | 441 | 442) => NormalizedStatus::NoRoute,
        Some(130 | 131 | 132 | 171 | 420 | 421 | 422 | 423) => NormalizedStatus::InvalidPoint,
        Some(124 | 125 | 412) => NormalizedStatus::InvalidProfile,
        _ if http_status >= 500 => NormalizedStatus::EngineUnavailable,
        _ => NormalizedStatus::EngineError,
    };

    RouteResult {
        status,
        profile_hash,
        map_dataset_id: map_dataset_id.to_owned(),
        distance_m: None,
        duration_s: None,
        path_names: Vec::new(),
        warnings,
    }
}

fn point_json(lat: f64, lon: f64) -> Value {
    json!({"lat": lat, "lon": lon})
}

fn insert_optional_f64(target: &mut Map<String, Value>, key: &str, value: Option<f64>) {
    if let Some(value) = value {
        target.insert(key.to_owned(), json!(value));
    }
}

fn extract_path_names(root: &Value) -> Vec<String> {
    let mut names = Vec::new();
    let Some(legs) = root
        .get("trip")
        .and_then(|trip| trip.get("legs"))
        .and_then(Value::as_array)
    else {
        return names;
    };

    for leg in legs {
        let Some(maneuvers) = leg.get("maneuvers").and_then(Value::as_array) else {
            continue;
        };
        for maneuver in maneuvers {
            let Some(street_names) = maneuver.get("street_names").and_then(Value::as_array) else {
                continue;
            };
            for name in street_names.iter().filter_map(Value::as_str) {
                if !names.iter().any(|existing| existing == name) {
                    names.push(name.to_owned());
                }
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeoPoint, VehicleProfile};

    fn profile(height_m: Option<f64>) -> VehicleProfile {
        VehicleProfile {
            height_m,
            width_m: Some(2.2),
            length_m: Some(9.0),
            weight_t: Some(10.0),
            axle_load_t: Some(4.0),
            axle_count: Some(2),
            hazmat: false,
            hgv: true,
            exclude_unpaved: false,
            use_truck_route: false,
        }
    }

    #[test]
    fn request_maps_vehicle_dimensions_per_request() {
        let request = RouteRequest {
            from: GeoPoint {
                lat: 49.0,
                lon: 24.0,
            },
            to: GeoPoint {
                lat: 49.0,
                lon: 24.01,
            },
            via: vec![],
            profile: profile(Some(3.1)),
        };
        let payload = route_payload(&request);
        assert_eq!(payload["costing"], "truck");
        assert_eq!(payload["costing_options"]["truck"]["height"], 3.1);
        assert_eq!(payload["costing_options"]["truck"]["weight"], 10.0);
        assert_eq!(
            payload["costing_options"]["truck"]["exclude_restricted"],
            true
        );
    }

    #[test]
    fn success_is_normalized_without_leaking_valhalla_shape() {
        let body = r#"{
          "trip": {
            "summary": {"length": 1.25, "time": 90.0},
            "legs": [{"maneuvers": [
              {"street_names": ["T01_HEIGHT"]},
              {"street_names": ["T01_HEIGHT", "OPEN"]}
            ]}]
          }
        }"#;
        let result = normalize_route_response(&profile(Some(3.1)), "fixture-a", 200, body);
        assert_eq!(result.status, NormalizedStatus::Ok);
        assert_eq!(result.distance_m, Some(1250.0));
        assert_eq!(result.duration_s, Some(90.0));
        assert_eq!(result.path_names, vec!["T01_HEIGHT", "OPEN"]);
    }

    #[test]
    fn no_path_is_normalized() {
        let body = r#"{"error_code":442,"error":"No path could be found for input"}"#;
        let result = normalize_route_response(&profile(Some(4.0)), "fixture-a", 400, body);
        assert_eq!(result.status, NormalizedStatus::NoRoute);
    }

    #[test]
    fn invalid_location_is_distinct_from_no_route() {
        let body = r#"{"error_code":171,"error":"No suitable edges near location"}"#;
        let result = normalize_route_response(&profile(Some(4.0)), "fixture-a", 400, body);
        assert_eq!(result.status, NormalizedStatus::InvalidPoint);
    }
}
