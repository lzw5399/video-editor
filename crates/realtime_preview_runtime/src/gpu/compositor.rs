use std::error::Error;
use std::fmt;
use std::sync::mpsc;
use std::time::Duration;

use draft_model::{MaterialId, SegmentFitMode, SegmentVisual};
use render_graph::{
    RenderCanvasBackgroundMode, RenderGraph, RenderMaterial, RenderTextOverlay, RenderVideoLayer,
};

use crate::{
    PlaybackGeneration, PreviewFrameInput, PreviewFrameProvider,
    RealtimePreviewCapabilityClassifier, RealtimePreviewDiagnostic,
    RealtimePreviewDiagnosticDomain, RealtimePreviewGraphSupport, RealtimePreviewSupport,
};

use super::{
    RealtimePreviewGpuDevice, RealtimePreviewGpuPresentationTarget, RealtimePreviewGpuTarget,
    RealtimePreviewPipelineSet, RealtimePreviewTexture, RealtimePreviewTextureCache,
    RealtimePreviewTextureCacheError,
};

use super::text::{TextRasterizationError, rasterize_text_overlay};

#[derive(Debug)]
pub struct RealtimePreviewCompositor {
    device: RealtimePreviewGpuDevice,
    classifier: RealtimePreviewCapabilityClassifier,
    pipelines: RealtimePreviewPipelineSet,
}

impl RealtimePreviewCompositor {
    pub fn new(
        device: RealtimePreviewGpuDevice,
        classifier: RealtimePreviewCapabilityClassifier,
    ) -> Self {
        Self {
            device,
            classifier,
            pipelines: RealtimePreviewPipelineSet::phase11_subset(),
        }
    }

    pub fn render_offscreen(
        &mut self,
        graph: &RenderGraph,
        target: &RealtimePreviewGpuTarget,
        frame_provider: &mut impl PreviewFrameProvider,
        texture_cache: &mut RealtimePreviewTextureCache,
    ) -> Result<RealtimePreviewCompositorOutput, RealtimePreviewCompositorError> {
        let _target_has_device_texture = target.texture().is_some();
        let mut diagnostics = Vec::new();
        validate_target_matches_graph(target, graph, &mut diagnostics);
        let capability = self.classifier.classify(graph);
        diagnostics.extend(capability.diagnostics);

        let mut support = summarize_support(capability.support, &diagnostics);
        if let Some(texture) = target.texture() {
            if support != RealtimePreviewGraphSupport::Unsupported {
                let (pixels, submitted_draws) = render_wgpu_graph(
                    graph,
                    target,
                    &self.device,
                    texture,
                    frame_provider,
                    &mut diagnostics,
                    &mut support,
                )?;
                return Ok(RealtimePreviewCompositorOutput {
                    width: target.width(),
                    height: target.height(),
                    pixels,
                    submitted_draws,
                    render_backend: RealtimePreviewCompositorBackend::WgpuRenderPass,
                    support,
                    diagnostics,
                });
            }
        }

        let mut pixels = canvas_pixels(graph, target)?;
        if support == RealtimePreviewGraphSupport::Unsupported {
            return Ok(RealtimePreviewCompositorOutput {
                width: target.width(),
                height: target.height(),
                pixels,
                submitted_draws: 0,
                render_backend: RealtimePreviewCompositorBackend::CpuReference,
                support,
                diagnostics,
            });
        }

        let mut submitted_draws = 0;
        let mut layers = graph.video_layers.iter().collect::<Vec<_>>();
        layers.sort_by(|first, second| {
            first
                .stack_index
                .cmp(&second.stack_index)
                .then_with(|| first.track_id.cmp(&second.track_id))
                .then_with(|| first.segment_id.cmp(&second.segment_id))
        });

        for layer in layers {
            let Some(material) = material_for(graph, &layer.material_id) else {
                diagnostics.push(layer_diagnostic(
                    layer,
                    "render graph layer references a missing material",
                ));
                support = RealtimePreviewGraphSupport::Unsupported;
                continue;
            };

            let source_position = layer.source_timerange.start;
            let frame = match frame_provider.frame_for(
                &layer.material_id,
                source_position,
                PlaybackGeneration::initial(),
            ) {
                Ok(frame) => frame,
                Err(error) => {
                    diagnostics.push(layer_diagnostic(layer, error.to_string()));
                    support = RealtimePreviewGraphSupport::Unsupported;
                    continue;
                }
            };
            let texture = match texture_cache.upload_frame(&self.device, frame) {
                Ok(texture) => texture,
                Err(error) => {
                    diagnostics.push(texture_cache_diagnostic(layer, error));
                    support = RealtimePreviewGraphSupport::Unsupported;
                    continue;
                }
            };

            let visual = sampled_visual_for(graph, layer).unwrap_or(&layer.visual);
            draw_textured_quad(&mut pixels, target, material, layer, visual, &texture)?;
            submitted_draws += 1;
        }

        Ok(RealtimePreviewCompositorOutput {
            width: target.width(),
            height: target.height(),
            pixels,
            submitted_draws,
            render_backend: RealtimePreviewCompositorBackend::CpuReference,
            support,
            diagnostics,
        })
    }

    pub fn pipelines(&self) -> &RealtimePreviewPipelineSet {
        &self.pipelines
    }

