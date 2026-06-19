use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::ArtifactStoreError;
use crate::paths::{blob_tmp_path, derived_root_path, validate_derived_relative_path};
use crate::schema::ArtifactStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GcMode {
    DryRun,
    Apply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TombstoneReason {
    GarbageCollected,
    SourceDeleted,
    ManualCleanup,
}

impl TombstoneReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GarbageCollected => "garbageCollected",
            Self::SourceDeleted => "sourceDeleted",
            Self::ManualCleanup => "manualCleanup",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GcCandidate {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub blob_relative_path: String,
    pub blob_fingerprint: String,
    pub byte_count: u64,
    pub reason: TombstoneReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GcPlan {
    pub candidates: Vec<GcCandidate>,
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GcOutcome {
    pub mode: Option<GcMode>,
    pub candidates: Vec<GcCandidate>,
    pub deleted_artifact_ids: Vec<String>,
    pub deleted_blob_count: u64,
    pub reclaimable_bytes: u64,
    pub released_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TempSweepOutcome {
    pub removed_temp_files: usize,
}

pub fn plan_garbage_collection(store: &ArtifactStore) -> Result<GcPlan, ArtifactStoreError> {
    let live_artifacts = live_artifact_ids(store)?;
    let mut statement = store
        .connection()
        .prepare(
            "SELECT artifact_id, artifact_kind, blob_relative_path, blob_fingerprint, byte_count
             FROM artifact
             WHERE blob_relative_path IS NOT NULL
                AND blob_fingerprint IS NOT NULL
                AND status IN ('dirty', 'failed', 'tombstoned')
                AND artifact_id NOT IN (SELECT artifact_id FROM artifact_tombstone)
             ORDER BY artifact_id",
        )
        .map_err(|source| sqlite_error(store, source))?;
    let mut candidates = Vec::new();
    for row in statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|source| sqlite_error(store, source))?
    {
        let (artifact_id, artifact_kind, blob_relative_path, blob_fingerprint, byte_count) =
            row.map_err(|source| sqlite_error(store, source))?;
        if live_artifacts.contains(&artifact_id) {
            continue;
        }
        let Ok(byte_count) = u64::try_from(byte_count) else {
            continue;
        };
        candidates.push(GcCandidate {
            artifact_id,
            artifact_kind,
            blob_relative_path,
            blob_fingerprint,
            byte_count,
            reason: TombstoneReason::GarbageCollected,
        });
    }
    let reclaimable_bytes = candidates
        .iter()
        .map(|candidate| candidate.byte_count)
        .sum::<u64>();
    Ok(GcPlan {
        candidates,
        reclaimable_bytes,
    })
}

pub fn collect_garbage(
    store: &mut ArtifactStore,
    mode: GcMode,
) -> Result<GcOutcome, ArtifactStoreError> {
    let plan = plan_garbage_collection(store)?;
    if mode == GcMode::DryRun {
        return Ok(GcOutcome {
            mode: Some(mode),
            candidates: plan.candidates,
            deleted_artifact_ids: Vec::new(),
            deleted_blob_count: 0,
            reclaimable_bytes: plan.reclaimable_bytes,
            released_bytes: 0,
        });
    }

    let derived_root = derived_root_path(&store.config.bundle_path);
    let mut deleted_artifact_ids = Vec::new();
    let mut deleted_blob_count = 0_u64;
    let mut released_bytes = 0_u64;
    for candidate in &plan.candidates {
        let path = match deletable_blob_path(&derived_root, &candidate.blob_relative_path) {
            Ok(path) => path,
            Err(_) => continue,
        };
        if fs::metadata(&path)
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
        {
            fs::remove_file(&path).map_err(|source| ArtifactStoreError::Io {
                path: path.clone(),
                source,
            })?;
            deleted_blob_count += 1;
            released_bytes += candidate.byte_count;
        }
        mark_blob_tombstone(store, candidate)?;
        deleted_artifact_ids.push(candidate.artifact_id.clone());
    }

    Ok(GcOutcome {
        mode: Some(mode),
        candidates: plan.candidates,
        deleted_artifact_ids,
        deleted_blob_count,
        reclaimable_bytes: plan.reclaimable_bytes,
        released_bytes,
    })
}

pub fn mark_blob_tombstone(
    store: &mut ArtifactStore,
    candidate: &GcCandidate,
) -> Result<(), ArtifactStoreError> {
    let now = now_unix_ms();
    let db_path = store.db_path.clone();
    let tx = store
        .connection_mut()
        .transaction()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: db_path.clone(),
            source,
        })?;
    tx.execute(
        "INSERT INTO artifact_tombstone (
            tombstone_id, artifact_id, blob_relative_path, blob_fingerprint, byte_count, reason,
            created_at_unix_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(tombstone_id) DO UPDATE SET
            blob_relative_path = excluded.blob_relative_path,
            blob_fingerprint = excluded.blob_fingerprint,
            byte_count = excluded.byte_count,
            reason = excluded.reason,
            created_at_unix_ms = excluded.created_at_unix_ms",
        params![
            format!("{}:{}", candidate.artifact_id, candidate.blob_fingerprint),
            candidate.artifact_id,
            candidate.blob_relative_path,
            candidate.blob_fingerprint,
            i64::try_from(candidate.byte_count).map_err(|_| ArtifactStoreError::RangeOverflow {
                start_us: candidate.byte_count,
                duration_us: 0,
            })?,
            candidate.reason.as_str(),
            now,
        ],
    )
    .map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path.clone(),
        source,
    })?;
    tx.execute(
        "UPDATE artifact
         SET status = 'tombstoned', dirty = 1, updated_at_unix_ms = ?2
         WHERE artifact_id = ?1",
        params![candidate.artifact_id, now],
    )
    .map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path.clone(),
        source,
    })?;
    tx.commit().map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path,
        source,
    })?;
    Ok(())
}

