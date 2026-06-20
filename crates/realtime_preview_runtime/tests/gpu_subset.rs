use std::cell::RefCell;
use std::rc::Rc;

use draft_model::{
    Draft, DraftId, Material, MaterialId, MaterialKind, Microseconds, RationalFrameRate, Segment,
    SegmentFitMode, SegmentOpacity, SegmentPosition, SegmentScale, SegmentVisual, SourceTimerange,
    TargetTimerange, TextAlignment, TextLayoutRegion, TextSegment, TextStyle, Track, TrackId,
    TrackKind,
};
use media_runtime::{
    MediaSessionId, NativeTextureLeaseRegistry, NativeTextureLeaseResourceKind, RuntimeDeviceId,
    TextureBackend, VideoColorMetadata,
};
use realtime_preview_runtime::gpu::{
    RealtimePreviewCompositor, RealtimePreviewCompositorBackend, RealtimePreviewGpuBackend,
    RealtimePreviewGpuDevice, RealtimePreviewGpuDeviceDescriptor, RealtimePreviewGpuTarget,
    RealtimePreviewTargetFormat, RealtimePreviewTextureCache,
};
use realtime_preview_runtime::{
    CpuVideoFrame, DecodedVideoFrameCache, FrameColorInfo, PlaybackGeneration, PreviewFrameInput,
    PreviewFrameProvider, PreviewFrameProviderError, RealtimePreviewCapabilityClassifier,
    RealtimePreviewGraphInput, RealtimePreviewGraphSupport, SoftwareVideoFrameProvider,
    TextureHandleDescriptor, prepare_realtime_preview_graph,
};
use render_graph::{
    OutputDimensions, RenderAudioMix, RenderCanvas, RenderCanvasBackground,
    RenderCanvasBackgroundMode, RenderGraph, RenderGraphNodeId, RenderIntentSupport,
    RenderMaterial, RenderSampledFrame, RenderVideoLayer,
};

#[test]
fn gpu_subset_solid_canvas_produces_deterministic_pixels() {
    let output = render_graph_with_provider(solid_canvas_graph("#112233"), EmptyProvider);

    assert_eq!(output.width, 4);
    assert_eq!(output.height, 4);
    assert_eq!(
        rgba_at(&output.pixels, 0, 0, output.width),
        [0x11, 0x22, 0x33, 0xff]
    );
    assert_eq!(
        rgba_at(&output.pixels, 3, 3, output.width),
        [0x11, 0x22, 0x33, 0xff]
    );
    assert_eq!(output.submitted_draws, 0);
    assert_eq!(output.support, RealtimePreviewGraphSupport::Supported);
    assert_eq!(
        output.render_backend,
        RealtimePreviewCompositorBackend::CpuReference,
        "mock/offscreen deterministic output is a CPU reference path, not product GPU compositor success"
    );
}

#[test]
fn gpu_subset_textured_quads_use_graph_stack_order_and_provider_frames() {
    let image_id = MaterialId::from("image");
    let video_id = MaterialId::from("video");
    let graph = textured_graph(&image_id, &video_id, 1_000);
    let provider = ImageThenSoftwareVideoProvider::new(&image_id, &video_id);
    let output = render_graph_with_provider(graph, provider);

    assert_eq!(
        rgba_at(&output.pixels, 0, 0, output.width),
        [255, 0, 0, 255]
    );
    assert_eq!(
        rgba_at(&output.pixels, 3, 3, output.width),
        [0, 0, 255, 255]
    );
    assert_eq!(
        rgba_at(&output.pixels, 1, 1, output.width),
        [255, 0, 0, 255]
    );
    assert_eq!(
        rgba_at(&output.pixels, 2, 2, output.width),
        [0, 0, 255, 255]
    );
    assert_eq!(output.submitted_draws, 2);
    assert_eq!(
        output.render_backend,
        RealtimePreviewCompositorBackend::CpuReference
    );
}