    pub fn present_to_surface(
        &mut self,
        graph: &RenderGraph,
        target: &mut RealtimePreviewGpuPresentationTarget,
        frame_provider: &mut impl PreviewFrameProvider,
        _texture_cache: &mut RealtimePreviewTextureCache,
    ) -> Result<RealtimePreviewSurfacePresentationOutput, RealtimePreviewCompositorError> {
        let device_ref = self
            .device
            .device()
            .ok_or(RealtimePreviewCompositorError::WgpuDeviceUnavailable)?;
        let queue = self
            .device
            .queue()
            .ok_or(RealtimePreviewCompositorError::WgpuQueueUnavailable)?;
        let mut diagnostics = Vec::new();
        validate_target_dimensions(target, graph, "presentation surface", &mut diagnostics);
        let capability = self.classifier.classify(graph);
        diagnostics.extend(capability.diagnostics);
        let mut support = summarize_support(capability.support, &diagnostics);

        if support == RealtimePreviewGraphSupport::Unsupported {
            return Ok(RealtimePreviewSurfacePresentationOutput {
                width: target.width(),
                height: target.height(),
                pixels: None,
                submitted_draws: 0,
                presented_frames: 0,
                render_backend: RealtimePreviewCompositorBackend::WgpuSurfacePresent,
                support,
                diagnostics,
            });
        }

        let surface_texture = acquire_surface_texture(target.surface())?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let (encoder, submitted_draws) = encode_wgpu_graph_to_view(
            graph,
            target,
            device_ref,
            queue,
            &view,
            frame_provider,
            &mut diagnostics,
            &mut support,
        )?;
        if support == RealtimePreviewGraphSupport::Unsupported {
            drop(view);
            drop(surface_texture);
            return Ok(RealtimePreviewSurfacePresentationOutput {
                width: target.width(),
                height: target.height(),
                pixels: None,
                submitted_draws: 0,
                presented_frames: 0,
                render_backend: RealtimePreviewCompositorBackend::WgpuSurfacePresent,
                support,
                diagnostics,
            });
        }

        let submission = queue.submit([encoder.finish()]);
        poll_wgpu(device_ref, Some(submission))?;
        surface_texture.present();

        Ok(RealtimePreviewSurfacePresentationOutput {
            width: target.width(),
            height: target.height(),
            pixels: None,
            submitted_draws,
            presented_frames: 1,
            render_backend: RealtimePreviewCompositorBackend::WgpuSurfacePresent,
            support,
            diagnostics,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewCompositorOutput {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub submitted_draws: u32,
    pub render_backend: RealtimePreviewCompositorBackend,
    pub support: RealtimePreviewGraphSupport,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewSurfacePresentationOutput {
    pub width: u32,
    pub height: u32,
    pub pixels: Option<Vec<u8>>,
    pub submitted_draws: u32,
    pub presented_frames: u32,
    pub render_backend: RealtimePreviewCompositorBackend,
    pub support: RealtimePreviewGraphSupport,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimePreviewCompositorBackend {
    CpuReference,
    WgpuRenderPass,
    WgpuSurfacePresent,
}

#[derive(Debug)]
pub enum RealtimePreviewCompositorError {
    InvalidCanvasColor(String),
    ExternalTextureRequiresGpuCompositor { handle_id: String, backend: String },
    PixelBufferOverflow,
    WgpuDeviceUnavailable,
    WgpuQueueUnavailable,
    WgpuFrameUpload(String),
    WgpuLayerTextureHandleUnsupported { handle_id: String, backend: String },
    WgpuReadbackMap(String),
    WgpuReadbackTimeout,
    WgpuPoll(String),
    WgpuSurfaceAcquire(String),
}

impl fmt::Display for RealtimePreviewCompositorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCanvasColor(color) => write!(formatter, "invalid canvas color {color}"),
            Self::ExternalTextureRequiresGpuCompositor { backend, .. } => write!(
                formatter,
                "external {backend} texture handles require the GPU compositor path"
            ),
            Self::PixelBufferOverflow => formatter.write_str("compositor pixel buffer overflow"),
            Self::WgpuDeviceUnavailable => formatter.write_str("wgpu device is unavailable"),
            Self::WgpuQueueUnavailable => formatter.write_str("wgpu queue is unavailable"),
            Self::WgpuFrameUpload(error) => write!(formatter, "wgpu frame upload failed: {error}"),
            Self::WgpuLayerTextureHandleUnsupported { backend, .. } => write!(
                formatter,
                "external {backend} texture handles are not imported into the wgpu compositor yet"
            ),
            Self::WgpuReadbackMap(error) => write!(formatter, "wgpu readback map failed: {error}"),
            Self::WgpuReadbackTimeout => formatter.write_str("wgpu readback timed out"),
            Self::WgpuPoll(error) => write!(formatter, "wgpu device poll failed: {error}"),
            Self::WgpuSurfaceAcquire(error) => {
                write!(formatter, "wgpu surface texture acquire failed: {error}")
            }
        }
    }
}

impl Error for RealtimePreviewCompositorError {}

fn validate_target_matches_graph(
    target: &RealtimePreviewGpuTarget,
    graph: &RenderGraph,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
) {
    validate_target_dimensions(target, graph, "offscreen target", diagnostics);
}

fn validate_target_dimensions(
    target: &impl WgpuRenderTargetInfo,
    graph: &RenderGraph,
    label: &'static str,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
) {
    if target.width() != graph.canvas.width || target.height() != graph.canvas.height {
        let message = format!("{label} dimensions must match render graph canvas");
        diagnostics.push(RealtimePreviewDiagnostic::new(
            None,
            RealtimePreviewDiagnosticDomain::Surface,
            RealtimePreviewSupport::Unsupported {
                reason: message.clone(),
            },
            message,
            None,
            true,
        ));
    }
}

fn canvas_pixels(
    graph: &RenderGraph,
    target: &RealtimePreviewGpuTarget,
) -> Result<Vec<u8>, RealtimePreviewCompositorError> {
    let color = match graph.canvas.background.mode {
        RenderCanvasBackgroundMode::Black => [0, 0, 0, 255],
        RenderCanvasBackgroundMode::SolidColor => parse_hex_color(
            graph
                .canvas
                .background
                .color
                .as_deref()
                .unwrap_or("#000000"),
        )?,
        RenderCanvasBackgroundMode::BlurFill | RenderCanvasBackgroundMode::Image => [0, 0, 0, 255],
    };
    let len = target
        .width()
        .checked_mul(target.height())
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(RealtimePreviewCompositorError::PixelBufferOverflow)?;
    let mut pixels = vec![0; len];
    for chunk in pixels.chunks_exact_mut(4) {
        chunk.copy_from_slice(&color);
    }
    Ok(pixels)
}

fn render_wgpu_graph(
    graph: &RenderGraph,
    target: &RealtimePreviewGpuTarget,
    device: &RealtimePreviewGpuDevice,
    texture: &wgpu::Texture,
    frame_provider: &mut impl PreviewFrameProvider,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
) -> Result<(Vec<u8>, u32), RealtimePreviewCompositorError> {
    let device_ref = device
        .device()
        .ok_or(RealtimePreviewCompositorError::WgpuDeviceUnavailable)?;
    let queue = device
        .queue()
        .ok_or(RealtimePreviewCompositorError::WgpuQueueUnavailable)?;
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let (mut encoder, submitted_draws) = encode_wgpu_graph_to_view(
        graph,
        target,
        device_ref,
        queue,
        &view,
        frame_provider,
        diagnostics,
        support,
    )?;

    let unpadded_bytes_per_row = target.width() as usize * target.format().bytes_per_pixel();
    let padded_bytes_per_row = align_to(
        unpadded_bytes_per_row,
        wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize,
    );
    let buffer_size = padded_bytes_per_row
        .checked_mul(target.height() as usize)
        .ok_or(RealtimePreviewCompositorError::PixelBufferOverflow)?;
    let readback = device_ref.create_buffer(&wgpu::BufferDescriptor {
        label: Some("realtime-preview-wgpu-readback"),
        size: buffer_size as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row as u32),
                rows_per_image: Some(target.height()),
            },
        },
        wgpu::Extent3d {
            width: target.width(),
            height: target.height(),
            depth_or_array_layers: 1,
        },
    );

    let submission = queue.submit([encoder.finish()]);
    poll_wgpu(device_ref, Some(submission))?;

    let slice = readback.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    poll_wgpu(device_ref, None)?;
    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| RealtimePreviewCompositorError::WgpuReadbackTimeout)?
        .map_err(|error| RealtimePreviewCompositorError::WgpuReadbackMap(error.to_string()))?;

