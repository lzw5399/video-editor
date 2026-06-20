use draft_model::{
    RuntimeBinaryCapability as CommandRuntimeBinaryCapability, RuntimeBinaryKind,
    RuntimeCapabilityReport as CommandRuntimeCapabilityReport,
    RuntimeCapabilityStatus as CommandRuntimeCapabilityStatus, RuntimeCodecCapability,
    RuntimeDeviceId as CommandRuntimeDeviceId, RuntimeFallbackDecodePathCapability,
    RuntimeFallbackLadderCapability, RuntimeFeatureCapability as CommandRuntimeFeatureCapability,
    RuntimeFontCapability as CommandRuntimeFontCapability,
    RuntimeLicensePosture as CommandRuntimeLicensePosture, RuntimeMacosMediaIoCapabilities,
    RuntimeMediaIoCapabilities, RuntimeMediaIoFallbackReason, RuntimePixelFormatCapability,
    RuntimeSelectedDecodePath, RuntimeTextureBackend, RuntimeTextureInteropCapability,
    RuntimeVideoPixelFormat, RuntimeWindowsMediaIoCapabilities,
};
use media_runtime::{
    BinaryKind, CodecCapability, DiscoveryError, FallbackDecodePathCapability,
    FallbackLadderCapability, MacosMediaIoCapabilities, MediaIoFallbackReason,
    PixelFormatCapability, RuntimeBinaryCapability, RuntimeCapabilities, RuntimeCapabilityStatus,
    RuntimeDeviceId, RuntimeFeatureCapability, RuntimeFontCapability, RuntimeLicensePosture,
    RuntimeMediaIoCapabilities as MediaIoCapabilities, SelectedDecodePath, TextureBackend,
    TextureInteropCapability, VideoPixelFormat, WindowsMediaIoCapabilities,
    discover_runtime_config,
};
use media_runtime_desktop::{DesktopFfmpegExecutor, probe_desktop_runtime_capabilities};

pub fn probe_runtime_capabilities_command() -> Result<CommandRuntimeCapabilityReport, DiscoveryError>
{
    let runtime = discover_runtime_config()?;
    let executor = DesktopFfmpegExecutor::default();
    Ok(command_runtime_capability_report(
        probe_desktop_runtime_capabilities(&executor, &runtime),
    ))
}

fn command_runtime_capability_report(
    report: RuntimeCapabilities,
) -> CommandRuntimeCapabilityReport {
    let ffmpeg = report.ffmpeg;
    CommandRuntimeCapabilityReport {
        status: command_status(ffmpeg.status),
        executor_name: ffmpeg.executor_name,
        ffmpeg: command_binary_capability(ffmpeg.ffmpeg),
        ffprobe: command_binary_capability(ffmpeg.ffprobe),
        h264_encoder: command_feature_capability(ffmpeg.h264_encoder),
        aac_encoder: command_feature_capability(ffmpeg.aac_encoder),
        ass_filter: command_feature_capability(ffmpeg.ass_filter),
        subtitles_filter: command_feature_capability(ffmpeg.subtitles_filter),
        font_readiness: command_font_capability(ffmpeg.font_readiness),
        license_posture: command_license_posture(ffmpeg.license_posture),
        media_io: command_media_io_capabilities(report.media_io),
        diagnostics: ffmpeg.diagnostics,
    }
}

fn command_status(status: RuntimeCapabilityStatus) -> CommandRuntimeCapabilityStatus {
    match status {
        RuntimeCapabilityStatus::Ready => CommandRuntimeCapabilityStatus::Ready,
        RuntimeCapabilityStatus::Warning => CommandRuntimeCapabilityStatus::Warning,
        RuntimeCapabilityStatus::Unavailable => CommandRuntimeCapabilityStatus::Unavailable,
    }
}

fn command_binary_kind(kind: BinaryKind) -> RuntimeBinaryKind {
    match kind {
        BinaryKind::Ffmpeg => RuntimeBinaryKind::Ffmpeg,
        BinaryKind::Ffprobe => RuntimeBinaryKind::Ffprobe,
    }
}

fn command_binary_capability(
    capability: RuntimeBinaryCapability,
) -> CommandRuntimeBinaryCapability {
    CommandRuntimeBinaryCapability {
        kind: command_binary_kind(capability.kind),
        path: capability.path.display().to_string(),
        source: capability.source,
        version: capability.version,
        configure_summary: capability.configure_summary,
        status: command_status(capability.status),
        diagnostic: capability.diagnostic,
    }
}

