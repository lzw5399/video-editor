use std::error::Error;
use std::fmt;

use draft_model::{MaterialId, SegmentFitMode, SegmentVisual};
use render_graph::{RenderCanvasBackgroundMode, RenderGraph, RenderMaterial, RenderVideoLayer};

use crate::{
    PlaybackGeneration, PreviewFrameProvider, RealtimePreviewCapabilityClassifier,
    RealtimePreviewDiagnostic, RealtimePreviewDiagnosticDomain, RealtimePreviewGraphSupport,
    RealtimePreviewSupport,
};

use super::{
    RealtimePreviewGpuDevice, RealtimePreviewGpuTarget, RealtimePreviewPipelineSet,
    RealtimePreviewTexture, RealtimePreviewTextureCache, RealtimePreviewTextureCacheError,
};

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

        let mut pixels = canvas_pixels(graph, target)?;
        let mut support = summarize_support(capability.support, &diagnostics);
        if support == RealtimePreviewGraphSupport::Unsupported {
            return Ok(RealtimePreviewCompositorOutput {
                width: target.width(),
                height: target.height(),
                pixels,
                submitted_draws: 0,
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
            support,
            diagnostics,
        })
    }

    pub fn pipelines(&self) -> &RealtimePreviewPipelineSet {
        &self.pipelines
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewCompositorOutput {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub submitted_draws: u32,
    pub support: RealtimePreviewGraphSupport,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
}

#[derive(Debug)]
pub enum RealtimePreviewCompositorError {
    InvalidCanvasColor(String),
    PixelBufferOverflow,
}

impl fmt::Display for RealtimePreviewCompositorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCanvasColor(color) => write!(formatter, "invalid canvas color {color}"),
            Self::PixelBufferOverflow => formatter.write_str("compositor pixel buffer overflow"),
        }
    }
}

impl Error for RealtimePreviewCompositorError {}

fn validate_target_matches_graph(
    target: &RealtimePreviewGpuTarget,
    graph: &RenderGraph,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
) {
    if target.width() != graph.canvas.width || target.height() != graph.canvas.height {
        diagnostics.push(RealtimePreviewDiagnostic::new(
            None,
            RealtimePreviewDiagnosticDomain::Surface,
            RealtimePreviewSupport::Unsupported {
                reason: "offscreen target dimensions must match render graph canvas".to_owned(),
            },
            "offscreen target dimensions must match render graph canvas",
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
                &texture.pixels()[source_index..source_index + 4],
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

fn texture_cache_diagnostic(
    layer: &RenderVideoLayer,
    error: RealtimePreviewTextureCacheError,
) -> RealtimePreviewDiagnostic {
    layer_diagnostic(layer, error.to_string())
}
