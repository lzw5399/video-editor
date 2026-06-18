use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaIoFallbackReason {
    UnsupportedCodec,
    UnsupportedPixelFormat,
    HardwareDecodeUnavailable,
    TextureInteropUnavailable,
    DeviceMismatch,
    AllocationFailure,
    PlatformApiFailure,
    FfmpegUnavailable,
    UserDisabledHardwareDecode,
    UnsupportedPlatform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectedDecodePath {
    NativeHardwareTexture,
    NativeHardwareCpuCopy,
    NativeSoftwareCpuFrame,
    FfmpegCpuFrame,
    FfmpegPreviewArtifact,
}
