use realtime_preview_runtime::gpu::{
    RealtimePreviewGpuBackend, RealtimePreviewGpuDevice, RealtimePreviewGpuDeviceDescriptor,
    RealtimePreviewGpuTarget, RealtimePreviewTargetFormat,
};

#[test]
fn offscreen_compositor_mock_device_creates_valid_target_without_adapter() {
    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: RealtimePreviewGpuBackend::Mock,
        label: Some("offscreen-compositor-test".to_owned()),
    })
    .expect("mock GPU device should not require a physical adapter");

    assert_eq!(device.backend(), RealtimePreviewGpuBackend::Mock);
    assert!(!device.uses_physical_adapter());

    let target = device
        .create_offscreen_target(640, 360, 1_000, RealtimePreviewTargetFormat::Rgba8UnormSrgb)
        .expect("mock offscreen target should be constructible");

    assert_eq!(target.width(), 640);
    assert_eq!(target.height(), 360);
    assert_eq!(target.scale_factor_millis(), 1_000);
    assert_eq!(target.format(), RealtimePreviewTargetFormat::Rgba8UnormSrgb);
    assert_eq!(target.pixel_len(), 640 * 360 * 4);
}

#[test]
fn offscreen_compositor_target_rejects_invalid_dimensions() {
    let zero_width = RealtimePreviewGpuTarget::offscreen(
        0,
        360,
        1_000,
        RealtimePreviewTargetFormat::Rgba8UnormSrgb,
    )
    .expect_err("zero width must be rejected");

    assert!(
        zero_width
            .to_string()
            .contains("offscreen target dimensions must be nonzero")
    );
}

#[test]
fn offscreen_compositor_auto_backend_selects_supported_desktop_path() {
    let selected = RealtimePreviewGpuBackend::Auto.resolve_for_current_platform();

    #[cfg(target_os = "windows")]
    assert_eq!(selected, RealtimePreviewGpuBackend::D3d12);

    #[cfg(target_os = "macos")]
    assert_eq!(selected, RealtimePreviewGpuBackend::Metal);

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    assert_eq!(selected, RealtimePreviewGpuBackend::OffscreenOnly);
}

#[test]
#[ignore = "manual platform smoke: run with VIDEO_EDITOR_TEST_WGPU=1 on Windows/macOS GPU hosts"]
fn real_wgpu_adapter_smoke_is_opt_in() {
    if std::env::var("VIDEO_EDITOR_TEST_WGPU").ok().as_deref() != Some("1") {
        eprintln!("set VIDEO_EDITOR_TEST_WGPU=1 to run the real adapter smoke");
        return;
    }

    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: RealtimePreviewGpuBackend::Auto,
        label: Some("real-wgpu-adapter-smoke".to_owned()),
    })
    .expect("real wgpu adapter should initialize on a supported platform host");

    assert!(device.uses_physical_adapter());
    assert!(matches!(
        device.backend(),
        RealtimePreviewGpuBackend::D3d12 | RealtimePreviewGpuBackend::Metal
    ));
}