#[test]
fn gpu_subset_samples_video_frame_at_target_relative_source_time() {
    let video_id = MaterialId::from("video");
    let mut graph = single_video_graph(&video_id, 4, 4);
    graph.target_timerange = TargetTimerange::new(500_000, 33_333);
    graph.video_layers[0].source_timerange = SourceTimerange::new(100_000, 1_000_000);
    graph.video_layers[0].target_timerange = TargetTimerange::new(250_000, 1_000_000);
    let requests = Rc::new(RefCell::new(Vec::new()));
    let provider = RecordingFrameProvider::new(video_id, requests.clone());

    let output = render_graph_with_provider(graph, provider);

    assert_eq!(output.submitted_draws, 1);
    assert_eq!(*requests.borrow(), vec![Microseconds::new(350_000)]);
}

#[test]
fn gpu_subset_opacity_affects_composited_color() {
    let image_id = MaterialId::from("image");
    let video_id = MaterialId::from("video");
    let graph = textured_graph(&image_id, &video_id, 500);
    let provider = ImageThenSoftwareVideoProvider::new(&image_id, &video_id);
    let output = render_graph_with_provider(graph, provider);

    assert_eq!(
        rgba_at(&output.pixels, 2, 2, output.width),
        [0, 0, 128, 255]
    );
    assert_eq!(output.submitted_draws, 2);
    assert_eq!(
        output.render_backend,
        RealtimePreviewCompositorBackend::CpuReference
    );
}

#[test]
fn gpu_subset_unsupported_intent_does_not_submit_draws() {
    let image_id = MaterialId::from("image");
    let video_id = MaterialId::from("video");
    let mut graph = textured_graph(&image_id, &video_id, 1_000);
    graph.canvas.background.support = RenderIntentSupport::Unsupported;
    graph.canvas.background.reason = "unsupported test canvas".to_owned();
    let provider = ImageThenSoftwareVideoProvider::new(&image_id, &video_id);
    let output = render_graph_with_provider(graph, provider);

    assert_eq!(output.support, RealtimePreviewGraphSupport::Unsupported);
    assert_eq!(output.submitted_draws, 0);
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.reason.contains("unsupported test canvas"))
    );
}

#[test]
#[ignore = "manual platform smoke: run with VIDEO_EDITOR_TEST_WGPU=1 on Windows/macOS GPU hosts"]
fn real_wgpu_compositor_clears_canvas_with_render_pass() {
    if std::env::var("VIDEO_EDITOR_TEST_WGPU").ok().as_deref() != Some("1") {
        eprintln!("set VIDEO_EDITOR_TEST_WGPU=1 to run the real compositor smoke");
        return;
    }

    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: RealtimePreviewGpuBackend::Auto,
        label: Some("real-wgpu-compositor-canvas-test".to_owned()),
    })
    .expect("real wgpu adapter should initialize on a supported platform host");
    assert!(device.uses_physical_adapter());

    let target = device
        .create_offscreen_target(4, 4, 1_000, RealtimePreviewTargetFormat::Rgba8UnormSrgb)
        .expect("real GPU target should allocate a wgpu texture");
    let mut compositor = RealtimePreviewCompositor::new(
        device,
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );
    let mut texture_cache = RealtimePreviewTextureCache::new();
    let output = compositor
        .render_offscreen(
            &solid_canvas_graph("#112233"),
            &target,
            &mut EmptyProvider,
            &mut texture_cache,
        )
        .expect("real GPU compositor should render a solid canvas");

    assert_eq!(
        output.render_backend,
        RealtimePreviewCompositorBackend::WgpuRenderPass
    );
    assert_eq!(output.submitted_draws, 0);
    assert_eq!(
        rgba_at(&output.pixels, 0, 0, output.width),
        [0x11, 0x22, 0x33, 0xff]
    );
}

