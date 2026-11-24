use cargo_geo_probe::valhalla::{
    ENGINE_NAME, ENGINE_VERSION, matrix_payload, normalize_matrix_response, normalize_route_response,
    normalize_transport_failure, route_payload,
};
use cargo_geo_probe::{
    CacheKey, GeoPoint, MatrixRequest, RouteRequest, TransportFailure, VehicleProfile,
};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("request") => request_command(&args[2..]),
        Some("request-via") => request_via_command(&args[2..]),
        Some("normalize") => normalize_command(&args[2..]),
        Some("matrix-request") => matrix_request_command(&args[2..]),
        Some("matrix-normalize") => matrix_normalize_command(&args[2..]),
        Some("cache-key") => cache_key_command(&args[2..]),
        Some("transport-failure") => transport_failure_command(&args[2..]),
        _ => Err(usage()),
    }
}

fn request_command(args: &[String]) -> Result<(), String> {
    if args.len() != 6 {
        return Err(usage());
    }
    let request = RouteRequest {
        from: GeoPoint {
            lat: parse_f64(&args[2])?,
            lon: parse_f64(&args[3])?,
        },
        to: GeoPoint {
            lat: parse_f64(&args[4])?,
            lon: parse_f64(&args[5])?,
        },
        via: vec![],
        profile: profile(&args[0], parse_bool(&args[1])?)?,
    };
    print_json(&route_payload(&request))
}

fn request_via_command(args: &[String]) -> Result<(), String> {
    if args.len() != 8 {
        return Err(usage());
    }
    let request = RouteRequest {
        from: GeoPoint {
            lat: parse_f64(&args[2])?,
            lon: parse_f64(&args[3])?,
        },
        via: vec![GeoPoint {
            lat: parse_f64(&args[4])?,
            lon: parse_f64(&args[5])?,
        }],
        to: GeoPoint {
            lat: parse_f64(&args[6])?,
            lon: parse_f64(&args[7])?,
        },
        profile: profile(&args[0], parse_bool(&args[1])?)?,
    };
    print_json(&route_payload(&request))
}

fn normalize_command(args: &[String]) -> Result<(), String> {
    if args.len() != 5 {
        return Err(usage());
    }
    let profile = profile(&args[0], parse_bool(&args[1])?)?;
    let http_status = parse_u16(&args[2])?;
    let body = read_file(&args[3])?;
    print_json(&normalize_route_response(
        &profile,
        &args[4],
        http_status,
        &body,
    ))
}

fn matrix_request_command(args: &[String]) -> Result<(), String> {
    if args.len() != 4 {
        return Err(usage());
    }
    let sources: Vec<GeoPoint> = serde_json::from_str(&read_file(&args[2])?)
        .map_err(|error| format!("invalid sources JSON: {error}"))?;
    let targets: Vec<GeoPoint> = serde_json::from_str(&read_file(&args[3])?)
        .map_err(|error| format!("invalid targets JSON: {error}"))?;
    let request = MatrixRequest {
        sources,
        targets,
        profile: profile(&args[0], parse_bool(&args[1])?)?,
    };
    print_json(&matrix_payload(&request))
}

fn matrix_normalize_command(args: &[String]) -> Result<(), String> {
    if args.len() != 7 {
        return Err(usage());
    }
    let profile = profile(&args[0], parse_bool(&args[1])?)?;
    let http_status = parse_u16(&args[2])?;
    let body = read_file(&args[3])?;
    let source_count = parse_usize(&args[5])?;
    let target_count = parse_usize(&args[6])?;
    print_json(&normalize_matrix_response(
        &profile,
        &args[4],
        source_count,
        target_count,
        http_status,
        &body,
    ))
}

fn cache_key_command(args: &[String]) -> Result<(), String> {
    if args.len() != 7 {
        return Err(usage());
    }
    let request = RouteRequest {
        from: GeoPoint {
            lat: parse_f64(&args[2])?,
            lon: parse_f64(&args[3])?,
        },
        to: GeoPoint {
            lat: parse_f64(&args[4])?,
            lon: parse_f64(&args[5])?,
        },
        via: vec![],
        profile: profile(&args[0], parse_bool(&args[1])?)?,
    };
    let key = CacheKey::for_route(ENGINE_NAME, ENGINE_VERSION, &args[6], &request);
    print_json(&key)
}

fn transport_failure_command(args: &[String]) -> Result<(), String> {
    if args.len() != 4 {
        return Err(usage());
    }
    let profile = profile(&args[0], parse_bool(&args[1])?)?;
    let failure = match args[2].as_str() {
        "connection" => TransportFailure::Connection,
        "timeout" => TransportFailure::Timeout,
        other => return Err(format!("unknown transport failure: {other}")),
    };
    print_json(&normalize_transport_failure(&profile, &args[3], failure))
}

fn profile(name: &str, exclude_unpaved: bool) -> Result<VehicleProfile, String> {
    let mut profile = match name {
        "P0" => VehicleProfile {
            height_m: None,
            width_m: None,
            length_m: None,
            weight_t: None,
            axle_load_t: None,
            axle_count: None,
            hazmat: false,
            hgv: true,
            exclude_unpaved,
            use_truck_route: false,
        },
        "P1" => complete_profile(2.5, 2.0, 6.0, 3.5, 2.0, 2, false),
        "P2" => complete_profile(3.1, 2.2, 9.0, 10.0, 4.0, 2, false),
        "P3" => complete_profile(4.0, 2.55, 16.5, 24.0, 8.0, 5, false),
        "P4" => complete_profile(4.0, 2.55, 16.5, 24.0, 8.0, 5, true),
        _ => return Err(format!("unknown vehicle profile: {name}")),
    };
    profile.exclude_unpaved = exclude_unpaved;
    Ok(profile)
}

fn complete_profile(
    height_m: f64,
    width_m: f64,
    length_m: f64,
    weight_t: f64,
    axle_load_t: f64,
    axle_count: u8,
    hazmat: bool,
) -> VehicleProfile {
    VehicleProfile {
        height_m: Some(height_m),
        width_m: Some(width_m),
        length_m: Some(length_m),
        weight_t: Some(weight_t),
        axle_load_t: Some(axle_load_t),
        axle_count: Some(axle_count),
        hazmat,
        hgv: true,
        exclude_unpaved: false,
        use_truck_route: false,
    }
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn read_file(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("expected true/false, got {value}")),
    }
}

fn parse_f64(value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|error| format!("invalid number {value}: {error}"))
}

fn parse_u16(value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|error| format!("invalid HTTP status {value}: {error}"))
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid integer {value}: {error}"))
}

fn usage() -> String {
    "route-contract commands: request, request-via, normalize, matrix-request, matrix-normalize, cache-key, transport-failure; profiles: P0..P4".into()
}
