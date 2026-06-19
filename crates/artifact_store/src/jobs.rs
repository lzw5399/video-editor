use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ArtifactStoreError;
use crate::schema::ArtifactStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    Proxy,
    Thumbnail,
    Waveform,
    GraphSnapshot,
    PreviewFrame,
    PreviewSegment,
    FfmpegScript,
    SyncManifest,
}

impl ArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proxy => "proxy",
            Self::Thumbnail => "thumbnail",
            Self::Waveform => "waveform",
            Self::GraphSnapshot => "graphSnapshot",
            Self::PreviewFrame => "previewFrame",
            Self::PreviewSegment => "previewSegment",
            Self::FfmpegScript => "ffmpegScript",
            Self::SyncManifest => "syncManifest",
        }
    }

    fn from_db(value: &str) -> Result<Self, ArtifactStoreError> {
        match value {
            "proxy" => Ok(Self::Proxy),
            "thumbnail" => Ok(Self::Thumbnail),
            "waveform" => Ok(Self::Waveform),
            "graphSnapshot" => Ok(Self::GraphSnapshot),
            "previewFrame" => Ok(Self::PreviewFrame),
            "previewSegment" => Ok(Self::PreviewSegment),
            "ffmpegScript" => Ok(Self::FfmpegScript),
            "syncManifest" => Ok(Self::SyncManifest),
            _ => invalid_job(value, "unknown artifact kind"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerationJobStatus {
    Waiting,
    Running,
    Completed,
    Failed,
    CancelRequested,
    Cancelled,
    Resumable,
}

impl GenerationJobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::CancelRequested => "cancelRequested",
            Self::Cancelled => "cancelled",
            Self::Resumable => "resumable",
        }
    }

    fn from_db(value: &str) -> Result<Self, ArtifactStoreError> {
        match value {
            "waiting" => Ok(Self::Waiting),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelRequested" => Ok(Self::CancelRequested),
            "cancelled" => Ok(Self::Cancelled),
            "resumable" => Ok(Self::Resumable),
            _ => invalid_job(value, "unknown generation job status"),
        }
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerationChunkStatus {
    Waiting,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl GenerationChunkStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn from_db(value: &str) -> Result<Self, ArtifactStoreError> {
        match value {
            "waiting" => Ok(Self::Waiting),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => invalid_job(value, "unknown generation chunk status"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerationCancellationState {
    NotRequested,
    Requested,
    Acknowledged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerationProgress {
    pub target_start_us: Option<u64>,
    pub target_duration_us: Option<u64>,
    pub progress_per_mille: Option<u16>,
}

impl GenerationProgress {
    pub fn new(
        target_start_us: Option<u64>,
        target_duration_us: Option<u64>,
        progress_per_mille: Option<u16>,
    ) -> Self {
        Self {
            target_start_us,
            target_duration_us,
            progress_per_mille,
        }
    }

    fn validate(&self) -> Result<(), ArtifactStoreError> {
        if let Some(progress) = self.progress_per_mille {
            if progress > 1000 {
                return invalid_job("progress", "progress per-mille must be <= 1000");
            }
        }
        validate_optional_us(self.target_start_us)?;
        validate_optional_us(self.target_duration_us)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactGenerationRequest {
    pub job_id: String,
    pub artifact_id: Option<String>,
    pub kind: ArtifactKind,
    pub stable_key: String,
    pub generation_parameters_json: Value,
    pub source_fingerprint: Option<String>,
    pub runtime_capability_fingerprint: Option<String>,
    pub output_profile_fingerprint: Option<String>,
    pub graph_fingerprint: Option<String>,
    pub chunks: Vec<GenerationProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactGenerationJob {
    pub job_id: String,
    pub artifact_id: Option<String>,
    pub kind: ArtifactKind,
    pub status: GenerationJobStatus,
    pub progress: GenerationProgress,
    pub generation_parameters_json: Value,
    pub chunks: Vec<GenerationChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerationChunk {
    pub job_id: String,
    pub chunk_index: u32,
    pub status: GenerationChunkStatus,
    pub target_start_us: Option<u64>,
    pub target_duration_us: Option<u64>,
    pub progress_per_mille: Option<u16>,
    pub blob_relative_path: Option<String>,
    pub blob_fingerprint: Option<String>,
    pub byte_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerationResumePlan {
    pub job_id: String,
    pub kind: ArtifactKind,
    pub pending_chunks: Vec<GenerationChunk>,
    pub completed_chunks: Vec<GenerationChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerationStatusSummary {
    pub job_id: String,
    pub kind: ArtifactKind,
    pub display_label: String,
    pub status: GenerationJobStatus,
    pub progress_per_mille: Option<u16>,
    pub can_retry: bool,
    pub can_resume: bool,
    pub can_cancel: bool,
    pub error_category: Option<String>,
}

pub fn create_generation_job(
    store: &mut ArtifactStore,
    request: ArtifactGenerationRequest,
) -> Result<ArtifactGenerationJob, ArtifactStoreError> {
    validate_request(&request)?;
    let params_json = serde_json::to_string(&request.generation_parameters_json)
        .map_err(|error| invalid_job_err(&request.job_id, format!("parameters: {error}")))?;
    let now = now_unix_ms();
    let db_path = store.db_path.clone();
    let tx = store
        .connection_mut()
        .transaction()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: db_path.clone(),
            source,
        })?;

    if let Some(artifact_id) = &request.artifact_id {
        tx.execute(
            "INSERT INTO artifact (
                artifact_id, artifact_kind, stable_key, schema_fingerprint, generator_fingerprint,
                runtime_capability_fingerprint, source_fingerprint, graph_fingerprint,
                output_profile_fingerprint, generation_parameters_json, status, dirty, byte_count,
                created_at_unix_ms, updated_at_unix_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'waiting', 0, 0, ?11, ?11)
            ON CONFLICT(artifact_id) DO UPDATE SET
                artifact_kind = excluded.artifact_kind,
                stable_key = excluded.stable_key,
                runtime_capability_fingerprint = excluded.runtime_capability_fingerprint,
                source_fingerprint = excluded.source_fingerprint,
                graph_fingerprint = excluded.graph_fingerprint,
                output_profile_fingerprint = excluded.output_profile_fingerprint,
                generation_parameters_json = excluded.generation_parameters_json,
                status = CASE WHEN artifact.status = 'ready' THEN artifact.status ELSE excluded.status END,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![
                artifact_id,
                request.kind.as_str(),
                request.stable_key,
                "artifact-store-schema:v1",
                "artifact-generation:v1",
                request.runtime_capability_fingerprint,
                request.source_fingerprint,
                request.graph_fingerprint,
                request.output_profile_fingerprint,
                params_json,
                now,
            ],
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: db_path.clone(),
            source,
        })?;
    }

    tx.execute(
        "INSERT INTO generation_job (
            job_id, artifact_id, job_kind, generation_parameters_json, status,
            progress_per_mille, cancel_requested, created_at_unix_ms, updated_at_unix_ms
        ) VALUES (?1, ?2, ?3, ?4, 'waiting', ?5, 0, ?6, ?6)
        ON CONFLICT(job_id) DO UPDATE SET
            artifact_id = excluded.artifact_id,
            job_kind = excluded.job_kind,
            generation_parameters_json = excluded.generation_parameters_json,
            status = CASE
                WHEN generation_job.status IN ('completed', 'failed', 'cancelled') THEN generation_job.status
                ELSE excluded.status
            END,
            progress_per_mille = CASE
                WHEN generation_job.status IN ('completed', 'failed', 'cancelled') THEN generation_job.progress_per_mille
                ELSE excluded.progress_per_mille
            END,
            updated_at_unix_ms = excluded.updated_at_unix_ms",
        params![
            request.job_id,
            request.artifact_id,
            request.kind.as_str(),
            serde_json::to_string(&request.generation_parameters_json)
                .map_err(|error| invalid_job_err("parameters", error.to_string()))?,
            initial_progress(&request.chunks),
            now,
        ],
    )
    .map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path.clone(),
        source,
    })?;

    tx.execute(
        "DELETE FROM generation_chunk WHERE job_id = ?1",
        [request.job_id.as_str()],
    )
    .map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path.clone(),
        source,
    })?;
    for (index, chunk) in request.chunks.iter().enumerate() {
        tx.execute(
            "INSERT INTO generation_chunk (
                job_id, chunk_index, status, target_start_us, target_duration_us,
                blob_relative_path, blob_fingerprint, byte_count, created_at_unix_ms, updated_at_unix_ms
            ) VALUES (?1, ?2, 'waiting', ?3, ?4, NULL, NULL, 0, ?5, ?5)",
            params![
                request.job_id,
                index as i64,
                optional_us_i64(chunk.target_start_us)?,
                optional_us_i64(chunk.target_duration_us)?,
                now,
            ],
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: db_path.clone(),
            source,
        })?;
    }
    tx.commit().map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path,
        source,
    })?;

    get_generation_job(store, &request.job_id)?
        .ok_or_else(|| invalid_job_err(&request.job_id, "job was not persisted"))
}