#[test]
#[ignore = "manual platform smoke: run with VIDEO_EDITOR_TEST_WGPU=1 on Windows/macOS GPU hosts"]
fn real_wgpu_compositor_samples_textured_layers_with_render_pass() {
    if std::env::var("VIDEO_EDITOR_TEST_WGPU").ok().as_deref() != Some("1") {
        eprintln!("set VIDEO_EDITOR_TEST_WGPU=1 to run the real compositor smoke");
        return;
    }

    let image_id = MaterialId::from("image");
    let video_id = MaterialId::from("video");
    let graph = textured_graph(&image_id, &video_id, 1_000);
    let provider = ImageThenSoftwareVideoProvider::new(&image_id, &video_id);
    let output = render_graph_with_real_wgpu_provider(graph, provider);

    assert_eq!(
        output.render_backend,
        RealtimePreviewCompositorBackend::WgpuRenderPass
    );
    assert_eq!(output.submitted_draws, 2);
    assert_eq!(
        rgba_at(&output.pixels, 1, 1, output.width),
        [255, 0, 0, 255]
    );
    assert_eq!(
        rgba_at(&output.pixels, 2, 2, output.width),
        [0, 0, 255, 255]
    );
}

#[test]
#[ignore = "manual platform smoke: run with VIDEO_EDITOR_TEST_WGPU=1 on Windows/macOS GPU hosts"]
fn real_wgpu_compositor_renders_bundled_text_overlay_with_render_pass() {
    if std::env::var("VIDEO_EDITOR_TEST_WGPU").ok().as_deref() != Some("1") {
        eprintln!("set VIDEO_EDITOR_TEST_WGPU=1 to run the real compositor smoke");
        return;
    }

    let output = render_graph_with_real_wgpu_provider(text_overlay_graph(), EmptyProvider);

    assert_eq!(
        output.render_backend,
        RealtimePreviewCompositorBackend::WgpuRenderPass
    );
    assert_eq!(
        output.submitted_draws, 1,
        "text overlay should submit one draw; diagnostics: {:?}",
        output.diagnostics
    );
    assert!(
        output
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 120 && pixel[1] < 80 && pixel[2] < 80),
        "real GPU text overlay render should produce visible red text pixels"
    );
}

#[test]
fn nv12_opaque_texture_handles_do_not_satisfy_gpu_compositor_success() {
    if std::env::var("VIDEO_EDITOR_TEST_WGPU").ok().as_deref() != Some("1") {
        eprintln!("set VIDEO_EDITOR_TEST_WGPU=1 to run the real compositor import contract");
        return;
    }

    let video_id = MaterialId::from("video");
    let descriptor = nv12_texture_descriptor(&video_id, "opaque-nv12-texture");
    let registry = NativeTextureLeaseRegistry::new();
    registry
        .register_resource(
            descriptor
                .to_texture_handle()
                .expect("descriptor should convert to a texture handle"),
            NativeTextureLeaseResourceKind::PlatformOpaque,
            "opaque-platform-resource".to_owned(),
        )
        .expect("opaque native texture lease should register");

    let output = render_graph_with_real_wgpu_registry(
        single_video_graph(&video_id, 2, 2),
        TextureHandleProvider::new(video_id, descriptor),
        registry,
    );

    assert_eq!(output.support, RealtimePreviewGraphSupport::Unsupported);
    assert_eq!(output.submitted_draws, 0);
    assert!(
        output.diagnostics.iter().any(|diagnostic| diagnostic
            .reason
            .contains("external metalTexture:nv12 texture handles are not imported")),
        "opaque NV12 handles must fail closed, diagnostics: {:?}",
        output.diagnostics
    );
}

