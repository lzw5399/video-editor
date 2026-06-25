use draft_model::{FilterKind, SegmentBlendMode, SegmentMask};
use render_graph::{
    RenderBlendIntent, RenderFilterIntent, RenderIntentSupport, RenderMaskIntent, RenderVideoLayer,
};

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

#[derive(Debug, Clone, PartialEq)]
pub struct MaskBlendPreviewPass {
    pub mask: RenderMaskIntent,
    pub blend: RenderBlendIntent,
    pub support: RealtimePreviewSupport,
    pub reason: String,
    pub requires_wgpu_render_pass: bool,
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
    pub effect_active: f32,
    pub mask_kind: f32,
    pub mask_x: f32,
    pub mask_y: f32,
    pub mask_width: f32,
    pub mask_height: f32,
    pub mask_feather: f32,
    pub mask_opacity: f32,
    pub mask_inverted: f32,
    pub blend_mode: f32,
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
            effect_active: 0.0,
            mask_kind: 0.0,
            mask_x: 0.0,
            mask_y: 0.0,
            mask_width: 1.0,
            mask_height: 1.0,
            mask_feather: 0.0,
            mask_opacity: 1.0,
            mask_inverted: 0.0,
            blend_mode: 0.0,
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
            self.effect_active,
            self.mask_kind,
            self.mask_x,
            self.mask_y,
            self.mask_width,
            self.mask_height,
            self.mask_feather,
            self.mask_opacity,
            self.mask_inverted,
            self.blend_mode,
            0.0,
            0.0,
            0.0,
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

pub fn apply_phase19_mask_blend(layer: &RenderVideoLayer) -> MaskBlendPreviewPass {
    let mask_support = support_from_intent(layer.mask.capability.preview, &layer.mask.reason);
    let blend_support = support_from_intent(layer.blend.capability.preview, &layer.blend.reason);
    let support = match (&mask_support, &blend_support) {
        (RealtimePreviewSupport::Unsupported { reason }, _) => {
            RealtimePreviewSupport::Unsupported {
                reason: reason.clone(),
            }
        }
        (_, RealtimePreviewSupport::Unsupported { reason }) => {
            RealtimePreviewSupport::Unsupported {
                reason: reason.clone(),
            }
        }
        (RealtimePreviewSupport::Degraded { reason }, _) => RealtimePreviewSupport::Degraded {
            reason: reason.clone(),
        },
        (_, RealtimePreviewSupport::Degraded { reason }) => RealtimePreviewSupport::Degraded {
            reason: reason.clone(),
        },
        _ => RealtimePreviewSupport::Supported,
    };
    let requires_wgpu_render_pass = matches!(support, RealtimePreviewSupport::Supported)
        && (!matches!(layer.mask.mask, SegmentMask::None)
            || !matches!(layer.blend.blend_mode, SegmentBlendMode::Normal));
    let reason = match &support {
        RealtimePreviewSupport::Supported => format!(
            "{} and {} are applied by the WGPU native compositor",
            layer.mask.capability.capability_id, layer.blend.capability.capability_id
        ),
        RealtimePreviewSupport::Degraded { reason }
        | RealtimePreviewSupport::Unsupported { reason } => reason.clone(),
    };

    MaskBlendPreviewPass {
        mask: layer.mask.clone(),
        blend: layer.blend.clone(),
        support,
        reason,
        requires_wgpu_render_pass,
    }
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

    uniforms.effect_active = 1.0;
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
    apply_mask_blend_uniforms(layer, &mut uniforms);
    uniforms
}

fn apply_mask_blend_uniforms(layer: &RenderVideoLayer, uniforms: &mut EffectPreviewUniforms) {
    match &layer.mask.mask {
        SegmentMask::None | SegmentMask::ExternalReference { .. } => {}
        SegmentMask::Rectangle {
            x_millis,
            y_millis,
            width_millis,
            height_millis,
            feather_millis,
            opacity_millis,
            inverted,
        } => {
            uniforms.mask_kind = 1.0;
            assign_mask_uniforms(
                uniforms,
                *x_millis,
                *y_millis,
                *width_millis,
                *height_millis,
                *feather_millis,
                *opacity_millis,
                *inverted,
            );
        }
        SegmentMask::Ellipse {
            x_millis,
            y_millis,
            width_millis,
            height_millis,
            feather_millis,
            opacity_millis,
            inverted,
        } => {
            uniforms.mask_kind = 2.0;
            assign_mask_uniforms(
                uniforms,
                *x_millis,
                *y_millis,
                *width_millis,
                *height_millis,
                *feather_millis,
                *opacity_millis,
                *inverted,
            );
        }
    }

    uniforms.blend_mode = match layer.blend.blend_mode {
        SegmentBlendMode::Normal | SegmentBlendMode::ExternalReference { .. } => 0.0,
        SegmentBlendMode::Multiply => 1.0,
        SegmentBlendMode::Screen => 2.0,
    };
}

fn assign_mask_uniforms(
    uniforms: &mut EffectPreviewUniforms,
    x_millis: u32,
    y_millis: u32,
    width_millis: u32,
    height_millis: u32,
    feather_millis: u32,
    opacity_millis: u32,
    inverted: bool,
) {
    uniforms.mask_x = normalized_millis(x_millis);
    uniforms.mask_y = normalized_millis(y_millis);
    uniforms.mask_width = normalized_millis(width_millis).max(0.001);
    uniforms.mask_height = normalized_millis(height_millis).max(0.001);
    uniforms.mask_feather = normalized_millis(feather_millis);
    uniforms.mask_opacity = normalized_millis(opacity_millis);
    uniforms.mask_inverted = if inverted { 1.0 } else { 0.0 };
}

fn normalized_millis(value: u32) -> f32 {
    value.min(1_000) as f32 / 1_000.0
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