pub fn list_generation_jobs(
    store: &ArtifactStore,
) -> Result<Vec<ArtifactGenerationJob>, ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare("SELECT job_id FROM generation_job ORDER BY created_at_unix_ms, job_id")
        .map_err(|source| sqlite_error(store, source))?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|source| sqlite_error(store, source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| sqlite_error(store, source))?;
    ids.into_iter()
        .map(|id| {
            get_generation_job(store, &id)?
                .ok_or_else(|| invalid_job_err(&id, "job disappeared while listing"))
        })
        .collect()
}

pub fn next_pending_chunk(
    store: &ArtifactStore,
    job_id: &str,
) -> Result<Option<GenerationChunk>, ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(
            "SELECT job_id, chunk_index, status, target_start_us, target_duration_us,
                blob_relative_path, blob_fingerprint, byte_count
             FROM generation_chunk
             WHERE job_id = ?1 AND status IN ('waiting', 'failed', 'cancelled')
             ORDER BY chunk_index
             LIMIT 1",
        )
        .map_err(|source| sqlite_error(store, source))?;
    statement
        .query_row([job_id], chunk_from_row)
        .optional()
        .map_err(|source| sqlite_error(store, source))
}

pub fn start_generation_chunk(
    store: &mut ArtifactStore,
    job_id: &str,
    chunk_index: u32,
) -> Result<GenerationChunk, ArtifactStoreError> {
    transition_chunk(
        store,
        job_id,
        chunk_index,
        GenerationChunkStatus::Running,
        None,
        None,
        0,
    )?;
    set_job_status_if_not_terminal(store, job_id, GenerationJobStatus::Running)?;
    get_generation_chunk(store, job_id, chunk_index)?
        .ok_or_else(|| invalid_job_err(job_id, "chunk was not found after start"))
}