#[test]
#[ignore = "manual platform smoke: run with VIDEO_EDITOR_TEST_WGPU=1 on WGPU hosts with ExternalTexture support"]
fn real_wgpu_compositor_samples_nv12_external_texture_planes() {
    if std::env::var("VIDEO_EDITOR_TEST_WGPU").ok().as_deref() != Some("1") {
        eprintln!("set VIDEO_EDITOR_TEST_WGPU=1 to run the real compositor import smoke");
        return;
    }

    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: RealtimePreviewGpuBackend::Auto,
        label: Some("real-wgpu-nv12-external-texture-test".to_owned()),
    })
    .expect("real GPU device should bootstrap");
    assert!(device.uses_physical_adapter());
    if !device.supports_external_texture() {
        eprintln!("WGPU adapter does not expose ExternalTexture; fail-closed contract covers this");
        return;
    }

    let video_id = MaterialId::from("video");
    let descriptor = nv12_texture_descriptor(&video_id, "wgpu-nv12-texture");
    let planes = device
        .create_nv12_external_texture_planes(2, 2, &[235, 235, 235, 235], &[128, 128])
        .expect("NV12 planes should allocate as WGPU textures");
    let registry = NativeTextureLeaseRegistry::new();
    registry
        .register_resource(
            descriptor
                .to_texture_handle()
                .expect("descriptor should convert to a texture handle"),
            NativeTextureLeaseResourceKind::WgpuExternalTexturePlanes,
            planes,
        )
        .expect("WGPU external texture planes should register");
    let target = device
        .create_offscreen_target(2, 2, 1_000, RealtimePreviewTargetFormat::Rgba8UnormSrgb)
        .expect("real GPU target should be valid");
    let mut compositor = RealtimePreviewCompositor::new(
        device,
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );
    let mut provider = TextureHandleProvider::new(video_id, descriptor);
    let mut texture_cache =
        RealtimePreviewTextureCache::new().with_native_texture_registry(registry);

    let output = compositor
        .render_offscreen(
            &single_video_graph(&provider.material_id, 2, 2),
            &target,
            &mut provider,
            &mut texture_cache,
        )
        .expect("real GPU compositor should render NV12 external texture planes");

    assert_eq!(
        output.render_backend,
        RealtimePreviewCompositorBackend::WgpuRenderPass
    );
    assert_eq!(output.support, RealtimePreviewGraphSupport::Supported);
    assert_eq!(output.submitted_draws, 1);
    assert!(
        output
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 160 && pixel[1] > 160 && pixel[2] > 160),
        "NV12 luma should produce visible GPU-sampled output pixels"
    );
}

#[test]
fn gpu_subset_gpu_module_does_not_import_forbidden_runtime_boundaries() {
    let gpu_sources = [
        include_str!("../src/gpu/mod.rs"),
        include_str!("../src/gpu/device.rs"),
        include_str!("../src/gpu/surface.rs"),
        include_str!("../src/gpu/compositor.rs"),
        include_str!("../src/gpu/pipelines.rs"),
        include_str!("../src/gpu/texture_cache.rs"),
    ]
    .join("\n");

    for forbidden in [
        "ffmpeg_compiler",
        "media_runtime_desktop",
        "Electron",
        "Command::new",
        "std::process",
    ] {
        assert!(
            !gpu_sources.contains(forbidden),
            "GPU module must not import or execute forbidden boundary: {forbidden}"
        );
    }
}

#[test]
fn gpu_subset_external_texture_shader_uses_current_wgpu_sampling_signature() {
    let compositor_source = include_str!("../src/gpu/compositor.rs");

    assert!(
        compositor_source.contains("@group(0) @binding(1) var layer_sampler: sampler;"),
        "external texture shader must bind an explicit sampler for wgpu 29"
    );
    assert!(
        compositor_source
            .contains("textureSampleBaseClampToEdge(layer_texture, layer_sampler, in.uv)"),
        "external texture shader must use the three-argument wgpu 29 sampling signature"
    );
    assert!(
        !compositor_source.contains("textureSampleBaseClampToEdge(layer_texture, in.uv)"),
        "old two-argument external texture sampling crashes the real WGPU pipeline"
    );
}

fn render_graph_with_provider(
    graph: RenderGraph,
    mut provider: impl PreviewFrameProvider,
) -> realtime_preview_runtime::gpu::RealtimePreviewCompositorOutput {
    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: RealtimePreviewGpuBackend::Mock,
        label: Some("gpu-subset-test".to_owned()),
    })
    .expect("mock GPU device should bootstrap");
    let target = RealtimePreviewGpuTarget::offscreen(
        4,
        4,
        1_000,
        RealtimePreviewTargetFormat::Rgba8UnormSrgb,
    )
    .expect("offscreen target should be valid");
    let mut compositor = RealtimePreviewCompositor::new(
        device,
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );
    let mut texture_cache = RealtimePreviewTextureCache::new();

    compositor
        .render_offscreen(&graph, &target, &mut provider, &mut texture_cache)
        .expect("mock offscreen composition should render")
}

