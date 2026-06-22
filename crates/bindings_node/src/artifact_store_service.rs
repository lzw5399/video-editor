use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use artifact_store::ArtifactStoreError;
use artifact_store::fingerprint::fingerprint_file;
use artifact_store::gc::{GcMode, collect_garbage};
use artifact_store::generation::{
    ArtifactGenerator, GeneratedArtifact, GeneratedArtifactMime, GenerationWorkerContext,
    ProxyGenerationRequest, ThumbnailGenerationRequest, WaveformGenerationRequest,
    generate_thumbnail_artifact,
};
use artifact_store::jobs::{
    ArtifactGenerationJob, GenerationJobStatus, GenerationStatusSummary, cancel_generation_job,
    job_status_summary, list_active_generation_jobs, restart_generation_job, resume_generation_job,
};
use artifact_store::quota::{QuotaState, compute_quota_state};
use artifact_store::resource_index::{index_draft_resources, resource_ref_for_material};
use artifact_store::schema::{ArtifactStore, open_artifact_store};
use draft_model::{
    ArtifactGenerationActionCommandPayload, ArtifactGenerationTaskSummary,
    ArtifactMaintenanceResult, ArtifactQuotaStatus, ArtifactStatusSummary, ArtifactTaskStatus,
    CommandError, CommandErrorKind, CommandName, CommandPayload, CommandResultEnvelope,
    DisplayableArtifactRef, Draft, GetArtifactQuotaStatusCommandPayload,
    GetArtifactStatusCommandPayload, Material, MaterialArtifactStatus, MaterialId, MaterialKind,
    MaterialStatus, RefreshArtifactStatusCommandPayload,
    RunArtifactGarbageCollectionCommandPayload,
};
use media_runtime::{FfmpegExecutor, RuntimeConfig, discover_runtime_config};
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::resolve_material_uri;
use rusqlite::OptionalExtension;
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactStoreCommandKind {
    GetStatus,
    RefreshStatus,
    RetryGeneration,
    ResumeGeneration,
    CancelGeneration,
    GetQuota,
    RunGarbageCollection,
}

#[derive(Debug)]
pub struct ArtifactStoreBindingService {
    session_id: String,
    bundle_path: PathBuf,
    store: ArtifactStore,
    draft: Option<Draft>,
    material_id: Option<MaterialId>,
}

impl ArtifactStoreBindingService {
    pub fn open(
        session_id: String,
        bundle_path: String,
        material_id: Option<MaterialId>,
    ) -> Result<Self, ArtifactBindingError> {
        validate_session_id(&session_id)?;
        let bundle_path = validate_bundle_path(bundle_path)?;
        let store = open_artifact_store(bundle_path).map_err(ArtifactBindingError::Store)?;
        let bundle_path = std::fs::canonicalize(&store.config.bundle_path)
            .unwrap_or_else(|_| store.config.bundle_path.clone());
        let project_snapshot = crate::project_session_service::project_session_artifact_snapshot(
            &session_id,
            &bundle_path,
        )
        .map_err(|_| ArtifactBindingError::InvalidSession)?;
        let bundle_path = project_snapshot
            .as_ref()
            .map(|snapshot| snapshot.bundle_path.clone())
            .unwrap_or(bundle_path);
        let draft = project_snapshot.map(|snapshot| snapshot.draft);
        Ok(Self {
            session_id,
            bundle_path,
            store,
            draft,
            material_id,
        })
    }

    pub fn status(&self) -> Result<ArtifactStatusSummary, ArtifactBindingError> {
        artifact_status_response_from_store(
            &self.session_id,
            &self.store,
            self.draft.as_ref(),
            self.material_id.as_ref(),
        )
    }

    pub fn refresh_status(&mut self) -> Result<ArtifactStatusSummary, ArtifactBindingError> {
        if let Some(draft) = self.draft.clone() {
            refresh_material_thumbnails(&self.bundle_path, &draft, self.material_id.as_ref())?;
        }
        self.status()
    }