pub fn complete_generation_chunk(
    store: &mut ArtifactStore,
    job_id: &str,
    chunk_index: u32,
    blob_relative_path: Option<&str>,
    blob_fingerprint: Option<&str>,
    byte_count: u64,
) -> Result<GenerationChunk, ArtifactStoreError> {
    transition_chunk(
        store,
        job_id,
        chunk_index,
        GenerationChunkStatus::Completed,
        blob_relative_path,
        blob_fingerprint,
        byte_count,
    )?;
    refresh_job_rollup(store, job_id)?;
    get_generation_chunk(store, job_id, chunk_index)?
        .ok_or_else(|| invalid_job_err(job_id, "chunk was not found after completion"))
}

pub fn fail_generation_chunk(
    store: &mut ArtifactStore,
    job_id: &str,
    chunk_index: u32,
    _message: &str,
) -> Result<GenerationChunk, ArtifactStoreError> {
    transition_chunk(
        store,
        job_id,
        chunk_index,
        GenerationChunkStatus::Failed,
        None,
        None,
        0,
    )?;
    refresh_job_rollup(store, job_id)?;
    get_generation_chunk(store, job_id, chunk_index)?
        .ok_or_else(|| invalid_job_err(job_id, "chunk was not found after failure"))
}

pub fn cancel_generation_job(
    store: &mut ArtifactStore,
    job_id: &str,
) -> Result<ArtifactGenerationJob, ArtifactStoreError> {
    ensure_job_not_terminal(store, job_id)?;
    store
        .connection()
        .execute(
            "UPDATE generation_job
             SET status = 'cancelRequested', cancel_requested = 1, updated_at_unix_ms = ?2
             WHERE job_id = ?1 AND status NOT IN ('completed', 'failed', 'cancelled')",
            params![job_id, now_unix_ms()],
        )
        .map_err(|source| sqlite_error(store, source))?;
    get_generation_job(store, job_id)?
        .ok_or_else(|| invalid_job_err(job_id, "job was not found after cancel"))
}

pub fn acknowledge_generation_cancelled(
    store: &mut ArtifactStore,
    job_id: &str,
) -> Result<ArtifactGenerationJob, ArtifactStoreError> {
    ensure_job_not_terminal(store, job_id)?;
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
        "UPDATE generation_chunk
         SET status = 'cancelled', updated_at_unix_ms = ?2
         WHERE job_id = ?1 AND status IN ('waiting', 'running', 'failed')",
        params![job_id, now],
    )
    .map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path.clone(),
        source,
    })?;
    tx.execute(
        "UPDATE generation_job
         SET status = 'cancelled', cancel_requested = 1, updated_at_unix_ms = ?2
         WHERE job_id = ?1",
        params![job_id, now],
    )
    .map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path.clone(),
        source,
    })?;
    tx.commit().map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path,
        source,
    })?;
    get_generation_job(store, job_id)?
        .ok_or_else(|| invalid_job_err(job_id, "job was not found after cancel acknowledgement"))
}

