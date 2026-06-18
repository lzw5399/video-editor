use serde::{Deserialize, Serialize};

const MEDIA_IO_FALLBACK_LADDER: [SelectedDecodePath; 5] = [
    SelectedDecodePath::NativeHardwareTexture,
    SelectedDecodePath::NativeHardwareCpuCopy,
    SelectedDecodePath::NativeSoftwareCpuFrame,
    SelectedDecodePath::FfmpegCpuFrame,
    SelectedDecodePath::FfmpegPreviewArtifact,
];

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaIoFallbackCandidate {
    pub path: SelectedDecodePath,
    pub available: bool,
    pub reason: Option<MediaIoFallbackReason>,
    pub diagnostic: Option<String>,
}

impl MediaIoFallbackCandidate {
    pub fn available(path: SelectedDecodePath) -> Self {
        Self {
            path,
            available: true,
            reason: None,
            diagnostic: None,
        }
    }

    pub fn unavailable(
        path: SelectedDecodePath,
        reason: MediaIoFallbackReason,
        diagnostic: impl Into<String>,
    ) -> Self {
        Self {
            path,
            available: false,
            reason: Some(reason),
            diagnostic: Some(diagnostic.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaIoFallbackDiagnostic {
    pub path: SelectedDecodePath,
    pub available: bool,
    pub reason: Option<MediaIoFallbackReason>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaIoFallbackSelection {
    pub selected_path: SelectedDecodePath,
    pub reason: Option<MediaIoFallbackReason>,
    pub diagnostics: Vec<MediaIoFallbackDiagnostic>,
}

pub fn media_io_fallback_ladder() -> Vec<SelectedDecodePath> {
    MEDIA_IO_FALLBACK_LADDER.to_vec()
}

pub fn select_media_io_fallback(
    candidates: Vec<MediaIoFallbackCandidate>,
    reason: MediaIoFallbackReason,
) -> Option<MediaIoFallbackSelection> {
    let mut diagnostics = Vec::new();

    for path in MEDIA_IO_FALLBACK_LADDER {
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.path == path)
            .cloned()
            .unwrap_or_else(|| MediaIoFallbackCandidate {
                path,
                available: false,
                reason: Some(reason),
                diagnostic: Some("fallback path was not advertised by this runtime".to_owned()),
            });

        diagnostics.push(MediaIoFallbackDiagnostic {
            path,
            available: candidate.available,
            reason: candidate.reason,
            diagnostic: candidate.diagnostic,
        });

        if candidate.available {
            return Some(MediaIoFallbackSelection {
                selected_path: path,
                reason: (path != SelectedDecodePath::NativeHardwareTexture).then_some(reason),
                diagnostics,
            });
        }
    }

    None
}
