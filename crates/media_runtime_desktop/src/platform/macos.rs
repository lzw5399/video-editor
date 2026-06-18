use media_runtime::{
    MacosMediaIoCapabilities, MediaIoFallbackReason, RuntimeCapabilityStatus,
    RuntimeFeatureCapability,
};

pub fn probe_macos_media_io_capabilities() -> MacosMediaIoCapabilities {
    platform_macos_capabilities()
}

#[cfg(target_os = "macos")]
fn platform_macos_capabilities() -> MacosMediaIoCapabilities {
    MacosMediaIoCapabilities {
        status: RuntimeCapabilityStatus::Warning,
        av_foundation: pending_feature("AVFoundation"),
        video_toolbox: pending_feature("VideoToolbox"),
        core_video: pending_feature("CoreVideo"),
        metal_texture_interop: pending_feature("Metal texture interop"),
        fallback_reason: Some(MediaIoFallbackReason::HardwareDecodeUnavailable),
        diagnostic: Some(
            "macOS AVFoundation/VideoToolbox/CoreVideo/Metal probing is present but native decode proof is pending."
                .to_owned(),
        ),
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_macos_capabilities() -> MacosMediaIoCapabilities {
    MacosMediaIoCapabilities {
        status: RuntimeCapabilityStatus::Unavailable,
        av_foundation: unsupported_feature("AVFoundation"),
        video_toolbox: unsupported_feature("VideoToolbox"),
        core_video: unsupported_feature("CoreVideo"),
        metal_texture_interop: unsupported_feature("Metal texture interop"),
        fallback_reason: Some(MediaIoFallbackReason::UnsupportedPlatform),
        diagnostic: Some(
            "macOS AVFoundation/VideoToolbox/CoreVideo/Metal media IO is unavailable on this unsupported platform."
                .to_owned(),
        ),
    }
}

#[cfg(target_os = "macos")]
fn pending_feature(name: &str) -> RuntimeFeatureCapability {
    RuntimeFeatureCapability {
        name: name.to_owned(),
        available: false,
        status: RuntimeCapabilityStatus::Warning,
        diagnostic: Some(format!(
            "{name} capability probe is pending native implementation"
        )),
    }
}

#[cfg(not(target_os = "macos"))]
fn unsupported_feature(name: &str) -> RuntimeFeatureCapability {
    RuntimeFeatureCapability {
        name: name.to_owned(),
        available: false,
        status: RuntimeCapabilityStatus::Unavailable,
        diagnostic: Some(format!(
            "{name} capability is unavailable on this unsupported platform"
        )),
    }
}