pub fn resume_generation_job(
    store: &ArtifactStore,
    job_id: &str,
) -> Result<Option<GenerationResumePlan>, ArtifactStoreError> {
    let Some(job) = get_generation_job(store, job_id)? else {
        return Ok(None);
    };
    if job.status == GenerationJobStatus::Completed {
        return Ok(None);
    }
    let mut pending_chunks = Vec::new();
    let mut completed_chunks = Vec::new();
    for chunk in job.chunks {
        match chunk.status {
            GenerationChunkStatus::Completed => completed_chunks.push(chunk),
            GenerationChunkStatus::Waiting
            | GenerationChunkStatus::Failed
            | GenerationChunkStatus::Cancelled
            | GenerationChunkStatus::Running => pending_chunks.push(chunk),
        }
    }
    if pending_chunks.is_empty() {
        return Ok(None);
    }
    Ok(Some(GenerationResumePlan {
        job_id: job.job_id,
        kind: job.kind,
        pending_chunks,
        completed_chunks,
    }))
}

pub fn list_active_generation_jobs(
    store: &ArtifactStore,
) -> Result<Vec<ArtifactGenerationJob>, ArtifactStoreError> {
    Ok(list_generation_jobs(store)?
        .into_iter()
        .filter(|job| {
            !matches!(
                job.status,
                GenerationJobStatus::Completed
                    | GenerationJobStatus::Failed
                    | GenerationJobStatus::Cancelled
            )
        })
        .collect())
}

pub fn job_status_summary(
    store: &ArtifactStore,
    job_id: &str,
) -> Result<Option<GenerationStatusSummary>, ArtifactStoreError> {
    let Some(job) = get_generation_job(store, job_id)? else {
        return Ok(None);
    };
    let has_pending = job
        .chunks
        .iter()
        .any(|chunk| chunk.status != GenerationChunkStatus::Completed);
    Ok(Some(GenerationStatusSummary {
        job_id: job.job_id,
        kind: job.kind,
        display_label: format!("{} generation", job.kind.as_str()),
        status: job.status,
        progress_per_mille: job.progress.progress_per_mille,
        can_retry: matches!(job.status, GenerationJobStatus::Failed),
        can_resume: has_pending
            && matches!(
                job.status,
                GenerationJobStatus::Failed
                    | GenerationJobStatus::Cancelled
                    | GenerationJobStatus::Resumable
            ),
        can_cancel: matches!(
            job.status,
            GenerationJobStatus::Waiting
                | GenerationJobStatus::Running
                | GenerationJobStatus::CancelRequested
                | GenerationJobStatus::Resumable
        ),
        error_category: if matches!(job.status, GenerationJobStatus::Failed) {
            Some("generationFailed".to_owned())
        } else {
            None
        },
    }))
}

