use draft_model::Microseconds;
use serde::{Deserialize, Serialize};

use crate::{
    PlaybackGeneration, RealtimePreviewDiagnostic, RealtimePreviewFallbackReason,
    RealtimePreviewTelemetry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PreviewCancellationToken(u64);

impl PreviewCancellationToken {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewRequestMode {
    Seek,
    Scrub,
    PlaybackTick,
    FirstFrame,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewFrameRequest {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<PreviewCancellationToken>,
    pub mode: PreviewRequestMode,
    pub queue_latency_ms: u64,
    pub render_duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<RealtimePreviewFallbackReason>,
    pub cache_hit: bool,
    pub repeated_frame: bool,
    pub dropped_frame: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewFrameResult {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub presented: bool,
    pub stale_rejected: bool,
    pub canceled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<PreviewCancellationToken>,
    pub backend: RealtimePreviewBackendUsed,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<RealtimePreviewFallbackReason>,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
    pub telemetry: RealtimePreviewTelemetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePreviewBackendUsed {
    Mock,
    Gpu,
    Offscreen,
    PreviewArtifact,
    FfmpegArtifact,
    None,
}