fn render_graph_with_real_wgpu_provider(
    graph: RenderGraph,
    mut provider: impl PreviewFrameProvider,
) -> realtime_preview_runtime::gpu::RealtimePreviewCompositorOutput {
    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: RealtimePreviewGpuBackend::Auto,
        label: Some("real-wgpu-gpu-subset-test".to_owned()),
    })
    .expect("real GPU device should bootstrap");
    assert!(device.uses_physical_adapter());
    let target = device
        .create_offscreen_target(
            graph.canvas.width,
            graph.canvas.height,
            1_000,
            RealtimePreviewTargetFormat::Rgba8UnormSrgb,
        )
        .expect("real GPU target should be valid");
    let mut compositor = RealtimePreviewCompositor::new(
        device,
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );
    let mut texture_cache = RealtimePreviewTextureCache::new();

    compositor
        .render_offscreen(&graph, &target, &mut provider, &mut texture_cache)
        .expect("real GPU composition should render")
}

fn render_graph_with_real_wgpu_registry(
    graph: RenderGraph,
    mut provider: impl PreviewFrameProvider,
    registry: NativeTextureLeaseRegistry,
) -> realtime_preview_runtime::gpu::RealtimePreviewCompositorOutput {
    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: RealtimePreviewGpuBackend::Auto,
        label: Some("real-wgpu-gpu-registry-test".to_owned()),
    })
    .expect("real GPU device should bootstrap");
    assert!(device.uses_physical_adapter());
    let target = device
        .create_offscreen_target(
            graph.canvas.width,
            graph.canvas.height,
            1_000,
            RealtimePreviewTargetFormat::Rgba8UnormSrgb,
        )
        .expect("real GPU target should be valid");
    let mut compositor = RealtimePreviewCompositor::new(
        device,
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );
    let mut texture_cache =
        RealtimePreviewTextureCache::new().with_native_texture_registry(registry);

    compositor
        .render_offscreen(&graph, &target, &mut provider, &mut texture_cache)
        .expect("real GPU composition should render")
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

fn textured_graph(image_id: &MaterialId, video_id: &MaterialId, video_opacity: u32) -> RenderGraph {
    let mut graph = solid_canvas_graph("#000000");
    graph.materials = vec![
        material(image_id, MaterialKind::Image),
        material(video_id, MaterialKind::Video),
    ];
    graph.video_layers = vec![
        layer(
            "image-segment",
            image_id,
            MaterialKind::Image,
            0,
            -500,
            500,
            1_000,
        ),
        layer(
            "video-segment",
            video_id,
            MaterialKind::Video,
            1,
            500,
            -500,
            video_opacity,
        ),
    ];
    graph
}

fn single_video_graph(video_id: &MaterialId, width: u32, height: u32) -> RenderGraph {
    let mut graph = solid_canvas_graph("#000000");
    graph.canvas.width = width;
    graph.canvas.height = height;
    graph.materials = vec![material(video_id, MaterialKind::Video)];
    graph.materials[0].width = Some(width);
    graph.materials[0].height = Some(height);
    graph.video_layers = vec![layer(
        "video-segment",
        video_id,
        MaterialKind::Video,
        0,
        0,
        0,
        1_000,
    )];
    graph.video_layers[0].visual.transform.scale = SegmentScale {
        x_millis: 1_000,
        y_millis: 1_000,
    };
    graph
}

fn nv12_texture_descriptor(material_id: &MaterialId, handle_id: &str) -> TextureHandleDescriptor {
    TextureHandleDescriptor::new(
        material_id.clone(),
        Microseconds::ZERO,
        handle_id,
        MediaSessionId("session-texture-1".to_owned()),
        PlaybackGeneration::initial(),
        "metalTexture",
        RuntimeDeviceId {
            backend: TextureBackend::MetalTexture,
            adapter_id: "metal-adapter".to_owned(),
            device_id: "metal-device".to_owned(),
        },
        2,
        2,
        "nv12",
        VideoColorMetadata::unknown_with_diagnostic("test texture color"),
    )
    .expect("NV12 texture descriptor should be valid")
}