pub fn generation_cancel_requested(
    store: &ArtifactStore,
    job_id: &str,
) -> Result<bool, ArtifactStoreError> {
    store
        .connection()
        .query_row(
            "SELECT cancel_requested FROM generation_job WHERE job_id = ?1",
            [job_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map(|value| value.unwrap_or(0) != 0)
        .map_err(|source| sqlite_error(store, source))
}

fn validate_request(request: &ArtifactGenerationRequest) -> Result<(), ArtifactStoreError> {
    if request.job_id.trim().is_empty() {
        return invalid_job("job", "job id must not be empty");
    }
    if request.stable_key.trim().is_empty() {
        return invalid_job(&request.job_id, "stable key must not be empty");
    }
    if request.chunks.is_empty() {
        return invalid_job(&request.job_id, "job must contain at least one chunk");
    }
    for chunk in &request.chunks {
        chunk.validate()?;
    }
    Ok(())
}

fn transition_chunk(
    store: &mut ArtifactStore,
    job_id: &str,
    chunk_index: u32,
    next_status: GenerationChunkStatus,
    blob_relative_path: Option<&str>,
    blob_fingerprint: Option<&str>,
    byte_count: u64,
) -> Result<(), ArtifactStoreError> {
    ensure_job_not_terminal(store, job_id)?;
    let changed = store
        .connection()
        .execute(
            "UPDATE generation_chunk
             SET status = ?3, blob_relative_path = ?4, blob_fingerprint = ?5,
                byte_count = ?6, updated_at_unix_ms = ?7
             WHERE job_id = ?1 AND chunk_index = ?2",
            params![
                job_id,
                i64::from(chunk_index),
                next_status.as_str(),
                blob_relative_path,
                blob_fingerprint,
                i64::try_from(byte_count)
                    .map_err(|_| invalid_job_err(job_id, "byte count overflow"))?,
                now_unix_ms(),
            ],
        )
        .map_err(|source| sqlite_error(store, source))?;
    if changed == 0 {
        return invalid_job(job_id, "chunk does not exist");
    }
    Ok(())
}

fn refresh_job_rollup(store: &mut ArtifactStore, job_id: &str) -> Result<(), ArtifactStoreError> {
    let chunks = chunks_for_job(store, job_id)?;
    let completed = chunks
        .iter()
        .filter(|chunk| chunk.status == GenerationChunkStatus::Completed)
        .count();
    let progress = if chunks.is_empty() {
        None
    } else {
        Some(((completed as u64 * 1000) / chunks.len() as u64) as u16)
    };
    let status = if chunks
        .iter()
        .all(|chunk| chunk.status == GenerationChunkStatus::Completed)
    {
        GenerationJobStatus::Completed
    } else if chunks
        .iter()
        .any(|chunk| chunk.status == GenerationChunkStatus::Failed)
    {
        GenerationJobStatus::Failed
    } else if chunks
        .iter()
        .any(|chunk| chunk.status == GenerationChunkStatus::Cancelled)
    {
        GenerationJobStatus::Cancelled
    } else if chunks
        .iter()
        .any(|chunk| chunk.status == GenerationChunkStatus::Running)
    {
        GenerationJobStatus::Running
    } else {
        GenerationJobStatus::Waiting
    };
    store
        .connection()
        .execute(
            "UPDATE generation_job
             SET status = ?2, progress_per_mille = ?3, updated_at_unix_ms = ?4
             WHERE job_id = ?1 AND status NOT IN ('completed', 'cancelled')",
            params![job_id, status.as_str(), progress, now_unix_ms()],
        )
        .map_err(|source| sqlite_error(store, source))?;
    Ok(())
}

fn set_job_status_if_not_terminal(
    store: &mut ArtifactStore,
    job_id: &str,
    status: GenerationJobStatus,
) -> Result<(), ArtifactStoreError> {
    store
        .connection()
        .execute(
            "UPDATE generation_job
             SET status = ?2, updated_at_unix_ms = ?3
             WHERE job_id = ?1 AND status NOT IN ('completed', 'failed', 'cancelled')",
            params![job_id, status.as_str(), now_unix_ms()],
        )
        .map_err(|source| sqlite_error(store, source))?;
    Ok(())
}

fn ensure_job_not_terminal(store: &ArtifactStore, job_id: &str) -> Result<(), ArtifactStoreError> {
    let Some(status) = job_status(store, job_id)? else {
        return invalid_job(job_id, "job does not exist");
    };
    if status.is_terminal() {
        return invalid_job(job_id, "terminal job state cannot be overwritten");
    }
    Ok(())
}

fn get_generation_job(
    store: &ArtifactStore,
    job_id: &str,
) -> Result<Option<ArtifactGenerationJob>, ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(
            "SELECT job_id, artifact_id, job_kind, generation_parameters_json, status, progress_per_mille
             FROM generation_job
             WHERE job_id = ?1",
        )
        .map_err(|source| sqlite_error(store, source))?;
    let row = statement
        .query_row([job_id], |row| {
            let job_id: String = row.get(0)?;
            let artifact_id: Option<String> = row.get(1)?;
            let kind_string: String = row.get(2)?;
            let params_json: String = row.get(3)?;
            let status_string: String = row.get(4)?;
            let progress_per_mille: Option<u16> = row.get(5)?;
            Ok((
                job_id,
                artifact_id,
                kind_string,
                params_json,
                status_string,
                progress_per_mille,
            ))
        })
        .optional()
        .map_err(|source| sqlite_error(store, source))?;
    let Some((job_id, artifact_id, kind_string, params_json, status_string, progress_per_mille)) =
        row
    else {
        return Ok(None);
    };
    let generation_parameters_json = serde_json::from_str(&params_json)
        .map_err(|error| invalid_job_err(&job_id, format!("invalid parameters JSON: {error}")))?;
    let chunks = chunks_for_job(store, &job_id)?;
    Ok(Some(ArtifactGenerationJob {
        job_id,
        artifact_id,
        kind: ArtifactKind::from_db(&kind_string)?,
        status: GenerationJobStatus::from_db(&status_string)?,
        progress: GenerationProgress::new(None, None, progress_per_mille),
        generation_parameters_json,
        chunks,
    }))
}

fn get_generation_chunk(
    store: &ArtifactStore,
    job_id: &str,
    chunk_index: u32,
) -> Result<Option<GenerationChunk>, ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(
            "SELECT job_id, chunk_index, status, target_start_us, target_duration_us,
                blob_relative_path, blob_fingerprint, byte_count
             FROM generation_chunk
             WHERE job_id = ?1 AND chunk_index = ?2",
        )
        .map_err(|source| sqlite_error(store, source))?;
    statement
        .query_row(params![job_id, i64::from(chunk_index)], chunk_from_row)
        .optional()
        .map_err(|source| sqlite_error(store, source))
}