    let mapped = slice.get_mapped_range();
    let mut pixels = vec![0_u8; target.pixel_len()];
    for row in 0..target.height() as usize {
        let source_start = row * padded_bytes_per_row;
        let source_end = source_start + unpadded_bytes_per_row;
        let dest_start = row * unpadded_bytes_per_row;
        let dest_end = dest_start + unpadded_bytes_per_row;
        pixels[dest_start..dest_end].copy_from_slice(&mapped[source_start..source_end]);
    }
    drop(mapped);
    readback.unmap();

    Ok((pixels, submitted_draws))
}

trait WgpuRenderTargetInfo {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn format(&self) -> super::RealtimePreviewTargetFormat;
}

impl WgpuRenderTargetInfo for RealtimePreviewGpuTarget {
    fn width(&self) -> u32 {
        self.width()
    }

    fn height(&self) -> u32 {
        self.height()
    }

    fn format(&self) -> super::RealtimePreviewTargetFormat {
        self.format()
    }
}

impl WgpuRenderTargetInfo for RealtimePreviewGpuPresentationTarget {
    fn width(&self) -> u32 {
        self.width()
    }

    fn height(&self) -> u32 {
        self.height()
    }

    fn format(&self) -> super::RealtimePreviewTargetFormat {
        self.format()
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_wgpu_graph_to_view(
    graph: &RenderGraph,
    target: &impl WgpuRenderTargetInfo,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    frame_provider: &mut impl PreviewFrameProvider,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
) -> Result<(wgpu::CommandEncoder, u32), RealtimePreviewCompositorError> {
    let clear_color = canvas_clear_color(graph)?;
    let pipeline_resources = if graph.video_layers.is_empty() && graph.text_overlays.is_empty() {
        None
    } else {
        Some(RealtimePreviewWgpuPipelines::new(device, target.format()))
    };
    let layer_draws = if let Some(resources) = pipeline_resources.as_ref() {
        prepare_wgpu_layer_draws(
            graph,
            target,
            device,
            queue,
            resources,
            frame_provider,
            diagnostics,
            support,
        )?
    } else {
        Vec::new()
    };
    let submitted_draws = layer_draws.len().min(u32::MAX as usize) as u32;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("realtime-preview-wgpu-graph-encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("realtime-preview-wgpu-graph-render-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some(resources) = pipeline_resources.as_ref() {
            pass.set_pipeline(&resources.pipeline);
            for draw in &layer_draws {
                pass.set_bind_group(0, &draw.bind_group, &[]);
                pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
                pass.draw(0..6, 0..1);
            }
        }
    }
    Ok((encoder, submitted_draws))
}

struct RealtimePreviewWgpuPipelines {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl RealtimePreviewWgpuPipelines {
    fn new(device: &wgpu::Device, format: super::RealtimePreviewTargetFormat) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("realtime-preview-textured-quad-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("realtime-preview-textured-quad-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("realtime-preview-textured-quad-shader"),
            source: wgpu::ShaderSource::Wgsl(TEXTURED_QUAD_SHADER.into()),
        });
        let vertex_attributes = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 8,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: 16,
                shader_location: 2,
            },
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("realtime-preview-textured-quad-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &vertex_attributes,
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: format.wgpu_format(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("realtime-preview-textured-quad-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }
}

struct WgpuLayerDraw {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
}

fn prepare_wgpu_layer_draws(
    graph: &RenderGraph,
    target: &impl WgpuRenderTargetInfo,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &RealtimePreviewWgpuPipelines,
    frame_provider: &mut impl PreviewFrameProvider,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
) -> Result<Vec<WgpuLayerDraw>, RealtimePreviewCompositorError> {
    let mut draws = Vec::new();
    let mut layers = graph_draw_layers(graph);
    layers.sort_by(|first, second| {
        first
            .stack_index()
            .cmp(&second.stack_index())
            .then_with(|| first.track_id().cmp(second.track_id()))
            .then_with(|| first.segment_id().cmp(second.segment_id()))
    });

    for layer in layers {
        match layer {
            GraphDrawLayer::Video(layer) => {
                push_wgpu_video_layer_draw(
                    graph,
                    target,
                    device,
                    queue,
                    resources,
                    frame_provider,
                    diagnostics,
                    support,
                    &mut draws,
                    layer,
                )?;
            }
            GraphDrawLayer::Text(text) => {
                push_wgpu_text_layer_draw(
                    graph,
                    target,
                    device,
                    queue,
                    resources,
                    diagnostics,
                    support,
                    &mut draws,
                    text,
                )?;
            }
        }
    }

    if *support == RealtimePreviewGraphSupport::Unsupported {
        return Ok(Vec::new());
    }

    Ok(draws)
}

#[derive(Clone, Copy)]
enum GraphDrawLayer<'a> {
    Video(&'a RenderVideoLayer),
    Text(&'a RenderTextOverlay),
}

impl<'a> GraphDrawLayer<'a> {
    fn stack_index(&self) -> u32 {
        match self {
            Self::Video(layer) => layer.stack_index,
            Self::Text(text) => text.overlay.stack_index,
        }
    }

    fn track_id(&self) -> &draft_model::TrackId {
        match self {
            Self::Video(layer) => &layer.track_id,
            Self::Text(text) => &text.overlay.track_id,
        }
    }

    fn segment_id(&self) -> &draft_model::SegmentId {
        match self {
            Self::Video(layer) => &layer.segment_id,
            Self::Text(text) => &text.overlay.segment_id,
        }
    }
}

fn graph_draw_layers(graph: &RenderGraph) -> Vec<GraphDrawLayer<'_>> {
    graph
        .video_layers
        .iter()
        .map(GraphDrawLayer::Video)
        .chain(graph.text_overlays.iter().map(GraphDrawLayer::Text))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn push_wgpu_video_layer_draw(
    graph: &RenderGraph,
    target: &impl WgpuRenderTargetInfo,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &RealtimePreviewWgpuPipelines,
    frame_provider: &mut impl PreviewFrameProvider,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
    draws: &mut Vec<WgpuLayerDraw>,
    layer: &RenderVideoLayer,
) -> Result<(), RealtimePreviewCompositorError> {
    let Some(material) = material_for(graph, &layer.material_id) else {
        diagnostics.push(layer_diagnostic(
            layer,
            "render graph layer references a missing material",
        ));
        *support = RealtimePreviewGraphSupport::Unsupported;
        return Ok(());
    };
    let frame = match frame_provider.frame_for(
        &layer.material_id,
        layer.source_timerange.start,
        PlaybackGeneration::initial(),
    ) {
        Ok(input) => input,
        Err(error) => {
            diagnostics.push(layer_diagnostic(layer, error.to_string()));
            *support = RealtimePreviewGraphSupport::Unsupported;
            return Ok(());
        }
    };
    let texture = match upload_wgpu_layer_texture(device, queue, frame) {
        Ok(texture) => texture,
        Err(error) => {
            diagnostics.push(layer_diagnostic(layer, error.to_string()));
            *support = RealtimePreviewGraphSupport::Unsupported;
            return Ok(());
        }
    };
    let visual = sampled_visual_for(graph, layer).unwrap_or(&layer.visual);
    push_wgpu_texture_draw(
        device,
        queue,
        resources,
        draws,
        texture,
        textured_quad_vertices(target, material, visual),
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn push_wgpu_text_layer_draw(
    graph: &RenderGraph,
    target: &impl WgpuRenderTargetInfo,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &RealtimePreviewWgpuPipelines,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
    draws: &mut Vec<WgpuLayerDraw>,
    text: &RenderTextOverlay,
) -> Result<(), RealtimePreviewCompositorError> {
    let rasterized = match rasterize_text_overlay(
        text,
        sampled_text_for(graph, text),
        target.width(),
        target.height(),
    ) {
        Ok(layer) => layer,
        Err(error) => {
            diagnostics.push(text_diagnostic(text, error));
            *support = RealtimePreviewGraphSupport::Unsupported;
            return Ok(());
        }
    };
    let texture = upload_wgpu_rgba_texture(
        device,
        queue,
        rasterized.width,
        rasterized.height,
        rasterized.stride_bytes,
        &rasterized.pixels,
        "realtime-preview-text-layer-texture",
    )?;
    push_wgpu_texture_draw(
        device,
        queue,
        resources,
        draws,
        texture,
        textured_rect_vertices(
            target,
            rasterized.x,
            rasterized.y,
            rasterized.width,
            rasterized.height,
            text.visual.transform.opacity.value_millis.min(1_000) as f32 / 1_000.0,
        ),
    );
    Ok(())
}

fn push_wgpu_texture_draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &RealtimePreviewWgpuPipelines,
    draws: &mut Vec<WgpuLayerDraw>,
    texture: wgpu::Texture,
    vertices: Vec<u8>,
) {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("realtime-preview-textured-quad-bind-group"),
        layout: &resources.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&resources.sampler),
            },
        ],
    });
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("realtime-preview-textured-quad-vertices"),
        size: vertices.len() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, &vertices);
    draws.push(WgpuLayerDraw {
        _texture: texture,
        _view: view,
        bind_group,
        vertex_buffer,
    });
}