pub fn sweep_temporary_blobs(
    bundle_path: impl AsRef<Path>,
) -> Result<TempSweepOutcome, ArtifactStoreError> {
    let tmp_dir = blob_tmp_path(bundle_path.as_ref());
    fs::create_dir_all(&tmp_dir).map_err(|source| ArtifactStoreError::Io {
        path: tmp_dir.clone(),
        source,
    })?;
    let mut removed_temp_files = 0;
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
            removed_temp_files += 1;
        }
    }
    Ok(TempSweepOutcome { removed_temp_files })
}

fn live_artifact_ids(store: &ArtifactStore) -> Result<BTreeSet<String>, ArtifactStoreError> {
    let mut live = BTreeSet::new();
    extend_live_ids(
        store,
        &mut live,
        "SELECT artifact_id FROM artifact
         WHERE status IN ('ready', 'waiting', 'running')
            OR dirty = 0
         ORDER BY artifact_id",
    )?;
    extend_live_ids(
        store,
        &mut live,
        "SELECT artifact_id FROM (
            SELECT artifact_dependency.artifact_id
            FROM artifact_dependency
            JOIN resource ON resource.resource_id = artifact_dependency.dependency_key
            WHERE artifact_dependency.dependency_kind = 'resource'
                AND resource.status = 'ready'
            UNION
            SELECT artifact_dependency.artifact_id
            FROM artifact_dependency
            JOIN resource ON resource.resource_id = 'material:' || artifact_dependency.dependency_key
            WHERE artifact_dependency.dependency_kind = 'material'
                AND resource.status = 'ready'
         )
         ORDER BY artifact_id",
    )?;
    extend_live_ids(
        store,
        &mut live,
        "SELECT artifact_id FROM generation_job
         WHERE artifact_id IS NOT NULL
            AND status NOT IN ('completed', 'failed', 'cancelled')
         ORDER BY artifact_id",
    )?;
    extend_live_ids(
        store,
        &mut live,
        "SELECT generation_job.artifact_id
         FROM generation_job
         JOIN generation_chunk ON generation_chunk.job_id = generation_job.job_id
         WHERE generation_job.artifact_id IS NOT NULL
            AND generation_chunk.status IN ('waiting', 'running')
         ORDER BY generation_job.artifact_id",
    )?;
    Ok(live)
}

fn extend_live_ids(
    store: &ArtifactStore,
    live: &mut BTreeSet<String>,
    sql: &str,
) -> Result<(), ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(sql)
        .map_err(|source| sqlite_error(store, source))?;
    for artifact_id in statement
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .map_err(|source| sqlite_error(store, source))?
    {
        if let Some(artifact_id) = artifact_id.map_err(|source| sqlite_error(store, source))? {
            live.insert(artifact_id);
        }
    }
    Ok(())
}

fn deletable_blob_path(
    derived_root: &Path,
    relative_path: &str,
) -> Result<PathBuf, ArtifactStoreError> {
    let path = validate_derived_relative_path(derived_root, relative_path)?;
    if !relative_path.replace('\\', "/").starts_with("blobs/") {
        return Err(ArtifactStoreError::InvalidDerivedPath {
            path: relative_path.to_owned(),
            reason: "GC only deletes derived blob paths".to_owned(),
        });
    }
    if relative_path.replace('\\', "/").starts_with("blobs/tmp/") {
        return Err(ArtifactStoreError::InvalidDerivedPath {
            path: relative_path.to_owned(),
            reason: "temporary blobs are swept separately".to_owned(),
        });
    }
    Ok(path)
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn sqlite_error(store: &ArtifactStore, source: rusqlite::Error) -> ArtifactStoreError {
    ArtifactStoreError::Sqlite {
        path: store.db_path.clone(),
        source,
    }
}
