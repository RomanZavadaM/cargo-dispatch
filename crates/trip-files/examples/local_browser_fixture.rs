#![forbid(unsafe_code)]

use cargo_dispatch_contracts::{
    TripNavigationTarget, TripPackageHeader, TripPointKind, TripRoutePoint, TripVersion,
};
use cargo_dispatch_domain::{OperatorId, PointId, TripId, WorkingRouteId};
use cargo_dispatch_planner::PlannerTripExportDraft;
use cargo_dispatch_trip_files::{TripFolder, export_package};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

fn point(
    byte: u8,
    sequence: u32,
    title: &str,
    latitude_e7: i32,
    longitude_e7: i32,
) -> TripRoutePoint {
    TripRoutePoint {
        point_id: PointId::from_bytes([byte; 16]).expect("synthetic non-nil point id"),
        sequence,
        kind: TripPointKind::Load,
        title: title.to_owned(),
        navigation: Some(TripNavigationTarget {
            latitude_e7,
            longitude_e7,
            address: Some(format!("Synthetic stop {sequence}")),
        }),
    }
}

fn write_sample_pdf(path: &Path) -> io::Result<()> {
    let stream = "BT /F1 18 Tf 72 720 Td (Cargo Dispatch local browser test PDF) Tj ET\n";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
    ];

    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );

    let mut file = fs::File::create(path)?;
    file.write_all(&bytes)?;
    file.sync_all()
}

fn main() -> io::Result<()> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("local-browser-test-data"));

    if output.exists() {
        fs::remove_dir_all(&output)?;
    }
    fs::create_dir_all(&output)?;

    let trip_id = TripId::from_bytes([0x41; 16]).expect("synthetic trip id");
    let route = vec![
        point(0x51, 1, "Synthetic depot", 498_393_000, 240_296_000),
        point(0x52, 2, "Synthetic customer A", 498_430_000, 240_360_000),
        point(0x53, 3, "Synthetic customer B", 498_485_000, 240_250_000),
        point(0x54, 4, "Synthetic customer C", 498_520_000, 240_410_000),
    ];

    let package = output.join("synthetic-trip-package.zip");
    let active = output.join("active-trip");
    let draft = PlannerTripExportDraft {
        header: TripPackageHeader::new(
            trip_id,
            TripVersion::new(1).expect("synthetic trip version"),
            None,
        )
        .expect("synthetic package header"),
        working_route_id: WorkingRouteId::from_bytes([0x42; 16]).expect("synthetic route id"),
        working_route_revision: 1,
        created_at_unix_ms: 1_795_550_000_000,
        created_by: OperatorId::from_bytes([0x43; 16]).expect("synthetic operator id"),
        ordered_point_ids: route.iter().map(|item| item.point_id).collect(),
    };

    export_package(&package, &draft, &route)?;
    TripFolder::import(&active, &package)?;
    write_sample_pdf(&output.join("sample-control-to-driver.pdf"))?;

    fs::write(
        output.join("README.txt"),
        "Synthetic Cargo Dispatch local-browser fixture.\r\n\
Select the active-trip folder in Control.\r\n\
All route points, IDs and coordinates are synthetic test data.\r\n",
    )?;

    println!("{}", active.display());
    Ok(())
}
