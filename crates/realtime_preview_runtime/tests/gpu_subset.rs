use draft_model::{
    DraftId, MaterialId, MaterialKind, Microseconds, RationalFrameRate, SegmentFitMode,
    SegmentOpacity, SegmentPosition, SegmentScale, SegmentVisual, SourceTimerange, TargetTimerange,
    TrackId,
};
use realtime_preview_runtime::gpu::{
    RealtimePreviewCompositor, RealtimePreviewGpuBackend, RealtimePreviewGpuDevice,
    RealtimePreviewGpuDeviceDescriptor, RealtimePreviewGpuTarget, RealtimePreviewTargetFormat,
    RealtimePreviewTextureCache,
};
use realtime_preview_runtime::{
    CpuVideoFrame, DecodedVideoFrameCache, FrameColorInfo, PlaybackGeneration, PreviewFrameInput,
    PreviewFrameProvider, PreviewFrameProviderError, RealtimePreviewCapabilityClassifier,
    RealtimePreviewGraphSupport, SoftwareVideoFrameProvider,
};
use render_graph::{
    RenderAudioMix, RenderCanvas, RenderCanvasBackground, RenderCanvasBackgroundMode, RenderGraph,
    RenderIntentSupport, RenderMaterial, RenderSampledFrame, RenderVideoLayer,
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

fn solid_canvas_graph(color: &str) -> RenderGraph {
    RenderGraph {
        draft_id: DraftId::from("draft"),
        canvas: RenderCanvas {
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

fn material(material_id: &MaterialId, kind: MaterialKind) -> RenderMaterial {
    RenderMaterial {
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

    RenderVideoLayer {
        track_id: TrackId::from(format!("track-{stack_index}")),
        segment_id: draft_model::SegmentId::from(segment_id),
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
