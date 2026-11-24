#![forbid(unsafe_code)]

use cargo_dispatch_contracts::{
    CURRENT_SCHEMA_VERSION, PackageContentFingerprint, ReplanRequest, TRIP_PACKAGE_SCHEMA_NAME,
    TripPackageHeader, TripPointKind, TripRoutePoint, TripVersion, validate_route_points,
};
use cargo_dispatch_control::{
    InMemoryTripRegistry, ReplanBuildError, ReplanRequestDraft, StoredTripPlan, TripImportCandidate,
};
use cargo_dispatch_domain::{DocumentId, PointId, TripId};
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Point {
    id: [u8; 16],
    #[serde(with = "Kind")]
    kind: TripPointKind,
    title: String,
}
impl From<&TripRoutePoint> for Point {
    fn from(p: &TripRoutePoint) -> Self {
        Self {
            id: p.point_id.as_bytes(),
            kind: p.kind.clone(),
            title: p.title.clone(),
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
}

impl Manifest {
    fn candidate(&self) -> io::Result<TripImportCandidate> {
        if self.schema != TRIP_PACKAGE_SCHEMA_NAME
            || self.schema_version != [CURRENT_SCHEMA_VERSION.major, CURRENT_SCHEMA_VERSION.minor]
        {
            return Err(invalid("unsupported trip package schema"));
        }
        let header = TripPackageHeader::new(
            TripId::from_bytes(self.trip).map_err(invalid)?,
            TripVersion::new(self.version).map_err(invalid)?,
            self.base
                .map(TripVersion::new)
                .transpose()
                .map_err(invalid)?,
        )
        .map_err(invalid)?;
        let route_points = points(&self.points)?;
        validate_route_points(&route_points).map_err(invalid)?;
        let hash = Sha256::digest(serde_json::to_vec(self)?);
        Ok(TripImportCandidate {
            header,
            route_points,
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
    DocumentStored(DocumentEntry),
}

fn apply(
    reg: &mut InMemoryTripRegistry,
    documents: &mut std::collections::BTreeMap<DocumentId, StoredDocument>,
    trip: TripId,
    event: &Change,
) -> io::Result<()> {
    match event {
        Change::Import(m) => {
            if m.trip != trip.as_bytes() {
                return Err(invalid("different trip"));
            }
            reg.import(m.candidate()?).map_err(invalid)?;
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
    }
    Ok(())
}

pub struct TripFolder {
    path: PathBuf,
    trip: TripId,
    cursor: u64,
    registry: InMemoryTripRegistry,
    documents: std::collections::BTreeMap<DocumentId, StoredDocument>,
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
            registry: InMemoryTripRegistry::default(),
            documents: std::collections::BTreeMap::new(),
        };
        for entry in entries {
            if entry != this.next_path()? {
                return Err(invalid("incomplete trip history"));
            }
            let event: Change = serde_json::from_slice(&bounded(File::open(entry)?)?)?;
            apply(&mut this.registry, &mut this.documents, trip, &event)?;
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
    pub fn documents(&self) -> Vec<StoredDocument> {
        self.documents.values().cloned().collect()
    }

    #[must_use]
    pub fn document(&self, document_id: DocumentId) -> Option<StoredDocument> {
        self.documents.get(&document_id).cloned()
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
        let mut updated_documents = self.documents.clone();
        apply(&mut updated, &mut updated_documents, self.trip, &event)?;
        if updated.get(self.trip) == self.registry.get(self.trip)
            && updated_documents == self.documents
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
        self.documents = updated_documents;
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

    pub fn store_document(
        &mut self,
        direction: DocumentDirection,
        document_id: DocumentId,
        original_name: &str,
        mime_type: &str,
        point_id: Option<PointId>,
        created_at_unix_ms: u64,
        source: &Path,
    ) -> io::Result<StoredDocument> {
        if original_name.trim().is_empty() {
            return Err(invalid("document original name is required"));
        }
        if mime_type.trim().is_empty() {
            return Err(invalid("document mime type is required"));
        }
        if let Some(point_id) = point_id
            && !self
                .state()
                .route_points
                .iter()
                .any(|point| point.point_id == point_id)
        {
            return Err(invalid("document point does not belong to trip"));
        }

        let directory = self.path.join("documents").join(document_subdir(direction));
        fs::create_dir_all(&directory)?;

        let extension = safe_extension(original_name);
        let stored_name = match extension {
            Some(extension) => format!("{}.{}", id_hex(document_id.as_bytes()), extension),
            None => id_hex(document_id.as_bytes()),
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
            document_id,
            direction,
            original_name: original_name.to_owned(),
            stored_name: stored_name.clone(),
            mime_type: mime_type.to_owned(),
            size,
            sha256,
            created_at_unix_ms,
            point_id,
        };

        if let Some(existing) = self.documents.get(&document_id) {
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
            document_id: document_id.as_bytes(),
            direction,
            original_name: original_name.to_owned(),
            stored_name,
            mime_type: mime_type.to_owned(),
            size,
            sha256,
            created_at_unix_ms,
            point_id: point_id.map(PointId::as_bytes),
        }))?;

        self.document(document_id)
            .ok_or_else(|| invalid("document record was not stored"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_contracts::{ReplanReasonCode, ReplanRequest};
    use cargo_dispatch_control::ReplanRequestDraft;
    use cargo_dispatch_domain::{OperatorId, ReplanRequestId, WorkingRouteId};
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
            })
            .collect();
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(trip, TripVersion::new(1).unwrap(), None).unwrap(),
            working_route_id: WorkingRouteId::from_bytes([trip_byte + 1; 16]).unwrap(),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([trip_byte + 2; 16]).unwrap(),
            ordered_point_ids: route.iter().map(|point| point.point_id).collect(),
        };
        export_package(path, &draft, &route)?;
        Ok((trip, route))
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
            })
            .collect();
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(trip, TripVersion::new(1).unwrap(), None).unwrap(),
            working_route_id: WorkingRouteId::from_bytes([2; 16]).unwrap(),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([3; 16]).unwrap(),
            ordered_point_ids: route.iter().map(|p| p.point_id).collect(),
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
            DocumentDirection::FromOperator,
            document_a,
            "TTN-test.pdf",
            "application/pdf",
            Some(route_a[0].point_id),
            10,
            &source_a,
        )?;
        let retry_a = control_a.store_document(
            DocumentDirection::FromOperator,
            document_a,
            "TTN-test.pdf",
            "application/pdf",
            Some(route_a[0].point_id),
            11,
            &source_a,
        )?;
        assert_eq!(retry_a, stored_a);
        assert_eq!(control_a.documents().len(), 1);

        let stored_b = control_b.store_document(
            DocumentDirection::FromDriver,
            document_b,
            "photo.PNG",
            "image/png",
            Some(route_b[1].point_id),
            20,
            &source_b,
        )?;
        assert_eq!(control_b.documents(), vec![stored_b.clone()]);
        assert!(
            control_b
                .store_document(
                    DocumentDirection::FromDriver,
                    DocumentId::from_bytes([82; 16]).unwrap(),
                    "wrong-point.png",
                    "image/png",
                    Some(route_a[0].point_id),
                    21,
                    &source_b,
                )
                .is_err()
        );

        fs::write(&source_a, b"different bytes for the same logical document")?;
        assert!(
            control_a
                .store_document(
                    DocumentDirection::FromOperator,
                    document_a,
                    "TTN-test.pdf",
                    "application/pdf",
                    Some(route_a[0].point_id),
                    12,
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
}
