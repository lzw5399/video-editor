use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

use draft_model::{MaterialId, SegmentFitMode, SegmentVisual};
use media_runtime::{ColorMatrix, ColorRange, ColorTransfer, VideoColorMetadata};
use render_graph::{
    RenderCanvasBackgroundMode, RenderGraph, RenderMaterial, RenderTextOverlay, RenderVideoLayer,
};

use crate::effects::{
    EffectPreviewUniforms, apply_phase19_mask_blend, preview_effect_uniforms_for_layer,
};
use crate::{
    PlaybackGeneration, PreviewFrameInput, PreviewFrameProvider,
    RealtimePreviewCapabilityClassifier, RealtimePreviewDiagnostic,
    RealtimePreviewDiagnosticDomain, RealtimePreviewGraphSupport, RealtimePreviewSupport,
    RealtimePreviewUiChrome,
};

use super::{
    RealtimePreviewExternalTexturePlanes, RealtimePreviewGpuDevice,
    RealtimePreviewGpuPresentationTarget, RealtimePreviewGpuTarget, RealtimePreviewPipelineSet,
    RealtimePreviewTexture, RealtimePreviewTextureCache, RealtimePreviewTextureCacheError,
};

use super::text::{TextRasterizationError, TextRasterizationTarget, rasterize_text_overlay};
use super::texture_cache::RealtimePreviewCachedTextLayer;

pub struct RealtimePreviewCompositor {
    device: RealtimePreviewGpuDevice,
    classifier: RealtimePreviewCapabilityClassifier,
    pipelines: RealtimePreviewPipelineSet,
    wgpu_pipelines: BTreeMap<super::RealtimePreviewTargetFormat, RealtimePreviewWgpuPipelines>,
}

impl fmt::Debug for RealtimePreviewCompositor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RealtimePreviewCompositor")
            .field("device", &self.device)
            .field("classifier", &self.classifier)
            .field("pipelines", &self.pipelines)
            .field("cached_wgpu_pipeline_count", &self.wgpu_pipelines.len())
            .finish()
    }
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
            wgpu_pipelines: BTreeMap::new(),
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
                let gpu_device = self.device.clone();
                let device_ref = gpu_device
                    .device()
                    .ok_or(RealtimePreviewCompositorError::WgpuDeviceUnavailable)?;
                let queue_ref = gpu_device
                    .queue()
                    .ok_or(RealtimePreviewCompositorError::WgpuQueueUnavailable)?;
                let pipeline_resources = self.wgpu_pipeline_resources_for_graph(
                    device_ref,
                    queue_ref,
                    graph,
                    target.format(),
                )?;
                let (pixels, submitted_draws) = render_wgpu_graph(
                    graph,
                    target,
                    &gpu_device,
                    texture,
                    frame_provider,
                    texture_cache,
                    &mut diagnostics,
                    &mut support,
                    pipeline_resources,
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

            let source_position = sampled_source_position(graph, layer);
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

    pub fn poll_surface_submissions(&self) -> Result<(), RealtimePreviewCompositorError> {
        let gpu_device = self.device.clone();
        let device_ref = gpu_device
            .device()
            .ok_or(RealtimePreviewCompositorError::WgpuDeviceUnavailable)?;
        poll_wgpu_nonblocking(device_ref)
    }

    pub fn wait_for_surface_submission(
        &self,
        fence: &RealtimePreviewSurfaceSubmissionFence,
        timeout: Duration,
    ) -> Result<(), RealtimePreviewCompositorError> {
        if fence.is_complete() {
            return Ok(());
        }
        let gpu_device = self.device.clone();
        let device_ref = gpu_device
            .device()
            .ok_or(RealtimePreviewCompositorError::WgpuDeviceUnavailable)?;
        poll_wgpu_wait(device_ref, Some(fence.submission_index.clone()), timeout)?;
        fence.mark_complete();
        Ok(())
    }

    fn wgpu_pipeline_resources_for_graph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        graph: &RenderGraph,
        format: super::RealtimePreviewTargetFormat,
    ) -> Result<Option<&RealtimePreviewWgpuPipelines>, RealtimePreviewCompositorError> {
        if graph.video_layers.is_empty() && graph.text_overlays.is_empty() {
            return Ok(None);
        }
        if !self.wgpu_pipelines.contains_key(&format) {
            let resources = RealtimePreviewWgpuPipelines::new(device, queue, format)?;
            self.wgpu_pipelines.insert(format, resources);
        }
        Ok(self.wgpu_pipelines.get(&format))
    }

    pub fn present_to_surface(
        &mut self,
        graph: &RenderGraph,
        target: &mut RealtimePreviewGpuPresentationTarget,
        frame_provider: &mut impl PreviewFrameProvider,
        texture_cache: &mut RealtimePreviewTextureCache,
    ) -> Result<RealtimePreviewSurfacePresentationOutput, RealtimePreviewCompositorError> {
        self.present_to_surface_with_generation(
            graph,
            target,
            frame_provider,
            texture_cache,
            PlaybackGeneration::initial(),
            &RealtimePreviewUiChrome::default(),
        )
    }

    pub fn present_to_surface_with_generation(
        &mut self,
        graph: &RenderGraph,
        target: &mut RealtimePreviewGpuPresentationTarget,
        frame_provider: &mut impl PreviewFrameProvider,
        texture_cache: &mut RealtimePreviewTextureCache,
        playback_generation: PlaybackGeneration,
        ui_chrome: &RealtimePreviewUiChrome,
    ) -> Result<RealtimePreviewSurfacePresentationOutput, RealtimePreviewCompositorError> {
        let gpu_device = self.device.clone();
        let device_ref = gpu_device
            .device()
            .ok_or(RealtimePreviewCompositorError::WgpuDeviceUnavailable)?;
        let queue = gpu_device
            .queue()
            .ok_or(RealtimePreviewCompositorError::WgpuQueueUnavailable)?;
        let mut diagnostics = Vec::new();
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
                submission_fence: None,
                render_backend: RealtimePreviewCompositorBackend::WgpuSurfacePresent,
                support,
                diagnostics,
            });
        }

        target.prepare_for_present().map_err(|error| {
            RealtimePreviewCompositorError::WgpuSurfaceAcquire(error.to_string())
        })?;
        let surface_texture = acquire_surface_texture(target)?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let pipeline_resources =
            self.wgpu_pipeline_resources_for_graph(device_ref, queue, graph, target.format())?;
        let (encoder, submitted_draws) = encode_wgpu_graph_to_view(
            graph,
            target,
            device_ref,
            queue,
            &view,
            frame_provider,
            texture_cache,
            &mut diagnostics,
            &mut support,
            playback_generation,
            pipeline_resources,
            ui_chrome,
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
                submission_fence: None,
                render_backend: RealtimePreviewCompositorBackend::WgpuSurfacePresent,
                support,
                diagnostics,
            });
        }

        let submission = queue.submit([encoder.finish()]);
        let submission_fence = RealtimePreviewSurfaceSubmissionFence::submitted(queue, submission);
        surface_texture.present();

        Ok(RealtimePreviewSurfacePresentationOutput {
            width: target.width(),
            height: target.height(),
            pixels: None,
            submitted_draws,
            presented_frames: 1,
            submission_fence: Some(submission_fence),
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

#[derive(Debug, Clone)]
pub struct RealtimePreviewSurfacePresentationOutput {
    pub width: u32,
    pub height: u32,
    pub pixels: Option<Vec<u8>>,
    pub submitted_draws: u32,
    pub presented_frames: u32,
    pub submission_fence: Option<RealtimePreviewSurfaceSubmissionFence>,
    pub render_backend: RealtimePreviewCompositorBackend,
    pub support: RealtimePreviewGraphSupport,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
}

#[derive(Clone)]
pub struct RealtimePreviewSurfaceSubmissionFence {
    submission_index: wgpu::SubmissionIndex,
    completed: Arc<AtomicBool>,
}

impl fmt::Debug for RealtimePreviewSurfaceSubmissionFence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RealtimePreviewSurfaceSubmissionFence")
            .field("submission_index", &self.submission_index)
            .field("completed", &self.is_complete())
            .finish()
    }
}

