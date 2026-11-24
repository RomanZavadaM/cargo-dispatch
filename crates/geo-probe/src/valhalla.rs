use crate::{
    MatrixCell, MatrixRequest, MatrixResult, NormalizedStatus, RouteRequest, RouteResult,
    TransportFailure, VehicleProfile,
};
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

    json!({
        "locations": locations,
        "costing": "truck",
        "costing_options": {
            "truck": truck_options(&request.profile)
        },
        "directions_options": {
            "units": "kilometers"
        }
    })
}

pub fn matrix_payload(request: &MatrixRequest) -> Value {
    let sources: Vec<Value> = request
        .sources
        .iter()
        .map(|point| point_json(point.lat, point.lon))
        .collect();
    let targets: Vec<Value> = request
        .targets
        .iter()
        .map(|point| point_json(point.lat, point.lon))
        .collect();

    json!({
        "sources": sources,
        "targets": targets,
        "costing": "truck",
        "costing_options": {
            "truck": truck_options(&request.profile)
        },
        "units": "kilometers",
        "verbose": true
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
            return empty_route_result(
                if http_status >= 500 {
                    NormalizedStatus::EngineUnavailable
                } else {
                    NormalizedStatus::EngineError
                },
                profile_hash,
                map_dataset_id,
                warnings,
            );
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

        return empty_route_result(
            NormalizedStatus::EngineError,
            profile_hash,
            map_dataset_id,
            warnings,
        );
    }

    let code = parsed.get("error_code").and_then(Value::as_u64);
    let status = match code {
        Some(170 | 441 | 442) => NormalizedStatus::NoRoute,
        Some(130 | 131 | 132 | 171 | 420 | 421 | 422 | 423) => NormalizedStatus::InvalidPoint,
        Some(124 | 125 | 412) => NormalizedStatus::InvalidProfile,
        _ if http_status >= 500 => NormalizedStatus::EngineUnavailable,
        _ => NormalizedStatus::EngineError,
    };

    empty_route_result(status, profile_hash, map_dataset_id, warnings)
}

pub fn normalize_matrix_response(
    profile: &VehicleProfile,
    map_dataset_id: &str,
    source_count: usize,
    target_count: usize,
    http_status: u16,
    body: &str,
) -> MatrixResult {
    let warnings = profile.warnings();
    let profile_hash = profile.profile_hash();

    if !(200..300).contains(&http_status) {
        return MatrixResult {
            status: if http_status >= 500 {
                NormalizedStatus::EngineUnavailable
            } else {
                NormalizedStatus::EngineError
            },
            profile_hash,
            map_dataset_id: map_dataset_id.to_owned(),
            source_count,
            target_count,
            cells: Vec::new(),
            warnings,
        };
    }

    let parsed: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => {
            return MatrixResult {
                status: NormalizedStatus::EngineError,
                profile_hash,
                map_dataset_id: map_dataset_id.to_owned(),
                source_count,
                target_count,
                cells: Vec::new(),
                warnings,
            };
        }
    };

    let Some(raw_matrix) = parsed.get("sources_to_targets").and_then(Value::as_array) else {
        return MatrixResult {
            status: NormalizedStatus::EngineError,
            profile_hash,
            map_dataset_id: map_dataset_id.to_owned(),
            source_count,
            target_count,
            cells: Vec::new(),
            warnings,
        };
    };

    let mut cells = Vec::with_capacity(source_count.saturating_mul(target_count));

    if raw_matrix.first().is_some_and(Value::is_array) {
        for (source_index, row) in raw_matrix.iter().enumerate() {
            let Some(entries) = row.as_array() else {
                continue;
            };
            for (target_index, entry) in entries.iter().enumerate() {
                cells.push(matrix_cell(entry, source_index, target_index));
            }
        }
    } else {
        for (index, entry) in raw_matrix.iter().enumerate() {
            let source_index = entry
                .get("from_index")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or_else(|| index / target_count.max(1));
            let target_index = entry
                .get("to_index")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or_else(|| index % target_count.max(1));
            cells.push(matrix_cell(entry, source_index, target_index));
        }
    }

    let expected = source_count.saturating_mul(target_count);
    let status = if cells.len() == expected {
        NormalizedStatus::Ok
    } else {
        NormalizedStatus::EngineError
    };

    MatrixResult {
        status,
        profile_hash,
        map_dataset_id: map_dataset_id.to_owned(),
        source_count,
        target_count,
        cells,
        warnings,
    }
}