fn command_feature_capability(
    capability: RuntimeFeatureCapability,
) -> CommandRuntimeFeatureCapability {
    CommandRuntimeFeatureCapability {
        name: capability.name,
        available: capability.available,
        status: command_status(capability.status),
        diagnostic: capability.diagnostic,
    }
}

fn command_font_capability(capability: RuntimeFontCapability) -> CommandRuntimeFontCapability {
    CommandRuntimeFontCapability {
        env_text_font_path: capability
            .env_text_font_path
            .map(|path| path.display().to_string()),
        available_font_paths: capability
            .available_font_paths
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
        bundled_font_ref: capability.bundled_font_ref,
        bundled_font_family: capability.bundled_font_family,
        bundled_font_path: capability
            .bundled_font_path
            .map(|path| path.display().to_string()),
        bundled_font_license: capability.bundled_font_license,
        status: command_status(capability.status),
        diagnostic: capability.diagnostic,
    }
}

fn command_license_posture(posture: RuntimeLicensePosture) -> CommandRuntimeLicensePosture {
    CommandRuntimeLicensePosture {
        external_runtime: posture.external_runtime,
        redistributable_build: posture.redistributable_build,
        source: posture.source,
        message: posture.message,
    }
}

fn command_media_io_capabilities(capabilities: MediaIoCapabilities) -> RuntimeMediaIoCapabilities {
    RuntimeMediaIoCapabilities {
        windows: command_windows_media_io_capabilities(capabilities.windows),
        macos: command_macos_media_io_capabilities(capabilities.macos),
        codecs: capabilities
            .codecs
            .into_iter()
            .map(command_codec_capability)
            .collect(),
        pixel_formats: capabilities
            .pixel_formats
            .into_iter()
            .map(command_pixel_format_capability)
            .collect(),
        texture_interop: command_texture_interop_capability(capabilities.texture_interop),
        fallback_ladder: command_fallback_ladder_capability(capabilities.fallback_ladder),
    }
}

fn command_windows_media_io_capabilities(
    capabilities: WindowsMediaIoCapabilities,
) -> RuntimeWindowsMediaIoCapabilities {
    RuntimeWindowsMediaIoCapabilities {
        status: command_status(capabilities.status),
        media_foundation: command_feature_capability(capabilities.media_foundation),
        dxva: command_feature_capability(capabilities.dxva),
        d3d_texture_interop: command_feature_capability(capabilities.d3d_texture_interop),
        fallback_reason: capabilities.fallback_reason.map(command_fallback_reason),
        diagnostic: capabilities.diagnostic,
    }
}

fn command_macos_media_io_capabilities(
    capabilities: MacosMediaIoCapabilities,
) -> RuntimeMacosMediaIoCapabilities {
    RuntimeMacosMediaIoCapabilities {
        status: command_status(capabilities.status),
        av_foundation: command_feature_capability(capabilities.av_foundation),
        video_toolbox: command_feature_capability(capabilities.video_toolbox),
        core_video: command_feature_capability(capabilities.core_video),
        metal_texture_interop: command_feature_capability(capabilities.metal_texture_interop),
        fallback_reason: capabilities.fallback_reason.map(command_fallback_reason),
        diagnostic: capabilities.diagnostic,
    }
}

fn command_codec_capability(capability: CodecCapability) -> RuntimeCodecCapability {
    RuntimeCodecCapability {
        codec: capability.codec,
        containers: capability.containers,
        first_native_hardware_decode_target: capability.first_native_hardware_decode_target,
        status: command_status(capability.status),
        fallback_reason: capability.fallback_reason.map(command_fallback_reason),
        diagnostic: capability.diagnostic,
    }
}

fn command_pixel_format_capability(
    capability: PixelFormatCapability,
) -> RuntimePixelFormatCapability {
    RuntimePixelFormatCapability {
        pixel_format: command_video_pixel_format(capability.pixel_format),
        status: command_status(capability.status),
        fallback_reason: capability.fallback_reason.map(command_fallback_reason),
        diagnostic: capability.diagnostic,
    }
}

fn command_texture_interop_capability(
    capability: TextureInteropCapability,
) -> RuntimeTextureInteropCapability {
    RuntimeTextureInteropCapability {
        status: command_status(capability.status),
        backend: capability.backend.map(command_texture_backend),
        device_id: capability.device_id.map(command_device_id),
        compatible_with_preview_device: capability.compatible_with_preview_device,
        fallback_reason: capability.fallback_reason.map(command_fallback_reason),
        diagnostic: capability.diagnostic,
    }
}

