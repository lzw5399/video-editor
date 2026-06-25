use std::error::Error;
use std::fmt;

use draft_model::{
    DirtyDomain, DirtyRange, Draft, MaterialId, Microseconds, PreviewArtifactResponse,
    PreviewCacheEntryRef, PreviewCacheInvalidationResponse, PreviewDiagnostic,
    PreviewDiagnosticKind, PreviewOutputProfile, PreviewStatus, StartExportCommandPayload,
    TargetTimerange,
};
pub use editor_runtime::export::{
    ExportCommandError, SchedulerExportStatusResponse, export_error_diagnostic,
};
use media_runtime::{FfmpegExecutor, RuntimeConfig};
use preview_service::{
    PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile, PreviewFrameRequest,
    PreviewFrameResponse, PreviewInvalidationRequest, PreviewSegmentRequest,
    PreviewSegmentResponse, PreviewServiceConfig, PreviewServiceError, PreviewServiceErrorKind,
    invalidate_preview_cache, request_preview_frame, request_preview_segment,
};

use crate::task_runtime_service::{
    TaskRuntimeTelemetrySource, record_task_runtime_scheduler_snapshot,
};

#[derive(Debug)]
pub enum PreviewCommandError {
    Service(PreviewServiceError),
}

impl fmt::Display for PreviewCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Service(error) => write!(formatter, "preview service failed: {error}"),
        }
    }
}

impl Error for PreviewCommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Service(error) => Some(error),
        }
    }
}

