use cargo_dispatch_contracts::{
    CURRENT_SCHEMA_VERSION, SchemaVersion, TRIP_PACKAGE_SCHEMA_NAME, TripPackageHeader, TripVersion,
};
use cargo_dispatch_domain::TripId;

#[test]
fn public_contract_keeps_schema_and_plan_versions_independent() {
    let trip_id = TripId::from_bytes([
        0x91, 0x82, 0x73, 0x64, 0x55, 0x46, 0x37, 0x28, 0x19, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        0x10,
    ])
    .expect("fixture UUID is non-nil");

    let header = TripPackageHeader::new(
        trip_id,
        TripVersion::new(3).expect("valid trip version"),
        Some(TripVersion::new(2).expect("valid base version")),
    )
    .expect("valid package identity");

    assert_eq!(header.schema_name, TRIP_PACKAGE_SCHEMA_NAME);
    assert_eq!(CURRENT_SCHEMA_VERSION, SchemaVersion::new(1, 4));
    assert_eq!(header.trip_version.get(), 3);
    assert_eq!(header.base_version.map(TripVersion::get), Some(2));
}