fn command_fallback_ladder_capability(
    capability: FallbackLadderCapability,
) -> RuntimeFallbackLadderCapability {
    RuntimeFallbackLadderCapability {
        paths: capability
            .paths
            .into_iter()
            .map(command_fallback_decode_path_capability)
            .collect(),
    }
}

fn command_fallback_decode_path_capability(
    capability: FallbackDecodePathCapability,
) -> RuntimeFallbackDecodePathCapability {
    RuntimeFallbackDecodePathCapability {
        path: command_selected_decode_path(capability.path),
        status: command_status(capability.status),
        fallback_reason: capability.fallback_reason.map(command_fallback_reason),
        diagnostic: capability.diagnostic,
    }
}

fn command_fallback_reason(reason: MediaIoFallbackReason) -> RuntimeMediaIoFallbackReason {
    match reason {
        MediaIoFallbackReason::UnsupportedCodec => RuntimeMediaIoFallbackReason::UnsupportedCodec,
        MediaIoFallbackReason::UnsupportedPixelFormat => {
            RuntimeMediaIoFallbackReason::UnsupportedPixelFormat
        }
        MediaIoFallbackReason::HardwareDecodeUnavailable => {
            RuntimeMediaIoFallbackReason::HardwareDecodeUnavailable
        }
        MediaIoFallbackReason::TextureInteropUnavailable => {
            RuntimeMediaIoFallbackReason::TextureInteropUnavailable
        }
        MediaIoFallbackReason::DeviceMismatch => RuntimeMediaIoFallbackReason::DeviceMismatch,
        MediaIoFallbackReason::AllocationFailure => RuntimeMediaIoFallbackReason::AllocationFailure,
        MediaIoFallbackReason::PlatformApiFailure => {
            RuntimeMediaIoFallbackReason::PlatformApiFailure
        }
        MediaIoFallbackReason::FfmpegUnavailable => RuntimeMediaIoFallbackReason::FfmpegUnavailable,
        MediaIoFallbackReason::UserDisabledHardwareDecode => {
            RuntimeMediaIoFallbackReason::UserDisabledHardwareDecode
        }
        MediaIoFallbackReason::UnsupportedPlatform => {
            RuntimeMediaIoFallbackReason::UnsupportedPlatform
        }
    }
}

fn command_selected_decode_path(path: SelectedDecodePath) -> RuntimeSelectedDecodePath {
    match path {
        SelectedDecodePath::NativeHardwareTexture => {
            RuntimeSelectedDecodePath::NativeHardwareTexture
        }
        SelectedDecodePath::NativeHardwareCpuCopy => {
            RuntimeSelectedDecodePath::NativeHardwareCpuCopy
        }
        SelectedDecodePath::NativeSoftwareCpuFrame => {
            RuntimeSelectedDecodePath::NativeSoftwareCpuFrame
        }
        SelectedDecodePath::FfmpegCpuFrame => RuntimeSelectedDecodePath::FfmpegCpuFrame,
        SelectedDecodePath::FfmpegPreviewArtifact => {
            RuntimeSelectedDecodePath::FfmpegPreviewArtifact
        }
    }
}

fn command_texture_backend(backend: TextureBackend) -> RuntimeTextureBackend {
    match backend {
        TextureBackend::D3d11Texture2D => RuntimeTextureBackend::D3d11Texture2D,
        TextureBackend::D3d12Resource => RuntimeTextureBackend::D3d12Resource,
        TextureBackend::MetalTexture => RuntimeTextureBackend::MetalTexture,
        TextureBackend::CoreVideoPixelBuffer => RuntimeTextureBackend::CoreVideoPixelBuffer,
    }
}

fn command_device_id(device_id: RuntimeDeviceId) -> CommandRuntimeDeviceId {
    CommandRuntimeDeviceId {
        backend: command_texture_backend(device_id.backend),
        adapter_id: device_id.adapter_id,
        device_id: device_id.device_id,
    }
}

fn command_video_pixel_format(pixel_format: VideoPixelFormat) -> RuntimeVideoPixelFormat {
    match pixel_format {
        VideoPixelFormat::Nv12 => RuntimeVideoPixelFormat::Nv12,
        VideoPixelFormat::Bgra8 => RuntimeVideoPixelFormat::Bgra8,
        VideoPixelFormat::Rgba8 => RuntimeVideoPixelFormat::Rgba8,
        VideoPixelFormat::P010 => RuntimeVideoPixelFormat::P010,
        VideoPixelFormat::Yuv420P => RuntimeVideoPixelFormat::Yuv420P,
        VideoPixelFormat::Unknown => RuntimeVideoPixelFormat::Unknown,
    }
}