fn chunks_for_job(
    store: &ArtifactStore,
    job_id: &str,
) -> Result<Vec<GenerationChunk>, ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(
            "SELECT job_id, chunk_index, status, target_start_us, target_duration_us,
                blob_relative_path, blob_fingerprint, byte_count
             FROM generation_chunk
             WHERE job_id = ?1
             ORDER BY chunk_index",
        )
        .map_err(|source| sqlite_error(store, source))?;
    statement
        .query_map([job_id], chunk_from_row)
        .map_err(|source| sqlite_error(store, source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| sqlite_error(store, source))
}

fn chunk_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GenerationChunk> {
    let status_string: String = row.get(2)?;
    let status = GenerationChunkStatus::from_db(&status_string).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let chunk_index_i64: i64 = row.get(1)?;
    let byte_count_i64: i64 = row.get(7)?;
    Ok(GenerationChunk {
        job_id: row.get(0)?,
        chunk_index: u32::try_from(chunk_index_i64).map_err(|_| rusqlite::Error::InvalidQuery)?,
        status,
        target_start_us: optional_i64_us(row.get(3)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        target_duration_us: optional_i64_us(row.get(4)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        progress_per_mille: if status == GenerationChunkStatus::Completed {
            Some(1000)
        } else {
            Some(0)
        },
        blob_relative_path: row.get(5)?,
        blob_fingerprint: row.get(6)?,
        byte_count: u64::try_from(byte_count_i64).map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

fn job_status(
    store: &ArtifactStore,
    job_id: &str,
) -> Result<Option<GenerationJobStatus>, ArtifactStoreError> {
    let status = store
        .connection()
        .query_row(
            "SELECT status FROM generation_job WHERE job_id = ?1",
            [job_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|source| sqlite_error(store, source))?;
    status
        .map(|status| GenerationJobStatus::from_db(&status))
        .transpose()
}

fn initial_progress(chunks: &[GenerationProgress]) -> Option<u16> {
    if chunks
        .iter()
        .any(|chunk| chunk.progress_per_mille.is_some())
    {
        Some(
            chunks
                .iter()
                .filter_map(|chunk| chunk.progress_per_mille)
                .min()
                .unwrap_or(0),
        )
    } else {
        Some(0)
    }
}

fn validate_optional_us(value: Option<u64>) -> Result<(), ArtifactStoreError> {
    if value.is_some_and(|value| value > i64::MAX as u64) {
        return invalid_job("range", "microsecond values must fit SQLite integer");
    }
    Ok(())
}

fn optional_us_i64(value: Option<u64>) -> Result<Option<i64>, ArtifactStoreError> {
    value
        .map(|value| {
            i64::try_from(value).map_err(|_| invalid_job_err("range", "microsecond value overflow"))
        })
        .transpose()
}

fn optional_i64_us(value: Option<i64>) -> Result<Option<u64>, ArtifactStoreError> {
    value
        .map(|value| {
            u64::try_from(value).map_err(|_| invalid_job_err("range", "negative microsecond value"))
        })
        .transpose()
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

fn invalid_job<T>(id: &str, reason: &str) -> Result<T, ArtifactStoreError> {
    Err(invalid_job_err(id, reason))
}

fn invalid_job_err(id: &str, reason: impl Into<String>) -> ArtifactStoreError {
    ArtifactStoreError::InvalidDerivedPath {
        path: id.to_owned(),
        reason: reason.into(),
    }
}
