use media_runtime::{
    MediaIoFallbackReason, RuntimeCapabilityStatus, RuntimeFeatureCapability,
    WindowsMediaIoCapabilities,
};

pub fn probe_windows_media_io_capabilities() -> WindowsMediaIoCapabilities {
    platform_windows_capabilities()
}

#[cfg(windows)]
fn platform_windows_capabilities() -> WindowsMediaIoCapabilities {
    WindowsMediaIoCapabilities {
        status: RuntimeCapabilityStatus::Warning,
        media_foundation: pending_feature("Media Foundation"),
        dxva: pending_feature("DXVA"),
        d3d_texture_interop: pending_feature("D3D texture interop"),
        fallback_reason: Some(MediaIoFallbackReason::HardwareDecodeUnavailable),
        diagnostic: Some(
            "Windows Media Foundation/DXVA/D3D probing is present but native decode proof is pending."
                .to_owned(),
        ),
    }
}

#[cfg(not(windows))]
fn platform_windows_capabilities() -> WindowsMediaIoCapabilities {
    WindowsMediaIoCapabilities {
        status: RuntimeCapabilityStatus::Unavailable,
        media_foundation: unsupported_feature("Media Foundation"),
        dxva: unsupported_feature("DXVA"),
        d3d_texture_interop: unsupported_feature("D3D texture interop"),
        fallback_reason: Some(MediaIoFallbackReason::UnsupportedPlatform),
        diagnostic: Some(
            "Windows Media Foundation/DXVA/D3D media IO is unavailable on this unsupported platform."
                .to_owned(),
        ),
    }
}

#[cfg(windows)]
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

#[cfg(not(windows))]
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
