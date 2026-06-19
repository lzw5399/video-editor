use std::path::{Path, PathBuf};

use artifact_store::ArtifactStoreError;
use artifact_store::gc::{GcMode, collect_garbage};
use artifact_store::jobs::{
    ArtifactGenerationJob, GenerationJobStatus, GenerationStatusSummary, cancel_generation_job,
    job_status_summary, list_active_generation_jobs, restart_generation_job, resume_generation_job,
};
use artifact_store::quota::{QuotaState, compute_quota_state};
use artifact_store::schema::{ArtifactStore, open_artifact_store};
use draft_model::{
    ArtifactGenerationActionCommandPayload, ArtifactGenerationTaskSummary,
    ArtifactMaintenanceResult, ArtifactQuotaStatus, ArtifactStatusSummary, ArtifactTaskStatus,
    CommandError, CommandErrorKind, CommandName, CommandPayload, CommandResultEnvelope,
    GetArtifactQuotaStatusCommandPayload, GetArtifactStatusCommandPayload, MaterialArtifactStatus,
    RefreshArtifactStatusCommandPayload, RunArtifactGarbageCollectionCommandPayload,
};

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
    store: ArtifactStore,
}

impl ArtifactStoreBindingService {
    pub fn open(session_id: String, bundle_path: String) -> Result<Self, ArtifactBindingError> {
        validate_session_id(&session_id)?;
        let bundle_path = validate_bundle_path(bundle_path)?;
        let store = open_artifact_store(bundle_path).map_err(ArtifactBindingError::Store)?;
        Ok(Self { session_id, store })
    }

    pub fn status(&self) -> Result<ArtifactStatusSummary, ArtifactBindingError> {
        artifact_status_response_from_store(&self.session_id, &self.store)
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
    UnknownJob,
    ActionUnavailable,
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
    Ok(ArtifactStatusSummary {
        session_id: session_id.to_owned(),
        status_label: if tasks.is_empty() {
            "暂无资源任务".to_owned()
        } else {
            "生成中".to_owned()
        },
        materials: Vec::<MaterialArtifactStatus>::new(),
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
            let service = service_from_refresh_payload(payload)?;
            serialize(service.status()?)
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
    ArtifactStoreBindingService::open(payload.session_id, payload.bundle_path)
}

fn service_from_refresh_payload(
    payload: RefreshArtifactStatusCommandPayload,
) -> Result<ArtifactStoreBindingService, ArtifactBindingError> {
    ArtifactStoreBindingService::open(payload.session_id, payload.bundle_path)
}

fn service_from_quota_payload(
    payload: GetArtifactQuotaStatusCommandPayload,
) -> Result<ArtifactStoreBindingService, ArtifactBindingError> {
    ArtifactStoreBindingService::open(payload.session_id, payload.bundle_path)
}

fn service_from_gc_payload(
    payload: &RunArtifactGarbageCollectionCommandPayload,
) -> Result<ArtifactStoreBindingService, ArtifactBindingError> {
    ArtifactStoreBindingService::open(payload.session_id.clone(), payload.bundle_path.clone())
}

fn service_from_action_payload(
    payload: &ArtifactGenerationActionCommandPayload,
) -> Result<ArtifactStoreBindingService, ArtifactBindingError> {
    if payload.job_id.trim().is_empty() {
        return Err(ArtifactBindingError::InvalidInput);
    }
    ArtifactStoreBindingService::open(payload.session_id.clone(), payload.bundle_path.clone())
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
        ArtifactBindingError::UnknownJob => "Artifact generation job was not found",
        ArtifactBindingError::ActionUnavailable => "Artifact generation action is unavailable",
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