fn sampled_text_for<'a>(
    graph: &'a RenderGraph,
    text: &RenderTextOverlay,
) -> Option<&'a render_graph::graph::RenderSampledTextOverlay> {
    graph.sampled_animation_states.first().and_then(|sample| {
        sample.text_overlays.iter().find(|sampled| {
            sampled.track_id == text.overlay.track_id
                && sampled.segment_id == text.overlay.segment_id
                && sampled.material_id == text.material_id
        })
    })
}

fn upload_wgpu_rgba_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    stride_bytes: u32,
    pixels: &[u8],
    label: &'static str,
) -> Result<wgpu::Texture, RealtimePreviewCompositorError> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride_bytes),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    Ok(texture)
}

fn upload_wgpu_layer_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: PreviewFrameInput,
) -> Result<wgpu::Texture, RealtimePreviewCompositorError> {
    let frame = match input {
        PreviewFrameInput::CpuRgba(frame) | PreviewFrameInput::StaticImage(frame) => frame,
        PreviewFrameInput::TextureHandle(handle) => {
            return Err(
                RealtimePreviewCompositorError::WgpuLayerTextureHandleUnsupported {
                    handle_id: handle.handle_id,
                    backend: handle.backend,
                },
            );
        }
        PreviewFrameInput::Unavailable { reason } => {
            return Err(RealtimePreviewCompositorError::WgpuFrameUpload(reason));
        }
    };
    frame
        .validate()
        .map_err(|error| RealtimePreviewCompositorError::WgpuFrameUpload(error.to_string()))?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("realtime-preview-layer-texture"),
        size: wgpu::Extent3d {
            width: frame.width,
            height: frame.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &frame.pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(frame.stride_bytes),
            rows_per_image: Some(frame.height),
        },
        wgpu::Extent3d {
            width: frame.width,
            height: frame.height,
            depth_or_array_layers: 1,
        },
    );
    Ok(texture)
}