impl From<PreviewServiceError> for PreviewCommandError {
    fn from(error: PreviewServiceError) -> Self {
        Self::Service(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewFrameArtifactRequest {
    pub draft: Draft,
    pub target_time: Microseconds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSegmentArtifactRequest {
    pub draft: Draft,
    pub target_timerange: TargetTimerange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewCacheInvalidationCommand {
    pub entries: Vec<PreviewCacheEntryRef>,
    pub changed_ranges: Vec<DirtyRange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub changed_graph_node_ids: Vec<String>,
    pub changed_domains: Vec<DirtyDomain>,
    pub runtime_capability_fingerprint: Option<String>,
    pub output_profile_fingerprint: Option<String>,
    pub full_draft: bool,
    pub reason: String,
    pub artifact_schema_version: u32,
    pub generator_version: String,
}

pub(crate) fn global_export_registry() -> &'static editor_runtime::ExportService {
    editor_runtime::export::global_export_registry()
}

pub(crate) fn start_export(
    runtime: RuntimeConfig,
    payload: StartExportCommandPayload,
) -> Result<SchedulerExportStatusResponse, ExportCommandError> {
    global_export_registry().start_export(runtime, payload)
}

pub(crate) fn status(job_id: &str) -> Result<SchedulerExportStatusResponse, ExportCommandError> {
    global_export_registry().status(job_id)
}

pub(crate) fn cancel(job_id: &str) -> Result<SchedulerExportStatusResponse, ExportCommandError> {
    global_export_registry().cancel(job_id)
}

pub(crate) fn record_export_task_runtime_telemetry_snapshot() {
    let snapshot = global_export_registry().telemetry_snapshot();
    record_task_runtime_scheduler_snapshot(TaskRuntimeTelemetrySource::Export, &snapshot);
}

pub fn request_preview_frame_with_executor(
    executor: &impl FfmpegExecutor,
    config: &PreviewServiceConfig,
    payload: PreviewFrameArtifactRequest,
) -> Result<PreviewArtifactResponse, PreviewCommandError> {
    let response = request_preview_frame(
        executor,
        config,
        &PreviewFrameRequest {
            draft: payload.draft,
            target_time: payload.target_time,
        },
    )?;
    Ok(frame_response(response))
}

pub fn request_preview_segment_with_executor(
    executor: &impl FfmpegExecutor,
    config: &PreviewServiceConfig,
    payload: PreviewSegmentArtifactRequest,
) -> Result<PreviewArtifactResponse, PreviewCommandError> {
    let response = request_preview_segment(
        executor,
        config,
        &PreviewSegmentRequest {
            draft: payload.draft,
            target_timerange: payload.target_timerange,
        },
    )?;
    Ok(segment_response(response))
}

pub fn invalidate_preview_cache_command(
    payload: PreviewCacheInvalidationCommand,
) -> PreviewCacheInvalidationResponse {
    let artifact_schema_version = payload.artifact_schema_version;
    let generator_version = payload.generator_version.clone();
    let mut request = PreviewInvalidationRequest::new(
        payload.changed_ranges,
        payload.changed_material_ids,
        payload.changed_graph_node_ids,
        if payload.changed_domains.is_empty() {
            vec![draft_model::DirtyDomain::PreviewCache]
        } else {
            payload.changed_domains
        },
        payload.reason,
    );
    request.runtime_capability_fingerprint = payload.runtime_capability_fingerprint;
    request.output_profile_fingerprint = payload.output_profile_fingerprint;
    request.full_draft = payload.full_draft;

    let entries = payload
        .entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| cache_entry_ref(index, entry))
        .collect::<Vec<_>>();
    let result = invalidate_preview_cache(&entries, &request);

    PreviewCacheInvalidationResponse {
        invalidated_count: u32::try_from(result.invalidated.len()).unwrap_or(u32::MAX),
        retained_count: u32::try_from(result.retained.len()).unwrap_or(u32::MAX),
        status: PreviewStatus::Invalidated,
        dirty_ranges: request.dirty_ranges,
        changed_material_ids: request.changed_material_ids,
        changed_graph_node_ids: request.changed_graph_node_keys,
        changed_domains: request.changed_domains,
        runtime_capability_fingerprint: request.runtime_capability_fingerprint,
        output_profile_fingerprint: request.output_profile_fingerprint,
        full_draft: request.full_draft,
        reason: request.reason,
        artifact_schema_version,
        generator_version,
    }
}

fn frame_response(response: PreviewFrameResponse) -> PreviewArtifactResponse {
    artifact_response(
        response.artifact,
        response.cache_entry.key.target_timerange,
        response.from_cache,
    )
}

fn segment_response(response: PreviewSegmentResponse) -> PreviewArtifactResponse {
    artifact_response(
        response.artifact,
        response.cache_entry.key.target_timerange,
        response.from_cache,
    )
}

fn artifact_response(
    artifact: PreviewArtifact,
    target_timerange: draft_model::TargetTimerange,
    from_cache: bool,
) -> PreviewArtifactResponse {
    PreviewArtifactResponse {
        profile: output_profile(artifact.profile),
        path: artifact.path,
        mime_type: artifact.mime_type,
        status: if from_cache {
            PreviewStatus::Cached
        } else {
            PreviewStatus::Generated
        },
        target_timerange,
        diagnostic: None,
    }
}

fn cache_entry_ref(index: usize, entry: PreviewCacheEntryRef) -> PreviewCacheEntry {
    let profile = cache_profile(entry.profile);
    PreviewCacheEntry {
        key: PreviewCacheKey {
            key_id: format!("binding-entry-{index}"),
            profile,
            target_timerange: entry.target_timerange,
            graph_node_keys: entry.graph_node_ids,
            semantic_fingerprint: entry
                .semantic_fingerprint
                .unwrap_or_else(|| "binding-provided".to_owned()),
            input_fingerprint: entry.input_fingerprint.unwrap_or_default(),
            output_profile_fingerprint: entry.output_profile_fingerprint.unwrap_or_default(),
            runtime_capability_fingerprint: entry
                .runtime_capability_fingerprint
                .unwrap_or_default(),
            material_dependencies: entry.material_dependencies,
            artifact_schema_version: entry.artifact_schema_version,
            generator_version: entry.generator_version,
        },
        artifact: PreviewArtifact {
            profile,
            path: entry.artifact_path,
            mime_type: profile.mime_type().to_owned(),
        },
    }
}

pub fn preview_diagnostic(error: &PreviewServiceError) -> PreviewDiagnostic {
    PreviewDiagnostic {
        kind: match error.kind {
            PreviewServiceErrorKind::EngineFailed => PreviewDiagnosticKind::EngineFailed,
            PreviewServiceErrorKind::RenderGraphFailed => PreviewDiagnosticKind::RenderGraphFailed,
            PreviewServiceErrorKind::CompileFailed => PreviewDiagnosticKind::CompileFailed,
            PreviewServiceErrorKind::IoFailed => PreviewDiagnosticKind::IoFailed,
            PreviewServiceErrorKind::RuntimeUnavailable => {
                PreviewDiagnosticKind::RuntimeUnavailable
            }
            PreviewServiceErrorKind::RuntimeFailed => PreviewDiagnosticKind::RuntimeFailed,
        },
        message: error.message.clone(),
        stdout_summary: error.stdout_summary.clone(),
        stderr_summary: error.stderr_summary.clone(),
    }
}

fn cache_profile(profile: PreviewOutputProfile) -> PreviewCacheProfile {
    match profile {
        PreviewOutputProfile::FramePng => PreviewCacheProfile::FramePng,
        PreviewOutputProfile::SegmentMp4 => PreviewCacheProfile::SegmentMp4,
    }
}

fn output_profile(profile: PreviewCacheProfile) -> PreviewOutputProfile {
    match profile {
        PreviewCacheProfile::FramePng => PreviewOutputProfile::FramePng,
        PreviewCacheProfile::SegmentMp4 => PreviewOutputProfile::SegmentMp4,
    }
}