pub fn normalize_transport_failure(
    profile: &VehicleProfile,
    map_dataset_id: &str,
    failure: TransportFailure,
) -> RouteResult {
    let status = match failure {
        TransportFailure::Connection => NormalizedStatus::EngineUnavailable,
        TransportFailure::Timeout => NormalizedStatus::Timeout,
    };

    empty_route_result(
        status,
        profile.profile_hash(),
        map_dataset_id,
        profile.warnings(),
    )
}

fn matrix_cell(entry: &Value, source_index: usize, target_index: usize) -> MatrixCell {
    let distance_m = entry
        .get("distance")
        .and_then(Value::as_f64)
        .map(|km| km * 1000.0);
    let duration_s = entry.get("time").and_then(Value::as_f64);
    let status = if distance_m.is_some() && duration_s.is_some() {
        NormalizedStatus::Ok
    } else {
        NormalizedStatus::NoRoute
    };

    MatrixCell {
        source_index,
        target_index,
        status,
        distance_m,
        duration_s,
    }
}

fn empty_route_result(
    status: NormalizedStatus,
    profile_hash: String,
    map_dataset_id: &str,
    warnings: Vec<crate::WarningCode>,
) -> RouteResult {
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

fn truck_options(profile: &VehicleProfile) -> Value {
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
    truck.insert(
        "use_truck_route".into(),
        json!(if profile.use_truck_route { 1.0 } else { 0.0 }),
    );
    Value::Object(truck)
}

fn point_json(lat: f64, lon: f64) -> Value {
    json!({
        "lat": lat,
        "lon": lon,
        "search_filter": {
            "exclude_restricted": true,
            "search_cutoff": 100.0
        }
    })
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
    use crate::{GeoPoint, MatrixRequest, TransportFailure, VehicleProfile};

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
            payload["locations"][0]["search_filter"]["exclude_restricted"],
            true
        );
        assert_eq!(
            payload["locations"][0]["search_filter"]["search_cutoff"],
            100.0
        );
    }

    #[test]
    fn matrix_request_uses_same_per_request_profile() {
        let request = MatrixRequest {
            sources: vec![GeoPoint {
                lat: 49.1,
                lon: 24.0,
            }],
            targets: vec![GeoPoint {
                lat: 49.1,
                lon: 24.01,
            }],
            profile: profile(Some(3.1)),
        };
        let payload = matrix_payload(&request);
        assert_eq!(payload["costing"], "truck");
        assert_eq!(payload["costing_options"]["truck"]["height"], 3.1);
        assert_eq!(payload["verbose"], true);
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
    fn matrix_unreachable_cell_is_explicit() {
        let body = r#"{
          "sources_to_targets": [[
            {"distance": 1.25, "time": 90.0, "from_index": 0, "to_index": 0},
            {"distance": null, "time": null, "from_index": 0, "to_index": 1}
          ]]
        }"#;
        let result = normalize_matrix_response(&profile(Some(3.1)), "fixture-a", 1, 2, 200, body);
        assert_eq!(result.status, NormalizedStatus::Ok);
        assert_eq!(result.cells[0].status, NormalizedStatus::Ok);
        assert_eq!(result.cells[1].status, NormalizedStatus::NoRoute);
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

    #[test]
    fn transport_failure_is_not_reported_as_verified_route() {
        let result = normalize_transport_failure(
            &profile(Some(3.1)),
            "fixture-a",
            TransportFailure::Connection,
        );
        assert_eq!(result.status, NormalizedStatus::EngineUnavailable);
        assert_eq!(result.distance_m, None);
    }
}