    pub fn quota(&self) -> Result<ArtifactQuotaStatus, ArtifactBindingError> {
        let quota = compute_quota_state(&self.store).map_err(ArtifactBindingError::Store)?;
        Ok(quota_status_response_from_store(&quota))
    }

    pub fn collect_garbage(
        &mut self,
        dry_run: bool,
    ) -> Result<ArtifactMaintenanceResult, ArtifactBindingError> {
        let mode = if dry_run {
            GcMode::DryRun
        } else {
            GcMode::Apply
        };
        let outcome =
            collect_garbage(&mut self.store, mode).map_err(ArtifactBindingError::Store)?;
        let reclaimable_label = format_bytes(outcome.reclaimable_bytes);
        let released_label = format_bytes(outcome.released_bytes);
        Ok(ArtifactMaintenanceResult {
            session_id: self.session_id.clone(),
            status_label: if dry_run {
                "缓存空间正常".to_owned()
            } else {
                "缓存清理完成".to_owned()
            },
            mode: if dry_run { "dryRun" } else { "apply" }.to_owned(),
            affected_count: u32::try_from(outcome.candidates.len()).unwrap_or(u32::MAX),
            reclaimable_label,
            released_label,
            completed: true,
        })
    }

    pub fn cancel_generation(
        &mut self,
        job_id: &str,
    ) -> Result<ArtifactGenerationTaskSummary, ArtifactBindingError> {
        let job =
            cancel_generation_job(&mut self.store, job_id).map_err(ArtifactBindingError::Store)?;
        Ok(task_summary_from_job(&job))
    }

    pub fn resume_generation(
        &mut self,
        job_id: &str,
    ) -> Result<ArtifactGenerationTaskSummary, ArtifactBindingError> {
        let plan =
            resume_generation_job(&self.store, job_id).map_err(ArtifactBindingError::Store)?;
        if plan.is_none() {
            return Err(ArtifactBindingError::ActionUnavailable);
        }
        let job =
            restart_generation_job(&mut self.store, job_id).map_err(ArtifactBindingError::Store)?;
        Ok(task_summary_from_job(&job))
    }

    pub fn retry_generation(
        &mut self,
        job_id: &str,
    ) -> Result<ArtifactGenerationTaskSummary, ArtifactBindingError> {
        let summary =
            job_status_summary(&self.store, job_id).map_err(ArtifactBindingError::Store)?;
        let summary = summary.ok_or(ArtifactBindingError::UnknownJob)?;
        if !summary.can_retry {
            return Err(ArtifactBindingError::ActionUnavailable);
        }
        let job =
            restart_generation_job(&mut self.store, job_id).map_err(ArtifactBindingError::Store)?;
        Ok(task_summary_from_job(&job))
    }
}

#[derive(Debug)]
pub enum ArtifactBindingError {
    InvalidInput,
    InvalidSession,
    UnknownJob,
    ActionUnavailable,
    RuntimeUnavailable,
    Store(ArtifactStoreError),
}

pub fn handle_artifact_store_command(
    command: CommandName,
    payload: CommandPayload,
) -> CommandResultEnvelope<serde_json::Value> {
    match artifact_command_result(command.clone(), payload) {
        Ok(value) => CommandResultEnvelope {
            ok: true,
            data: Some(value),
            error: None,
            events: Vec::new(),
        },
        Err(error) => artifact_error_envelope(error, command),
    }
}

