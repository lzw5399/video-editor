//! Rust-owned realtime preview runtime contracts.

pub mod capabilities;
pub mod clock;
pub mod diagnostics;
pub mod effects;
pub mod fallback;
pub mod frame_provider;
pub mod gpu;
pub mod graph_prepare;
pub mod media_io_adapter;
pub mod parity;
pub mod platform;
pub mod request;
pub mod scheduler;
pub mod session;
pub mod software_video_provider;
pub mod telemetry;

pub use capabilities::{
    RealtimePreviewCapabilityClassifier, RealtimePreviewCapabilityReport,
    RealtimePreviewGraphSupport,
};
pub use diagnostics::{
    RealtimePreviewDiagnostic, RealtimePreviewDiagnosticDomain, RealtimePreviewSupport,
};
pub use fallback::{RealtimePreviewFallbackReason, fallback_reason_from_media_io};
pub use frame_provider::{
    CpuVideoFrame, FrameColorInfo, FrameValidationError, FrameValidationErrorKind,
    PreviewFrameInput, PreviewFrameProvider, PreviewFrameProviderError, TextureHandleDescriptor,
};
pub use gpu::{
    RealtimePreviewCompositor, RealtimePreviewCompositorBackend, RealtimePreviewCompositorError,
    RealtimePreviewSurfacePresentationOutput,
};
pub use graph_prepare::{
    PreparedRealtimePreviewGraph, RealtimePreviewGraphInput, RealtimePreviewGraphPrepareError,
    RealtimePreviewGraphPrepareErrorKind, prepare_realtime_preview_graph,
};
pub use media_io_adapter::{
    MediaIoFrameProvider, MediaIoHandoffError, PendingPreviewFrameRelease,
    PreviewDecodeDeviceContext, PreviewDecodeDiagnostic, PreviewFrameStorageKind,
    PreviewFrameStoragePreference, PreviewMaterialDecodeOutput, PreviewMaterialDecodeRequest,
    PreviewMaterialDecodeSource, PreviewMediaIoTelemetry,
};
pub use parity::{RealtimePreviewParityDiagnostic, realtime_preview_parity_diagnostics};
pub use request::{
    PreviewCancellationToken, PreviewRequestMode, RealtimePreviewAudioSyncState,
    RealtimePreviewBackendUsed, RealtimePreviewFrameRequest, RealtimePreviewFrameResult,
};
pub use scheduler::{
    REALTIME_PLAYBACK_IDLE_POLL_INTERVAL, RealtimePlaybackCadence, RealtimePlaybackCadenceError,
    RealtimePlaybackDueTick, RealtimePlaybackPresentationQueuePolicy,
    RealtimePlaybackPresentedFrame, RealtimePlaybackScheduler, RealtimePlaybackSchedulerConfig,
    RealtimePlaybackSchedulerError, RealtimePlaybackSchedulerEvidence,
    RealtimePlaybackSchedulerEvidenceSource, RealtimePlaybackSchedulerPresentation,
    RealtimePlaybackSchedulerPresenter, RealtimePlaybackSelectedSegment,
    RealtimePlaybackTextOverlayEvidence, RealtimePlaybackTimeline, RealtimePreviewUiChrome,
};
pub use session::{
    PreviewGpuBackend, PreviewSessionId, RealtimePreviewError, RealtimePreviewRuntime,
    RealtimePreviewSessionConfig,
};
pub use software_video_provider::{DecodedVideoFrameCache, SoftwareVideoFrameProvider};
pub use task_runtime::{
    PlaybackGeneration, PlaybackRate, PlaybackRateError, PlaybackState, TimelineClock,
    TimelineFreshness,
};
pub use telemetry::{
    RealtimePreviewFramePacingSample, RealtimePreviewFramePacingTelemetry, RealtimePreviewTelemetry,
};
