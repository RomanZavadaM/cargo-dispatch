//! A file the driver sends in parts. Unfinished files wait in `incoming/` of
//! the trip folder: they survive a restart and are not documents until the last
//! part has arrived.

use super::*;
use std::io::{Seek, SeekFrom};

/// What the sender says about the whole file. It must not change between parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadDescriptor {
    pub original_name: String,
    pub mime_type: String,
    pub point_id: Option<PointId>,
    pub total_size: u64,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadProgress {
    /// Bytes Control holds; the next part starts here.
    pub received: u64,
    /// The file is stored as a trip document.
    pub complete: bool,
    /// The checksum of the stored document.
    pub sha256: Option<[u8; 32]>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadFile {
    original_name: String,
    mime_type: String,
    point: Option<[u8; 16]>,
    total_size: u64,
    created_at_unix_ms: u64,
}

fn refused(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.to_owned())
}

fn stored(document: &StoredDocument) -> UploadProgress {
    UploadProgress {
        received: document.size,
        complete: true,
        sha256: Some(document.sha256),
    }
}

fn same_bytes(path: &Path, offset: u64, expected: &[u8]) -> io::Result<bool> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut actual = vec![0_u8; expected.len()];
    file.read_exact(&mut actual)?;
    Ok(actual == expected)
}

fn length_of(path: &Path) -> io::Result<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error),
    }
}

impl TripFolder {
    /// Where unfinished files wait.
    pub(crate) fn incoming_directory(&self) -> PathBuf {
        self.path.join("incoming")
    }

    fn part_paths(&self, document_id: DocumentId) -> (PathBuf, PathBuf) {
        let name = id_hex(document_id.as_bytes());
        let directory = self.incoming_directory();
        (
            directory.join(format!("{name}.part")),
            // Not a `.json` of the trip history: that lives in the trip folder itself.
            directory.join(format!("{name}.upload")),
        )
    }

    /// How much of a file Control already holds.
    pub fn upload_progress(&self, document_id: DocumentId) -> io::Result<UploadProgress> {
        if let Some(document) = self.documents.get(&document_id) {
            return Ok(stored(document));
        }
        let (part, _) = self.part_paths(document_id);
        Ok(UploadProgress {
            received: length_of(&part)?,
            complete: false,
            sha256: None,
        })
    }

    /// Accepts the part that starts at `offset`. A part that is already held is
    /// accepted again only with the same bytes. When the last byte arrives the
    /// file becomes a document from the driver.
    ///
    /// `InvalidData` means that the part does not fit what Control holds;
    /// `InvalidInput` that the request itself is wrong.
    pub fn append_upload(
        &mut self,
        document_id: DocumentId,
        descriptor: &UploadDescriptor,
        offset: u64,
        part: &[u8],
        part_sha256: Option<[u8; 32]>,
    ) -> io::Result<UploadProgress> {
        if descriptor.original_name.trim().is_empty() || descriptor.mime_type.trim().is_empty() {
            return Err(refused("file name and type are required"));
        }
        if let Some(point_id) = descriptor.point_id
            && !self
                .state()
                .route_points
                .iter()
                .any(|point| point.point_id == point_id)
        {
            return Err(refused("document point does not belong to trip"));
        }
        let length = u64::try_from(part.len()).map_err(invalid)?;
        let end = offset
            .checked_add(length)
            .filter(|end| *end <= descriptor.total_size)
            .ok_or_else(|| refused("part exceeds the stated file size"))?;
        if let Some(expected) = part_sha256 {
            let actual: [u8; 32] = Sha256::digest(part).into();
            if actual != expected {
                return Err(refused("part checksum does not match"));
            }
        }

        if let Some(document) = self.documents.get(&document_id) {
            let path = stored_document_path(&self.path, document);
            if document.direction != DocumentDirection::FromDriver
                || document.size != descriptor.total_size
                || document.original_name != descriptor.original_name
                || document.point_id != descriptor.point_id
                || !same_bytes(&path, offset, part)?
            {
                return Err(invalid("another file has this document id"));
            }
            return Ok(stored(document));
        }

        let (part_path, description_path) = self.part_paths(document_id);
        fs::create_dir_all(self.incoming_directory())?;
        let description = UploadFile {
            original_name: descriptor.original_name.clone(),
            mime_type: descriptor.mime_type.clone(),
            point: descriptor.point_id.map(|point| point.as_bytes()),
            total_size: descriptor.total_size,
            created_at_unix_ms: descriptor.created_at_unix_ms,
        };
        let known: Option<UploadFile> = match File::open(&description_path) {
            Ok(file) => serde_json::from_slice(&bounded(file)?).ok(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        let created_at_unix_ms = match &known {
            Some(known) => {
                if known.original_name != description.original_name
                    || known.mime_type != description.mime_type
                    || known.point != description.point
                    || known.total_size != description.total_size
                {
                    return Err(invalid("the file changed between parts"));
                }
                // The first part dates the file.
                known.created_at_unix_ms
            }
            None => {
                // Without a description nothing is known about earlier bytes.
                if length_of(&part_path)? != 0 {
                    fs::remove_file(&part_path)?;
                }
                let mut temporary = NamedTempFile::new_in(self.incoming_directory())?;
                temporary.write_all(&serde_json::to_vec(&description)?)?;
                temporary.as_file().sync_all()?;
                temporary
                    .persist(&description_path)
                    .map_err(|error| error.error)?;
                description.created_at_unix_ms
            }
        };

        let held = length_of(&part_path)?;
        let received = if offset == held {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&part_path)?;
            file.write_all(part)?;
            file.sync_all()?;
            end
        } else if end <= held && same_bytes(&part_path, offset, part)? {
            held
        } else {
            return Err(invalid("the part does not continue the file"));
        };

        if received < descriptor.total_size {
            return Ok(UploadProgress {
                received,
                complete: false,
                sha256: None,
            });
        }
        let document = self.store_document(
            DocumentStoreDraft {
                document_id,
                direction: DocumentDirection::FromDriver,
                original_name: descriptor.original_name.clone(),
                mime_type: descriptor.mime_type.clone(),
                created_at_unix_ms,
                point_id: descriptor.point_id,
            },
            &part_path,
        )?;
        // The document is recorded; leftovers only take space.
        let _ = fs::remove_file(&part_path);
        let _ = fs::remove_file(&description_path);
        Ok(stored(&document))
    }

    /// A document and its file without reading the file through: for sending it
    /// in parts, where the receiver checks the checksum of the whole file.
    pub fn document_path(&self, document_id: DocumentId) -> io::Result<(StoredDocument, PathBuf)> {
        let document = self
            .documents
            .get(&document_id)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "document not found"))?;
        let path = stored_document_path(&self.path, &document);
        if fs::metadata(&path)?.len() != document.size {
            return Err(invalid("stored document integrity mismatch"));
        }
        Ok((document, path))
    }
}