fn text_overlay_graph() -> RenderGraph {
    let mut draft = Draft::new("text-gpu", "Text GPU");
    draft.canvas_config.width = 128;
    draft.canvas_config.height = 72;
    draft.materials.push(Material::new(
        "text-material",
        MaterialKind::Text,
        "text://title",
        "text-material",
    ));

    let mut segment = Segment::new(
        "text-a",
        "text-material",
        SourceTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
    );
    let mut style = TextStyle::default_title();
    style.font_size = 18;
    style.color = "#ff0000".to_owned();
    style.alignment = TextAlignment::Left;
    segment.text = Some(TextSegment {
        content: "标题".to_owned(),
        source: Default::default(),
        style,
        text_box: Default::default(),
        layout_region: TextLayoutRegion {
            x_millis: 0,
            y_millis: 0,
            width_millis: 1_000,
            height_millis: 1_000,
        },
        wrapping: Default::default(),
        bubble: None,
        effect: None,
    });

    let mut track = Track::new("text-track", TrackKind::Text, "Text");
    track.segments.push(segment);
    draft.tracks.push(track);

    let graph = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft,
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(128, 72),
    })
    .expect("text overlay graph should prepare")
    .graph;
    assert_eq!(graph.text_overlays.len(), 1);
    assert_eq!(graph.text_overlays[0].overlay.content, "标题");
    graph
}

fn material(material_id: &MaterialId, kind: MaterialKind) -> RenderMaterial {
    RenderMaterial {
        node_id: RenderGraphNodeId::material(&DraftId::from("draft"), material_id),
        material_id: material_id.clone(),
        kind,
        uri: format!("file:///{}.png", material_id.as_str()),
        display_name: material_id.as_str().to_owned(),
        duration: Some(Microseconds::new(1_000_000)),
        frame_rate: Some(RationalFrameRate::new(30, 1)),
        width: Some(1),
        height: Some(1),
        has_video: true,
        has_audio: false,
    }
}

fn layer(
    segment_id: &str,
    material_id: &MaterialId,
    material_kind: MaterialKind,
    stack_index: u32,
    x: i32,
    y: i32,
    opacity: u32,
) -> RenderVideoLayer {
    let mut visual = SegmentVisual::default();
    visual.fit_mode = SegmentFitMode::Stretch;
    visual.transform.position = SegmentPosition { x, y };
    visual.transform.scale = SegmentScale {
        x_millis: 500,
        y_millis: 500,
    };
    visual.transform.opacity = SegmentOpacity {
        value_millis: opacity,
    };

    let track_id = TrackId::from(format!("track-{stack_index}"));
    let segment_id = draft_model::SegmentId::from(segment_id);

    RenderVideoLayer {
        node_id: RenderGraphNodeId::video_segment(
            &DraftId::from("draft"),
            &track_id,
            &segment_id,
            material_id,
        ),
        track_id,
        segment_id,
        material_id: material_id.clone(),
        material_kind,
        stack_index,
        source_timerange: SourceTimerange::new(0, 1_000_000),
        target_timerange: TargetTimerange::new(0, 1_000_000),
        keyframes: Vec::new(),
        filters: Vec::new(),
        transition: None,
        visual,
    }
}

fn rgba_at(pixels: &[u8], x: u32, y: u32, width: u32) -> [u8; 4] {
    let index = ((y * width + x) * 4) as usize;
    [
        pixels[index],
        pixels[index + 1],
        pixels[index + 2],
        pixels[index + 3],
    ]
}

struct EmptyProvider;

impl PreviewFrameProvider for EmptyProvider {
    fn provider_name(&self) -> &'static str {
        "empty-provider"
    }

    fn frame_for(
        &mut self,
        material_id: &MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
        Err(PreviewFrameProviderError::unavailable(
            self.provider_name(),
            material_id.clone(),
            source_position,
            playback_generation,
            "no frame expected for solid canvas tests",
        ))
    }
}

