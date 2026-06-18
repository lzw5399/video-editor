use std::path::PathBuf;

use media_runtime::{
    MediaIoFallbackReason, MediaOpenRequest, RuntimeDeviceId, SelectedDecodePath, StreamId,
    TextureBackend,
};
use media_runtime_desktop::{
    select_windows_texture_interop_fallback, WindowsMediaReader, WindowsTextureInteropPolicy,
};

#[test]
fn windows_texture_interop_defaults_to_native_frame_fallback_until_device_is_proven() {
    let selection = select_windows_texture_interop_fallback(true, None, false).expect(
        "native decode frame path should remain available when D3D texture interop is unproven",
    );

    assert_eq!(
        selection.selected_path,
        SelectedDecodePath::NativeHardwareCpuCopy
    );
    assert_eq!(
        selection.reason,
        Some(MediaIoFallbackReason::TextureInteropUnavailable)
    );
    assert_eq!(
        selection.diagnostics[0].path,
        SelectedDecodePath::NativeHardwareTexture
    );
    assert_eq!(
        selection.diagnostics[0].reason,
        Some(MediaIoFallbackReason::TextureInteropUnavailable)
    );
}

#[test]
fn windows_texture_interop_reports_device_mismatch_before_cpu_or_ffmpeg_fallback() {
    let preview_device = RuntimeDeviceId {
        backend: TextureBackend::D3d12Resource,
        adapter_id: "preview-adapter".to_owned(),
        device_id: "preview-device".to_owned(),
    };
    let native_device = RuntimeDeviceId {
        backend: TextureBackend::D3d11Texture2D,
        adapter_id: "native-adapter".to_owned(),
        device_id: "native-device".to_owned(),
    };

    let selection =
        select_windows_texture_interop_fallback(true, Some((&preview_device, &native_device)), true)
            .expect("native decode frame path should remain available when devices mismatch");

    assert_eq!(
        selection.selected_path,
        SelectedDecodePath::NativeHardwareCpuCopy
    );
    assert_eq!(
        selection.reason,
        Some(MediaIoFallbackReason::DeviceMismatch)
    );
    assert_eq!(
        selection.diagnostics[0].reason,
        Some(MediaIoFallbackReason::DeviceMismatch)
    );
}

#[test]
fn windows_texture_interop_selects_texture_only_for_compatible_d3d_device() {
    let device = RuntimeDeviceId {
        backend: TextureBackend::D3d11Texture2D,
        adapter_id: "luid-00000001-00000002".to_owned(),
        device_id: "vendor-10de-device-2684".to_owned(),
    };

    let selection = select_windows_texture_interop_fallback(true, Some((&device, &device)), true)
        .expect("compatible D3D device should select native texture path");

    assert_eq!(
        selection.selected_path,
        SelectedDecodePath::NativeHardwareTexture
    );
    assert_eq!(selection.reason, None);
}

#[test]
#[cfg(not(windows))]
fn windows_reader_reports_unsupported_platform_without_panicking() {
    let reader = WindowsMediaReader::new();
    let error = reader
        .open_session(MediaOpenRequest {
            material_uri: PathBuf::from("/tmp/input.mp4"),
            requested_streams: vec![StreamId(0)],
        })
        .expect_err("Windows native reader should be unavailable on non-Windows");

    assert!(error.message.contains("UnsupportedPlatform"));
}

#[test]
#[cfg(windows)]
fn windows_native_decode_proof_is_explicitly_env_gated() {
    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!("skipping Windows native decode proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1");
        return;
    }

    let _reader = WindowsMediaReader::new();
    panic!(
        "Windows native Media Foundation/DXVA fixture decode must be implemented before this env-gated proof can pass"
    );
}

#[test]
#[cfg(windows)]
fn windows_texture_decode_degrades_when_texture_interop_is_disabled() {
    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!("skipping Windows texture fallback proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1");
        return;
    }

    let _reader =
        WindowsMediaReader::new().with_texture_interop_policy(WindowsTextureInteropPolicy::disabled());
    panic!(
        "Windows native texture fallback proof must be implemented before this env-gated proof can pass"
    );
}
