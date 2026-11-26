#![forbid(unsafe_code)]

use cargo_dispatch_contracts::{
    CURRENT_SCHEMA_VERSION, PackageContentFingerprint, ReplanRequest, TRIP_PACKAGE_SCHEMA_NAME,
    TripNavigationTarget, TripPackageHeader, TripPointKind, TripRoadGeometry,
    TripRoadGeometryPoint, TripRoutePoint, TripRoutingIdentity, TripVersion, validate_route_points,
    validate_trip_road_geometry,
};
use cargo_dispatch_control::{
    ImportResult, InMemoryTripRegistry, ReplanBuildError, ReplanRequestDraft,
    RouteGeometryUpdateError, StoredTripPlan, TripImportCandidate,
};
use cargo_dispatch_domain::{DocumentId, PointId, TripId};
use cargo_dispatch_geo::VehicleRoutingProfile;
use cargo_dispatch_planner::PlannerTripExportDraft;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const LIMIT: u64 = 4 * 1024 * 1024;

fn invalid(message: impl std::fmt::Debug) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("{message:?}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentDirection {
    FromOperator,
    FromDriver,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDocument {
    pub document_id: DocumentId,
    pub direction: DocumentDirection,
    pub original_name: String,
    pub stored_name: String,
    pub mime_type: String,
    pub size: u64,
    pub sha256: [u8; 32],
    pub created_at_unix_ms: u64,
    pub point_id: Option<PointId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentStoreDraft {
    pub document_id: DocumentId,
    pub direction: DocumentDirection,
    pub original_name: String,
    pub mime_type: String,
    pub created_at_unix_ms: u64,
    pub point_id: Option<PointId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriverLocationSample {
    pub latitude_e7: i32,
    pub longitude_e7: i32,
    pub accuracy_cm: Option<u32>,
    pub observed_at_unix_ms: u64,
}

impl DriverLocationSample {
    pub fn validate(self) -> io::Result<Self> {
        if !(-900_000_000..=900_000_000).contains(&self.latitude_e7) {
            return Err(invalid("latitude out of range"));
        }
        if !(-1_800_000_000..=1_800_000_000).contains(&self.longitude_e7) {
            return Err(invalid("longitude out of range"));
        }
        if self.observed_at_unix_ms == 0 {
            return Err(invalid("location timestamp is required"));
        }
        Ok(self)
    }

    #[must_use]
    pub fn latitude(self) -> f64 {
        f64::from(self.latitude_e7) / 10_000_000.0
    }

    #[must_use]
    pub fn longitude(self) -> f64 {
        f64::from(self.longitude_e7) / 10_000_000.0
    }

    #[must_use]
    pub fn accuracy_m(self) -> Option<f64> {
        self.accuracy_cm.map(|value| f64::from(value) / 100.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSummary {
    pub trip_id: TripId,
    pub final_plan_version: TripVersion,
    pub completed_at_unix_ms: u64,
    pub completed_point_ids: Vec<PointId>,
    pub documents_from_operator: u64,
    pub documents_from_driver: u64,
    pub plan_version_count: u64,
    pub replan_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionSummaryFile {
    trip_id: [u8; 16],
    final_plan_version: u64,
    completed_at_unix_ms: u64,
    completed_point_ids: Vec<[u8; 16]>,
    documents_from_operator: u64,
    documents_from_driver: u64,
    plan_version_count: u64,
    replan_count: u64,
}

impl ExecutionSummaryFile {
    fn typed(&self) -> io::Result<ExecutionSummary> {
        Ok(ExecutionSummary {
            trip_id: TripId::from_bytes(self.trip_id).map_err(invalid)?,
            final_plan_version: TripVersion::new(self.final_plan_version).map_err(invalid)?,
            completed_at_unix_ms: self.completed_at_unix_ms,
            completed_point_ids: self
                .completed_point_ids
                .iter()
                .copied()
                .map(PointId::from_bytes)
                .collect::<Result<Vec<_>, _>>()
                .map_err(invalid)?,
            documents_from_operator: self.documents_from_operator,
            documents_from_driver: self.documents_from_driver,
            plan_version_count: self.plan_version_count,
            replan_count: self.replan_count,
        })
    }
}

impl From<&ExecutionSummary> for ExecutionSummaryFile {
    fn from(summary: &ExecutionSummary) -> Self {
        Self {
            trip_id: summary.trip_id.as_bytes(),
            final_plan_version: summary.final_plan_version.get(),
            completed_at_unix_ms: summary.completed_at_unix_ms,
            completed_point_ids: summary
                .completed_point_ids
                .iter()
                .copied()
                .map(PointId::as_bytes)
                .collect(),
            documents_from_operator: summary.documents_from_operator,
            documents_from_driver: summary.documents_from_driver,
            plan_version_count: summary.plan_version_count,
            replan_count: summary.replan_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentEntry {
    document_id: [u8; 16],
    direction: DocumentDirection,
    original_name: String,
    stored_name: String,
    mime_type: String,
    size: u64,
    sha256: [u8; 32],
    created_at_unix_ms: u64,
    point_id: Option<[u8; 16]>,
}

impl DocumentEntry {
    fn record(&self) -> io::Result<StoredDocument> {
        Ok(StoredDocument {
            document_id: DocumentId::from_bytes(self.document_id).map_err(invalid)?,
            direction: self.direction,
            original_name: self.original_name.clone(),
            stored_name: self.stored_name.clone(),
            mime_type: self.mime_type.clone(),
            size: self.size,
            sha256: self.sha256,
            created_at_unix_ms: self.created_at_unix_ms,
            point_id: self
                .point_id
                .map(PointId::from_bytes)
                .transpose()
                .map_err(invalid)?,
        })
    }
}

fn id_hex(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn safe_extension(original_name: &str) -> Option<String> {
    let extension = Path::new(original_name).extension()?.to_str()?;
    if extension.is_empty()
        || extension.len() > 12
        || !extension.chars().all(|ch| ch.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(extension.to_ascii_lowercase())
}

fn document_subdir(direction: DocumentDirection) -> &'static str {
    match direction {
        DocumentDirection::FromOperator => "from_operator",
        DocumentDirection::FromDriver => "from_driver",
    }
}

fn stored_document_path(root: &Path, document: &StoredDocument) -> PathBuf {
    root.join("documents")
        .join(document_subdir(document.direction))
        .join(&document.stored_name)
}

fn hash_file(path: &Path) -> io::Result<(u64, [u8; 32])> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size = size
            .checked_add(u64::try_from(read).map_err(invalid)?)
            .ok_or_else(|| invalid("document size overflow"))?;
    }
    Ok((size, hasher.finalize().into()))
}

fn hash_hex(hash: [u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn safe_archive_name(name: &str) -> io::Result<()> {
    let path = Path::new(name);
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(_)), None) if !name.trim().is_empty() => Ok(()),
        _ => Err(invalid("archive name must be one safe path component")),
    }
}

fn collect_checksum_files(root: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    fn walk(root: &Path, current: &Path, out: &mut Vec<(String, PathBuf)>) -> io::Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let path = entry.path();
            if file_type.is_symlink() {
                return Err(invalid(
                    "symlinks are not allowed in completed trip package",
                ));
            }
            if file_type.is_dir() {
                if path == root.join("checksums") {
                    continue;
                }
                walk(root, &path, out)?;
                continue;
            }
            if !file_type.is_file() {
                return Err(invalid("unsupported filesystem entry in trip package"));
            }
            let relative = path.strip_prefix(root).map_err(invalid)?;
            let mut logical = String::new();
            for component in relative.components() {
                let std::path::Component::Normal(component) = component else {
                    return Err(invalid("invalid trip package path"));
                };
                let text = component
                    .to_str()
                    .ok_or_else(|| invalid("trip package filename must be UTF-8"))?;
                if !logical.is_empty() {
                    logical.push('/');
                }
                logical.push_str(text);
            }
            out.push((logical, path));
        }
        Ok(())
    }

    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn write_checksums(root: &Path) -> io::Result<()> {
    let files = collect_checksum_files(root)?;
    let directory = root.join("checksums");
    fs::create_dir_all(&directory)?;
    let destination = directory.join("sha256.txt");
    if destination.try_exists()? {
        return verify_checksums(root);
    }
    let mut temporary = NamedTempFile::new_in(&directory)?;
    for (relative, path) in files {
        let (_, hash) = hash_file(&path)?;
        writeln!(temporary, "{}  {}", hash_hex(hash), relative)?;
    }
    publish(temporary, &destination)
}

fn write_execution_summary(root: &Path, summary: &ExecutionSummary) -> io::Result<()> {
    let directory = root.join("reports");
    fs::create_dir_all(&directory)?;
    let destination = directory.join("execution_summary.json");
    let expected = ExecutionSummaryFile::from(summary);
    if destination.try_exists()? {
        let existing: ExecutionSummaryFile =
            serde_json::from_slice(&bounded(File::open(&destination)?)?)?;
        if existing == expected {
            return Ok(());
        }
        return Err(invalid(
            "execution summary already exists with different content",
        ));
    }
    let bytes = serde_json::to_vec_pretty(&expected)?;
    let mut temporary = NamedTempFile::new_in(&directory)?;
    temporary.write_all(&bytes)?;
    publish(temporary, &destination)
}

fn parse_hash_hex(value: &str) -> io::Result<[u8; 32]> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid("invalid SHA-256 in checksum index"));
    }
    let mut hash = [0_u8; 32];
    for (index, byte) in hash.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16).map_err(invalid)?;
    }
    Ok(hash)
}

fn verify_checksums(root: &Path) -> io::Result<()> {
    let index = fs::read_to_string(root.join("checksums").join("sha256.txt"))?;
    let mut expected = std::collections::BTreeMap::new();
    for line in index.lines() {
        let Some((hash, relative)) = line.split_once("  ") else {
            return Err(invalid("invalid checksum index line"));
        };
        if expected
            .insert(relative.to_owned(), parse_hash_hex(hash)?)
            .is_some()
        {
            return Err(invalid("duplicate checksum path"));
        }
    }

    let files = collect_checksum_files(root)?;
    if expected.len() != files.len() {
        return Err(invalid("checksum index file count mismatch"));
    }
    for (relative, path) in files {
        let Some(expected_hash) = expected.remove(&relative) else {
            return Err(invalid("file missing from checksum index"));
        };
        let (_, actual_hash) = hash_file(&path)?;
        if actual_hash != expected_hash {
            return Err(invalid("completed trip checksum mismatch"));
        }
    }
    if !expected.is_empty() {
        return Err(invalid("checksum index references missing file"));
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(remote = "TripPointKind")]
enum Kind {
    Load,
    Unload,
    CombinedLoadUnload,
    Rest,
    Break,
    Service,
    BorderCustoms,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Navigation {
    latitude_e7: i32,
    longitude_e7: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<String>,
}

impl From<&TripNavigationTarget> for Navigation {
    fn from(target: &TripNavigationTarget) -> Self {
        Self {
            latitude_e7: target.latitude_e7,
            longitude_e7: target.longitude_e7,
            address: target.address.clone(),
        }
    }
}

impl From<&Navigation> for TripNavigationTarget {
    fn from(target: &Navigation) -> Self {
        Self {
            latitude_e7: target.latitude_e7,
            longitude_e7: target.longitude_e7,
            address: target.address.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeometryPoint {
    latitude_e7: i32,
    longitude_e7: i32,
}

impl From<&TripRoadGeometryPoint> for GeometryPoint {
    fn from(point: &TripRoadGeometryPoint) -> Self {
        Self {
            latitude_e7: point.latitude_e7,
            longitude_e7: point.longitude_e7,
        }
    }
}

impl From<&GeometryPoint> for TripRoadGeometryPoint {
    fn from(point: &GeometryPoint) -> Self {
        Self {
            latitude_e7: point.latitude_e7,
            longitude_e7: point.longitude_e7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutingIdentityFile {
    engine: String,
    engine_version: String,
    map_dataset_id: String,
    profile_fingerprint: String,
}

impl From<&TripRoutingIdentity> for RoutingIdentityFile {
    fn from(identity: &TripRoutingIdentity) -> Self {
        Self {
            engine: identity.engine.clone(),
            engine_version: identity.engine_version.clone(),
            map_dataset_id: identity.map_dataset_id.clone(),
            profile_fingerprint: identity.profile_fingerprint.clone(),
        }
    }
}

impl From<&RoutingIdentityFile> for TripRoutingIdentity {
    fn from(identity: &RoutingIdentityFile) -> Self {
        Self {
            engine: identity.engine.clone(),
            engine_version: identity.engine_version.clone(),
            map_dataset_id: identity.map_dataset_id.clone(),
            profile_fingerprint: identity.profile_fingerprint.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoadGeometryFile {
    routing: RoutingIdentityFile,
    points: Vec<GeometryPoint>,
}

impl From<&TripRoadGeometry> for RoadGeometryFile {
    fn from(geometry: &TripRoadGeometry) -> Self {
        Self {
            routing: RoutingIdentityFile::from(&geometry.routing),
            points: geometry.points.iter().map(GeometryPoint::from).collect(),
        }
    }
}

impl From<&RoadGeometryFile> for TripRoadGeometry {
    fn from(geometry: &RoadGeometryFile) -> Self {
        Self {
            routing: TripRoutingIdentity::from(&geometry.routing),
            points: geometry
                .points
                .iter()
                .map(TripRoadGeometryPoint::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Point {
    id: [u8; 16],
    #[serde(with = "Kind")]
    kind: TripPointKind,
    title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    navigation: Option<Navigation>,
}
impl From<&TripRoutePoint> for Point {
    fn from(p: &TripRoutePoint) -> Self {
        Self {
            id: p.point_id.as_bytes(),
            kind: p.kind.clone(),
            title: p.title.clone(),
            navigation: p.navigation.as_ref().map(Navigation::from),
        }
    }
}

fn points(rows: &[Point]) -> io::Result<Vec<TripRoutePoint>> {
    rows.iter()
        .enumerate()
        .map(|(i, p)| {
            Ok(TripRoutePoint {
                point_id: PointId::from_bytes(p.id).map_err(invalid)?,
                sequence: u32::try_from(i + 1).map_err(invalid)?,
                kind: p.kind.clone(),
                title: p.title.clone(),
                navigation: p.navigation.as_ref().map(TripNavigationTarget::from),
            })
        })
        .collect()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    schema_version: [u16; 2],
    trip: [u8; 16],
    version: u64,
    base: Option<u64>,
    working_route: [u8; 16],
    revision: u64,
    created_at_unix_ms: u64,
    operator: [u8; 16],
    points: Vec<Point>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    routing_profile: Option<VehicleRoutingProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    road_geometry: Option<RoadGeometryFile>,
}

impl Manifest {
    fn validated_routing_profile(&self) -> io::Result<Option<VehicleRoutingProfile>> {
        let Some(profile) = self.routing_profile.clone() else {
            return Ok(None);
        };
        profile.validate().map_err(invalid)?;
        if let Some(geometry) = &self.road_geometry
            && geometry.routing.profile_fingerprint != profile.profile_fingerprint()
        {
            return Err(invalid("routing profile does not match geometry fingerprint"));
        }
        Ok(Some(profile))
    }

    fn candidate(&self) -> io::Result<TripImportCandidate> {
        if self.schema != TRIP_PACKAGE_SCHEMA_NAME
            || self.schema_version[0] != CURRENT_SCHEMA_VERSION.major
            || self.schema_version[1] > CURRENT_SCHEMA_VERSION.minor
        {
            return Err(invalid("unsupported trip package schema"));
        }
        let mut header = TripPackageHeader::new(
            TripId::from_bytes(self.trip).map_err(invalid)?,
            TripVersion::new(self.version).map_err(invalid)?,
            self.base
                .map(TripVersion::new)
                .transpose()
                .map_err(invalid)?,
        )
        .map_err(invalid)?;
        header.schema_version = cargo_dispatch_contracts::SchemaVersion::new(
            self.schema_version[0],
            self.schema_version[1],
        );
        let route_points = points(&self.points)?;
        validate_route_points(&route_points).map_err(invalid)?;
        self.validated_routing_profile()?;
        let route_geometry = self.road_geometry.as_ref().map(TripRoadGeometry::from);
        if let Some(geometry) = &route_geometry {
            validate_trip_road_geometry(geometry).map_err(invalid)?;
        }
        let hash = Sha256::digest(serde_json::to_vec(self)?);
        Ok(TripImportCandidate {
            header,
            route_points,
            route_geometry,
            content_fingerprint: PackageContentFingerprint::from_bytes(hash.into()),
        })
    }
}

fn publish(file: NamedTempFile, path: &Path) -> io::Result<()> {
    file.as_file().sync_all()?;
    file.persist_noclobber(path).map_err(|err| err.error)?;
    #[cfg(unix)]
    File::open(parent(path)?)?.sync_all()?;
    Ok(())
}

fn parent(path: &Path) -> io::Result<&Path> {
    Ok(path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new(".")))
}

pub fn export_package(
    path: &Path,
    draft: &PlannerTripExportDraft,
    route: &[TripRoutePoint],
) -> io::Result<()> {
    validate_route_points(route).map_err(invalid)?;
    if draft
        .ordered_point_ids
        .iter()
        .copied()
        .ne(route.iter().map(|p| p.point_id))
    {
        return Err(invalid("route differs from Planner export"));
    }
    let h = &draft.header;
    if draft.routing_profile.is_some() && h.schema_version.minor < 3 {
        return Err(invalid("routing profile requires trip package schema 1.3"));
    }
    let manifest = Manifest {
        schema: h.schema_name.clone(),
        schema_version: [h.schema_version.major, h.schema_version.minor],
        trip: h.trip_id.as_bytes(),
        version: h.trip_version.get(),
        base: h.base_version.map(TripVersion::get),
        working_route: draft.working_route_id.as_bytes(),
        revision: draft.working_route_revision,
        created_at_unix_ms: draft.created_at_unix_ms,
        operator: draft.created_by.as_bytes(),
        points: route.iter().map(Point::from).collect(),
        routing_profile: draft.routing_profile.clone(),
        road_geometry: draft.route_geometry.as_ref().map(RoadGeometryFile::from),
    };
    manifest.candidate()?;
    let data = serde_json::to_vec(&manifest)?;
    if data.len() as u64 > LIMIT {
        return Err(invalid("manifest too large"));
    }
    let mut file = NamedTempFile::new_in(parent(path)?)?;
    let mut zip = ZipWriter::new(file.as_file_mut());
    zip.start_file(
        "plan.json",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    zip.write_all(&data)?;
    zip.finish()?;
    publish(file, path)
}

fn bounded(reader: impl Read) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    reader.take(LIMIT + 1).read_to_end(&mut data)?;
    if data.len() as u64 > LIMIT {
        return Err(invalid("file too large"));
    }
    Ok(data)
}

fn read_package(path: &Path) -> io::Result<Manifest> {
    let file = File::open(path)?;
    if file.metadata()?.len() > LIMIT + 65536 {
        return Err(invalid("archive too large"));
    }
    let mut zip = ZipArchive::new(file)?;
    if zip.len() != 1 {
        return Err(invalid("expected plan.json only"));
    }
    Ok(serde_json::from_slice(&bounded(
        zip.by_name("plan.json")?,
    )?)?)
}

#[derive(Debug, Serialize, Deserialize)]
enum Change {
    Import(Manifest),
    Complete([u8; 16]),
    OperatorEdit(Vec<Point>),
    DriverOrder(Vec<[u8; 16]>),
    RouteGeometry {
        local_route_revision: u64,
        geometry: RoadGeometryFile,
    },
    DocumentStored(DocumentEntry),
    DriverLocation(DriverLocationSample),
}

fn apply(
    reg: &mut InMemoryTripRegistry,
    routing_profile: &mut Option<VehicleRoutingProfile>,
    documents: &mut std::collections::BTreeMap<DocumentId, StoredDocument>,
    locations: &mut Vec<DriverLocationSample>,
    trip: TripId,
    event: &Change,
) -> io::Result<()> {
    match event {
        Change::Import(m) => {
            if m.trip != trip.as_bytes() {
                return Err(invalid("different trip"));
            }
            let incoming_profile = m.validated_routing_profile()?;
            let result = reg.import(m.candidate()?).map_err(invalid)?;
            if matches!(
                result,
                ImportResult::AppliedFirstVersion
                    | ImportResult::AppliedNewerVersion { .. }
                    | ImportResult::IdempotentSameVersion
            ) {
                *routing_profile = incoming_profile;
            }
        }
        Change::Complete(id) => {
            reg.mark_point_completed(trip, PointId::from_bytes(*id).map_err(invalid)?)
                .map_err(invalid)?;
        }
        Change::OperatorEdit(rows) => {
            let current = reg.get(trip).ok_or_else(|| invalid("missing trip"))?;
            reg.edit_active_route(&current, points(rows)?)
                .map_err(invalid)?;
        }
        Change::DriverOrder(order) => {
            let current = reg.get(trip).ok_or_else(|| invalid("missing trip"))?;
            let mut open: std::collections::BTreeMap<_, _> = current
                .route_points
                .iter()
                .filter(|p| !current.completed_point_ids.contains(&p.point_id))
                .map(|p| (p.point_id.as_bytes(), p))
                .collect();
            if open.len() != order.len() {
                return Err(invalid("expected every open point"));
            }
            let mut proposed = current.route_points.clone();
            for (slot, id) in proposed
                .iter_mut()
                .filter(|p| !current.completed_point_ids.contains(&p.point_id))
                .zip(order)
            {
                let sequence = slot.sequence;
                *slot = open
                    .remove(id)
                    .ok_or_else(|| invalid("invalid open point"))?
                    .clone();
                slot.sequence = sequence;
            }
            reg.edit_active_route(&current, proposed).map_err(invalid)?;
        }
        Change::RouteGeometry {
            local_route_revision,
            geometry,
        } => {
            let current = reg.get(trip).ok_or_else(|| invalid("missing trip"))?;
            if current.local_route_revision != *local_route_revision {
                return Err(invalid("stale route geometry revision"));
            }
            let geometry = TripRoadGeometry::from(geometry);
            reg.update_route_geometry(&current, geometry)
                .map_err(|error| match error {
                    RouteGeometryUpdateError::TripNotFound => invalid("missing trip"),
                    RouteGeometryUpdateError::StaleView => invalid("stale route geometry state"),
                    RouteGeometryUpdateError::InvalidGeometry(error) => invalid(error),
                })?;
        }
        Change::DocumentStored(entry) => {
            let document = entry.record()?;
            match documents.get(&document.document_id) {
                Some(existing) if existing == &document => {}
                Some(_) => return Err(invalid("document id metadata conflict")),
                None => {
                    documents.insert(document.document_id, document);
                }
            }
        }
        Change::DriverLocation(sample) => {
            let sample = sample.validate()?;
            if locations.last().copied() != Some(sample) {
                locations.push(sample);
            }
        }
    }
    Ok(())
}

pub struct TripFolder {
    path: PathBuf,
    trip: TripId,
    cursor: u64,
    plan_version_count: u64,
    registry: InMemoryTripRegistry,
    routing_profile: Option<VehicleRoutingProfile>,
    documents: std::collections::BTreeMap<DocumentId, StoredDocument>,
    locations: Vec<DriverLocationSample>,
}

impl TripFolder {
    fn load(path: &Path, trip: TripId) -> io::Result<Self> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                entries.push(path);
            }
        }
        entries.sort();
        let mut this = Self {
            path: path.to_owned(),
            trip,
            cursor: 0,
            plan_version_count: 0,
            registry: InMemoryTripRegistry::default(),
            routing_profile: None,
            documents: std::collections::BTreeMap::new(),
            locations: Vec::new(),
        };
        for entry in entries {
            if entry != this.next_path()? {
                return Err(invalid("incomplete trip history"));
            }
            let event: Change = serde_json::from_slice(&bounded(File::open(entry)?)?)?;
            let before = this.registry.get(trip);
            let is_import = matches!(&event, Change::Import(_));
            apply(
                &mut this.registry,
                &mut this.routing_profile,
                &mut this.documents,
                &mut this.locations,
                trip,
                &event,
            )?;
            if is_import && this.registry.get(trip) != before {
                this.plan_version_count = this
                    .plan_version_count
                    .checked_add(1)
                    .ok_or_else(|| invalid("plan version count overflow"))?;
            }
            this.cursor += 1;
        }
        for document in this.documents.values() {
            let (size, sha256) = hash_file(&stored_document_path(path, document))?;
            if size != document.size || sha256 != document.sha256 {
                return Err(invalid("stored document integrity mismatch"));
            }
        }
        Ok(this)
    }

    pub fn open(path: &Path, trip: TripId) -> io::Result<Self> {
        let this = Self::load(path, trip)?;
        if this.cursor == 0 {
            return Err(invalid("empty trip folder"));
        }
        Ok(this)
    }

    pub fn open_detected(path: &Path) -> io::Result<Self> {
        let first = path.join("00000000000000000001.json");
        let event: Change = serde_json::from_slice(&bounded(File::open(first)?)?)?;
        let Change::Import(manifest) = event else {
            return Err(invalid("trip history must start with import"));
        };
        let trip = manifest.candidate()?.header.trip_id;
        Self::open(path, trip)
    }

    #[must_use]
    pub const fn trip_id(&self) -> TripId {
        self.trip
    }

    pub fn import(path: &Path, package: &Path) -> io::Result<Self> {
        let plan = read_package(package)?;
        let trip = plan.candidate()?.header.trip_id;
        fs::create_dir_all(path)?;
        let mut this = Self::load(path, trip)?;
        this.record(Change::Import(plan))?;
        Ok(this)
    }

    pub fn state(&self) -> StoredTripPlan {
        self.registry
            .get(self.trip)
            .expect("folder contains imported trip")
    }

    #[must_use]
    pub fn routing_profile(&self) -> Option<&VehicleRoutingProfile> {
        self.routing_profile.as_ref()
    }

    #[must_use]
    pub fn documents(&self) -> Vec<StoredDocument> {
        self.documents.values().cloned().collect()
    }

    #[must_use]
    pub fn document(&self, document_id: DocumentId) -> Option<StoredDocument> {
        self.documents.get(&document_id).cloned()
    }

    pub fn verified_document_path(
        &self,
        document_id: DocumentId,
    ) -> io::Result<(StoredDocument, PathBuf)> {
        let document = self
            .documents
            .get(&document_id)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "document not found"))?;
        let path = stored_document_path(&self.path, &document);
        let (size, sha256) = hash_file(&path)?;
        if size != document.size || sha256 != document.sha256 {
            return Err(invalid("stored document integrity mismatch"));
        }
        Ok((document, path))
    }

    #[must_use]
    pub fn locations(&self) -> &[DriverLocationSample] {
        &self.locations
    }

    #[must_use]
    pub fn latest_location(&self) -> Option<DriverLocationSample> {
        self.locations.last().copied()
    }

    fn execution_summary(&self, completed_at_unix_ms: u64) -> io::Result<ExecutionSummary> {
        let state = self.state();
        let documents_from_operator = u64::try_from(
            self.documents
                .values()
                .filter(|document| document.direction == DocumentDirection::FromOperator)
                .count(),
        )
        .map_err(invalid)?;
        let documents_from_driver = u64::try_from(
            self.documents
                .values()
                .filter(|document| document.direction == DocumentDirection::FromDriver)
                .count(),
        )
        .map_err(invalid)?;

        Ok(ExecutionSummary {
            trip_id: self.trip,
            final_plan_version: state.current_version,
            completed_at_unix_ms,
            completed_point_ids: state.completed_point_ids.iter().copied().collect(),
            documents_from_operator,
            documents_from_driver,
            plan_version_count: self.plan_version_count,
            replan_count: self.plan_version_count.saturating_sub(1),
        })
    }

    pub fn build_replan_request(
        &self,
        draft: ReplanRequestDraft,
    ) -> Result<ReplanRequest, ReplanBuildError> {
        self.registry.build_replan_request(self.trip, draft)
    }

    fn next_path(&self) -> io::Result<PathBuf> {
        let n = self
            .cursor
            .checked_add(1)
            .ok_or_else(|| invalid("history exhausted"))?;
        Ok(self.path.join(format!("{n:020}.json")))
    }

    fn record(&mut self, event: Change) -> io::Result<()> {
        let path = self.next_path()?;
        if path.try_exists()? {
            return Err(invalid("trip changed; reopen folder"));
        }
        let mut updated = self.registry.clone();
        let mut updated_routing_profile = self.routing_profile.clone();
        let mut updated_documents = self.documents.clone();
        let mut updated_locations = self.locations.clone();
        let is_import = matches!(&event, Change::Import(_));
        apply(
            &mut updated,
            &mut updated_routing_profile,
            &mut updated_documents,
            &mut updated_locations,
            self.trip,
            &event,
        )?;
        let registry_changed = updated.get(self.trip) != self.registry.get(self.trip);
        if !registry_changed
            && updated_routing_profile == self.routing_profile
            && updated_documents == self.documents
            && updated_locations == self.locations
        {
            return Ok(());
        }
        let bytes = serde_json::to_vec(&event)?;
        if bytes.len() as u64 > LIMIT {
            return Err(invalid("trip entry too large"));
        }
        let mut file = NamedTempFile::new_in(&self.path)?;
        file.write_all(&bytes)?;
        publish(file, &path)?;
        self.registry = updated;
        self.routing_profile = updated_routing_profile;
        self.documents = updated_documents;
        self.locations = updated_locations;
        if is_import && registry_changed {
            self.plan_version_count = self
                .plan_version_count
                .checked_add(1)
                .ok_or_else(|| invalid("plan version count overflow"))?;
        }
        self.cursor += 1;
        Ok(())
    }

    pub fn complete(&mut self, point: PointId) -> io::Result<()> {
        self.record(Change::Complete(point.as_bytes()))
    }

    pub fn edit(&mut self, shown: &StoredTripPlan, route: &[TripRoutePoint]) -> io::Result<()> {
        if self.state() != *shown {
            return Err(invalid("stale route"));
        }
        validate_route_points(route).map_err(invalid)?;
        self.record(Change::OperatorEdit(
            route.iter().map(Point::from).collect(),
        ))
    }

    pub fn driver_order(&mut self, shown: &StoredTripPlan, order: &[PointId]) -> io::Result<()> {
        if self.state() != *shown {
            return Err(invalid("stale route"));
        }
        self.record(Change::DriverOrder(
            order.iter().map(|p| p.as_bytes()).collect(),
        ))
    }

    pub fn set_route_geometry(
        &mut self,
        displayed: &StoredTripPlan,
        geometry: &TripRoadGeometry,
    ) -> io::Result<()> {
        if self.state() != *displayed {
            return Err(invalid("stale route geometry state"));
        }
        validate_trip_road_geometry(geometry).map_err(invalid)?;
        self.record(Change::RouteGeometry {
            local_route_revision: displayed.local_route_revision,
            geometry: RoadGeometryFile::from(geometry),
        })
    }

    pub fn record_location(&mut self, sample: DriverLocationSample) -> io::Result<()> {
        let sample = sample.validate()?;
        if let Some(latest) = self.latest_location() {
            if latest == sample {
                return Ok(());
            }
            if sample.observed_at_unix_ms <= latest.observed_at_unix_ms {
                return Err(invalid("location timestamp must advance"));
            }
        }
        self.record(Change::DriverLocation(sample))
    }

    pub fn store_document(
        &mut self,
        draft: DocumentStoreDraft,
        source: &Path,
    ) -> io::Result<StoredDocument> {
        if draft.original_name.trim().is_empty() {
            return Err(invalid("document original name is required"));
        }
        if draft.mime_type.trim().is_empty() {
            return Err(invalid("document mime type is required"));
        }
        if let Some(point_id) = draft.point_id
            && !self
                .state()
                .route_points
                .iter()
                .any(|point| point.point_id == point_id)
        {
            return Err(invalid("document point does not belong to trip"));
        }

        let directory = self
            .path
            .join("documents")
            .join(document_subdir(draft.direction));
        fs::create_dir_all(&directory)?;

        let extension = safe_extension(&draft.original_name);
        let stored_name = match extension {
            Some(extension) => format!("{}.{}", id_hex(draft.document_id.as_bytes()), extension),
            None => id_hex(draft.document_id.as_bytes()),
        };
        let destination = directory.join(&stored_name);

        let mut source_file = File::open(source)?;
        if !source_file.metadata()?.is_file() {
            return Err(invalid("document source must be a regular file"));
        }

        let mut temporary = NamedTempFile::new_in(&directory)?;
        let mut hasher = Sha256::new();
        let mut size = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = source_file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            temporary.write_all(&buffer[..read])?;
            size = size
                .checked_add(u64::try_from(read).map_err(invalid)?)
                .ok_or_else(|| invalid("document size overflow"))?;
        }
        temporary.as_file().sync_all()?;
        let sha256: [u8; 32] = hasher.finalize().into();

        let candidate = StoredDocument {
            document_id: draft.document_id,
            direction: draft.direction,
            original_name: draft.original_name.clone(),
            stored_name: stored_name.clone(),
            mime_type: draft.mime_type.clone(),
            size,
            sha256,
            created_at_unix_ms: draft.created_at_unix_ms,
            point_id: draft.point_id,
        };

        if let Some(existing) = self.documents.get(&draft.document_id) {
            if existing.direction == candidate.direction
                && existing.original_name == candidate.original_name
                && existing.mime_type == candidate.mime_type
                && existing.size == candidate.size
                && existing.sha256 == candidate.sha256
                && existing.point_id == candidate.point_id
            {
                return Ok(existing.clone());
            }
            return Err(invalid("document id content or metadata conflict"));
        }

        if destination.try_exists()? {
            let (existing_size, existing_hash) = hash_file(&destination)?;
            if existing_size != size || existing_hash != sha256 {
                return Err(invalid(
                    "unindexed document file conflicts with incoming content",
                ));
            }
        } else {
            publish(temporary, &destination)?;
        }

        self.record(Change::DocumentStored(DocumentEntry {
            document_id: draft.document_id.as_bytes(),
            direction: draft.direction,
            original_name: draft.original_name,
            stored_name,
            mime_type: draft.mime_type,
            size,
            sha256,
            created_at_unix_ms: draft.created_at_unix_ms,
            point_id: draft.point_id.map(PointId::as_bytes),
        }))?;

        self.document(draft.document_id)
            .ok_or_else(|| invalid("document record was not stored"))
    }

    pub fn archive(
        self,
        completed_root: &Path,
        archive_name: &str,
        completed_at_unix_ms: u64,
    ) -> io::Result<PathBuf> {
        safe_archive_name(archive_name)?;
        fs::create_dir_all(completed_root)?;
        let destination = completed_root.join(archive_name);
        if destination.try_exists()? {
            return Err(invalid("completed trip destination already exists"));
        }

        let fresh = Self::open(&self.path, self.trip)?;
        if fresh.cursor != self.cursor
            || fresh.plan_version_count != self.plan_version_count
            || fresh.state() != self.state()
            || fresh.documents != self.documents
        {
            return Err(invalid("trip changed; reopen before completion"));
        }

        let summary = fresh.execution_summary(completed_at_unix_ms)?;
        write_execution_summary(&fresh.path, &summary)?;
        write_checksums(&fresh.path)?;
        verify_checksums(&fresh.path)?;

        fs::rename(&fresh.path, &destination)?;
        let verified = verify_archive(&destination, self.trip)?;
        if verified != summary {
            return Err(invalid("completed trip summary changed during archive"));
        }
        Ok(destination)
    }
}

pub fn verify_archive(path: &Path, trip: TripId) -> io::Result<ExecutionSummary> {
    verify_checksums(path)?;
    let recovered = TripFolder::open(path, trip)?;
    let summary_file: ExecutionSummaryFile = serde_json::from_slice(&bounded(File::open(
        path.join("reports").join("execution_summary.json"),
    )?)?)?;
    let summary = summary_file.typed()?;
    if summary.trip_id != trip {
        return Err(invalid("execution summary trip id mismatch"));
    }

    let state = recovered.state();
    let expected = recovered.execution_summary(summary.completed_at_unix_ms)?;
    if expected != summary {
        return Err(invalid("execution summary does not match recovered trip"));
    }
    if state.current_version != summary.final_plan_version {
        return Err(invalid("execution summary final version mismatch"));
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_contracts::{ReplanReasonCode, ReplanRequest, SchemaVersion};
    use cargo_dispatch_control::ReplanRequestDraft;
    use cargo_dispatch_domain::{OperatorId, ReplanRequestId, WorkingRouteId};
    use cargo_dispatch_geo::{TollPreference, VehicleRoutingProfile};
    use cargo_dispatch_planner::{
        InMemoryTripExportHistory, PlannerTripExportRecord, WorkingRouteState,
        apply_planner_replan, build_trip_export_draft, manual_working_route,
        transition_working_route,
    };

    fn create_test_package(
        path: &Path,
        trip_byte: u8,
        first_point_byte: u8,
    ) -> io::Result<(TripId, Vec<TripRoutePoint>)> {
        let trip = TripId::from_bytes([trip_byte; 16]).unwrap();
        let route: Vec<_> = (0..2u8)
            .map(|offset| TripRoutePoint {
                point_id: PointId::from_bytes([first_point_byte + offset; 16]).unwrap(),
                sequence: u32::from(offset) + 1,
                kind: TripPointKind::Load,
                title: format!("Точка {}", offset + 1),
                navigation: Some(TripNavigationTarget {
                    latitude_e7: 498_393_000 + i32::from(offset) * 1_000,
                    longitude_e7: 240_296_000 + i32::from(offset) * 1_000,
                    address: Some(format!("Synthetic address {}", offset + 1)),
                }),
            })
            .collect();
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(trip, TripVersion::new(1).unwrap(), None).unwrap(),
            working_route_id: WorkingRouteId::from_bytes([trip_byte + 1; 16]).unwrap(),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([trip_byte + 2; 16]).unwrap(),
            ordered_point_ids: route.iter().map(|point| point.point_id).collect(),
            routing_profile: None,
            route_geometry: None,
        };
        export_package(path, &draft, &route)?;
        Ok((trip, route))
    }

    #[test]
    fn schema_1_1_reader_accepts_legacy_point_without_navigation() -> io::Result<()> {
        let point = Point {
            id: [1; 16],
            kind: TripPointKind::Load,
            title: "Legacy point".to_owned(),
            navigation: None,
        };
        let encoded = serde_json::to_value(&point)?;
        assert!(encoded.get("navigation").is_none());
        let decoded: Point = serde_json::from_value(encoded)?;
        assert!(decoded.navigation.is_none());

        let trip = TripId::from_bytes([2; 16]).unwrap();
        let manifest = Manifest {
            schema: TRIP_PACKAGE_SCHEMA_NAME.to_owned(),
            schema_version: [1, 0],
            trip: trip.as_bytes(),
            version: 1,
            base: None,
            working_route: WorkingRouteId::from_bytes([3; 16]).unwrap().as_bytes(),
            revision: 1,
            created_at_unix_ms: 1,
            operator: OperatorId::from_bytes([4; 16]).unwrap().as_bytes(),
            points: vec![decoded],
            routing_profile: None,
            road_geometry: None,
        };
        let candidate = manifest.candidate()?;
        assert_eq!(candidate.header.schema_version.major, 1);
        assert_eq!(candidate.header.schema_version.minor, 0);
        assert!(candidate.route_points[0].navigation.is_none());
        Ok(())
    }

    #[test]
    fn schema_1_2_roundtrips_road_geometry_and_legacy_remains_optional() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let package = tmp.path().join("geometry.zip");
        let folder = tmp.path().join("geometry-trip");
        let trip = TripId::from_bytes([31; 16]).unwrap();
        let route = vec![
            TripRoutePoint {
                point_id: PointId::from_bytes([32; 16]).unwrap(),
                sequence: 1,
                kind: TripPointKind::Load,
                title: "A".to_owned(),
                navigation: None,
            },
            TripRoutePoint {
                point_id: PointId::from_bytes([33; 16]).unwrap(),
                sequence: 2,
                kind: TripPointKind::Unload,
                title: "B".to_owned(),
                navigation: None,
            },
        ];
        let mut header =
            TripPackageHeader::new(trip, TripVersion::new(1).unwrap(), None).unwrap();
        header.schema_version = SchemaVersion::new(1, 2);
        let draft = PlannerTripExportDraft {
            header,
            working_route_id: WorkingRouteId::from_bytes([34; 16]).unwrap(),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([35; 16]).unwrap(),
            ordered_point_ids: route.iter().map(|point| point.point_id).collect(),
            routing_profile: None,
            route_geometry: Some(TripRoadGeometry {
                routing: TripRoutingIdentity {
                    engine: "valhalla".to_owned(),
                    engine_version: "3.9.2".to_owned(),
                    map_dataset_id: "fixture".to_owned(),
                    profile_fingerprint: "profile".to_owned(),
                },
                points: vec![
                    TripRoadGeometryPoint {
                        latitude_e7: 498_393_000,
                        longitude_e7: 240_296_000,
                    },
                    TripRoadGeometryPoint {
                        latitude_e7: 498_430_000,
                        longitude_e7: 240_360_000,
                    },
                ],
            }),
        };

        export_package(&package, &draft, &route)?;
        let mut imported = TripFolder::import(&folder, &package)?;
        let state = imported.state();
        assert_eq!(state.current_version.get(), 1);
        assert_eq!(state.route_geometry, draft.route_geometry);

        imported.driver_order(&state, &[route[1].point_id, route[0].point_id])?;
        assert!(imported.state().route_geometry.is_none());

        let recovered = TripFolder::open(&folder, trip)?;
        assert!(recovered.state().route_geometry.is_none());
        Ok(())
    }

    fn routing_profile_fixture() -> VehicleRoutingProfile {
        VehicleRoutingProfile {
            height_m: None,
            width_m: Some(2.45),
            length_m: Some(12.0),
            gross_weight_t: Some(18.0),
            axle_load_t: None,
            axle_count: Some(2),
            hazmat: false,
            hgv: true,
            exclude_unpaved: true,
            prefer_truck_routes: true,
            toll_preference: TollPreference::Allow,
            max_speed_kmh: None,
        }
    }

    #[test]
    fn schema_1_3_roundtrips_actual_routing_profile() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let package = tmp.path().join("profile.zip");
        let folder = tmp.path().join("profile-trip");
        let trip = TripId::from_bytes([41; 16]).unwrap();
        let route = vec![
            TripRoutePoint {
                point_id: PointId::from_bytes([42; 16]).unwrap(),
                sequence: 1,
                kind: TripPointKind::Load,
                title: "A".to_owned(),
                navigation: Some(TripNavigationTarget {
                    latitude_e7: 498_393_000,
                    longitude_e7: 240_296_000,
                    address: None,
                }),
            },
            TripRoutePoint {
                point_id: PointId::from_bytes([43; 16]).unwrap(),
                sequence: 2,
                kind: TripPointKind::Unload,
                title: "B".to_owned(),
                navigation: Some(TripNavigationTarget {
                    latitude_e7: 498_430_000,
                    longitude_e7: 240_360_000,
                    address: None,
                }),
            },
        ];
        let profile = routing_profile_fixture();
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(trip, TripVersion::new(1).unwrap(), None).unwrap(),
            working_route_id: WorkingRouteId::from_bytes([44; 16]).unwrap(),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([45; 16]).unwrap(),
            ordered_point_ids: route.iter().map(|point| point.point_id).collect(),
            routing_profile: Some(profile.clone()),
            route_geometry: None,
        };

        export_package(&package, &draft, &route)?;
        let imported = TripFolder::import(&folder, &package)?;
        assert_eq!(imported.routing_profile(), Some(&profile));
        drop(imported);
        let reopened = TripFolder::open(&folder, trip)?;
        assert_eq!(reopened.routing_profile(), Some(&profile));
        Ok(())
    }

    #[test]
    fn reroute_geometry_is_durable_and_rejects_stale_revision() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let package = tmp.path().join("reroute.zip");
        let folder = tmp.path().join("reroute-trip");
        let (trip, route) = create_test_package(&package, 51, 52)?;
        let mut control = TripFolder::import(&folder, &package)?;
        let before = control.state();
        control.driver_order(&before, &[route[1].point_id, route[0].point_id])?;
        let reordered = control.state();
        assert_eq!(reordered.local_route_revision, 1);
        assert!(reordered.route_geometry.is_none());

        let geometry = TripRoadGeometry {
            routing: TripRoutingIdentity {
                engine: "valhalla".to_owned(),
                engine_version: "3.9.2".to_owned(),
                map_dataset_id: "fixture".to_owned(),
                profile_fingerprint: "profile".to_owned(),
            },
            points: vec![
                TripRoadGeometryPoint {
                    latitude_e7: 498_430_000,
                    longitude_e7: 240_360_000,
                },
                TripRoadGeometryPoint {
                    latitude_e7: 498_393_000,
                    longitude_e7: 240_296_000,
                },
            ],
        };
        control.set_route_geometry(&reordered, &geometry)?;
        assert_eq!(control.state().route_geometry, Some(geometry.clone()));
        drop(control);

        let mut reopened = TripFolder::open(&folder, trip)?;
        assert_eq!(reopened.state().route_geometry, Some(geometry.clone()));
        let shown = reopened.state();
        reopened.driver_order(&shown, &[route[0].point_id, route[1].point_id])?;
        assert!(reopened.state().route_geometry.is_none());
        assert!(reopened.set_route_geometry(&reordered, &geometry).is_err());
        assert!(reopened.state().route_geometry.is_none());
        Ok(())
    }

    #[test]
    fn independent_control_recovers_driver_changes() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let package = tmp.path().join("plan.zip");
        let folder = tmp.path().join("2026-11-24_001");
        let trip = TripId::from_bytes([1; 16]).unwrap();
        let route: Vec<_> = (1..=3u8)
            .map(|n| TripRoutePoint {
                point_id: PointId::from_bytes([n; 16]).unwrap(),
                sequence: n.into(),
                kind: TripPointKind::Load,
                title: format!("Точка {n}"),
                navigation: None,
            })
            .collect();
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(trip, TripVersion::new(1).unwrap(), None).unwrap(),
            working_route_id: WorkingRouteId::from_bytes([2; 16]).unwrap(),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([3; 16]).unwrap(),
            ordered_point_ids: route.iter().map(|p| p.point_id).collect(),
            routing_profile: None,
            route_geometry: None,
        };
        export_package(&package, &draft, &route)?;
        let mut control = TripFolder::import(&folder, &package)?;
        let mut stale = TripFolder::open(&folder, trip)?;
        control.complete(route[2].point_id)?;
        assert!(stale.complete(route[0].point_id).is_err());
        let before = control.state();
        assert!(
            control
                .driver_order(&before, &[route[0].point_id, route[0].point_id])
                .is_err()
        );
        control.driver_order(&before, &[route[1].point_id, route[0].point_id])?;
        let after = control.state();
        assert_eq!(after.route_points[0].point_id, route[1].point_id);
        assert_eq!(after.route_points[2], route[2]);
        assert!(after.completed_point_ids.contains(&route[2].point_id));
        assert_eq!(TripFolder::import(&folder, &package)?.state(), after);
        fs::remove_file(&package)?;
        drop(control);
        assert_eq!(TripFolder::open(&folder, trip)?.state(), after);
        Ok(())
    }

    #[test]
    fn out_of_order_completion_survives_planner_round_trip_and_restart() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let package_v1 = tmp.path().join("trip-v1.zip");
        let package_v2 = tmp.path().join("trip-v2.zip");
        let folder = tmp.path().join("active-trip");

        let trip = TripId::from_bytes([10; 16]).unwrap();
        let working_route_id = WorkingRouteId::from_bytes([11; 16]).unwrap();
        let operator = OperatorId::from_bytes([12; 16]).unwrap();
        let point_ids: Vec<_> = (1..=4u8)
            .map(|n| PointId::from_bytes([n; 16]).unwrap())
            .collect();

        let route_v1: Vec<_> = point_ids
            .iter()
            .enumerate()
            .map(|(index, point_id)| TripRoutePoint {
                point_id: *point_id,
                sequence: u32::try_from(index + 1).unwrap(),
                kind: TripPointKind::Load,
                title: format!("Точка {}", index + 1),
                navigation: None,
            })
            .collect();

        let mut planner_route =
            manual_working_route(working_route_id, &point_ids).expect("planner route");
        planner_route.state = WorkingRouteState::Ready;

        let draft_v1 = build_trip_export_draft(
            &planner_route,
            trip,
            TripVersion::new(1).unwrap(),
            None,
            1,
            operator,
        )
        .expect("first trip export");
        let mut history = InMemoryTripExportHistory::default();
        history
            .append(PlannerTripExportRecord::from(&draft_v1))
            .expect("record first export");
        export_package(&package_v1, &draft_v1, &route_v1)?;

        let mut control = TripFolder::import(&folder, &package_v1)?;
        control.complete(point_ids[2])?;
        let completed_v1 = control.state().route_points[2].clone();

        let request: ReplanRequest = control
            .build_replan_request(ReplanRequestDraft {
                request_id: ReplanRequestId::from_bytes([13; 16]).unwrap(),
                created_at_unix_ms: 2,
                current_time_unix_ms: 3,
                reason_code: ReplanReasonCode::RoadProblem,
                operator_note: Some("Об'їзд після зміни ситуації".to_owned()),
                current_point_id: Some(point_ids[1]),
                affected_point_ids: vec![point_ids[0], point_ids[1], point_ids[3]],
            })
            .expect("control replan request");
        assert_eq!(request.completed_point_ids, vec![point_ids[2]]);

        let context = history
            .validate_replan_request(&request)
            .expect("planner accepts current control facts");
        let replanned = apply_planner_replan(
            &planner_route,
            &context,
            &[point_ids[1], point_ids[0], point_ids[3]],
        )
        .expect("planner preserves completed third point");
        assert_eq!(
            replanned
                .points
                .iter()
                .map(|point| point.point_id)
                .collect::<Vec<_>>(),
            vec![point_ids[1], point_ids[0], point_ids[2], point_ids[3]]
        );

        let ready_v2 = transition_working_route(&replanned, WorkingRouteState::Ready)
            .expect("replanned route becomes ready");
        let draft_v2 = build_trip_export_draft(
            &ready_v2,
            trip,
            TripVersion::new(2).unwrap(),
            Some(TripVersion::new(1).unwrap()),
            4,
            operator,
        )
        .expect("second trip export");
        history
            .append(PlannerTripExportRecord::from(&draft_v2))
            .expect("record second export");

        let order_v2 = [point_ids[1], point_ids[0], point_ids[2], point_ids[3]];
        let route_v2: Vec<_> = order_v2
            .iter()
            .enumerate()
            .map(|(index, point_id)| {
                let mut point = route_v1
                    .iter()
                    .find(|point| point.point_id == *point_id)
                    .expect("known point")
                    .clone();
                point.sequence = u32::try_from(index + 1).unwrap();
                point
            })
            .collect();
        assert_eq!(route_v2[2], completed_v1);

        export_package(&package_v2, &draft_v2, &route_v2)?;
        let updated = TripFolder::import(&folder, &package_v2)?;
        let state_v2 = updated.state();

        assert_eq!(state_v2.current_version, TripVersion::new(2).unwrap());
        assert!(state_v2.completed_point_ids.contains(&point_ids[2]));
        assert_eq!(state_v2.route_points[2], completed_v1);
        assert_eq!(
            state_v2
                .route_points
                .iter()
                .map(|point| point.point_id)
                .collect::<Vec<_>>(),
            order_v2
        );

        drop(updated);
        let recovered = TripFolder::open(&folder, trip)?.state();
        assert_eq!(recovered, state_v2);
        assert!(recovered.completed_point_ids.contains(&point_ids[2]));
        assert_eq!(recovered.route_points[2], completed_v1);

        Ok(())
    }

    #[test]
    fn two_active_trips_keep_documents_independent_and_recoverable() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let package_a = tmp.path().join("trip-a.zip");
        let package_b = tmp.path().join("trip-b.zip");
        let folder_a = tmp.path().join("active-a");
        let folder_b = tmp.path().join("active-b");
        let (trip_a, route_a) = create_test_package(&package_a, 40, 50)?;
        let (trip_b, route_b) = create_test_package(&package_b, 60, 70)?;

        let source_a = tmp.path().join("operator-ttn.pdf");
        let source_b = tmp.path().join("driver-photo.png");
        fs::write(&source_a, b"synthetic pdf from operator")?;
        fs::write(&source_b, b"synthetic image from driver")?;

        let mut control_a = TripFolder::import(&folder_a, &package_a)?;
        let mut control_b = TripFolder::import(&folder_b, &package_b)?;
        let document_a = DocumentId::from_bytes([80; 16]).unwrap();
        let document_b = DocumentId::from_bytes([81; 16]).unwrap();

        let stored_a = control_a.store_document(
            DocumentStoreDraft {
                document_id: document_a,
                direction: DocumentDirection::FromOperator,
                original_name: "TTN-test.pdf".to_owned(),
                mime_type: "application/pdf".to_owned(),
                created_at_unix_ms: 10,
                point_id: Some(route_a[0].point_id),
            },
            &source_a,
        )?;
        let retry_a = control_a.store_document(
            DocumentStoreDraft {
                document_id: document_a,
                direction: DocumentDirection::FromOperator,
                original_name: "TTN-test.pdf".to_owned(),
                mime_type: "application/pdf".to_owned(),
                created_at_unix_ms: 11,
                point_id: Some(route_a[0].point_id),
            },
            &source_a,
        )?;
        assert_eq!(retry_a, stored_a);
        assert_eq!(control_a.documents().len(), 1);

        let stored_b = control_b.store_document(
            DocumentStoreDraft {
                document_id: document_b,
                direction: DocumentDirection::FromDriver,
                original_name: "photo.PNG".to_owned(),
                mime_type: "image/png".to_owned(),
                created_at_unix_ms: 20,
                point_id: Some(route_b[1].point_id),
            },
            &source_b,
        )?;
        assert_eq!(control_b.documents(), vec![stored_b.clone()]);
        assert!(
            control_b
                .store_document(
                    DocumentStoreDraft {
                        document_id: DocumentId::from_bytes([82; 16]).unwrap(),
                        direction: DocumentDirection::FromDriver,
                        original_name: "wrong-point.png".to_owned(),
                        mime_type: "image/png".to_owned(),
                        created_at_unix_ms: 21,
                        point_id: Some(route_a[0].point_id),
                    },
                    &source_b,
                )
                .is_err()
        );

        fs::write(&source_a, b"different bytes for the same logical document")?;
        assert!(
            control_a
                .store_document(
                    DocumentStoreDraft {
                        document_id: document_a,
                        direction: DocumentDirection::FromOperator,
                        original_name: "TTN-test.pdf".to_owned(),
                        mime_type: "application/pdf".to_owned(),
                        created_at_unix_ms: 12,
                        point_id: Some(route_a[0].point_id),
                    },
                    &source_a,
                )
                .is_err()
        );

        drop(control_a);
        drop(control_b);

        let recovered_a = TripFolder::open(&folder_a, trip_a)?;
        let recovered_b = TripFolder::open(&folder_b, trip_b)?;
        assert_eq!(recovered_a.documents(), vec![stored_a.clone()]);
        assert_eq!(recovered_b.documents(), vec![stored_b.clone()]);
        assert_eq!(
            fs::read(stored_document_path(&folder_a, &stored_a))?,
            b"synthetic pdf from operator"
        );
        assert_eq!(
            fs::read(stored_document_path(&folder_b, &stored_b))?,
            b"synthetic image from driver"
        );

        fs::write(
            stored_document_path(&folder_b, &stored_b),
            b"corrupted stored content",
        )?;
        assert!(TripFolder::open(&folder_b, trip_b).is_err());
        assert!(TripFolder::open(&folder_a, trip_a).is_ok());

        Ok(())
    }

    #[test]
    fn completing_one_trip_verifies_and_archives_it_without_stopping_another() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let package_a = tmp.path().join("archive-a.zip");
        let package_b = tmp.path().join("archive-b.zip");
        let folder_a = tmp.path().join("active-a");
        let folder_b = tmp.path().join("active-b");
        let completed_root = tmp.path().join("completed").join("2026").join("11");
        let archive_name = "2026-11-24_trip-001_2828";

        let (trip_a, route_a) = create_test_package(&package_a, 90, 100)?;
        let (trip_b, route_b) = create_test_package(&package_b, 110, 120)?;
        let mut control_a = TripFolder::import(&folder_a, &package_a)?;
        let mut control_b = TripFolder::import(&folder_b, &package_b)?;

        control_a.complete(route_a[1].point_id)?;
        let source_a = tmp.path().join("archive-ttn.pdf");
        let source_b = tmp.path().join("still-active-photo.jpg");
        fs::write(&source_a, b"archive document bytes")?;
        fs::write(&source_b, b"second trip keeps working")?;

        let stored_a = control_a.store_document(
            DocumentStoreDraft {
                document_id: DocumentId::from_bytes([130; 16]).unwrap(),
                direction: DocumentDirection::FromOperator,
                original_name: "archive-ttn.pdf".to_owned(),
                mime_type: "application/pdf".to_owned(),
                created_at_unix_ms: 30,
                point_id: Some(route_a[1].point_id),
            },
            &source_a,
        )?;
        control_b.store_document(
            DocumentStoreDraft {
                document_id: DocumentId::from_bytes([131; 16]).unwrap(),
                direction: DocumentDirection::FromDriver,
                original_name: "still-active-photo.jpg".to_owned(),
                mime_type: "image/jpeg".to_owned(),
                created_at_unix_ms: 31,
                point_id: Some(route_b[0].point_id),
            },
            &source_b,
        )?;

        let stored_a_path = stored_document_path(&folder_a, &stored_a);
        fs::write(&stored_a_path, b"corrupted before completion")?;
        assert!(
            control_a
                .archive(&completed_root, archive_name, 100)
                .is_err()
        );
        assert!(folder_a.is_dir());
        assert!(!completed_root.join(archive_name).exists());

        fs::write(&stored_a_path, b"archive document bytes")?;
        let control_a = TripFolder::open(&folder_a, trip_a)?;
        let archived = control_a.archive(&completed_root, archive_name, 100)?;
        assert!(!folder_a.exists());
        assert!(archived.is_dir());

        let summary = verify_archive(&archived, trip_a)?;
        assert_eq!(summary.trip_id, trip_a);
        assert_eq!(summary.final_plan_version, TripVersion::new(1).unwrap());
        assert_eq!(summary.completed_point_ids, vec![route_a[1].point_id]);
        assert_eq!(summary.documents_from_operator, 1);
        assert_eq!(summary.documents_from_driver, 0);
        assert_eq!(summary.plan_version_count, 1);
        assert_eq!(summary.replan_count, 0);

        assert!(
            TripFolder::open(&archived, trip_a)?
                .archive(&completed_root, archive_name, 101)
                .is_err()
        );
        assert_eq!(
            fs::read_dir(&completed_root)?
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                .count(),
            1
        );

        control_b.complete(route_b[0].point_id)?;
        let active_b = control_b.state();
        assert!(active_b.completed_point_ids.contains(&route_b[0].point_id));
        assert_eq!(control_b.documents().len(), 1);
        drop(control_b);
        assert_eq!(TripFolder::open(&folder_b, trip_b)?.state(), active_b);

        fs::write(
            stored_document_path(&archived, &stored_a),
            b"tampered after archive",
        )?;
        assert!(verify_archive(&archived, trip_a).is_err());
        assert!(TripFolder::open(&folder_b, trip_b).is_ok());

        Ok(())
    }

    #[test]
    fn driver_location_is_durable_validated_and_idempotent() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let package = tmp.path().join("location-trip.zip");
        let folder = tmp.path().join("location-trip");
        let (trip, _) = create_test_package(&package, 140, 150)?;
        let mut control = TripFolder::import(&folder, &package)?;

        let sample = DriverLocationSample {
            latitude_e7: 498_393_000,
            longitude_e7: 240_296_000,
            accuracy_cm: Some(850),
            observed_at_unix_ms: 1_795_550_000_000,
        };
        control.record_location(sample)?;
        control.record_location(sample)?;
        assert_eq!(control.locations(), &[sample]);
        assert_eq!(control.latest_location(), Some(sample));

        let newer = DriverLocationSample {
            latitude_e7: 498_430_000,
            longitude_e7: 240_360_000,
            observed_at_unix_ms: sample.observed_at_unix_ms + 1_000,
            ..sample
        };
        control.record_location(newer)?;
        assert_eq!(control.latest_location(), Some(newer));

        assert!(
            control
                .record_location(DriverLocationSample {
                    longitude_e7: 240_400_000,
                    observed_at_unix_ms: newer.observed_at_unix_ms,
                    ..newer
                })
                .is_err()
        );
        assert!(
            control
                .record_location(DriverLocationSample {
                    observed_at_unix_ms: sample.observed_at_unix_ms + 500,
                    ..sample
                })
                .is_err()
        );
        assert_eq!(control.latest_location(), Some(newer));

        assert!(
            control
                .record_location(DriverLocationSample {
                    latitude_e7: 901_000_000,
                    ..sample
                })
                .is_err()
        );
        assert!(
            control
                .record_location(DriverLocationSample {
                    observed_at_unix_ms: 0,
                    ..sample
                })
                .is_err()
        );

        drop(control);
        let recovered = TripFolder::open(&folder, trip)?;
        assert_eq!(recovered.locations(), &[sample, newer]);
        assert_eq!(recovered.latest_location(), Some(newer));
        Ok(())
    }

    #[test]
    fn trip_folder_detects_its_trip_id_from_durable_history() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let package = tmp.path().join("detected-trip.zip");
        let folder = tmp.path().join("detected-trip");
        let (trip, _) = create_test_package(&package, 160, 170)?;
        TripFolder::import(&folder, &package)?;

        let detected = TripFolder::open_detected(&folder)?;
        assert_eq!(detected.trip_id(), trip);
        assert_eq!(detected.state().trip_id, trip);
        Ok(())
    }
}
