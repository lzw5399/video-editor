use media_runtime::{
    CodecCapability, FallbackDecodePathCapability, FallbackLadderCapability, FfmpegExecutor,
    MediaIoFallbackReason, PixelFormatCapability, RuntimeCapabilities, RuntimeCapabilityReport,
    RuntimeCapabilityStatus, RuntimeConfig, RuntimeMediaIoCapabilities, SelectedDecodePath,
    TextureInteropCapability, VideoPixelFormat, probe_runtime_capabilities,
};

use crate::platform::{probe_macos_media_io_capabilities, probe_windows_media_io_capabilities};

pub fn probe_desktop_runtime_capabilities(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
) -> RuntimeCapabilities {
    let ffmpeg = probe_runtime_capabilities(executor, runtime);
    RuntimeCapabilities {
        media_io: media_io_capabilities(&ffmpeg),
        ffmpeg,
    }
}

fn media_io_capabilities(ffmpeg: &RuntimeCapabilityReport) -> RuntimeMediaIoCapabilities {
    RuntimeMediaIoCapabilities {
        windows: probe_windows_media_io_capabilities(),
        macos: probe_macos_media_io_capabilities(),
        codecs: codec_capabilities(),
        pixel_formats: pixel_format_capabilities(),
        texture_interop: texture_interop_capability(),
        fallback_ladder: fallback_ladder_capability(ffmpeg),
    }
}

fn codec_capabilities() -> Vec<CodecCapability> {
    vec![
        CodecCapability {
            codec: "h264".to_owned(),
            containers: vec!["mp4".to_owned(), "mov".to_owned()],
            first_native_hardware_decode_target: true,
            status: RuntimeCapabilityStatus::Warning,
            fallback_reason: Some(MediaIoFallbackReason::HardwareDecodeUnavailable),
            diagnostic: Some(
                "H.264 MP4/MOV is the first native hardware-decode acceptance target; platform decode proof is pending."
                    .to_owned(),
            ),
        },
        unproven_codec("hevc", MediaIoFallbackReason::UnsupportedCodec),
        unproven_codec("prores", MediaIoFallbackReason::UnsupportedCodec),
        unproven_codec("av1", MediaIoFallbackReason::UnsupportedCodec),
    ]
}

fn unproven_codec(codec: &str, fallback_reason: MediaIoFallbackReason) -> CodecCapability {
    CodecCapability {
        codec: codec.to_owned(),
        containers: Vec::new(),
        first_native_hardware_decode_target: false,
        status: RuntimeCapabilityStatus::Unavailable,
        fallback_reason: Some(fallback_reason),
        diagnostic: Some(format!(
            "{codec} native hardware decode is not accepted until platform proof is added"
        )),
    }
}

fn pixel_format_capabilities() -> Vec<PixelFormatCapability> {
    vec![
        PixelFormatCapability {
            pixel_format: VideoPixelFormat::Nv12,
            status: RuntimeCapabilityStatus::Warning,
            fallback_reason: Some(MediaIoFallbackReason::HardwareDecodeUnavailable),
            diagnostic: Some("NV12 is the first native decode pixel-format target; platform proof is pending.".to_owned()),
        },
        PixelFormatCapability {
            pixel_format: VideoPixelFormat::Bgra8,
            status: RuntimeCapabilityStatus::Warning,
            fallback_reason: Some(MediaIoFallbackReason::TextureInteropUnavailable),
            diagnostic: Some("BGRA8 remains a CPU/software or texture-conversion candidate until native interop is proven.".to_owned()),
        },
        PixelFormatCapability {
            pixel_format: VideoPixelFormat::P010,
            status: RuntimeCapabilityStatus::Unavailable,
            fallback_reason: Some(MediaIoFallbackReason::UnsupportedPixelFormat),
            diagnostic: Some("P010 HDR decode is not accepted in the initial H.264 MP4/MOV target.".to_owned()),
        },
    ]
}

fn texture_interop_capability() -> TextureInteropCapability {
    TextureInteropCapability {
        status: RuntimeCapabilityStatus::Warning,
        backend: None,
        device_id: None,
        compatible_with_preview_device: false,
        fallback_reason: Some(MediaIoFallbackReason::TextureInteropUnavailable),
        diagnostic: Some(
            "CPU fallback remains required until native texture import compatibility is proven."
                .to_owned(),
        ),
    }
}

fn fallback_ladder_capability(ffmpeg: &RuntimeCapabilityReport) -> FallbackLadderCapability {
    let ffmpeg_ready = ffmpeg.status != RuntimeCapabilityStatus::Unavailable;
    FallbackLadderCapability {
        paths: vec![
            FallbackDecodePathCapability {
                path: SelectedDecodePath::NativeHardwareTexture,
                status: RuntimeCapabilityStatus::Warning,
                fallback_reason: Some(MediaIoFallbackReason::TextureInteropUnavailable),
                diagnostic: Some(
                    "Native hardware texture decode is preferred but not proven.".to_owned(),
                ),
            },
            FallbackDecodePathCapability {
                path: SelectedDecodePath::NativeHardwareCpuCopy,
                status: RuntimeCapabilityStatus::Warning,
                fallback_reason: Some(MediaIoFallbackReason::HardwareDecodeUnavailable),
                diagnostic: Some(
                    "Native hardware decode with CPU copy remains pending platform proof."
                        .to_owned(),
                ),
            },
            FallbackDecodePathCapability {
                path: SelectedDecodePath::NativeSoftwareCpuFrame,
                status: RuntimeCapabilityStatus::Warning,
                fallback_reason: Some(MediaIoFallbackReason::PlatformApiFailure),
                diagnostic: Some(
                    "Native software CPU decode remains pending platform implementation."
                        .to_owned(),
                ),
            },
            FallbackDecodePathCapability {
                path: SelectedDecodePath::FfmpegCpuFrame,
                status: if ffmpeg_ready {
                    RuntimeCapabilityStatus::Ready
                } else {
                    RuntimeCapabilityStatus::Unavailable
                },
                fallback_reason: if ffmpeg_ready {
                    None
                } else {
                    Some(MediaIoFallbackReason::FfmpegUnavailable)
                },
                diagnostic: if ffmpeg_ready {
                    Some("FFmpeg CPU frame decode is available as the structured fallback implementation.".to_owned())
                } else {
                    Some("FFmpeg CPU frame decode is unavailable because FFmpeg capability probing failed.".to_owned())
                },
            },
            FallbackDecodePathCapability {
                path: SelectedDecodePath::FfmpegPreviewArtifact,
                status: if ffmpeg_ready {
                    RuntimeCapabilityStatus::Ready
                } else {
                    RuntimeCapabilityStatus::Unavailable
                },
                fallback_reason: if ffmpeg_ready {
                    None
                } else {
                    Some(MediaIoFallbackReason::FfmpegUnavailable)
                },
                diagnostic: Some(
                    "Existing FFmpeg preview artifacts remain the final compatibility fallback."
                        .to_owned(),
                ),
            },
        ],
    }
}
