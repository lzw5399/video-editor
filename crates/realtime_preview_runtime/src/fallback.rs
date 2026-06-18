use serde::{Deserialize, Serialize};

use media_runtime::{MediaIoFallbackReason, SelectedDecodePath};

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
    MediaIoNativeCpuFrame,
    MediaIoTextureInteropUnavailable,
    MediaIoDeviceMismatch,
    MediaIoFfmpegCpuFrame,
    MediaIoPreviewArtifact,
    MediaIoDecodeUnavailable,
    Canceled,
    StaleGeneration,
}

pub fn fallback_reason_from_media_io(
    selected_path: SelectedDecodePath,
    reason: Option<MediaIoFallbackReason>,
) -> Option<RealtimePreviewFallbackReason> {
    match reason {
        Some(MediaIoFallbackReason::TextureInteropUnavailable) => {
            Some(RealtimePreviewFallbackReason::MediaIoTextureInteropUnavailable)
        }
        Some(MediaIoFallbackReason::DeviceMismatch) => {
            Some(RealtimePreviewFallbackReason::MediaIoDeviceMismatch)
        }
        Some(_) => Some(match selected_path {
            SelectedDecodePath::NativeHardwareTexture => {
                RealtimePreviewFallbackReason::MediaIoDecodeUnavailable
            }
            SelectedDecodePath::NativeHardwareCpuCopy
            | SelectedDecodePath::NativeSoftwareCpuFrame => {
                RealtimePreviewFallbackReason::MediaIoNativeCpuFrame
            }
            SelectedDecodePath::FfmpegCpuFrame => {
                RealtimePreviewFallbackReason::MediaIoFfmpegCpuFrame
            }
            SelectedDecodePath::FfmpegPreviewArtifact => {
                RealtimePreviewFallbackReason::MediaIoPreviewArtifact
            }
        }),
        None => match selected_path {
            SelectedDecodePath::NativeHardwareTexture => None,
            SelectedDecodePath::NativeHardwareCpuCopy
            | SelectedDecodePath::NativeSoftwareCpuFrame => {
                Some(RealtimePreviewFallbackReason::MediaIoNativeCpuFrame)
            }
            SelectedDecodePath::FfmpegCpuFrame => {
                Some(RealtimePreviewFallbackReason::MediaIoFfmpegCpuFrame)
            }
            SelectedDecodePath::FfmpegPreviewArtifact => {
                Some(RealtimePreviewFallbackReason::MediaIoPreviewArtifact)
            }
        },
    }
}
