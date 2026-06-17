use std::error::Error;
use std::fmt;

use draft_model::{
    InvalidatePreviewCacheCommandPayload, PreviewArtifactResponse, PreviewCacheEntryRef,
    PreviewCacheInvalidationResponse, PreviewDiagnostic, PreviewDiagnosticKind,
    PreviewOutputProfile, PreviewStatus, RequestPreviewFrameCommandPayload,
    RequestPreviewSegmentCommandPayload,
};
use media_runtime::FfmpegExecutor;
use preview_service::{
    PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile, PreviewFrameRequest,
    PreviewFrameResponse, PreviewInvalidationRequest, PreviewSegmentRequest,
    PreviewSegmentResponse, PreviewServiceConfig, PreviewServiceError, PreviewServiceErrorKind,
    invalidate_preview_cache, request_preview_frame, request_preview_segment,
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

pub fn request_preview_frame_with_executor(
    executor: &impl FfmpegExecutor,
    config: &PreviewServiceConfig,
    payload: RequestPreviewFrameCommandPayload,
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
    payload: RequestPreviewSegmentCommandPayload,
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
    payload: InvalidatePreviewCacheCommandPayload,
) -> PreviewCacheInvalidationResponse {
    let entries = payload
        .entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| cache_entry_ref(index, entry))
        .collect::<Vec<_>>();
    let result = invalidate_preview_cache(
        &entries,
        &PreviewInvalidationRequest {
            changed_ranges: payload.changed_ranges,
            changed_material_ids: payload.changed_material_ids,
            reason: payload.reason,
        },
    );

    PreviewCacheInvalidationResponse {
        invalidated_count: u32::try_from(result.invalidated.len()).unwrap_or(u32::MAX),
        retained_count: u32::try_from(result.retained.len()).unwrap_or(u32::MAX),
        status: PreviewStatus::Invalidated,
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
            semantic_fingerprint: "binding-provided".to_owned(),
            material_dependencies: entry.material_dependencies,
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