pub fn artifact_status_response_from_store(
    session_id: &str,
    store: &ArtifactStore,
    draft: Option<&Draft>,
    material_id: Option<&MaterialId>,
) -> Result<ArtifactStatusSummary, ArtifactBindingError> {
    let tasks = list_active_generation_jobs(store)
        .map_err(ArtifactBindingError::Store)?
        .into_iter()
        .map(|job| {
            job_status_summary(store, &job.job_id)
                .map_err(ArtifactBindingError::Store)?
                .map(task_summary_from_generation_summary)
                .ok_or(ArtifactBindingError::UnknownJob)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let quota = compute_quota_state(store).map_err(ArtifactBindingError::Store)?;
    let materials = match draft {
        Some(draft) => material_artifact_statuses(store, draft, material_id)?,
        None => Vec::new(),
    };
    Ok(ArtifactStatusSummary {
        session_id: session_id.to_owned(),
        status_label: if !materials.is_empty() && tasks.is_empty() {
            "资源就绪".to_owned()
        } else if tasks.is_empty() {
            "暂无资源任务".to_owned()
        } else {
            "生成中".to_owned()
        },
        materials,
        tasks,
        quota: quota_status_response_from_store(&quota),
        refresh_available: true,
    })
}

pub fn quota_status_response_from_store(quota: &QuotaState) -> ArtifactQuotaStatus {
    ArtifactQuotaStatus {
        status_label: quota.status_label.clone(),
        severity: quota.quota_severity.clone(),
        used_label: quota.labels.used_label.clone(),
        reclaimable_label: quota.labels.reclaimable_label.clone(),
        released_label: quota.labels.released_label.clone(),
        cleanup_available: quota.cleanup_available,
    }
}

fn artifact_command_result(
    command: CommandName,
    payload: CommandPayload,
) -> Result<serde_json::Value, ArtifactBindingError> {
    match (command, payload) {
        (CommandName::GetArtifactStatus, CommandPayload::GetArtifactStatus(payload)) => {
            let service = service_from_status_payload(payload)?;
            serialize(service.status()?)
        }
        (CommandName::RefreshArtifactStatus, CommandPayload::RefreshArtifactStatus(payload)) => {
            let mut service = service_from_refresh_payload(payload)?;
            serialize(service.refresh_status()?)
        }
        (CommandName::GetArtifactQuotaStatus, CommandPayload::GetArtifactQuotaStatus(payload)) => {
            let service = service_from_quota_payload(payload)?;
            serialize(service.quota()?)
        }
        (
            CommandName::RunArtifactGarbageCollection,
            CommandPayload::RunArtifactGarbageCollection(payload),
        ) => {
            let mut service = service_from_gc_payload(&payload)?;
            serialize(service.collect_garbage(payload.dry_run)?)
        }
        (
            CommandName::RetryArtifactGeneration,
            CommandPayload::RetryArtifactGeneration(payload),
        ) => {
            let mut service = service_from_action_payload(&payload)?;
            serialize(service.retry_generation(&payload.job_id)?)
        }
        (
            CommandName::ResumeArtifactGeneration,
            CommandPayload::ResumeArtifactGeneration(payload),
        ) => {
            let mut service = service_from_action_payload(&payload)?;
            serialize(service.resume_generation(&payload.job_id)?)
        }
        (
            CommandName::CancelArtifactGeneration,
            CommandPayload::CancelArtifactGeneration(payload),
        ) => {
            let mut service = service_from_action_payload(&payload)?;
            serialize(service.cancel_generation(&payload.job_id)?)
        }
        _ => Err(ArtifactBindingError::InvalidInput),
    }
}

fn service_from_status_payload(
    payload: GetArtifactStatusCommandPayload,
) -> Result<ArtifactStoreBindingService, ArtifactBindingError> {
    ArtifactStoreBindingService::open(payload.session_id, payload.bundle_path, payload.material_id)
}

fn service_from_refresh_payload(
    payload: RefreshArtifactStatusCommandPayload,
) -> Result<ArtifactStoreBindingService, ArtifactBindingError> {
    ArtifactStoreBindingService::open(payload.session_id, payload.bundle_path, payload.material_id)
}

fn service_from_quota_payload(
    payload: GetArtifactQuotaStatusCommandPayload,
) -> Result<ArtifactStoreBindingService, ArtifactBindingError> {
    ArtifactStoreBindingService::open(payload.session_id, payload.bundle_path, None)
}

fn service_from_gc_payload(
    payload: &RunArtifactGarbageCollectionCommandPayload,
) -> Result<ArtifactStoreBindingService, ArtifactBindingError> {
    ArtifactStoreBindingService::open(
        payload.session_id.clone(),
        payload.bundle_path.clone(),
        None,
    )
}

fn service_from_action_payload(
    payload: &ArtifactGenerationActionCommandPayload,
) -> Result<ArtifactStoreBindingService, ArtifactBindingError> {
    if payload.job_id.trim().is_empty() {
        return Err(ArtifactBindingError::InvalidInput);
    }
    ArtifactStoreBindingService::open(
        payload.session_id.clone(),
        payload.bundle_path.clone(),
        None,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ThumbnailArtifactRow {
    status: String,
    dirty: bool,
    blob_relative_path: Option<String>,
    source_fingerprint: Option<String>,
}

fn material_artifact_statuses(
    store: &ArtifactStore,
    draft: &Draft,
    material_filter: Option<&MaterialId>,
) -> Result<Vec<MaterialArtifactStatus>, ArtifactBindingError> {
    let _index = index_draft_resources(&store.config.bundle_path, draft)
        .map_err(ArtifactBindingError::Store)?;
    let mut statuses = Vec::new();
    for material in draft.materials.iter().filter(|material| {
        material_filter
            .map(|filter| &material.material_id == filter)
            .unwrap_or(true)
            && supports_thumbnail(material)
    }) {
        statuses.push(thumbnail_status_for_material(store, material)?);
    }
    Ok(statuses)
}

fn thumbnail_status_for_material(
    store: &ArtifactStore,
    material: &Material,
) -> Result<MaterialArtifactStatus, ArtifactBindingError> {
    let row = thumbnail_artifact_row(store, material.material_id.as_str())?;
    let (status, status_label, progress_per_mille, display_ref, error_category) = match row {
        Some(row) if row.status == "ready" && !row.dirty => match row.blob_relative_path {
            Some(relative_path) if store.config.derived_path.join(&relative_path).is_file() => (
                ArtifactTaskStatus::Ready,
                "资源就绪".to_owned(),
                Some(1000),
                Some(DisplayableArtifactRef {
                    label: format!("{} 缩略图", material.display_name),
                    project_relative_ref: format!("derived/{relative_path}"),
                    artifact_kind: "thumbnail".to_owned(),
                }),
                None,
            ),
            _ => (
                ArtifactTaskStatus::Dirty,
                "待刷新".to_owned(),
                None,
                None,
                Some("missingBlob".to_owned()),
            ),
        },
        Some(row) if row.status == "waiting" => (
            ArtifactTaskStatus::Waiting,
            "等待生成".to_owned(),
            None,
            None,
            None,
        ),
        Some(row) if row.status == "running" => (
            ArtifactTaskStatus::Running,
            "生成中".to_owned(),
            None,
            None,
            None,
        ),
        Some(row) if row.status == "failed" => (
            ArtifactTaskStatus::Failed,
            "生成失败".to_owned(),
            None,
            None,
            Some("generationFailed".to_owned()),
        ),
        Some(row) if row.status == "cancelled" => (
            ArtifactTaskStatus::Cancelled,
            "已取消".to_owned(),
            None,
            None,
            None,
        ),
        Some(_) | None => (
            ArtifactTaskStatus::Dirty,
            "待刷新".to_owned(),
            None,
            None,
            None,
        ),
    };

    Ok(MaterialArtifactStatus {
        material_id: material.material_id.clone(),
        material_label: material.display_name.clone(),
        artifact_kind: "thumbnail".to_owned(),
        status,
        status_label,
        progress_per_mille,
        can_refresh: material.status == MaterialStatus::Available,
        can_retry: matches!(status, ArtifactTaskStatus::Failed),
        can_resume: false,
        can_cancel: matches!(
            status,
            ArtifactTaskStatus::Waiting | ArtifactTaskStatus::Running
        ),
        display_ref,
        error_category,
    })
}

fn refresh_material_thumbnails(
    bundle_path: &Path,
    draft: &Draft,
    material_filter: Option<&MaterialId>,
) -> Result<(), ArtifactBindingError> {
    let index = index_draft_resources(bundle_path, draft).map_err(ArtifactBindingError::Store)?;
    let runtime =
        discover_runtime_config().map_err(|_| ArtifactBindingError::RuntimeUnavailable)?;
    let executor = DesktopFfmpegExecutor::with_timeout(Duration::from_secs(20));
    let mut generator = DesktopThumbnailGenerator { runtime, executor };

    for material in draft.materials.iter().filter(|material| {
        material_filter
            .map(|filter| &material.material_id == filter)
            .unwrap_or(true)
            && supports_thumbnail(material)
            && material.status == MaterialStatus::Available
    }) {
        let resource_ref = resource_ref_for_material(material.material_id.as_str());
        let Some(resource) = index.resource(resource_ref.resource_id.as_str()) else {
            continue;
        };
        let Some(source_path) =
            resolve_material_uri(bundle_path, &material.uri).map_err(|source| {
                ArtifactBindingError::Store(ArtifactStoreError::InvalidResourceRef {
                    resource_id: resource.resource_id.as_str().to_owned(),
                    reason: source.to_string(),
                })
            })?
        else {
            continue;
        };
        if !source_path.is_file() {
            continue;
        }

        let source_fingerprint =
            fingerprint_file(&source_path).map_err(ArtifactBindingError::Store)?;
        if ready_thumbnail_matches_source(
            bundle_path,
            material.material_id.as_str(),
            source_fingerprint.as_str(),
        )? {
            continue;
        }

        let target_time_us = thumbnail_target_time_us(material);
        let request = ThumbnailGenerationRequest {
            job_id: format!(
                "thumbnail-job-{}-{}",
                safe_artifact_identifier(material.material_id.as_str()),
                now_unix_ms()
            ),
            artifact_id: thumbnail_artifact_id(material.material_id.as_str()),
            resource_id: resource.resource_id.clone(),
            material_id: material.material_id.clone(),
            source_ref: source_path.display().to_string(),
            source_fingerprint: source_fingerprint.to_string(),
            runtime_capability_fingerprint: generator.runtime.ffmpeg.version.clone(),
            output_profile_fingerprint: "thumbnail-png-320w:v1".to_owned(),
            generation_parameters_json: json!({
                "kind": "materialThumbnail",
                "materialId": material.material_id.as_str(),
                "targetTimeUs": target_time_us,
                "maxWidth": 320,
                "format": "png"
            }),
            target_time_us,
            expected_mime: GeneratedArtifactMime::ImagePng,
            extension: "png".to_owned(),
        };
        generate_thumbnail_artifact(bundle_path, &mut generator, request)
            .map_err(ArtifactBindingError::Store)?;
    }
    Ok(())
}

fn thumbnail_artifact_row(
    store: &ArtifactStore,
    material_id: &str,
) -> Result<Option<ThumbnailArtifactRow>, ArtifactBindingError> {
    store
        .connection()
        .query_row(
            "SELECT status, dirty, blob_relative_path, source_fingerprint
             FROM artifact
             WHERE artifact_kind = 'thumbnail' AND artifact_id = ?1
             ORDER BY updated_at_unix_ms DESC
             LIMIT 1",
            [thumbnail_artifact_id(material_id)],
            |row| {
                Ok(ThumbnailArtifactRow {
                    status: row.get(0)?,
                    dirty: row.get::<_, i64>(1)? != 0,
                    blob_relative_path: row.get(2)?,
                    source_fingerprint: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|source| {
            ArtifactBindingError::Store(ArtifactStoreError::Sqlite {
                path: store.db_path.clone(),
                source,
            })
        })
}

fn ready_thumbnail_matches_source(
    bundle_path: &Path,
    material_id: &str,
    source_fingerprint: &str,
) -> Result<bool, ArtifactBindingError> {
    let store = open_artifact_store(bundle_path).map_err(ArtifactBindingError::Store)?;
    let Some(row) = thumbnail_artifact_row(&store, material_id)? else {
        return Ok(false);
    };
    Ok(row.status == "ready"
        && !row.dirty
        && row.source_fingerprint.as_deref() == Some(source_fingerprint)
        && row
            .blob_relative_path
            .as_deref()
            .map(|relative_path| store.config.derived_path.join(relative_path).is_file())
            .unwrap_or(false))
}

fn supports_thumbnail(material: &Material) -> bool {
    matches!(
        material.kind,
        MaterialKind::Video | MaterialKind::Image | MaterialKind::Sticker
    ) && material.metadata.has_video
}

fn thumbnail_target_time_us(material: &Material) -> u64 {
    material
        .metadata
        .duration
        .map(|duration| duration.get() / 2)
        .unwrap_or(0)
        .min(1_000_000)
}

fn thumbnail_artifact_id(material_id: &str) -> String {
    format!("thumbnail-{}", safe_artifact_identifier(material_id))
}

fn safe_artifact_identifier(value: &str) -> String {
    let safe = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if safe.is_empty() {
        "material".to_owned()
    } else {
        safe
    }
}

#[derive(Debug)]
struct DesktopThumbnailGenerator {
    runtime: RuntimeConfig,
    executor: DesktopFfmpegExecutor,
}

impl ArtifactGenerator for DesktopThumbnailGenerator {
    fn generate_proxy(
        &mut self,
        _context: &GenerationWorkerContext,
        request: &ProxyGenerationRequest,
    ) -> Result<GeneratedArtifact, ArtifactStoreError> {
        Err(ArtifactStoreError::InvalidDerivedPath {
            path: request.artifact_id.clone(),
            reason: "desktop thumbnail generator cannot generate proxy artifacts".to_owned(),
        })
    }

    fn generate_thumbnail(
        &mut self,
        _context: &GenerationWorkerContext,
        request: &ThumbnailGenerationRequest,
    ) -> Result<GeneratedArtifact, ArtifactStoreError> {
        let mut args = vec![
            OsString::from("-hide_banner"),
            OsString::from("-loglevel"),
            OsString::from("error"),
            OsString::from("-nostdin"),
            OsString::from("-y"),
        ];
        if request.target_time_us > 0 {
            args.push(OsString::from("-ss"));
            args.push(OsString::from(format_timestamp_us(request.target_time_us)));
        }
        args.extend([
            OsString::from("-i"),
            OsString::from(&request.source_ref),
            OsString::from("-an"),
            OsString::from("-frames:v"),
            OsString::from("1"),
            OsString::from("-vf"),
            OsString::from("scale=320:-2:flags=lanczos"),
            OsString::from("-f"),
            OsString::from("image2pipe"),
            OsString::from("-vcodec"),
            OsString::from("png"),
            OsString::from("pipe:1"),
        ]);

        let output = self
            .executor
            .run(&self.runtime.ffmpeg.path, &args)
            .map_err(|source| ArtifactStoreError::Io {
                path: self.runtime.ffmpeg.path.clone(),
                source,
            })?;
        if !output.status.success() {
            return Err(ArtifactStoreError::InvalidDerivedPath {
                path: request.artifact_id.clone(),
                reason: "ffmpeg thumbnail generation failed".to_owned(),
            });
        }
        Ok(GeneratedArtifact::new(
            GeneratedArtifactMime::ImagePng,
            request.extension.clone(),
            output.stdout,
        ))
    }

    fn generate_waveform(
        &mut self,
        _context: &GenerationWorkerContext,
        request: &WaveformGenerationRequest,
    ) -> Result<GeneratedArtifact, ArtifactStoreError> {
        Err(ArtifactStoreError::InvalidDerivedPath {
            path: request.artifact_id.clone(),
            reason: "desktop thumbnail generator cannot generate waveform artifacts".to_owned(),
        })
    }
}

fn format_timestamp_us(value: u64) -> String {
    format!("{}.{:06}", value / 1_000_000, value % 1_000_000)
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn task_summary_from_job(job: &ArtifactGenerationJob) -> ArtifactGenerationTaskSummary {
    ArtifactGenerationTaskSummary {
        job_id: job.job_id.clone(),
        artifact_kind: job.kind.as_str().to_owned(),
        display_label: format!("{} generation", job.kind.as_str()),
        status: task_status(job.status),
        status_label: status_label(job.status).to_owned(),
        progress_per_mille: job.progress.progress_per_mille,
        can_retry: matches!(job.status, GenerationJobStatus::Failed),
        can_resume: matches!(
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
    }
}

fn task_summary_from_generation_summary(
    summary: GenerationStatusSummary,
) -> ArtifactGenerationTaskSummary {
    ArtifactGenerationTaskSummary {
        job_id: summary.job_id,
        artifact_kind: summary.kind.as_str().to_owned(),
        display_label: summary.display_label,
        status: task_status(summary.status),
        status_label: status_label(summary.status).to_owned(),
        progress_per_mille: summary.progress_per_mille,
        can_retry: summary.can_retry,
        can_resume: summary.can_resume,
        can_cancel: summary.can_cancel,
        error_category: summary.error_category,
    }
}

fn task_status(status: GenerationJobStatus) -> ArtifactTaskStatus {
    match status {
        GenerationJobStatus::Waiting => ArtifactTaskStatus::Waiting,
        GenerationJobStatus::Running => ArtifactTaskStatus::Running,
        GenerationJobStatus::Completed => ArtifactTaskStatus::Ready,
        GenerationJobStatus::Failed => ArtifactTaskStatus::Failed,
        GenerationJobStatus::CancelRequested => ArtifactTaskStatus::CancelRequested,
        GenerationJobStatus::Cancelled => ArtifactTaskStatus::Cancelled,
        GenerationJobStatus::Resumable => ArtifactTaskStatus::Resumable,
    }
}

fn status_label(status: GenerationJobStatus) -> &'static str {
    match status {
        GenerationJobStatus::Waiting => "等待生成",
        GenerationJobStatus::Running => "生成中",
        GenerationJobStatus::Completed => "资源就绪",
        GenerationJobStatus::Failed => "生成失败",
        GenerationJobStatus::CancelRequested => "正在取消",
        GenerationJobStatus::Cancelled => "已取消",
        GenerationJobStatus::Resumable => "可继续",
    }
}

fn artifact_error_envelope(
    error: ArtifactBindingError,
    command: CommandName,
) -> CommandResultEnvelope<serde_json::Value> {
    let message = match error {
        ArtifactBindingError::InvalidInput => "Invalid artifact command payload",
        ArtifactBindingError::InvalidSession => "Artifact project session is invalid",
        ArtifactBindingError::UnknownJob => "Artifact generation job was not found",
        ArtifactBindingError::ActionUnavailable => "Artifact generation action is unavailable",
        ArtifactBindingError::RuntimeUnavailable => "Bundled media runtime is unavailable",
        ArtifactBindingError::Store(_) => "Artifact store operation failed",
    };
    CommandResultEnvelope {
        ok: false,
        data: None,
        error: Some(CommandError {
            kind: CommandErrorKind::ArtifactStoreFailed,
            message: message.to_owned(),
            command: command_wire_name(command),
        }),
        events: Vec::new(),
    }
}

fn serialize<T: serde::Serialize>(value: T) -> Result<serde_json::Value, ArtifactBindingError> {
    serde_json::to_value(value).map_err(|_| ArtifactBindingError::InvalidInput)
}

fn validate_session_id(session_id: &str) -> Result<(), ArtifactBindingError> {
    if session_id.trim().is_empty() {
        return Err(ArtifactBindingError::InvalidInput);
    }
    Ok(())
}

fn validate_bundle_path(bundle_path: String) -> Result<PathBuf, ArtifactBindingError> {
    let path = PathBuf::from(bundle_path);
    if path.as_os_str().is_empty() || !path_has_veproj_extension(&path) {
        return Err(ArtifactBindingError::InvalidInput);
    }
    Ok(path)
}

fn path_has_veproj_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "veproj")
}

fn command_wire_name(command: CommandName) -> Option<String> {
    serde_json::to_value(command)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