fn textured_quad_vertices(
    target: &impl WgpuRenderTargetInfo,
    material: &RenderMaterial,
    visual: &SegmentVisual,
) -> Vec<u8> {
    let source = cropped_source_dimensions(material, visual);
    let output = Dimensions {
        width: target.width(),
        height: target.height(),
    };
    let fitted = fit_dimensions(source, output, visual.fit_mode);
    let scaled = Dimensions {
        width: millis_of(fitted.width, visual.transform.scale.x_millis).max(1),
        height: millis_of(fitted.height, visual.transform.scale.y_millis).max(1),
    };
    let placement = layer_placement(visual, output, scaled);
    let anchor_x = f64::from(millis_of(scaled.width, visual.transform.anchor.x_millis));
    let anchor_y = f64::from(millis_of(scaled.height, visual.transform.anchor.y_millis));
    let pivot_x = placement.x as f64 + anchor_x;
    let pivot_y = placement.y as f64 + anchor_y;
    let radians = f64::from(visual.transform.rotation.degrees).to_radians();
    let sin = radians.sin();
    let cos = radians.cos();

    let left = placement.x as f64;
    let top = placement.y as f64;
    let right = left + f64::from(scaled.width);
    let bottom = top + f64::from(scaled.height);
    let crop = &visual.transform.crop;
    let u0 = f32::from(crop.left_millis.min(999) as u16) / 1_000.0;
    let v0 = f32::from(crop.top_millis.min(999) as u16) / 1_000.0;
    let u1 = (1.0 - f32::from(crop.right_millis.min(999) as u16) / 1_000.0).max(u0);
    let v1 = (1.0 - f32::from(crop.bottom_millis.min(999) as u16) / 1_000.0).max(v0);
    let opacity = visual.transform.opacity.value_millis.min(1_000) as f32 / 1_000.0;

    textured_vertices_from_corners(
        output,
        [
            vertex_corner(
                left, top, pivot_x, pivot_y, sin, cos, output, u0, v0, opacity,
            ),
            vertex_corner(
                right, top, pivot_x, pivot_y, sin, cos, output, u1, v0, opacity,
            ),
            vertex_corner(
                right, bottom, pivot_x, pivot_y, sin, cos, output, u1, v1, opacity,
            ),
            vertex_corner(
                left, bottom, pivot_x, pivot_y, sin, cos, output, u0, v1, opacity,
            ),
        ],
    )
}

