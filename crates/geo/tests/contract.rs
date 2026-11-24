use cargo_dispatch_geo::{
    GeoPoint, GeoWarning, MatrixRequest, NormalizedStatus, RouteRequest, RoutingIdentity,
    TollPreference, VehicleDimension, VehicleRoutingProfile,
};

fn truck_profile() -> VehicleRoutingProfile {
    VehicleRoutingProfile {
        height_m: Some(3.5),
        width_m: Some(2.5),
        length_m: Some(10.0),
        gross_weight_t: Some(18.0),
        axle_load_t: Some(7.5),
        axle_count: Some(2),
        hazmat: false,
        hgv: true,
        exclude_unpaved: true,
        prefer_truck_routes: true,
        toll_preference: TollPreference::PreferAvoid,
        max_speed_kmh: Some(90.0),
    }
}

#[test]
fn public_geo_contract_is_engine_independent_and_keeps_profile_identity_external() {
    let start = GeoPoint::new(50.0, 8.0).expect("valid coordinate");
    let end = GeoPoint::new(50.1, 8.1).expect("valid coordinate");

    let route = RouteRequest::new(vec![start, end], truck_profile()).expect("valid route");
    let matrix =
        MatrixRequest::new(vec![start], vec![start, end], truck_profile()).expect("valid matrix");

    assert_eq!(route.points().len(), 2);
    assert_eq!(matrix.sources().len(), 1);
    assert_eq!(matrix.targets().len(), 2);

    let identity = RoutingIdentity::new("routing-engine", "version", "map-build", "fingerprint")
        .expect("non-empty routing identity");
    assert_eq!(identity.engine(), "routing-engine");
    assert_eq!(identity.map_dataset_id(), "map-build");
    assert_eq!(NormalizedStatus::NoRoute, NormalizedStatus::NoRoute);
}

#[test]
fn missing_dimensions_are_explicit_warnings_not_hidden_defaults() {
    let mut profile = truck_profile();
    profile.height_m = None;

    assert_eq!(
        profile.warnings(),
        vec![GeoWarning::MissingVehicleDimension(
            VehicleDimension::Height
        )]
    );
}
