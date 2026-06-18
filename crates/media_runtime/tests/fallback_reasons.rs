use media_runtime::{MediaIoFallbackReason, SelectedDecodePath};

#[test]
fn fallback_reasons_serialize_with_stable_camel_case_names() {
    let reasons = [
        (MediaIoFallbackReason::UnsupportedCodec, "unsupportedCodec"),
        (
            MediaIoFallbackReason::UnsupportedPixelFormat,
            "unsupportedPixelFormat",
        ),
        (
            MediaIoFallbackReason::HardwareDecodeUnavailable,
            "hardwareDecodeUnavailable",
        ),
        (
            MediaIoFallbackReason::TextureInteropUnavailable,
            "textureInteropUnavailable",
        ),
        (MediaIoFallbackReason::DeviceMismatch, "deviceMismatch"),
        (
            MediaIoFallbackReason::AllocationFailure,
            "allocationFailure",
        ),
        (
            MediaIoFallbackReason::PlatformApiFailure,
            "platformApiFailure",
        ),
        (
            MediaIoFallbackReason::FfmpegUnavailable,
            "ffmpegUnavailable",
        ),
        (
            MediaIoFallbackReason::UserDisabledHardwareDecode,
            "userDisabledHardwareDecode",
        ),
    ];

    for (reason, expected) in reasons {
        let encoded = serde_json::to_string(&reason).expect("reason should serialize");
        assert_eq!(encoded, format!("\"{expected}\""));
    }
}

#[test]
fn fallback_reasons_selected_decode_paths_serialize_with_stable_camel_case_names() {
    let paths = [
        (
            SelectedDecodePath::NativeHardwareTexture,
            "nativeHardwareTexture",
        ),
        (
            SelectedDecodePath::NativeHardwareCpuCopy,
            "nativeHardwareCpuCopy",
        ),
        (
            SelectedDecodePath::NativeSoftwareCpuFrame,
            "nativeSoftwareCpuFrame",
        ),
        (SelectedDecodePath::FfmpegCpuFrame, "ffmpegCpuFrame"),
        (
            SelectedDecodePath::FfmpegPreviewArtifact,
            "ffmpegPreviewArtifact",
        ),
    ];

    for (path, expected) in paths {
        let encoded = serde_json::to_string(&path).expect("path should serialize");
        assert_eq!(encoded, format!("\"{expected}\""));
    }
}