fn textured_rect_vertices(
    target: &impl WgpuRenderTargetInfo,
    x: i64,
    y: i64,
    width: u32,
    height: u32,
    opacity: f32,
) -> Vec<u8> {
    let output = Dimensions {
        width: target.width(),
        height: target.height(),
    };
    let left = x as f64;
    let top = y as f64;
    let right = left + f64::from(width);
    let bottom = top + f64::from(height);
    textured_vertices_from_corners(
        output,
        [
            vertex_corner(left, top, 0.0, 0.0, 0.0, 1.0, output, 0.0, 0.0, opacity),
            vertex_corner(right, top, 0.0, 0.0, 0.0, 1.0, output, 1.0, 0.0, opacity),
            vertex_corner(right, bottom, 0.0, 0.0, 0.0, 1.0, output, 1.0, 1.0, opacity),
            vertex_corner(left, bottom, 0.0, 0.0, 0.0, 1.0, output, 0.0, 1.0, opacity),
        ],
    )
}

fn textured_vertices_from_corners(output: Dimensions, corners: [[f32; 5]; 4]) -> Vec<u8> {
    let _ = output;
    let indices = [0_usize, 1, 2, 0, 2, 3];
    let mut bytes = Vec::with_capacity(indices.len() * 5 * std::mem::size_of::<f32>());
    for index in indices {
        for value in corners[index] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}

fn cropped_source_dimensions(material: &RenderMaterial, visual: &SegmentVisual) -> Dimensions {
    let width = material.width.unwrap_or(1).max(1);
    let height = material.height.unwrap_or(1).max(1);
    let crop = &visual.transform.crop;
    let remaining_width = 1_000_u32
        .saturating_sub(crop.left_millis + crop.right_millis)
        .max(1);
    let remaining_height = 1_000_u32
        .saturating_sub(crop.top_millis + crop.bottom_millis)
        .max(1);
    Dimensions {
        width: millis_of(width, remaining_width).max(1),
        height: millis_of(height, remaining_height).max(1),
    }
}

#[allow(clippy::too_many_arguments)]
fn vertex_corner(
    x: f64,
    y: f64,
    pivot_x: f64,
    pivot_y: f64,
    sin: f64,
    cos: f64,
    output: Dimensions,
    u: f32,
    v: f32,
    opacity: f32,
) -> [f32; 5] {
    let local_x = x - pivot_x;
    let local_y = y - pivot_y;
    let rotated_x = pivot_x + local_x * cos - local_y * sin;
    let rotated_y = pivot_y + local_x * sin + local_y * cos;
    let ndc_x = (rotated_x / f64::from(output.width)) * 2.0 - 1.0;
    let ndc_y = 1.0 - (rotated_y / f64::from(output.height)) * 2.0;
    [ndc_x as f32, ndc_y as f32, u, v, opacity]
}

const TEXTURED_QUAD_SHADER: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = uv;
    out.opacity = opacity;
    return out;
}

@group(0) @binding(0) var layer_texture: texture_2d<f32>;
@group(0) @binding(1) var layer_sampler: sampler;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(layer_texture, layer_sampler, in.uv);
    return vec4<f32>(color.rgb, color.a * in.opacity);
}
"#;

fn canvas_clear_color(graph: &RenderGraph) -> Result<wgpu::Color, RealtimePreviewCompositorError> {
    let color = match graph.canvas.background.mode {
        RenderCanvasBackgroundMode::Black => [0, 0, 0, 255],
        RenderCanvasBackgroundMode::SolidColor => parse_hex_color(
            graph
                .canvas
                .background
                .color
                .as_deref()
                .unwrap_or("#000000"),
        )?,
        RenderCanvasBackgroundMode::BlurFill | RenderCanvasBackgroundMode::Image => [0, 0, 0, 255],
    };

    Ok(wgpu::Color {
        r: srgb_byte_to_linear(color[0]),
        g: srgb_byte_to_linear(color[1]),
        b: srgb_byte_to_linear(color[2]),
        a: f64::from(color[3]) / 255.0,
    })
}