struct ImageThenSoftwareVideoProvider {
    image_id: MaterialId,
    image_frame: CpuVideoFrame,
    video_provider: SoftwareVideoFrameProvider,
}

struct TextureHandleProvider {
    material_id: MaterialId,
    descriptor: TextureHandleDescriptor,
}

struct RecordingFrameProvider {
    material_id: MaterialId,
    requests: Rc<RefCell<Vec<Microseconds>>>,
}

impl RecordingFrameProvider {
    fn new(material_id: MaterialId, requests: Rc<RefCell<Vec<Microseconds>>>) -> Self {
        Self {
            material_id,
            requests,
        }
    }
}

impl TextureHandleProvider {
    fn new(material_id: MaterialId, descriptor: TextureHandleDescriptor) -> Self {
        Self {
            material_id,
            descriptor,
        }
    }
}

impl PreviewFrameProvider for TextureHandleProvider {
    fn provider_name(&self) -> &'static str {
        "texture-handle-provider"
    }

    fn frame_for(
        &mut self,
        material_id: &MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
        if material_id != &self.material_id {
            return Err(PreviewFrameProviderError::unavailable(
                self.provider_name(),
                material_id.clone(),
                source_position,
                playback_generation,
                "unexpected material id",
            ));
        }
        let mut descriptor = self.descriptor.clone();
        descriptor.source_position = source_position;
        descriptor.playback_generation = playback_generation;
        Ok(PreviewFrameInput::TextureHandle(descriptor))
    }
}

impl PreviewFrameProvider for RecordingFrameProvider {
    fn provider_name(&self) -> &'static str {
        "recording-frame-provider"
    }

    fn frame_for(
        &mut self,
        material_id: &MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
        if material_id != &self.material_id {
            return Err(PreviewFrameProviderError::unavailable(
                self.provider_name(),
                material_id.clone(),
                source_position,
                playback_generation,
                "unexpected material id",
            ));
        }
        self.requests.borrow_mut().push(source_position);
        PreviewFrameInput::cpu_rgba(
            material_id.clone(),
            source_position,
            playback_generation,
            1,
            1,
            vec![0, 0, 255, 255],
        )
        .map_err(|error| {
            PreviewFrameProviderError::invalid_frame(
                self.provider_name(),
                Some(material_id.clone()),
                Some(source_position),
                Some(playback_generation),
                error,
            )
        })
    }
}

impl ImageThenSoftwareVideoProvider {
    fn new(image_id: &MaterialId, video_id: &MaterialId) -> Self {
        let generation = PlaybackGeneration::new(1);
        let image_frame = CpuVideoFrame::new(
            image_id.clone(),
            Microseconds::ZERO,
            generation,
            1,
            1,
            4,
            FrameColorInfo::srgb_rgba8(),
            vec![255, 0, 0, 255],
        )
        .expect("image frame should be valid");
        let video_frame = CpuVideoFrame::new(
            video_id.clone(),
            Microseconds::ZERO,
            generation,
            1,
            1,
            4,
            FrameColorInfo::srgb_rgba8(),
            vec![0, 0, 255, 255],
        )
        .expect("video frame should be valid");
        let mut cache = DecodedVideoFrameCache::new();
        cache
            .insert_h264_frames(
                video_id.clone(),
                RationalFrameRate::new(30, 1),
                1,
                vec![(0, video_frame)],
            )
            .expect("video frame cache should accept H.264 frame");
        Self {
            image_id: image_id.clone(),
            image_frame,
            video_provider: SoftwareVideoFrameProvider::new(cache),
        }
    }
}

impl PreviewFrameProvider for ImageThenSoftwareVideoProvider {
    fn provider_name(&self) -> &'static str {
        "image-then-software-video-provider"
    }

    fn frame_for(
        &mut self,
        material_id: &MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
        if material_id == &self.image_id {
            let mut frame = self.image_frame.clone();
            frame.source_position = source_position;
            frame.playback_generation = playback_generation;
            return Ok(PreviewFrameInput::StaticImage(frame));
        }
        self.video_provider
            .frame_for(material_id, source_position, playback_generation)
    }
}
