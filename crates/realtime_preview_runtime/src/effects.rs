use draft_model::FilterKind;
use render_graph::{RenderFilterIntent, RenderIntentSupport, RenderVideoLayer};

use crate::RealtimePreviewSupport;

#[derive(Debug, Clone, PartialEq)]
pub struct EffectPreviewPass {
    pub order_index: u32,
    pub enabled: bool,
    pub kind: FilterKind,
    pub support: RealtimePreviewSupport,
    pub reason: String,
    pub requires_wgpu_render_pass: bool,
}

impl EffectPreviewPass {
    fn from_filter(filter: &RenderFilterIntent) -> Option<Self> {
        if !filter.enabled {
            return None;
        }
        Some(Self {
            order_index: filter.order_index,
            enabled: filter.enabled,
            kind: filter.kind.clone(),
            support: support_from_intent(
                filter.capability.preview,
                &filter.capability.preview_reason,
            ),
            reason: filter.capability.preview_reason.clone(),
            requires_wgpu_render_pass: filter.capability.preview == RenderIntentSupport::Supported,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EffectPreviewUniforms {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub opacity: f32,
    pub blur_radius_px: f32,
    pub texel_width: f32,
    pub texel_height: f32,
    pub active: f32,
}

impl EffectPreviewUniforms {
    pub(crate) fn identity(material_width: u32, material_height: u32) -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            opacity: 1.0,
            blur_radius_px: 0.0,
            texel_width: 1.0 / material_width.max(1) as f32,
            texel_height: 1.0 / material_height.max(1) as f32,
            active: 0.0,
        }
    }

    pub(crate) fn as_wgpu_bytes(&self) -> Vec<u8> {
        [
            self.brightness,
            self.contrast,
            self.saturation,
            self.opacity,
            self.blur_radius_px,
            self.texel_width,
            self.texel_height,
            self.active,
        ]
        .into_iter()
        .flat_map(f32::to_ne_bytes)
        .collect()
    }
}

pub fn apply_phase19_effects(layer: &RenderVideoLayer) -> Vec<EffectPreviewPass> {
    layer
        .filters
        .iter()
        .filter_map(EffectPreviewPass::from_filter)
        .filter(|pass| matches!(pass.support, RealtimePreviewSupport::Supported))
        .collect()
}

pub(crate) fn preview_effect_uniforms_for_layer(
    layer: &RenderVideoLayer,
    material_width: u32,
    material_height: u32,
) -> EffectPreviewUniforms {
    let passes = apply_phase19_effects(layer);
    let mut uniforms = EffectPreviewUniforms::identity(material_width, material_height);
    if passes.is_empty() {
        return uniforms;
    }

    uniforms.active = 1.0;
    for pass in passes {
        match pass.kind {
            FilterKind::GaussianBlur { radius_millis } => {
                uniforms.blur_radius_px = uniforms
                    .blur_radius_px
                    .max(radius_millis as f32 * 8.0 / 1_000.0);
            }
            FilterKind::BasicColorAdjustment {
                brightness_millis,
                contrast_millis,
                saturation_millis,
            } => {
                uniforms.brightness += brightness_millis as f32 / 1_000.0;
                uniforms.contrast *= contrast_millis as f32 / 1_000.0;
                uniforms.saturation *= saturation_millis as f32 / 1_000.0;
            }
            FilterKind::OpacityAdjustment { opacity_millis } => {
                uniforms.opacity *= opacity_millis.min(1_000) as f32 / 1_000.0;
            }
            FilterKind::ExternalReference { .. } => {}
        }
    }

    uniforms.brightness = uniforms.brightness.clamp(-1.0, 1.0);
    uniforms.contrast = uniforms.contrast.clamp(0.0, 4.0);
    uniforms.saturation = uniforms.saturation.clamp(0.0, 4.0);
    uniforms.opacity = uniforms.opacity.clamp(0.0, 1.0);
    uniforms.blur_radius_px = uniforms.blur_radius_px.clamp(0.0, 16.0);
    uniforms
}

fn support_from_intent(support: RenderIntentSupport, reason: &str) -> RealtimePreviewSupport {
    match support {
        RenderIntentSupport::Supported => RealtimePreviewSupport::Supported,
        RenderIntentSupport::Degraded => RealtimePreviewSupport::Degraded {
            reason: reason.to_owned(),
        },
        RenderIntentSupport::Unsupported => RealtimePreviewSupport::Unsupported {
            reason: reason.to_owned(),
        },
    }
}
