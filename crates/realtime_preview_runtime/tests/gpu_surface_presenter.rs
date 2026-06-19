use draft_model::{DraftId, Microseconds, RationalFrameRate, TargetTimerange};
use realtime_preview_runtime::gpu::{
    NativeParentWindowHandle, PreviewSurfaceBounds, PreviewSurfaceDescriptor,
    RealtimePreviewCompositor, RealtimePreviewCompositorBackend, RealtimePreviewGpuDevice,
    RealtimePreviewGpuDeviceDescriptor, RealtimePreviewTargetFormat, RealtimePreviewTextureCache,
};
use realtime_preview_runtime::{
    PlaybackGeneration, PreviewFrameInput, PreviewFrameProvider, PreviewFrameProviderError,
    RealtimePreviewCapabilityClassifier, RealtimePreviewGraphSupport,
};
use render_graph::{
    RenderAudioMix, RenderCanvas, RenderCanvasBackground, RenderCanvasBackgroundMode, RenderGraph,
    RenderGraphNodeId, RenderIntentSupport, RenderSampledFrame,
};

#[test]
fn gpu_surface_presenter_offscreen_targets_do_not_emit_product_surface_presentation_evidence() {
    let device = mock_device();
    let descriptor = PreviewSurfaceDescriptor::Offscreen {
        width: 4,
        height: 4,
        scale_factor_millis: 1_000,
    };

    let error = device
        .create_presentation_target(descriptor, RealtimePreviewTargetFormat::Rgba8UnormSrgb)
        .expect_err("offscreen targets are diagnostic only and must not present product evidence");

    assert_eq!(
        error.kind(),
        realtime_preview_runtime::gpu::PreviewSurfaceDiagnosticKind::PlatformUnavailable
    );
    assert!(
        error
            .message()
            .contains("offscreen targets cannot satisfy product presentation")
    );
}

#[test]
fn gpu_surface_presenter_mock_native_handles_do_not_emit_product_surface_presentation_evidence() {
    let device = mock_device();
    let descriptor = PreviewSurfaceDescriptor::NativeChild {
        parent_window_handle: NativeParentWindowHandle::Mock(42),
        bounds: bounds(),
    };

    let error = device
        .create_presentation_target(descriptor, RealtimePreviewTargetFormat::Rgba8UnormSrgb)
        .expect_err("mock native handles are diagnostics and must fail closed");

    assert_eq!(
        error.kind(),
        realtime_preview_runtime::gpu::PreviewSurfaceDiagnosticKind::PlatformUnavailable
    );
    assert!(
        error
            .message()
            .contains("mock targets cannot satisfy product presentation")
    );
}

#[test]
#[ignore = "manual platform smoke: run with VIDEO_EDITOR_TEST_WGPU_SURFACE=1 and a live native preview handle"]
fn gpu_surface_presenter_presented_surface_output_carries_no_cpu_pixels_or_readback_success() {
    if std::env::var("VIDEO_EDITOR_TEST_WGPU_SURFACE")
        .ok()
        .as_deref()
        != Some("1")
    {
        eprintln!(
            "set VIDEO_EDITOR_TEST_WGPU_SURFACE=1 with VIDEO_EDITOR_TEST_NATIVE_SURFACE_HANDLE"
        );
        return;
    }

    let raw_handle = std::env::var("VIDEO_EDITOR_TEST_NATIVE_SURFACE_HANDLE")
        .expect("native surface smoke requires VIDEO_EDITOR_TEST_NATIVE_SURFACE_HANDLE")
        .parse::<u64>()
        .expect("native surface handle must be a u64 pointer/integer");
    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: realtime_preview_runtime::gpu::RealtimePreviewGpuBackend::Auto,
        label: Some("gpu-surface-presenter-test".to_owned()),
    })
    .expect("real GPU device should bootstrap on the platform smoke host");
    let descriptor = PreviewSurfaceDescriptor::NativeChild {
        parent_window_handle: platform_handle(raw_handle),
        bounds: bounds(),
    };
    let mut target = device
        .create_presentation_target(descriptor, RealtimePreviewTargetFormat::Rgba8UnormSrgb)
        .expect("native preview descriptor should create a WGPU presentation target");
    let mut compositor = RealtimePreviewCompositor::new(
        device,
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );
    let mut texture_cache = RealtimePreviewTextureCache::new();

    let output = compositor
        .present_to_surface(
            &solid_canvas_graph("#112233"),
            &mut target,
            &mut EmptyProvider,
            &mut texture_cache,
        )
        .expect("surface presentation should render and present");

    assert_eq!(
        output.render_backend,
        RealtimePreviewCompositorBackend::WgpuSurfacePresent
    );
    assert_eq!(output.support, RealtimePreviewGraphSupport::Supported);
    assert_eq!(output.presented_frames, 1);
    assert!(
        output.pixels.is_none(),
        "product surface presentation must not expose CPU readback pixels as success evidence"
    );
}

fn mock_device() -> RealtimePreviewGpuDevice {
    RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: realtime_preview_runtime::gpu::RealtimePreviewGpuBackend::Mock,
        label: Some("gpu-surface-presenter-test".to_owned()),
    })
    .expect("mock GPU device should bootstrap")
}

fn bounds() -> PreviewSurfaceBounds {
    PreviewSurfaceBounds {
        x: 0,
        y: 0,
        width: 4,
        height: 4,
        scale_factor_millis: 1_000,
    }
}

#[cfg(target_os = "macos")]
fn platform_handle(raw: u64) -> NativeParentWindowHandle {
    NativeParentWindowHandle::MacosNsView(raw)
}

#[cfg(target_os = "windows")]
fn platform_handle(raw: u64) -> NativeParentWindowHandle {
    NativeParentWindowHandle::WindowsHwnd(raw)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_handle(raw: u64) -> NativeParentWindowHandle {
    NativeParentWindowHandle::Mock(raw)
}

struct EmptyProvider;

impl PreviewFrameProvider for EmptyProvider {
    fn provider_name(&self) -> &'static str {
        "empty-provider"
    }

    fn frame_for(
        &mut self,
        material_id: &draft_model::MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
        Err(PreviewFrameProviderError::unavailable(
            self.provider_name(),
            material_id.clone(),
            source_position,
            playback_generation,
            "empty graph should not request frames",
        ))
    }
}

fn solid_canvas_graph(color: &str) -> RenderGraph {
    RenderGraph {
        draft_id: DraftId::from("draft"),
        canvas: RenderCanvas {
            node_id: RenderGraphNodeId::canvas(&DraftId::from("draft")),
            width: 4,
            height: 4,
            background: RenderCanvasBackground {
                mode: RenderCanvasBackgroundMode::SolidColor,
                color: Some(color.to_owned()),
                material_id: None,
                support: RenderIntentSupport::Supported,
                reason: "solid color canvas background is directly supported".to_owned(),
            },
            diagnostics: Vec::new(),
        },
        target_timerange: TargetTimerange::new(0, 1_000_000),
        frame_rate: RationalFrameRate::new(30, 1),
        materials: Vec::new(),
        video_layers: Vec::new(),
        audio_mixes: Vec::<RenderAudioMix>::new(),
        text_overlays: Vec::new(),
        sampled_frames: vec![RenderSampledFrame {
            node_id: RenderGraphNodeId::sampled_frame(&DraftId::from("draft"), 0, 0),
            frame_index: 0,
            at: Microseconds::ZERO,
        }],
        sampled_animation_states: Vec::new(),
        visual_diagnostics: Vec::new(),
    }
}