impl RealtimePreviewSurfaceSubmissionFence {
    fn submitted(queue: &wgpu::Queue, submission_index: wgpu::SubmissionIndex) -> Self {
        let completed = Arc::new(AtomicBool::new(false));
        let callback_completed = Arc::clone(&completed);
        queue.on_submitted_work_done(move || {
            callback_completed.store(true, Ordering::Release);
        });
        Self {
            submission_index,
            completed,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }

    fn mark_complete(&self) {
        self.completed.store(true, Ordering::Release);
    }
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
    texture_cache: &mut RealtimePreviewTextureCache,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
    pipeline_resources: Option<&RealtimePreviewWgpuPipelines>,
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
        texture_cache,
        diagnostics,
        support,
        PlaybackGeneration::initial(),
        pipeline_resources,
        &RealtimePreviewUiChrome::default(),
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
}

impl WgpuRenderTargetInfo for RealtimePreviewGpuTarget {
    fn width(&self) -> u32 {
        self.width()
    }

    fn height(&self) -> u32 {
        self.height()
    }
}

impl WgpuRenderTargetInfo for RealtimePreviewGpuPresentationTarget {
    fn width(&self) -> u32 {
        self.width()
    }

    fn height(&self) -> u32 {
        self.height()
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
    texture_cache: &mut RealtimePreviewTextureCache,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
    playback_generation: PlaybackGeneration,
    pipeline_resources: Option<&RealtimePreviewWgpuPipelines>,
    ui_chrome: &RealtimePreviewUiChrome,
) -> Result<(wgpu::CommandEncoder, u32), RealtimePreviewCompositorError> {
    let clear_color = canvas_clear_color(graph)?;
    let layer_draws = if let Some(resources) = pipeline_resources {
        prepare_wgpu_layer_draws(
            graph,
            target,
            device,
            queue,
            resources,
            frame_provider,
            texture_cache,
            diagnostics,
            support,
            playback_generation,
            ui_chrome,
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
            for draw in &layer_draws {
                pass.set_pipeline(resources.pipeline(draw.pipeline_kind));
                pass.set_bind_group(0, &draw.bind_group, &[]);
                pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
                pass.draw(0..6, 0..1);
            }
        }
    }
    Ok((encoder, submitted_draws))
}

struct RealtimePreviewWgpuPipelines {
    texture_pipeline: wgpu::RenderPipeline,
    texture_multiply_pipeline: wgpu::RenderPipeline,
    texture_screen_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    external_pipeline: Option<wgpu::RenderPipeline>,
    external_multiply_pipeline: Option<wgpu::RenderPipeline>,
    external_screen_pipeline: Option<wgpu::RenderPipeline>,
    external_bind_group_layout: Option<wgpu::BindGroupLayout>,
    linear_sampler: wgpu::Sampler,
    text_sampler: wgpu::Sampler,
    ui_chrome_border_texture: Rc<wgpu::Texture>,
    ui_chrome_handle_texture: Rc<wgpu::Texture>,
}

impl RealtimePreviewWgpuPipelines {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: super::RealtimePreviewTargetFormat,
    ) -> Result<Self, RealtimePreviewCompositorError> {
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
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
        let texture_pipeline = create_wgpu_layer_pipeline(
            device,
            &pipeline_layout,
            &shader,
            &vertex_attributes,
            format,
            WgpuBlendMode::Normal,
            "realtime-preview-textured-quad-pipeline",
        );
        let texture_multiply_pipeline = create_wgpu_layer_pipeline(
            device,
            &pipeline_layout,
            &shader,
            &vertex_attributes,
            format,
            WgpuBlendMode::Multiply,
            "realtime-preview-textured-quad-multiply-pipeline",
        );
        let texture_screen_pipeline = create_wgpu_layer_pipeline(
            device,
            &pipeline_layout,
            &shader,
            &vertex_attributes,
            format,
            WgpuBlendMode::Screen,
            "realtime-preview-textured-quad-screen-pipeline",
        );
        let linear_sampler = device.create_sampler(&linear_layer_sampler_descriptor());
        let text_sampler = device.create_sampler(&text_layer_sampler_descriptor());
        let ui_chrome_border_texture = Rc::new(create_wgpu_rgba_texture(
            device,
            queue,
            1,
            1,
            4,
            &[32, 199, 217, 255],
            "realtime-preview-selection-border-texture",
        )?);
        let ui_chrome_handle_texture = Rc::new(create_wgpu_rgba_texture(
            device,
            queue,
            1,
            1,
            4,
            &[247, 251, 252, 255],
            "realtime-preview-selection-handle-texture",
        )?);
        let (
            external_pipeline,
            external_multiply_pipeline,
            external_screen_pipeline,
            external_bind_group_layout,
        ) = if device.features().contains(wgpu::Features::EXTERNAL_TEXTURE) {
            let external_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("realtime-preview-external-texture-bind-group-layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::ExternalTexture,
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
            let external_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("realtime-preview-external-texture-pipeline-layout"),
                    bind_group_layouts: &[Some(&external_bind_group_layout)],
                    immediate_size: 0,
                });
            let external_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("realtime-preview-external-texture-shader"),
                source: wgpu::ShaderSource::Wgsl(EXTERNAL_TEXTURE_QUAD_SHADER.into()),
            });
            let external_pipeline = create_wgpu_layer_pipeline(
                device,
                &external_pipeline_layout,
                &external_shader,
                &vertex_attributes,
                format,
                WgpuBlendMode::Normal,
                "realtime-preview-external-texture-pipeline",
            );
            let external_multiply_pipeline = create_wgpu_layer_pipeline(
                device,
                &external_pipeline_layout,
                &external_shader,
                &vertex_attributes,
                format,
                WgpuBlendMode::Multiply,
                "realtime-preview-external-texture-multiply-pipeline",
            );
            let external_screen_pipeline = create_wgpu_layer_pipeline(
                device,
                &external_pipeline_layout,
                &external_shader,
                &vertex_attributes,
                format,
                WgpuBlendMode::Screen,
                "realtime-preview-external-texture-screen-pipeline",
            );
            (
                Some(external_pipeline),
                Some(external_multiply_pipeline),
                Some(external_screen_pipeline),
                Some(external_bind_group_layout),
            )
        } else {
            (None, None, None, None)
        };

        Ok(Self {
            texture_pipeline,
            texture_multiply_pipeline,
            texture_screen_pipeline,
            texture_bind_group_layout: bind_group_layout,
            external_pipeline,
            external_multiply_pipeline,
            external_screen_pipeline,
            external_bind_group_layout,
            linear_sampler,
            text_sampler,
            ui_chrome_border_texture,
            ui_chrome_handle_texture,
        })
    }

    fn pipeline(&self, kind: WgpuLayerPipelineKind) -> &wgpu::RenderPipeline {
        match kind {
            WgpuLayerPipelineKind::Texture(blend) => self.texture_pipeline(blend),
            WgpuLayerPipelineKind::ExternalTexture(blend) => self.external_pipeline(blend),
        }
    }

    fn texture_pipeline(&self, blend: WgpuBlendMode) -> &wgpu::RenderPipeline {
        match blend {
            WgpuBlendMode::Normal => &self.texture_pipeline,
            WgpuBlendMode::Multiply => &self.texture_multiply_pipeline,
            WgpuBlendMode::Screen => &self.texture_screen_pipeline,
        }
    }

    fn external_pipeline(&self, blend: WgpuBlendMode) -> &wgpu::RenderPipeline {
        match blend {
            WgpuBlendMode::Normal => self.external_pipeline.as_ref(),
            WgpuBlendMode::Multiply => self.external_multiply_pipeline.as_ref(),
            WgpuBlendMode::Screen => self.external_screen_pipeline.as_ref(),
        }
        .expect("external texture draw should be rejected before render pass")
    }
}

