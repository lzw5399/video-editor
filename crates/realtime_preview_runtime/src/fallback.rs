use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePreviewFallbackReason {
    NoGpuAdapter,
    SurfaceUnavailable,
    SurfaceLost,
    UnsupportedGraphIntent,
    FrameProviderUnavailable,
    TextParityUnsupported,
    NativeChildWindowFailed,
    OffscreenReadbackRequired,
    PreviewArtifactCacheHit,
    FfmpegArtifactGenerated,
    Canceled,
    StaleGeneration,
}