fn srgb_byte_to_linear(value: u8) -> f64 {
    let srgb = f64::from(value) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

fn poll_wgpu(
    device: &wgpu::Device,
    submission_index: Option<wgpu::SubmissionIndex>,
) -> Result<(), RealtimePreviewCompositorError> {
    device
        .poll(wgpu::PollType::Wait {
            submission_index,
            timeout: Some(Duration::from_secs(5)),
        })
        .map(|_| ())
        .map_err(|error| RealtimePreviewCompositorError::WgpuPoll(error.to_string()))
}

fn acquire_surface_texture(
    surface: &wgpu::Surface<'static>,
) -> Result<wgpu::SurfaceTexture, RealtimePreviewCompositorError> {
    match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(texture)
        | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => Ok(texture),
        wgpu::CurrentSurfaceTexture::Timeout => Err(
            RealtimePreviewCompositorError::WgpuSurfaceAcquire("surface acquire timed out".into()),
        ),
        wgpu::CurrentSurfaceTexture::Occluded => Err(
            RealtimePreviewCompositorError::WgpuSurfaceAcquire("surface is occluded".into()),
        ),
        wgpu::CurrentSurfaceTexture::Outdated => Err(
            RealtimePreviewCompositorError::WgpuSurfaceAcquire("surface is outdated".into()),
        ),
        wgpu::CurrentSurfaceTexture::Lost => Err(
            RealtimePreviewCompositorError::WgpuSurfaceAcquire("surface is lost".into()),
        ),
        wgpu::CurrentSurfaceTexture::Validation => Err(
            RealtimePreviewCompositorError::WgpuSurfaceAcquire("surface validation failed".into()),
        ),
    }
}

fn align_to(value: usize, alignment: usize) -> usize {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

fn parse_hex_color(color: &str) -> Result<[u8; 4], RealtimePreviewCompositorError> {
    let trimmed = color.strip_prefix('#').unwrap_or(color);
    if trimmed.len() != 6 {
        return Err(RealtimePreviewCompositorError::InvalidCanvasColor(
            color.to_owned(),
        ));
    }
    let red = u8::from_str_radix(&trimmed[0..2], 16)
        .map_err(|_| RealtimePreviewCompositorError::InvalidCanvasColor(color.to_owned()))?;
    let green = u8::from_str_radix(&trimmed[2..4], 16)
        .map_err(|_| RealtimePreviewCompositorError::InvalidCanvasColor(color.to_owned()))?;
    let blue = u8::from_str_radix(&trimmed[4..6], 16)
        .map_err(|_| RealtimePreviewCompositorError::InvalidCanvasColor(color.to_owned()))?;
    Ok([red, green, blue, 255])
}

fn material_for<'a>(
    graph: &'a RenderGraph,
    material_id: &MaterialId,
) -> Option<&'a RenderMaterial> {
    graph
        .materials
        .iter()
        .find(|material| &material.material_id == material_id)
}

fn sampled_visual_for<'a>(
    graph: &'a RenderGraph,
    layer: &RenderVideoLayer,
) -> Option<&'a SegmentVisual> {
    graph
        .sampled_animation_states
        .first()
        .and_then(|sample| {
            sample.visual_layers.iter().find(|sampled_layer| {
                sampled_layer.track_id == layer.track_id
                    && sampled_layer.segment_id == layer.segment_id
                    && sampled_layer.material_id == layer.material_id
            })
        })
        .map(|sampled_layer| &sampled_layer.visual)
}

fn draw_textured_quad(
    pixels: &mut [u8],
    target: &RealtimePreviewGpuTarget,
    material: &RenderMaterial,
    layer: &RenderVideoLayer,
    visual: &SegmentVisual,
    texture: &RealtimePreviewTexture,
) -> Result<(), RealtimePreviewCompositorError> {
    let texture_pixels = texture.cpu_pixels().ok_or_else(|| {
        RealtimePreviewCompositorError::ExternalTextureRequiresGpuCompositor {
            handle_id: texture
                .external_handle()
                .map(|handle| handle.handle_id.clone())
                .unwrap_or_else(|| "unknown".to_owned()),
            backend: texture
                .external_handle()
                .map(|handle| handle.backend.clone())
                .unwrap_or_else(|| "unknown".to_owned()),
        }
    })?;
    let source = Dimensions {
        width: material.width.unwrap_or(texture.width).max(1),
        height: material.height.unwrap_or(texture.height).max(1),
    };
    let output = Dimensions {
        width: target.width(),
        height: target.height(),
    };
    let fitted = fit_dimensions(source, output, visual.fit_mode);
    let scaled = Dimensions {
        width: millis_of(fitted.width, visual.transform.scale.x_millis).max(1),
        height: millis_of(fitted.height, visual.transform.scale.y_millis).max(1),
    };
    let placement = layer_placement(visual, output, scaled);
    let opacity = visual.transform.opacity.value_millis.min(1_000);

    for dest_y in 0..scaled.height {
        let canvas_y = placement.y + i64::from(dest_y);
        if canvas_y < 0 || canvas_y >= i64::from(target.height()) {
            continue;
        }
        for dest_x in 0..scaled.width {
            let canvas_x = placement.x + i64::from(dest_x);
            if canvas_x < 0 || canvas_x >= i64::from(target.width()) {
                continue;
            }
            let source_x = ((u64::from(dest_x) * u64::from(texture.width))
                / u64::from(scaled.width))
            .min(u64::from(texture.width.saturating_sub(1))) as u32;
            let source_y = ((u64::from(dest_y) * u64::from(texture.height))
                / u64::from(scaled.height))
            .min(u64::from(texture.height.saturating_sub(1))) as u32;
            let source_index = ((source_y * texture.width + source_x) * 4) as usize;
            let dest_index = ((canvas_y as u32 * target.width() + canvas_x as u32) * 4) as usize;
            blend_pixel(
                &mut pixels[dest_index..dest_index + 4],
                &texture_pixels[source_index..source_index + 4],
                opacity,
            );
        }
    }

    let _ = layer;
    Ok(())
}

