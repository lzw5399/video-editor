use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::params;

use crate::ArtifactStoreError;
use crate::fingerprint::{ArtifactFingerprint, fingerprint_bytes, fingerprint_file};
use crate::paths::{
    blob_root_path, blob_tmp_path, derived_root_path, path_to_slash_string,
    validate_derived_relative_path,
};
use crate::schema::{ArtifactStore, open_artifact_store};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobWriteIntent {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub stable_key: String,
    pub schema_fingerprint: String,
    pub generator_fingerprint: String,
    pub runtime_capability_fingerprint: Option<String>,
    pub source_fingerprint: Option<String>,
    pub graph_fingerprint: Option<String>,
    pub output_profile_fingerprint: Option<String>,
    pub generation_parameters_json: serde_json::Value,
    pub expected_fingerprint: Option<ArtifactFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRecord {
    pub artifact_id: String,
    pub blob_relative_path: String,
    pub blob_fingerprint: ArtifactFingerprint,
    pub byte_count: u64,
}

#[derive(Debug)]
pub struct BlobStore {
    bundle_path: PathBuf,
    derived_root: PathBuf,
    store: ArtifactStore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRepairReport {
    pub demoted_artifact_ids: Vec<String>,
    pub removed_temp_files: usize,
}

impl BlobStore {
    pub fn open(bundle_path: impl AsRef<Path>) -> Result<Self, ArtifactStoreError> {
        let bundle_path = bundle_path.as_ref().to_path_buf();
        let derived_root = derived_root_path(&bundle_path);
        fs::create_dir_all(blob_root_path(&bundle_path)).map_err(|source| {
            ArtifactStoreError::Io {
                path: blob_root_path(&bundle_path),
                source,
            }
        })?;
        fs::create_dir_all(blob_tmp_path(&bundle_path)).map_err(|source| {
            ArtifactStoreError::Io {
                path: blob_tmp_path(&bundle_path),
                source,
            }
        })?;
        let store = open_artifact_store(&bundle_path)?;
        Ok(Self {
            bundle_path,
            derived_root,
            store,
        })
    }

    pub fn write_blob_atomic(
        &mut self,
        intent: BlobWriteIntent,
        bytes: &[u8],
    ) -> Result<BlobRecord, ArtifactStoreError> {
        let actual_fingerprint = fingerprint_bytes(bytes);
        if let Some(expected) = &intent.expected_fingerprint {
            if expected != &actual_fingerprint {
                return Err(ArtifactStoreError::FingerprintMismatch {
                    artifact_id: intent.artifact_id,
                    expected: expected.to_string(),
                    actual: actual_fingerprint.to_string(),
                });
            }
        }

        let relative_path = content_addressed_blob_path(&actual_fingerprint);
        let final_path = validate_derived_relative_path(&self.derived_root, &relative_path)?;
        let tmp_path = self.tmp_path(&intent.artifact_id);
        if let Some(parent) = tmp_path.parent() {
            fs::create_dir_all(parent).map_err(|source| ArtifactStoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).map_err(|source| ArtifactStoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let write_result = (|| {
            let mut file = File::create(&tmp_path).map_err(|source| ArtifactStoreError::Io {
                path: tmp_path.clone(),
                source,
            })?;
            file.write_all(bytes)
                .map_err(|source| ArtifactStoreError::Io {
                    path: tmp_path.clone(),
                    source,
                })?;
            file.sync_all().map_err(|source| ArtifactStoreError::Io {
                path: tmp_path.clone(),
                source,
            })?;
            if final_path.exists() {
                fs::remove_file(&tmp_path).map_err(|source| ArtifactStoreError::Io {
                    path: tmp_path.clone(),
                    source,
                })?;
            } else {
                fs::rename(&tmp_path, &final_path).map_err(|source| ArtifactStoreError::Io {
                    path: final_path.clone(),
                    source,
                })?;
            }
            sync_parent_if_possible(&final_path);
            Ok::<(), ArtifactStoreError>(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        write_result?;

        let mut verified = self.verify_blob(&relative_path)?;
        verified.artifact_id = intent.artifact_id.clone();
        if verified.blob_fingerprint != actual_fingerprint
            || verified.byte_count != bytes.len() as u64
        {
            return Err(ArtifactStoreError::FingerprintMismatch {
                artifact_id: intent.artifact_id,
                expected: actual_fingerprint.to_string(),
                actual: verified.blob_fingerprint.to_string(),
            });
        }

        self.commit_ready_artifact(&intent, &verified)?;
        Ok(verified)
    }

    pub fn verify_blob(&self, relative_path: &str) -> Result<BlobRecord, ArtifactStoreError> {
        let blob_path = validate_derived_relative_path(&self.derived_root, relative_path)?;
        let metadata = fs::metadata(&blob_path).map_err(|source| ArtifactStoreError::Io {
            path: blob_path.clone(),
            source,
        })?;
        let blob_fingerprint = fingerprint_file(&blob_path)?;
        Ok(BlobRecord {
            artifact_id: String::new(),
            blob_relative_path: path_to_slash_string(relative_path)?,
            blob_fingerprint,
            byte_count: metadata.len(),
        })
    }

    pub fn repair_blob_rows(&mut self) -> Result<BlobRepairReport, ArtifactStoreError> {
        let ready_rows = {
            let mut statement = self
                .store
                .connection()
                .prepare(
                    "SELECT artifact_id, blob_relative_path FROM artifact
                     WHERE status = 'ready' AND blob_relative_path IS NOT NULL
                     ORDER BY artifact_id",
                )
                .map_err(|source| self.sqlite_error(source))?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|source| self.sqlite_error(source))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|source| self.sqlite_error(source))?
        };

        let mut demoted_artifact_ids = Vec::new();
        for (artifact_id, relative_path) in ready_rows {
            let blob_path = match validate_derived_relative_path(&self.derived_root, &relative_path)
            {
                Ok(path) => path,
                Err(_) => {
                    demoted_artifact_ids.push(artifact_id);
                    continue;
                }
            };
            let missing_or_empty = fs::metadata(&blob_path)
                .map(|metadata| metadata.len() == 0)
                .unwrap_or(true);
            if missing_or_empty {
                demoted_artifact_ids.push(artifact_id);
            }
        }

        let removed_temp_files = self.clear_temp_files()?;
        for artifact_id in &demoted_artifact_ids {
            self.store
                .connection()
                .execute(
                    "UPDATE artifact
                     SET status = 'dirty', dirty = 1, updated_at_unix_ms = updated_at_unix_ms + 1
                     WHERE artifact_id = ?1",
                    [artifact_id],
                )
                .map_err(|source| self.sqlite_error(source))?;
        }

        Ok(BlobRepairReport {
            demoted_artifact_ids,
            removed_temp_files,
        })
    }

    fn commit_ready_artifact(
        &mut self,
        intent: &BlobWriteIntent,
        record: &BlobRecord,
    ) -> Result<(), ArtifactStoreError> {
        let params_json =
            serde_json::to_string(&intent.generation_parameters_json).map_err(|error| {
                ArtifactStoreError::InvalidDerivedPath {
                    path: intent.artifact_id.clone(),
                    reason: format!("generation parameters must serialize: {error}"),
                }
            })?;
        let db_path = self.store.db_path.clone();
        let tx = self
            .store
            .connection_mut()
            .transaction()
            .map_err(|source| sqlite_error(&db_path, source))?;
        tx.execute(
            "INSERT INTO artifact (
                artifact_id, artifact_kind, stable_key, blob_relative_path, blob_fingerprint,
                schema_fingerprint, generator_fingerprint, runtime_capability_fingerprint,
                source_fingerprint, graph_fingerprint, output_profile_fingerprint,
                generation_parameters_json, status, dirty, byte_count, created_at_unix_ms, updated_at_unix_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'ready', 0, ?13, ?14, ?14)
            ON CONFLICT(artifact_id) DO UPDATE SET
                artifact_kind = excluded.artifact_kind,
                stable_key = excluded.stable_key,
                blob_relative_path = excluded.blob_relative_path,
                blob_fingerprint = excluded.blob_fingerprint,
                schema_fingerprint = excluded.schema_fingerprint,
                generator_fingerprint = excluded.generator_fingerprint,
                runtime_capability_fingerprint = excluded.runtime_capability_fingerprint,
                source_fingerprint = excluded.source_fingerprint,
                graph_fingerprint = excluded.graph_fingerprint,
                output_profile_fingerprint = excluded.output_profile_fingerprint,
                generation_parameters_json = excluded.generation_parameters_json,
                status = 'ready',
                dirty = 0,
                byte_count = excluded.byte_count,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![
                &intent.artifact_id,
                &intent.artifact_kind,
                &intent.stable_key,
                &record.blob_relative_path,
                record.blob_fingerprint.to_string(),
                &intent.schema_fingerprint,
                &intent.generator_fingerprint,
                &intent.runtime_capability_fingerprint,
                &intent.source_fingerprint,
                &intent.graph_fingerprint,
                &intent.output_profile_fingerprint,
                params_json,
                record.byte_count as i64,
                now_unix_ms(),
            ],
        )
        .map_err(|source| sqlite_error(&db_path, source))?;
        tx.commit()
            .map_err(|source| sqlite_error(&db_path, source))?;
        Ok(())
    }

    fn clear_temp_files(&self) -> Result<usize, ArtifactStoreError> {
        let tmp_dir = blob_tmp_path(&self.bundle_path);
        fs::create_dir_all(&tmp_dir).map_err(|source| ArtifactStoreError::Io {
            path: tmp_dir.clone(),
            source,
        })?;
        let mut removed = 0;
        for entry in fs::read_dir(&tmp_dir).map_err(|source| ArtifactStoreError::Io {
            path: tmp_dir.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| ArtifactStoreError::Io {
                path: tmp_dir.clone(),
                source,
            })?;
            let path = entry.path();
            if entry
                .file_type()
                .map_err(|source| ArtifactStoreError::Io {
                    path: path.clone(),
                    source,
                })?
                .is_file()
            {
                fs::remove_file(&path).map_err(|source| ArtifactStoreError::Io {
                    path: path.clone(),
                    source,
                })?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    fn tmp_path(&self, artifact_id: &str) -> PathBuf {
        let safe_artifact_id = artifact_id
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                    ch
                } else {
                    '_'
                }
            })
            .collect::<String>();
        blob_tmp_path(&self.bundle_path).join(format!(
            "{safe_artifact_id}-{}-{}.tmp",
            std::process::id(),
            now_unix_ms()
        ))
    }

    fn sqlite_error(&self, source: rusqlite::Error) -> ArtifactStoreError {
        ArtifactStoreError::Sqlite {
            path: self.store.db_path.clone(),
            source,
        }
    }
}

fn content_addressed_blob_path(fingerprint: &ArtifactFingerprint) -> String {
    let hex = fingerprint.content_hex();
    let prefix = hex.get(0..2).unwrap_or("00");
    format!("blobs/blake3/v1/{prefix}/{hex}.bin")
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn sync_parent_if_possible(path: &Path) {
    if let Some(parent) = path.parent() {
        if let Ok(parent_dir) = File::open(parent) {
            let _ = parent_dir.sync_all();
        }
    }
}

fn sqlite_error(path: &Path, source: rusqlite::Error) -> ArtifactStoreError {
    ArtifactStoreError::Sqlite {
        path: path.to_path_buf(),
        source,
    }
}