fn create_wgpu_layer_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    vertex_attributes: &[wgpu::VertexAttribute],
    format: super::RealtimePreviewTargetFormat,
    blend_mode: WgpuBlendMode,
    label: &'static str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 20,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: vertex_attributes,
            }],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: format.wgpu_format(),
                blend: Some(blend_state_for(blend_mode)),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn blend_state_for(blend_mode: WgpuBlendMode) -> wgpu::BlendState {
    match blend_mode {
        WgpuBlendMode::Normal => wgpu::BlendState::ALPHA_BLENDING,
        WgpuBlendMode::Multiply => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        },
        WgpuBlendMode::Screen => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrc,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        },
    }
}

fn linear_layer_sampler_descriptor() -> wgpu::SamplerDescriptor<'static> {
    wgpu::SamplerDescriptor {
        label: Some("realtime-preview-linear-layer-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    }
}

fn text_layer_sampler_descriptor() -> wgpu::SamplerDescriptor<'static> {
    wgpu::SamplerDescriptor {
        label: Some("realtime-preview-text-layer-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    }
}

struct WgpuLayerDraw {
    _resources: WgpuLayerResources,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    pipeline_kind: WgpuLayerPipelineKind,
}

enum WgpuLayerTexture {
    Owned(wgpu::Texture),
    Imported(Rc<wgpu::Texture>),
    ExternalNv12 {
        planes: Rc<RealtimePreviewExternalTexturePlanes>,
        color: VideoColorMetadata,
    },
}

impl WgpuLayerTexture {
    fn texture(&self) -> &wgpu::Texture {
        match self {
            Self::Owned(texture) => texture,
            Self::Imported(texture) => texture.as_ref(),
            Self::ExternalNv12 { .. } => {
                panic!("external texture planes cannot be used as a sampled rgba texture")
            }
        }
    }
}

enum WgpuLayerResources {
    Texture {
        _texture: WgpuLayerTexture,
        _view: wgpu::TextureView,
        _effect_uniform_buffer: wgpu::Buffer,
    },
    ExternalTexture {
        _planes: Rc<RealtimePreviewExternalTexturePlanes>,
        _views: [wgpu::TextureView; 2],
        _external_texture: wgpu::ExternalTexture,
        _effect_uniform_buffer: wgpu::Buffer,
    },
}

#[derive(Clone, Copy)]
enum WgpuLayerPipelineKind {
    Texture(WgpuBlendMode),
    ExternalTexture(WgpuBlendMode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WgpuBlendMode {
    Normal,
    Multiply,
    Screen,
}

impl WgpuBlendMode {
    fn from_mask_blend_pass(pass: &crate::effects::MaskBlendPreviewPass) -> Self {
        match &pass.blend.blend_mode {
            draft_model::SegmentBlendMode::Multiply => Self::Multiply,
            draft_model::SegmentBlendMode::Screen => Self::Screen,
            draft_model::SegmentBlendMode::Normal
            | draft_model::SegmentBlendMode::ExternalReference { .. } => Self::Normal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WgpuLayerSamplerKind {
    Linear,
    Text,
}

fn text_layer_sampler_kind(visual: &SegmentVisual, raster_scale: u32) -> WgpuLayerSamplerKind {
    if raster_scale == 1
        && visual.transform.rotation.degrees == 0
        && visual.transform.scale.x_millis == 1_000
        && visual.transform.scale.y_millis == 1_000
    {
        WgpuLayerSamplerKind::Text
    } else {
        WgpuLayerSamplerKind::Linear
    }
}

fn prepare_wgpu_layer_draws(
    graph: &RenderGraph,
    target: &impl WgpuRenderTargetInfo,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &RealtimePreviewWgpuPipelines,
    frame_provider: &mut impl PreviewFrameProvider,
    texture_cache: &mut RealtimePreviewTextureCache,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
    playback_generation: PlaybackGeneration,
    ui_chrome: &RealtimePreviewUiChrome,
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
                    texture_cache,
                    diagnostics,
                    support,
                    &mut draws,
                    layer,
                    playback_generation,
                )?;
            }
            GraphDrawLayer::Text(text) => {
                push_wgpu_text_layer_draw(
                    graph,
                    target,
                    device,
                    queue,
                    resources,
                    texture_cache,
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

    push_wgpu_ui_chrome_draws(
        graph, target, device, queue, resources, &mut draws, ui_chrome,
    )?;

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
    texture_cache: &mut RealtimePreviewTextureCache,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
    draws: &mut Vec<WgpuLayerDraw>,
    layer: &RenderVideoLayer,
    playback_generation: PlaybackGeneration,
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
        sampled_source_position(graph, layer),
        playback_generation,
    ) {
        Ok(input) => input,
        Err(error) => {
            diagnostics.push(layer_diagnostic(layer, error.to_string()));
            *support = RealtimePreviewGraphSupport::Unsupported;
            return Ok(());
        }
    };
    let texture = match upload_wgpu_layer_texture(device, queue, texture_cache, frame) {
        Ok(texture) => texture,
        Err(error) => {
            diagnostics.push(layer_diagnostic(layer, error.to_string()));
            *support = RealtimePreviewGraphSupport::Unsupported;
            return Ok(());
        }
    };
    let visual = sampled_visual_for(graph, layer).unwrap_or(&layer.visual);
    let effect_uniforms = preview_effect_uniforms_for_layer(
        layer,
        material.width.unwrap_or(1),
        material.height.unwrap_or(1),
    );
    let blend_mode = WgpuBlendMode::from_mask_blend_pass(&apply_phase19_mask_blend(layer));
    push_wgpu_layer_draw(
        device,
        queue,
        resources,
        draws,
        texture,
        WgpuLayerSamplerKind::Linear,
        blend_mode,
        effect_uniforms,
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
    texture_cache: &mut RealtimePreviewTextureCache,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
    support: &mut RealtimePreviewGraphSupport,
    draws: &mut Vec<WgpuLayerDraw>,
    text: &RenderTextOverlay,
) -> Result<(), RealtimePreviewCompositorError> {
    let sampled = sampled_text_for(graph, text);
    let raster_scale = text_raster_scale(&text.visual);
    let target_rect = graph_canvas_rect_to_target(
        graph.canvas.width,
        graph.canvas.height,
        target.width(),
        target.height(),
        i64::from(text.overlay.layout_region.x),
        i64::from(text.overlay.layout_region.y),
        text.overlay.layout_width,
        text.overlay.layout_height,
    );
    let raster_target = TextRasterizationTarget {
        x: target_rect.x,
        y: target_rect.y,
        width: target_rect.width,
        height: target_rect.height,
        canvas_to_target_scale: canvas_to_target_scale(
            graph.canvas.width,
            graph.canvas.height,
            target.width(),
            target.height(),
        ),
    };
    let cache_key = text_texture_cache_key(text, sampled, raster_target, raster_scale);
    let cached = if let Some(cached) = texture_cache.cached_text_layer(&cache_key) {
        cached
    } else {
        let rasterized = match rasterize_text_overlay(text, sampled, raster_target, raster_scale) {
            Ok(layer) => layer,
            Err(error) => {
                diagnostics.push(text_diagnostic(text, error));
                *support = RealtimePreviewGraphSupport::Unsupported;
                return Ok(());
            }
        };
        let texture = Rc::new(create_wgpu_rgba_texture(
            device,
            queue,
            rasterized.width,
            rasterized.height,
            rasterized.stride_bytes,
            &rasterized.pixels,
            "realtime-preview-text-layer-texture",
        )?);
        texture_cache.insert_text_layer(
            cache_key,
            RealtimePreviewCachedTextLayer {
                width: rasterized.logical_width,
                height: rasterized.logical_height,
                x: rasterized.x,
                y: rasterized.y,
                texture,
            },
        )
    };
    push_wgpu_layer_draw(
        device,
        queue,
        resources,
        draws,
        WgpuLayerTexture::Imported(Rc::clone(&cached.texture)),
        text_layer_sampler_kind(&text.visual, raster_scale),
        WgpuBlendMode::Normal,
        EffectPreviewUniforms::identity(cached.width, cached.height),
        textured_text_rect_vertices(
            target,
            TargetRect {
                x: u32::try_from(cached.x).unwrap_or(u32::MAX),
                y: u32::try_from(cached.y).unwrap_or(u32::MAX),
                width: cached.width,
                height: cached.height,
            },
            &text.visual,
        ),
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn push_wgpu_ui_chrome_draws(
    graph: &RenderGraph,
    target: &impl WgpuRenderTargetInfo,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &RealtimePreviewWgpuPipelines,
    draws: &mut Vec<WgpuLayerDraw>,
    ui_chrome: &RealtimePreviewUiChrome,
) -> Result<(), RealtimePreviewCompositorError> {
    let Some(selected) = ui_chrome.selected_segment.as_ref() else {
        return Ok(());
    };
    let Some(text) = graph.text_overlays.iter().find(|text| {
        text.overlay.track_id.as_str() == selected.track_id
            && text.overlay.segment_id.as_str() == selected.segment_id
    }) else {
        return Ok(());
    };

    let target_rect = graph_canvas_rect_to_target(
        graph.canvas.width,
        graph.canvas.height,
        target.width(),
        target.height(),
        i64::from(text.overlay.layout_region.x),
        i64::from(text.overlay.layout_region.y),
        text.overlay.layout_width,
        text.overlay.layout_height,
    );
    let geometry = text_visual_geometry(target, target_rect, &text.visual);
    let border_texture = &resources.ui_chrome_border_texture;
    let handle_texture = &resources.ui_chrome_handle_texture;

    let thickness = 2.0;
    let handle_size = 10.0;
    let rotate_handle_size = 13.0;
    let rotate_gap = 24.0;
    let left = geometry.left - 1.0;
    let top = geometry.top - 1.0;
    let right = geometry.right + 1.0;
    let bottom = geometry.bottom + 1.0;

    for (rect, texture) in [
        (
            (left, top - thickness, right, top + thickness),
            Rc::clone(&border_texture),
        ),
        (
            (left, bottom - thickness, right, bottom + thickness),
            Rc::clone(&border_texture),
        ),
        (
            (left - thickness, top, left + thickness, bottom),
            Rc::clone(&border_texture),
        ),
        (
            (right - thickness, top, right + thickness, bottom),
            Rc::clone(&border_texture),
        ),
        (
            corner_rect(left, top, handle_size),
            Rc::clone(&handle_texture),
        ),
        (
            corner_rect(right, top, handle_size),
            Rc::clone(&handle_texture),
        ),
        (
            corner_rect(left, bottom, handle_size),
            Rc::clone(&handle_texture),
        ),
        (
            corner_rect(right, bottom, handle_size),
            Rc::clone(&handle_texture),
        ),
        (
            corner_rect(right + rotate_gap, top - rotate_gap, rotate_handle_size),
            Rc::clone(&handle_texture),
        ),
    ] {
        push_wgpu_layer_draw(
            device,
            queue,
            resources,
            draws,
            WgpuLayerTexture::Imported(texture),
            WgpuLayerSamplerKind::Text,
            WgpuBlendMode::Normal,
            EffectPreviewUniforms::identity(1, 1),
            solid_rect_vertices_from_geometry(geometry, rect, 1.0),
        );
    }

    Ok(())
}

fn text_texture_cache_key(
    text: &RenderTextOverlay,
    sampled: Option<&render_graph::graph::RenderSampledTextOverlay>,
    target: TextRasterizationTarget,
    raster_scale: u32,
) -> String {
    let font_size = sampled
        .map(|sample| sample.font_size)
        .unwrap_or(text.overlay.font_size);
    let color = sampled
        .map(|sample| sample.color.as_str())
        .unwrap_or(text.overlay.style.color.as_str());
    let line_height_millis = sampled
        .map(|sample| sample.line_height_millis)
        .unwrap_or(text.overlay.line_height_millis);
    let letter_spacing_millis = sampled
        .map(|sample| sample.letter_spacing_millis)
        .unwrap_or(text.overlay.letter_spacing_millis);
    let alignment = match text.overlay.alignment {
        draft_model::TextAlignment::Left => "left",
        draft_model::TextAlignment::Center => "center",
        draft_model::TextAlignment::Right => "right",
    };
    format!(
        "target={}x{}+{}+{};target_scale={:.6};raster_scale={raster_scale};track={};segment={};material={};content={:?};font_ref={:?};font_size={font_size};color={:?};line_height={line_height_millis};letter_spacing={letter_spacing_millis};alignment={alignment};text_box={}x{};layout={}x{}+{}+{};style={:?}",
        target.width,
        target.height,
        target.x,
        target.y,
        target.canvas_to_target_scale,
        text.overlay.track_id.as_str(),
        text.overlay.segment_id.as_str(),
        text.material_id.as_str(),
        text.overlay.content,
        text.overlay.font_ref,
        color,
        text.overlay.text_box.width,
        text.overlay.text_box.height,
        text.overlay.layout_width,
        text.overlay.layout_height,
        text.overlay.layout_region.x,
        text.overlay.layout_region.y,
        text.overlay.style,
    )
}

fn text_raster_scale(visual: &SegmentVisual) -> u32 {
    let max_scale = visual
        .transform
        .scale
        .x_millis
        .max(visual.transform.scale.y_millis);
    if max_scale > 1_750 {
        4
    } else if max_scale > 1_150 || visual.transform.rotation.degrees != 0 {
        2
    } else {
        1
    }
}

fn create_wgpu_rgba_texture(
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

fn push_wgpu_layer_draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resources: &RealtimePreviewWgpuPipelines,
    draws: &mut Vec<WgpuLayerDraw>,
    texture: WgpuLayerTexture,
    sampler_kind: WgpuLayerSamplerKind,
    blend_mode: WgpuBlendMode,
    effect_uniforms: EffectPreviewUniforms,
    vertices: Vec<u8>,
) {
    let effect_uniform_bytes = effect_uniforms.as_wgpu_bytes();
    let effect_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("realtime-preview-effect-uniforms"),
        size: effect_uniform_bytes.len() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&effect_uniform_buffer, 0, &effect_uniform_bytes);
    let (layer_resources, bind_group, pipeline_kind) = match texture {
        WgpuLayerTexture::ExternalNv12 { planes, color } => {
            let Some(layout) = resources.external_bind_group_layout.as_ref() else {
                return;
            };
            let views = planes.create_plane_views();
            let external_texture = device.create_external_texture(
                &wgpu::ExternalTextureDescriptor {
                    label: Some("realtime-preview-nv12-external-texture"),
                    width: planes.width,
                    height: planes.height,
                    format: wgpu::ExternalTextureFormat::Nv12,
                    yuv_conversion_matrix: nv12_yuv_conversion_matrix(&color),
                    gamut_conversion_matrix: IDENTITY_3X3,
                    src_transfer_function: nv12_src_transfer_function(&color),
                    dst_transfer_function: wgpu::ExternalTextureTransferFunction::default(),
                    sample_transform: IDENTITY_3X2,
                    load_transform: IDENTITY_3X2,
                },
                &[&views[0], &views[1]],
            );
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("realtime-preview-external-texture-bind-group"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::ExternalTexture(&external_texture),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&resources.linear_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: effect_uniform_buffer.as_entire_binding(),
                    },
                ],
            });
            (
                WgpuLayerResources::ExternalTexture {
                    _planes: planes,
                    _views: views,
                    _external_texture: external_texture,
                    _effect_uniform_buffer: effect_uniform_buffer,
                },
                bind_group,
                WgpuLayerPipelineKind::ExternalTexture(blend_mode),
            )
        }
        texture => {
            let view = texture
                .texture()
                .create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("realtime-preview-textured-quad-bind-group"),
                layout: &resources.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(match sampler_kind {
                            WgpuLayerSamplerKind::Linear => &resources.linear_sampler,
                            WgpuLayerSamplerKind::Text => &resources.text_sampler,
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: effect_uniform_buffer.as_entire_binding(),
                    },
                ],
            });
            (
                WgpuLayerResources::Texture {
                    _texture: texture,
                    _view: view,
                    _effect_uniform_buffer: effect_uniform_buffer,
                },
                bind_group,
                WgpuLayerPipelineKind::Texture(blend_mode),
            )
        }
    };
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("realtime-preview-textured-quad-vertices"),
        size: vertices.len() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, &vertices);
    draws.push(WgpuLayerDraw {
        _resources: layer_resources,
        bind_group,
        vertex_buffer,
        pipeline_kind,
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

fn sampled_source_position(
    graph: &RenderGraph,
    layer: &RenderVideoLayer,
) -> draft_model::Microseconds {
    let target_offset = graph
        .target_timerange
        .start
        .get()
        .saturating_sub(layer.target_timerange.start.get());
    let clamped_offset = target_offset.min(layer.source_timerange.duration.get().saturating_sub(1));
    draft_model::Microseconds::new(
        layer
            .source_timerange
            .start
            .get()
            .saturating_add(clamped_offset),
    )
}

fn upload_wgpu_layer_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_cache: &mut RealtimePreviewTextureCache,
    input: PreviewFrameInput,
) -> Result<WgpuLayerTexture, RealtimePreviewCompositorError> {
    let frame = match input {
        PreviewFrameInput::CpuRgba(frame) | PreviewFrameInput::StaticImage(frame) => frame,
        PreviewFrameInput::TextureHandle(handle) => {
            let lease = texture_cache
                .resolve_native_texture(&handle)
                .map_err(|error| {
                    RealtimePreviewCompositorError::WgpuFrameUpload(error.to_string())
                })?;
            if handle.pixel_format == "nv12" {
                if !device.features().contains(wgpu::Features::EXTERNAL_TEXTURE) {
                    return Err(
                        RealtimePreviewCompositorError::WgpuLayerTextureHandleUnsupported {
                            handle_id: handle.handle_id,
                            backend: format!("{}:external-texture-unavailable", handle.backend),
                        },
                    );
                }
                if let Some(planes) = lease.resource_as::<RealtimePreviewExternalTexturePlanes>() {
                    return Ok(WgpuLayerTexture::ExternalNv12 {
                        planes,
                        color: handle.color,
                    });
                }
                if let Some(planes) = texture_cache
                    .import_native_nv12_external_texture(device, &handle, &lease)
                    .map_err(|error| {
                        RealtimePreviewCompositorError::WgpuFrameUpload(error.to_string())
                    })?
                {
                    return Ok(WgpuLayerTexture::ExternalNv12 {
                        planes,
                        color: handle.color,
                    });
                }
                return Err(
                    RealtimePreviewCompositorError::WgpuLayerTextureHandleUnsupported {
                        handle_id: handle.handle_id,
                        backend: format!("{}:{}", handle.backend, handle.pixel_format),
                    },
                );
            }
            let Some(texture) = lease.resource_as::<wgpu::Texture>() else {
                return Err(
                    RealtimePreviewCompositorError::WgpuLayerTextureHandleUnsupported {
                        handle_id: handle.handle_id,
                        backend: handle.backend,
                    },
                );
            };
            if !matches!(handle.pixel_format.as_str(), "rgba8" | "bgra8") {
                return Err(
                    RealtimePreviewCompositorError::WgpuLayerTextureHandleUnsupported {
                        handle_id: handle.handle_id,
                        backend: format!("{}:{}", handle.backend, handle.pixel_format),
                    },
                );
            }
            return Ok(WgpuLayerTexture::Imported(texture));
        }
        PreviewFrameInput::Unavailable { reason } => {
            return Err(RealtimePreviewCompositorError::WgpuFrameUpload(reason));
        }
    };
    frame
        .validate()
        .map_err(|error| RealtimePreviewCompositorError::WgpuFrameUpload(error.to_string()))?;
    let texture = create_wgpu_rgba_texture(
        device,
        queue,
        frame.width,
        frame.height,
        frame.stride_bytes,
        &frame.pixels,
        "realtime-preview-layer-texture",
    )?;
    Ok(WgpuLayerTexture::Owned(texture))
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

fn textured_text_rect_vertices(
    target: &impl WgpuRenderTargetInfo,
    rect: TargetRect,
    visual: &SegmentVisual,
) -> Vec<u8> {
    let geometry = text_visual_geometry(target, rect, visual);
    textured_vertices_for_geometry(
        geometry,
        (geometry.left, geometry.top, geometry.right, geometry.bottom),
        0.0,
        0.0,
        1.0,
        1.0,
        geometry.opacity,
    )
}

#[derive(Clone, Copy)]
struct TextVisualGeometry {
    output: Dimensions,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    pivot_x: f64,
    pivot_y: f64,
    sin: f64,
    cos: f64,
    opacity: f32,
}

fn text_visual_geometry(
    target: &impl WgpuRenderTargetInfo,
    rect: TargetRect,
    visual: &SegmentVisual,
) -> TextVisualGeometry {
    let output = Dimensions {
        width: target.width(),
        height: target.height(),
    };
    let scaled = Dimensions {
        width: millis_of(rect.width, visual.transform.scale.x_millis).max(1),
        height: millis_of(rect.height, visual.transform.scale.y_millis).max(1),
    };
    let base_left = i64::from(rect.x).saturating_add(visual_offset_to_target(
        output.width,
        visual.transform.position.x,
    ));
    let base_top = i64::from(rect.y).saturating_sub(visual_offset_to_target(
        output.height,
        visual.transform.position.y,
    ));
    let base_anchor_x = i64::from(millis_of(rect.width, visual.transform.anchor.x_millis));
    let base_anchor_y = i64::from(millis_of(rect.height, visual.transform.anchor.y_millis));
    let scaled_anchor_x = i64::from(millis_of(scaled.width, visual.transform.anchor.x_millis));
    let scaled_anchor_y = i64::from(millis_of(scaled.height, visual.transform.anchor.y_millis));
    let left = base_left
        .saturating_add(base_anchor_x)
        .saturating_sub(scaled_anchor_x) as f64;
    let top = base_top
        .saturating_add(base_anchor_y)
        .saturating_sub(scaled_anchor_y) as f64;
    let right = left + f64::from(scaled.width);
    let bottom = top + f64::from(scaled.height);
    let pivot_x = base_left.saturating_add(base_anchor_x) as f64;
    let pivot_y = base_top.saturating_add(base_anchor_y) as f64;
    let radians = f64::from(visual.transform.rotation.degrees).to_radians();
    let sin = radians.sin();
    let cos = radians.cos();
    let opacity = visual.transform.opacity.value_millis.min(1_000) as f32 / 1_000.0;
    TextVisualGeometry {
        output,
        left,
        top,
        right,
        bottom,
        pivot_x,
        pivot_y,
        sin,
        cos,
        opacity,
    }
}

fn textured_vertices_for_geometry(
    geometry: TextVisualGeometry,
    rect: (f64, f64, f64, f64),
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    opacity: f32,
) -> Vec<u8> {
    let (left, top, right, bottom) = rect;
    textured_vertices_from_corners(
        geometry.output,
        [
            vertex_corner(
                left,
                top,
                geometry.pivot_x,
                geometry.pivot_y,
                geometry.sin,
                geometry.cos,
                geometry.output,
                u0,
                v0,
                opacity,
            ),
            vertex_corner(
                right,
                top,
                geometry.pivot_x,
                geometry.pivot_y,
                geometry.sin,
                geometry.cos,
                geometry.output,
                u1,
                v0,
                opacity,
            ),
            vertex_corner(
                right,
                bottom,
                geometry.pivot_x,
                geometry.pivot_y,
                geometry.sin,
                geometry.cos,
                geometry.output,
                u1,
                v1,
                opacity,
            ),
            vertex_corner(
                left,
                bottom,
                geometry.pivot_x,
                geometry.pivot_y,
                geometry.sin,
                geometry.cos,
                geometry.output,
                u0,
                v1,
                opacity,
            ),
        ],
    )
}

fn solid_rect_vertices_from_geometry(
    geometry: TextVisualGeometry,
    rect: (f64, f64, f64, f64),
    opacity: f32,
) -> Vec<u8> {
    textured_vertices_for_geometry(geometry, rect, 0.5, 0.5, 0.5, 0.5, opacity)
}

fn corner_rect(center_x: f64, center_y: f64, size: f64) -> (f64, f64, f64, f64) {
    let radius = size / 2.0;
    (
        center_x - radius,
        center_y - radius,
        center_x + radius,
        center_y + radius,
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
struct EffectUniforms {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    opacity: f32,
    blur_radius_px: f32,
    texel_width: f32,
    texel_height: f32,
    active: f32,
    mask_kind: f32,
    mask_x: f32,
    mask_y: f32,
    mask_width: f32,
    mask_height: f32,
    mask_feather: f32,
    mask_opacity: f32,
    mask_inverted: f32,
    blend_mode: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
};
@group(0) @binding(2) var<uniform> effects: EffectUniforms;

fn adjust_effect_color(color: vec4<f32>, vertex_opacity: f32) -> vec4<f32> {
    let gray = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let saturated = vec3<f32>(gray) + (color.rgb - vec3<f32>(gray)) * effects.saturation;
    let contrasted = (saturated - vec3<f32>(0.5)) * effects.contrast + vec3<f32>(0.5);
    let brightened = clamp(contrasted + vec3<f32>(effects.brightness), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(brightened, color.a * vertex_opacity * effects.opacity);
}

fn sample_effect_texture(uv: vec2<f32>) -> vec4<f32> {
    let radius = effects.blur_radius_px;
    if (radius <= 0.001) {
        return textureSample(layer_texture, layer_sampler, uv);
    }
    let step = vec2<f32>(effects.texel_width, effects.texel_height) * radius;
    let center = textureSample(layer_texture, layer_sampler, uv) * 0.40;
    let horizontal = (
        textureSample(layer_texture, layer_sampler, uv + vec2<f32>(step.x, 0.0)) +
        textureSample(layer_texture, layer_sampler, uv - vec2<f32>(step.x, 0.0))
    ) * 0.15;
    let vertical = (
        textureSample(layer_texture, layer_sampler, uv + vec2<f32>(0.0, step.y)) +
        textureSample(layer_texture, layer_sampler, uv - vec2<f32>(0.0, step.y))
    ) * 0.15;
    return center + horizontal + vertical;
}

fn mask_alpha(uv: vec2<f32>) -> f32 {
    if (effects.mask_kind < 0.5) {
        return 1.0;
    }

    let mask_min = vec2<f32>(effects.mask_x, effects.mask_y);
    let mask_size = vec2<f32>(max(effects.mask_width, 0.001), max(effects.mask_height, 0.001));
    let mask_max = mask_min + mask_size;
    var alpha = 0.0;

    if (effects.mask_kind < 1.5) {
        if (uv.x >= mask_min.x && uv.y >= mask_min.y && uv.x <= mask_max.x && uv.y <= mask_max.y) {
            let edge = min(min(uv.x - mask_min.x, mask_max.x - uv.x), min(uv.y - mask_min.y, mask_max.y - uv.y));
            if (effects.mask_feather > 0.0001) {
                alpha = smoothstep(0.0, effects.mask_feather, edge);
            } else {
                alpha = 1.0;
            }
        }
    } else {
        let center = mask_min + mask_size * 0.5;
        let radius = max(mask_size * 0.5, vec2<f32>(0.001, 0.001));
        let distance = length((uv - center) / radius);
        if (effects.mask_feather > 0.0001) {
            alpha = 1.0 - smoothstep(max(0.0, 1.0 - effects.mask_feather), 1.0, distance);
        } else if (distance <= 1.0) {
            alpha = 1.0;
        }
    }

    if (effects.mask_inverted > 0.5) {
        alpha = 1.0 - alpha;
    }
    return clamp(alpha * effects.mask_opacity, 0.0, 1.0);
}

fn apply_mask(color: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    let alpha = mask_alpha(uv);
    return vec4<f32>(color.rgb, color.a * alpha);
}

fn prepare_blend_color(color: vec4<f32>) -> vec4<f32> {
    if (effects.blend_mode > 0.5) {
        return vec4<f32>(color.rgb * color.a, color.a);
    }
    return color;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return prepare_blend_color(apply_mask(adjust_effect_color(sample_effect_texture(in.uv), in.opacity), in.uv));
}
"#;

const EXTERNAL_TEXTURE_QUAD_SHADER: &str = r#"
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

@group(0) @binding(0) var layer_texture: texture_external;
@group(0) @binding(1) var layer_sampler: sampler;
struct EffectUniforms {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    opacity: f32,
    blur_radius_px: f32,
    texel_width: f32,
    texel_height: f32,
    active: f32,
    mask_kind: f32,
    mask_x: f32,
    mask_y: f32,
    mask_width: f32,
    mask_height: f32,
    mask_feather: f32,
    mask_opacity: f32,
    mask_inverted: f32,
    blend_mode: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
};
@group(0) @binding(2) var<uniform> effects: EffectUniforms;

fn adjust_effect_color(color: vec4<f32>, vertex_opacity: f32) -> vec4<f32> {
    let gray = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let saturated = vec3<f32>(gray) + (color.rgb - vec3<f32>(gray)) * effects.saturation;
    let contrasted = (saturated - vec3<f32>(0.5)) * effects.contrast + vec3<f32>(0.5);
    let brightened = clamp(contrasted + vec3<f32>(effects.brightness), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(brightened, color.a * vertex_opacity * effects.opacity);
}

fn sample_effect_texture(uv: vec2<f32>) -> vec4<f32> {
    let radius = effects.blur_radius_px;
    if (radius <= 0.001) {
        return textureSampleBaseClampToEdge(layer_texture, layer_sampler, uv);
    }
    let step = vec2<f32>(effects.texel_width, effects.texel_height) * radius;
    let center = textureSampleBaseClampToEdge(layer_texture, layer_sampler, uv) * 0.40;
    let horizontal = (
        textureSampleBaseClampToEdge(layer_texture, layer_sampler, uv + vec2<f32>(step.x, 0.0)) +
        textureSampleBaseClampToEdge(layer_texture, layer_sampler, uv - vec2<f32>(step.x, 0.0))
    ) * 0.15;
    let vertical = (
        textureSampleBaseClampToEdge(layer_texture, layer_sampler, uv + vec2<f32>(0.0, step.y)) +
        textureSampleBaseClampToEdge(layer_texture, layer_sampler, uv - vec2<f32>(0.0, step.y))
    ) * 0.15;
    return center + horizontal + vertical;
}

fn mask_alpha(uv: vec2<f32>) -> f32 {
    if (effects.mask_kind < 0.5) {
        return 1.0;
    }

    let mask_min = vec2<f32>(effects.mask_x, effects.mask_y);
    let mask_size = vec2<f32>(max(effects.mask_width, 0.001), max(effects.mask_height, 0.001));
    let mask_max = mask_min + mask_size;
    var alpha = 0.0;

    if (effects.mask_kind < 1.5) {
        if (uv.x >= mask_min.x && uv.y >= mask_min.y && uv.x <= mask_max.x && uv.y <= mask_max.y) {
            let edge = min(min(uv.x - mask_min.x, mask_max.x - uv.x), min(uv.y - mask_min.y, mask_max.y - uv.y));
            if (effects.mask_feather > 0.0001) {
                alpha = smoothstep(0.0, effects.mask_feather, edge);
            } else {
                alpha = 1.0;
            }
        }
    } else {
        let center = mask_min + mask_size * 0.5;
        let radius = max(mask_size * 0.5, vec2<f32>(0.001, 0.001));
        let distance = length((uv - center) / radius);
        if (effects.mask_feather > 0.0001) {
            alpha = 1.0 - smoothstep(max(0.0, 1.0 - effects.mask_feather), 1.0, distance);
        } else if (distance <= 1.0) {
            alpha = 1.0;
        }
    }

    if (effects.mask_inverted > 0.5) {
        alpha = 1.0 - alpha;
    }
    return clamp(alpha * effects.mask_opacity, 0.0, 1.0);
}

fn apply_mask(color: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    let alpha = mask_alpha(uv);
    return vec4<f32>(color.rgb, color.a * alpha);
}

fn prepare_blend_color(color: vec4<f32>) -> vec4<f32> {
    if (effects.blend_mode > 0.5) {
        return vec4<f32>(color.rgb * color.a, color.a);
    }
    return color;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let color = if (effects.blur_radius_px <= 0.001) {
        textureSampleBaseClampToEdge(layer_texture, layer_sampler, in.uv)
    } else {
        sample_effect_texture(in.uv)
    };
    return prepare_blend_color(apply_mask(adjust_effect_color(color, in.opacity), in.uv));
}
"#;

const IDENTITY_3X2: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
const IDENTITY_3X3: [f32; 9] = [
    1.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, //
    0.0, 0.0, 1.0,
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Nv12Range {
    Limited,
    Full,
}

fn nv12_yuv_conversion_matrix(color: &VideoColorMetadata) -> [f32; 16] {
    let (kr, kb) = match color.matrix {
        ColorMatrix::Bt2020NonConstant => (0.2627, 0.0593),
        ColorMatrix::Bt709 | ColorMatrix::Identity | ColorMatrix::Unknown => (0.2126, 0.0722),
    };
    let range = match color.range {
        ColorRange::Full => Nv12Range::Full,
        ColorRange::Limited | ColorRange::Unknown => Nv12Range::Limited,
    };
    yuv_to_rgba_matrix(kr, kb, range)
}

fn nv12_src_transfer_function(color: &VideoColorMetadata) -> wgpu::ExternalTextureTransferFunction {
    match color.transfer {
        ColorTransfer::Srgb => srgb_transfer_function(),
        ColorTransfer::Bt709 | ColorTransfer::Unknown | ColorTransfer::Pq | ColorTransfer::Hlg => {
            bt709_transfer_function()
        }
    }
}

fn bt709_transfer_function() -> wgpu::ExternalTextureTransferFunction {
    wgpu::ExternalTextureTransferFunction {
        a: 1.099,
        b: 0.018,
        g: 1.0 / 0.45,
        k: 4.5,
    }
}

fn srgb_transfer_function() -> wgpu::ExternalTextureTransferFunction {
    wgpu::ExternalTextureTransferFunction {
        a: 1.055,
        b: 0.003_130_8,
        g: 2.4,
        k: 12.92,
    }
}

fn yuv_to_rgba_matrix(kr: f32, kb: f32, range: Nv12Range) -> [f32; 16] {
    let kg = 1.0 - kr - kb;
    let y_scale = match range {
        Nv12Range::Limited => 255.0 / 219.0,
        Nv12Range::Full => 1.0,
    };
    let chroma_scale = match range {
        Nv12Range::Limited => 255.0 / 224.0,
        Nv12Range::Full => 1.0,
    };
    let y_offset = match range {
        Nv12Range::Limited => 16.0 / 255.0,
        Nv12Range::Full => 0.0,
    };
    let chroma_offset = 128.0 / 255.0;

    let r_v = (2.0 - 2.0 * kr) * chroma_scale;
    let b_u = (2.0 - 2.0 * kb) * chroma_scale;
    let g_u = -(kb * (2.0 - 2.0 * kb) / kg) * chroma_scale;
    let g_v = -(kr * (2.0 - 2.0 * kr) / kg) * chroma_scale;
    let y = y_scale;

    [
        y,
        y,
        y,
        0.0,
        0.0,
        g_u,
        b_u,
        0.0,
        r_v,
        g_v,
        0.0,
        0.0,
        -(y * y_offset) - (r_v * chroma_offset),
        -(y * y_offset) - (g_u * chroma_offset) - (g_v * chroma_offset),
        -(y * y_offset) - (b_u * chroma_offset),
        1.0,
    ]
}

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
    poll_wgpu_wait(device, submission_index, Duration::from_secs(5))
}

fn poll_wgpu_wait(
    device: &wgpu::Device,
    submission_index: Option<wgpu::SubmissionIndex>,
    timeout: Duration,
) -> Result<(), RealtimePreviewCompositorError> {
    device
        .poll(wgpu::PollType::Wait {
            submission_index,
            timeout: Some(timeout),
        })
        .map(|_| ())
        .map_err(|error| RealtimePreviewCompositorError::WgpuPoll(error.to_string()))
}

fn poll_wgpu_nonblocking(device: &wgpu::Device) -> Result<(), RealtimePreviewCompositorError> {
    device
        .poll(wgpu::PollType::Poll)
        .map(|_| ())
        .map_err(|error| RealtimePreviewCompositorError::WgpuPoll(error.to_string()))
}

fn acquire_surface_texture(
    target: &RealtimePreviewGpuPresentationTarget,
) -> Result<wgpu::SurfaceTexture, RealtimePreviewCompositorError> {
    let mut last_transient = None;
    for _ in 0..8 {
        match target.surface().get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => return Ok(texture),
            wgpu::CurrentSurfaceTexture::Timeout => {
                last_transient = Some("surface acquire timed out");
                std::thread::sleep(Duration::from_millis(16));
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                last_transient = Some("surface is occluded");
                std::thread::sleep(Duration::from_millis(16));
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                return Err(RealtimePreviewCompositorError::WgpuSurfaceAcquire(
                    "surface is outdated".into(),
                ));
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                return Err(RealtimePreviewCompositorError::WgpuSurfaceAcquire(
                    "surface is lost".into(),
                ));
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RealtimePreviewCompositorError::WgpuSurfaceAcquire(
                    "surface validation failed".into(),
                ));
            }
        }
    }
    let reason = last_transient.unwrap_or("surface acquire failed");
    let details = target
        .drawable_lifecycle_diagnostic()
        .map(|diagnostic| format!("{reason}; {diagnostic}"))
        .unwrap_or_else(|| reason.to_owned());
    Err(RealtimePreviewCompositorError::WgpuSurfaceAcquire(details))
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

#[derive(Debug, Clone, Copy)]
struct TargetRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn graph_canvas_rect_to_target(
    canvas_width: u32,
    canvas_height: u32,
    target_width: u32,
    target_height: u32,
    x: i64,
    y: i64,
    width: u32,
    height: u32,
) -> TargetRect {
    let canvas_width = canvas_width.max(1);
    let canvas_height = canvas_height.max(1);
    let output = Dimensions {
        width: target_width.max(1),
        height: target_height.max(1),
    };
    let fitted = fit_dimensions(
        Dimensions {
            width: canvas_width,
            height: canvas_height,
        },
        output,
        SegmentFitMode::Fit,
    );
    let offset_x = (output.width.saturating_sub(fitted.width)) / 2;
    let offset_y = (output.height.saturating_sub(fitted.height)) / 2;
    TargetRect {
        x: offset_x.saturating_add(scale_canvas_i64(x, fitted.width, canvas_width)),
        y: offset_y.saturating_add(scale_canvas_i64(y, fitted.height, canvas_height)),
        width: scale_canvas_u32(width, fitted.width, canvas_width).max(1),
        height: scale_canvas_u32(height, fitted.height, canvas_height).max(1),
    }
}

fn canvas_to_target_scale(
    canvas_width: u32,
    canvas_height: u32,
    target_width: u32,
    target_height: u32,
) -> f32 {
    let canvas_width = canvas_width.max(1);
    let canvas_height = canvas_height.max(1);
    let fitted = fit_dimensions(
        Dimensions {
            width: canvas_width,
            height: canvas_height,
        },
        Dimensions {
            width: target_width.max(1),
            height: target_height.max(1),
        },
        SegmentFitMode::Fit,
    );
    let width_scale = fitted.width as f32 / canvas_width as f32;
    let height_scale = fitted.height as f32 / canvas_height as f32;
    width_scale.min(height_scale).max(0.001)
}

fn scale_canvas_u32(span: u32, target_span: u32, canvas_span: u32) -> u32 {
    round_div_u64(
        u64::from(span) * u64::from(target_span),
        u64::from(canvas_span.max(1)),
    )
    .max(1)
    .min(u64::from(u32::MAX)) as u32
}

fn scale_canvas_i64(position: i64, target_span: u32, canvas_span: u32) -> u32 {
    if position <= 0 {
        return 0;
    }
    round_div_u64(
        u64::try_from(position).unwrap_or(u64::MAX) * u64::from(target_span),
        u64::from(canvas_span.max(1)),
    )
    .min(u64::from(u32::MAX)) as u32
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

fn visual_offset_to_target(span: u32, value_millis: i32) -> i64 {
    (i64::from(span) * i64::from(value_millis)) / 2_000
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

#[cfg(test)]
mod tests {
    use media_runtime::{ColorPrimaries, ColorTransfer};

    use super::*;

    fn color(matrix: ColorMatrix, range: ColorRange) -> VideoColorMetadata {
        VideoColorMetadata {
            primaries: ColorPrimaries::Bt709,
            transfer: ColorTransfer::Bt709,
            matrix,
            range,
            diagnostics: Vec::new(),
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.000_5,
            "expected {actual} to be close to {expected}"
        );
    }

    #[test]
    fn nv12_color_metadata_preserves_bt709_limited_range_contract() {
        let matrix = nv12_yuv_conversion_matrix(&color(ColorMatrix::Bt709, ColorRange::Limited));

        assert_close(matrix[0], 1.164_383);
        assert_close(matrix[5], -0.213_249);
        assert_close(matrix[6], 2.112_402);
        assert_close(matrix[8], 1.792_741);
        assert_close(matrix[9], -0.532_909);
        assert_close(matrix[12], -0.972_945);
        assert_close(matrix[13], 0.301_483);
        assert_close(matrix[14], -1.133_402);
    }

    #[test]
    fn nv12_color_metadata_does_not_treat_full_range_as_limited() {
        let limited = nv12_yuv_conversion_matrix(&color(ColorMatrix::Bt709, ColorRange::Limited));
        let full = nv12_yuv_conversion_matrix(&color(ColorMatrix::Bt709, ColorRange::Full));

        assert_close(full[0], 1.0);
        assert_close(full[6], 1.855_6);
        assert_close(full[8], 1.574_8);
        assert!(
            (limited[0] - full[0]).abs() > 0.1,
            "full-range luma scale must not reuse limited-range expansion"
        );
        assert!(
            (limited[12] - full[12]).abs() > 0.15,
            "full-range chroma offsets must not reuse limited-range offsets"
        );
    }

    #[test]
    fn nv12_color_metadata_defaults_unknown_to_bt709_limited_sdr() {
        let unknown = nv12_yuv_conversion_matrix(&VideoColorMetadata::unknown_with_diagnostic(
            "container omitted color metadata",
        ));
        let limited = nv12_yuv_conversion_matrix(&color(ColorMatrix::Bt709, ColorRange::Limited));

        assert_eq!(unknown, limited);
    }

    #[test]
    fn nv12_unknown_transfer_defaults_to_bt709_linear_sampling_contract() {
        let transfer = nv12_src_transfer_function(&VideoColorMetadata::unknown_with_diagnostic(
            "container omitted color metadata",
        ));

        assert_close(transfer.a, 1.099);
        assert_close(transfer.b, 0.018);
        assert_close(transfer.g, 1.0 / 0.45);
        assert_close(transfer.k, 4.5);
    }

    #[test]
    fn nv12_srgb_transfer_uses_srgb_linear_sampling_contract() {
        let transfer = nv12_src_transfer_function(&VideoColorMetadata {
            primaries: ColorPrimaries::Bt709,
            transfer: ColorTransfer::Srgb,
            matrix: ColorMatrix::Bt709,
            range: ColorRange::Limited,
            diagnostics: Vec::new(),
        });

        assert_close(transfer.a, 1.055);
        assert_close(transfer.b, 0.003_130_8);
        assert_close(transfer.g, 2.4);
        assert_close(transfer.k, 12.92);
    }

    #[test]
    fn nv12_color_metadata_uses_bt2020_non_constant_matrix_when_declared() {
        let bt709 = nv12_yuv_conversion_matrix(&color(ColorMatrix::Bt709, ColorRange::Limited));
        let bt2020 =
            nv12_yuv_conversion_matrix(&color(ColorMatrix::Bt2020NonConstant, ColorRange::Limited));

        assert!(
            (bt709[8] - bt2020[8]).abs() > 0.1,
            "BT.2020 red chroma coefficient must not reuse BT.709"
        );
        assert!(
            (bt709[6] - bt2020[6]).abs() > 0.02,
            "BT.2020 blue chroma coefficient must not reuse BT.709"
        );
    }

    #[test]
    fn text_raster_scale_keeps_untransformed_text_one_to_one() {
        let visual = SegmentVisual::default();
        assert_eq!(
            text_raster_scale(&visual),
            1,
            "CoreText text is rasterized in final target pixels; untransformed text must not be downsampled by a fixed 2x texture"
        );

        let mut scaled = SegmentVisual::default();
        scaled.transform.scale.x_millis = 1_200;
        assert_eq!(text_raster_scale(&scaled), 2);

        let mut rotated = SegmentVisual::default();
        rotated.transform.rotation.degrees = 12;
        assert_eq!(text_raster_scale(&rotated), 2);
    }

    #[test]
    fn text_sampler_uses_nearest_filtering_for_bitmap_glyph_edges() {
        let sampler = text_layer_sampler_descriptor();
        assert_eq!(
            sampler.mag_filter,
            wgpu::FilterMode::Nearest,
            "bitmap text must not share the video layer linear sampler because it softens glyph edges"
        );
        assert_eq!(
            sampler.min_filter,
            wgpu::FilterMode::Nearest,
            "bitmap text downsampling is controlled by raster scale, not by per-sample linear filtering"
        );

        let video_sampler = linear_layer_sampler_descriptor();
        assert_eq!(video_sampler.mag_filter, wgpu::FilterMode::Linear);
        assert_eq!(video_sampler.min_filter, wgpu::FilterMode::Linear);
    }

    #[test]
    fn text_sampler_kind_only_uses_nearest_for_pixel_aligned_text() {
        let visual = SegmentVisual::default();
        assert_eq!(
            text_layer_sampler_kind(&visual, 1),
            WgpuLayerSamplerKind::Text
        );

        let mut rotated = SegmentVisual::default();
        rotated.transform.rotation.degrees = 8;
        assert_eq!(
            text_layer_sampler_kind(&rotated, 2),
            WgpuLayerSamplerKind::Linear,
            "rotated text should use linear resampling over a higher-resolution raster texture"
        );

        let mut scaled = SegmentVisual::default();
        scaled.transform.scale.x_millis = 1_250;
        assert_eq!(
            text_layer_sampler_kind(&scaled, 2),
            WgpuLayerSamplerKind::Linear,
            "scaled text should avoid nearest-neighbor jaggies"
        );
    }
}