fn blend_pixel(dest: &mut [u8], source: &[u8], opacity_millis: u32) {
    let source_alpha = ((u32::from(source[3]) * opacity_millis) + 500) / 1_000;
    let inverse_alpha = 255_u32.saturating_sub(source_alpha);
    let dest_alpha = u32::from(dest[3]);

    for channel in 0..3 {
        dest[channel] = (((u32::from(source[channel]) * source_alpha)
            + (u32::from(dest[channel]) * inverse_alpha)
            + 127)
            / 255) as u8;
    }
    dest[3] = (source_alpha + ((dest_alpha * inverse_alpha + 127) / 255)).min(255) as u8;
}

#[derive(Debug, Clone, Copy)]
struct Dimensions {
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy)]
struct Placement {
    x: i64,
    y: i64,
}

fn fit_dimensions(source: Dimensions, output: Dimensions, fit_mode: SegmentFitMode) -> Dimensions {
    match fit_mode {
        SegmentFitMode::Stretch => output,
        SegmentFitMode::Fit => {
            if u64::from(output.width) * u64::from(source.height)
                <= u64::from(output.height) * u64::from(source.width)
            {
                Dimensions {
                    width: output.width,
                    height: proportional_dimension(source.height, output.width, source.width),
                }
            } else {
                Dimensions {
                    width: proportional_dimension(source.width, output.height, source.height),
                    height: output.height,
                }
            }
        }
        SegmentFitMode::Fill => {
            if u64::from(output.width) * u64::from(source.height)
                >= u64::from(output.height) * u64::from(source.width)
            {
                Dimensions {
                    width: output.width,
                    height: proportional_dimension(source.height, output.width, source.width),
                }
            } else {
                Dimensions {
                    width: proportional_dimension(source.width, output.height, source.height),
                    height: output.height,
                }
            }
        }
    }
}

fn proportional_dimension(source_span: u32, target_span: u32, reference_span: u32) -> u32 {
    round_div_u64(
        u64::from(source_span) * u64::from(target_span),
        u64::from(reference_span.max(1)),
    )
    .max(1)
    .min(u64::from(u32::MAX)) as u32
}

fn layer_placement(visual: &SegmentVisual, output: Dimensions, layer: Dimensions) -> Placement {
    let center_x = normalized_millis_to_canvas_pixel(output.width, visual.transform.position.x);
    let center_y = normalized_millis_to_canvas_pixel(output.height, -visual.transform.position.y);
    Placement {
        x: center_x - i64::from(millis_of(layer.width, visual.transform.anchor.x_millis)),
        y: center_y - i64::from(millis_of(layer.height, visual.transform.anchor.y_millis)),
    }
}

fn normalized_millis_to_canvas_pixel(span: u32, value_millis: i32) -> i64 {
    (i64::from(span) * i64::from(1_000 + value_millis)) / 2_000
}

fn millis_of(span: u32, millis: u32) -> u32 {
    round_div_u64(u64::from(span) * u64::from(millis), 1_000).min(u64::from(u32::MAX)) as u32
}

fn round_div_u64(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    (numerator + denominator / 2) / denominator
}

fn summarize_support(
    initial: RealtimePreviewGraphSupport,
    diagnostics: &[RealtimePreviewDiagnostic],
) -> RealtimePreviewGraphSupport {
    if diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.support,
            RealtimePreviewSupport::Unsupported { .. }
        )
    }) {
        RealtimePreviewGraphSupport::Unsupported
    } else if initial == RealtimePreviewGraphSupport::Degraded
        || diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.support, RealtimePreviewSupport::Degraded { .. }))
    {
        RealtimePreviewGraphSupport::Degraded
    } else {
        RealtimePreviewGraphSupport::Supported
    }
}

fn layer_diagnostic(
    layer: &RenderVideoLayer,
    reason: impl Into<String>,
) -> RealtimePreviewDiagnostic {
    let reason = reason.into();
    RealtimePreviewDiagnostic::new(
        Some(layer.segment_id.as_str().to_owned()),
        RealtimePreviewDiagnosticDomain::MaterialFrame,
        RealtimePreviewSupport::Unsupported {
            reason: reason.clone(),
        },
        reason,
        None,
        true,
    )
}

fn text_diagnostic(
    text: &RenderTextOverlay,
    error: TextRasterizationError,
) -> RealtimePreviewDiagnostic {
    let reason = error.to_string();
    RealtimePreviewDiagnostic::new(
        Some(text.overlay.segment_id.as_str().to_owned()),
        RealtimePreviewDiagnosticDomain::Text,
        RealtimePreviewSupport::Unsupported {
            reason: reason.clone(),
        },
        reason,
        None,
        true,
    )
}

fn texture_cache_diagnostic(
    layer: &RenderVideoLayer,
    error: RealtimePreviewTextureCacheError,
) -> RealtimePreviewDiagnostic {
    layer_diagnostic(layer, error.to_string())
}
