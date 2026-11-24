use cargo_geo_probe::valhalla::{normalize_route_response, route_payload};
use cargo_geo_probe::{GeoPoint, RouteRequest, VehicleProfile};
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
        Some("normalize") => normalize_command(&args[2..]),
        _ => Err(usage()),
    }
}

fn request_command(args: &[String]) -> Result<(), String> {
    if args.len() != 6 {
        return Err(usage());
    }
    let profile = profile(&args[0], parse_bool(&args[1])?)?;
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
        profile,
    };
    println!(
        "{}",
        serde_json::to_string(&route_payload(&request)).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn normalize_command(args: &[String]) -> Result<(), String> {
    if args.len() != 5 {
        return Err(usage());
    }
    let profile = profile(&args[0], parse_bool(&args[1])?)?;
    let http_status = args[2]
        .parse::<u16>()
        .map_err(|error| format!("invalid HTTP status: {error}"))?;
    let body = fs::read_to_string(&args[3])
        .map_err(|error| format!("cannot read response {}: {error}", args[3]))?;
    let result = normalize_route_response(&profile, &args[4], http_status, &body);
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn profile(name: &str, exclude_unpaved: bool) -> Result<VehicleProfile, String> {
    let (height_m, width_m, length_m, weight_t, axle_load_t, axle_count, hazmat) =
        match name {
            "P1" => (2.5, 2.0, 6.0, 3.5, 2.0, 2, false),
            "P2" => (3.1, 2.2, 9.0, 10.0, 4.0, 2, false),
            "P3" => (4.0, 2.55, 16.5, 24.0, 8.0, 5, false),
            "P4" => (4.0, 2.55, 16.5, 24.0, 8.0, 5, true),
            _ => return Err(format!("unknown vehicle profile: {name}")),
        };
    Ok(VehicleProfile {
        height_m: Some(height_m),
        width_m: Some(width_m),
        length_m: Some(length_m),
        weight_t: Some(weight_t),
        axle_load_t: Some(axle_load_t),
        axle_count: Some(axle_count),
        hazmat,
        hgv: true,
        exclude_unpaved,
        use_truck_route: false,
    })
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

fn usage() -> String {
    "usage: route-contract request <P1|P2|P3|P4> <exclude_unpaved> <from_lat> <from_lon> <to_lat> <to_lon>\n       route-contract normalize <P1|P2|P3|P4> <exclude_unpaved> <http_status> <response_file> <dataset_id>".into()
}
