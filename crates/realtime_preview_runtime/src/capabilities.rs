use draft_model::{
    KeyframeProperty, MaterialKind, ProductionEffectCapabilityRegistry, SegmentBlendMode,
    SegmentFitMode, SegmentMask,
};
use render_graph::{
    RenderAudioEffectSlotSupport, RenderAudioMixClassification, RenderGraph, RenderIntentSupport,
    RenderRetimeIntent, RenderVideoLayer,
};
use serde::{Deserialize, Serialize};

use crate::gpu::text::text_preview_diagnostic;
use crate::{RealtimePreviewDiagnostic, RealtimePreviewDiagnosticDomain, RealtimePreviewSupport};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewCapabilityReport {
    pub support: RealtimePreviewGraphSupport,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePreviewGraphSupport {
    Supported,
    Degraded,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewCapabilityClassifier {
    pub runtime_backend_available: bool,
    pub surface_available: bool,
    pub gpu_text_parity: bool,
    pub bundled_text_font_registry_available: bool,
}

impl RealtimePreviewCapabilityClassifier {
    pub fn supported_for_tests() -> Self {
        Self {
            runtime_backend_available: true,
            surface_available: true,
            gpu_text_parity: false,
            bundled_text_font_registry_available: true,
        }
    }

    pub fn with_runtime_backend_available(mut self, available: bool) -> Self {
        self.runtime_backend_available = available;
        self
    }

    pub fn with_surface_available(mut self, available: bool) -> Self {
        self.surface_available = available;
        self
    }

    pub fn with_gpu_text_parity(mut self, enabled: bool) -> Self {
        self.gpu_text_parity = enabled;
        self
    }

    pub fn with_bundled_text_font_registry_available(mut self, available: bool) -> Self {
        self.bundled_text_font_registry_available = available;
        self
    }

    pub fn with_supported_production_effects(self) -> Self {
        let _registry = ProductionEffectCapabilityRegistry::phase19_first_party();
        self
    }

    pub fn classify(&self, graph: &RenderGraph) -> RealtimePreviewCapabilityReport {
        let mut diagnostics = Vec::new();

        self.classify_runtime(&mut diagnostics);
        self.classify_surface(&mut diagnostics);
        classify_canvas(graph, &mut diagnostics);
        classify_material_frames(graph, &mut diagnostics);
        classify_visual_layers(graph, &mut diagnostics);
        classify_text(self, graph, &mut diagnostics);
        classify_audio(graph, &mut diagnostics);

        RealtimePreviewCapabilityReport {
            support: summarize_support(&diagnostics),
            diagnostics,
        }
    }

    fn classify_runtime(&self, diagnostics: &mut Vec<RealtimePreviewDiagnostic>) {
        if self.runtime_backend_available {
            diagnostics.push(RealtimePreviewDiagnostic::new(
                None,
                RealtimePreviewDiagnosticDomain::Runtime,
                RealtimePreviewSupport::Supported,
                "realtime preview backend is available",
                None,
                false,
            ));
        } else {
            diagnostics.push(RealtimePreviewDiagnostic::new(
                None,
                RealtimePreviewDiagnosticDomain::Runtime,
                RealtimePreviewSupport::Unsupported {
                    reason: "realtime preview backend unavailable".to_owned(),
                },
                "realtime preview backend unavailable",
                None,
                true,
            ));
        }
    }

    fn classify_surface(&self, diagnostics: &mut Vec<RealtimePreviewDiagnostic>) {
        if self.surface_available {
            diagnostics.push(RealtimePreviewDiagnostic::new(
                None,
                RealtimePreviewDiagnosticDomain::Surface,
                RealtimePreviewSupport::Supported,
                "realtime preview surface is available",
                None,
                false,
            ));
        } else {
            diagnostics.push(RealtimePreviewDiagnostic::new(
                None,
                RealtimePreviewDiagnosticDomain::Surface,
                RealtimePreviewSupport::Degraded {
                    reason:
                        "native surface unavailable; realtime preview must use offscreen fallback"
                            .to_owned(),
                },
                "native surface unavailable; realtime preview must use offscreen fallback",
                None,
                true,
            ));
        }
    }
}

fn classify_canvas(graph: &RenderGraph, diagnostics: &mut Vec<RealtimePreviewDiagnostic>) {
    diagnostics.push(RealtimePreviewDiagnostic::new(
        None,
        RealtimePreviewDiagnosticDomain::Canvas,
        support_from_render_intent(
            graph.canvas.background.support,
            &graph.canvas.background.reason,
        ),
        graph.canvas.background.reason.clone(),
        None,
        graph.canvas.background.support != RenderIntentSupport::Supported,
    ));
}

fn classify_material_frames(graph: &RenderGraph, diagnostics: &mut Vec<RealtimePreviewDiagnostic>) {
    for material in &graph.materials {
        match material.kind {
            MaterialKind::Video | MaterialKind::Image => {
                if material.has_video && material.width.is_some() && material.height.is_some() {
                    diagnostics.push(RealtimePreviewDiagnostic::new(
                        Some(material.material_id.as_str().to_owned()),
                        RealtimePreviewDiagnosticDomain::MaterialFrame,
                        RealtimePreviewSupport::Supported,
                        "material frame is available for realtime preview",
                        None,
                        false,
                    ));
                } else {
                    diagnostics.push(RealtimePreviewDiagnostic::new(
                        Some(material.material_id.as_str().to_owned()),
                        RealtimePreviewDiagnosticDomain::MaterialFrame,
                        RealtimePreviewSupport::Unsupported {
                            reason:
                                "material does not expose a video/image frame for realtime preview"
                                    .to_owned(),
                        },
                        "material does not expose a video/image frame for realtime preview",
                        None,
                        true,
                    ));
                }
            }
            MaterialKind::Audio => {
                if material.has_audio {
                    diagnostics.push(RealtimePreviewDiagnostic::new(
                        Some(material.material_id.as_str().to_owned()),
                        RealtimePreviewDiagnosticDomain::Audio,
                        RealtimePreviewSupport::Supported,
                        "audio material is available for realtime preview mix",
                        None,
                        false,
                    ));
                } else {
                    diagnostics.push(RealtimePreviewDiagnostic::new(
                        Some(material.material_id.as_str().to_owned()),
                        RealtimePreviewDiagnosticDomain::Audio,
                        RealtimePreviewSupport::Unsupported {
                            reason: "audio material does not expose an audio stream for realtime preview".to_owned(),
                        },
                        "audio material does not expose an audio stream for realtime preview",
                        None,
                        true,
                    ));
                }
            }
            MaterialKind::Text | MaterialKind::Sticker => {}
        }
    }
}

fn classify_visual_layers(graph: &RenderGraph, diagnostics: &mut Vec<RealtimePreviewDiagnostic>) {
    for layer in &graph.video_layers {
        diagnostics.push(RealtimePreviewDiagnostic::new(
            Some(layer.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::VisualLayer,
            RealtimePreviewSupport::Supported,
            "visual layer ordering is graph-defined and realtime supported",
            None,
            false,
        ));
        classify_transform(layer, diagnostics);
        classify_retime(&layer.retime, &layer.segment_id, diagnostics);
        classify_filters_and_transitions(layer, diagnostics);
        classify_keyframes(graph, layer, diagnostics);
    }
}

fn classify_retime(
    retime: &RenderRetimeIntent,
    segment_id: &draft_model::SegmentId,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
) {
    if retime.support != RenderIntentSupport::Supported {
        let fallback_used = retime.support != RenderIntentSupport::Supported;
        diagnostics.push(RealtimePreviewDiagnostic::new(
            Some(segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Effect,
            support_from_render_intent(retime.support, &retime.reason),
            retime.reason.clone(),
            None,
            fallback_used,
        ));
    }

    if retime.audio.support != RenderIntentSupport::Supported {
        diagnostics.push(RealtimePreviewDiagnostic::new(
            Some(segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Audio,
            support_from_render_intent(retime.audio.support, &retime.audio.reason),
            retime.audio.reason.clone(),
            None,
            true,
        ));
    }
}

fn classify_transform(layer: &RenderVideoLayer, diagnostics: &mut Vec<RealtimePreviewDiagnostic>) {
    let visual = &layer.visual;
    if matches!(
        visual.fit_mode,
        SegmentFitMode::Fit | SegmentFitMode::Fill | SegmentFitMode::Stretch
    ) {
        diagnostics.push(RealtimePreviewDiagnostic::new(
            Some(layer.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Transform,
            RealtimePreviewSupport::Supported,
            "position scale rotation opacity crop and fit mode are realtime supported",
            None,
            false,
        ));
    } else {
        diagnostics.push(RealtimePreviewDiagnostic::new(
            Some(layer.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Transform,
            RealtimePreviewSupport::Unsupported {
                reason: "rotation transform is unsupported in realtime preview".to_owned(),
            },
            "rotation transform is unsupported in realtime preview",
            None,
            true,
        ));
    }

    match &visual.mask {
        SegmentMask::None | SegmentMask::Rectangle { .. } | SegmentMask::Ellipse { .. } => {
            diagnostics.push(supported_production_effect_diagnostic(
                Some(layer.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::VisualLayer,
                "mask capability is registry-backed and realtime supported",
            ));
        }
        SegmentMask::ExternalReference { reference } => {
            let reason = format!(
                "segment mask external reference {}:{} is unsupported in realtime preview",
                reference.provider, reference.effect_id
            );
            diagnostics.push(RealtimePreviewDiagnostic::new(
                Some(layer.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::VisualLayer,
                RealtimePreviewSupport::Unsupported {
                    reason: reason.clone(),
                },
                reason,
                None,
                true,
            ));
        }
    }
    match &visual.blend_mode {
        SegmentBlendMode::Normal | SegmentBlendMode::Multiply | SegmentBlendMode::Screen => {
            diagnostics.push(supported_production_effect_diagnostic(
                Some(layer.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::VisualLayer,
                "blend capability is registry-backed and realtime supported",
            ));
        }
        SegmentBlendMode::ExternalReference { reference } => {
            let reason = format!(
                "segment blendMode external reference {}:{} is unsupported in realtime preview",
                reference.provider, reference.effect_id
            );
            diagnostics.push(RealtimePreviewDiagnostic::new(
                Some(layer.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::VisualLayer,
                RealtimePreviewSupport::Unsupported {
                    reason: reason.clone(),
                },
                reason,
                None,
                true,
            ));
        }
    }
}

fn classify_filters_and_transitions(
    layer: &RenderVideoLayer,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
) {
    for filter in &layer.filters {
        let fallback_used = filter.capability.preview != RenderIntentSupport::Supported;
        diagnostics.push(RealtimePreviewDiagnostic::new(
            Some(layer.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Effect,
            support_from_render_intent(
                filter.capability.preview,
                &filter.capability.preview_reason,
            ),
            filter.capability.preview_reason.clone(),
            None,
            fallback_used,
        ));
    }
    if let Some(transition) = &layer.transition {
        let fallback_used = transition.capability.preview != RenderIntentSupport::Supported;
        if fallback_used {
            diagnostics.push(RealtimePreviewDiagnostic::new(
                Some(layer.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Effect,
                support_from_render_intent(
                    transition.capability.preview,
                    &transition.capability.preview_reason,
                ),
                transition.capability.preview_reason.clone(),
                None,
                true,
            ));
        } else {
            diagnostics.push(supported_production_effect_diagnostic(
                Some(layer.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Effect,
                "transition capability is registry-backed and realtime supported",
            ));
        }
    }
}

fn supported_production_effect_diagnostic(
    entity_id: Option<String>,
    domain: RealtimePreviewDiagnosticDomain,
    reason: impl Into<String>,
) -> RealtimePreviewDiagnostic {
    RealtimePreviewDiagnostic {
        entity_id,
        domain,
        support: RealtimePreviewSupport::Supported,
        reason: reason.into(),
        fallback: None,
        fallback_used: false,
        canceled: false,
        cancellation_token: None,
    }
}

fn classify_keyframes(
    graph: &RenderGraph,
    layer: &RenderVideoLayer,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
) {
    if layer.keyframes.is_empty() {
        return;
    }

    let has_sampled_state = graph.sampled_animation_states.iter().any(|state| {
        state
            .visual_layers
            .iter()
            .any(|sample| sample.segment_id == layer.segment_id)
    });
    let unsupported_property = layer
        .keyframes
        .iter()
        .find(|keyframe| !realtime_supported_keyframe_property(&keyframe.property));

    if let Some(keyframe) = unsupported_property {
        diagnostics.push(RealtimePreviewDiagnostic::new(
            Some(layer.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Keyframe,
            RealtimePreviewSupport::Unsupported {
                reason: format!(
                    "keyframe property {:?} is unsupported in realtime preview",
                    keyframe.property
                ),
            },
            format!(
                "keyframe property {:?} is unsupported in realtime preview",
                keyframe.property
            ),
            None,
            true,
        ));
    } else if has_sampled_state {
        diagnostics.push(RealtimePreviewDiagnostic::new(
            Some(layer.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Keyframe,
            RealtimePreviewSupport::Supported,
            "sampled keyframe state is engine-resolved and realtime supported",
            None,
            false,
        ));
    }
}

fn classify_text(
    classifier: &RealtimePreviewCapabilityClassifier,
    graph: &RenderGraph,
    diagnostics: &mut Vec<RealtimePreviewDiagnostic>,
) {
    for text in &graph.text_overlays {
        diagnostics.push(text_preview_diagnostic(
            text,
            classifier.gpu_text_parity,
            classifier.bundled_text_font_registry_available,
        ));
    }
}

fn classify_audio(graph: &RenderGraph, diagnostics: &mut Vec<RealtimePreviewDiagnostic>) {
    for mix in &graph.audio_mixes {
        if mix.retime.audio.support != RenderIntentSupport::Supported {
            diagnostics.push(RealtimePreviewDiagnostic::new(
                Some(mix.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Audio,
                support_from_render_intent(mix.retime.audio.support, &mix.retime.audio.reason),
                mix.retime.audio.reason.clone(),
                None,
                true,
            ));
            continue;
        }

        let unsupported_effect = mix
            .effect_slots
            .iter()
            .find(|slot| slot.enabled && slot.support == RenderAudioEffectSlotSupport::Unsupported);
        if let Some(effect) = unsupported_effect {
            diagnostics.push(RealtimePreviewDiagnostic::new(
                Some(mix.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Audio,
                RealtimePreviewSupport::Unsupported {
                    reason: format!(
                        "audio effect {} is unsupported in realtime preview",
                        effect.name
                    ),
                },
                format!(
                    "audio effect {} is unsupported in realtime preview",
                    effect.name
                ),
                None,
                true,
            ));
            continue;
        }

        let reason = match mix.classification {
            RenderAudioMixClassification::Audible => {
                "audio mix is available for realtime preview playback"
            }
            RenderAudioMixClassification::SilentMutedTrack => {
                "audio mix is intentionally silent because the track is muted"
            }
            RenderAudioMixClassification::SilentZeroGain => {
                "audio mix is intentionally silent because gain is zero"
            }
        };
        diagnostics.push(RealtimePreviewDiagnostic::new(
            Some(mix.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Audio,
            RealtimePreviewSupport::Supported,
            reason,
            None,
            false,
        ));
    }
}

fn realtime_supported_keyframe_property(property: &KeyframeProperty) -> bool {
    matches!(
        property,
        KeyframeProperty::VisualPositionX
            | KeyframeProperty::VisualPositionY
            | KeyframeProperty::VisualScaleX
            | KeyframeProperty::VisualScaleY
            | KeyframeProperty::VisualOpacity
            | KeyframeProperty::TextFontSize
            | KeyframeProperty::TextColor
            | KeyframeProperty::TextLineHeight
            | KeyframeProperty::TextLetterSpacing
            | KeyframeProperty::TextLayoutX
            | KeyframeProperty::TextLayoutY
            | KeyframeProperty::TextLayoutWidth
            | KeyframeProperty::TextLayoutHeight
            | KeyframeProperty::Volume
    )
}

pub(crate) fn support_from_render_intent(
    support: RenderIntentSupport,
    reason: &str,
) -> RealtimePreviewSupport {
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

fn summarize_support(diagnostics: &[RealtimePreviewDiagnostic]) -> RealtimePreviewGraphSupport {
    if diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.support,
            RealtimePreviewSupport::Unsupported { .. }
        )
    }) {
        RealtimePreviewGraphSupport::Unsupported
    } else if diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.support, RealtimePreviewSupport::Degraded { .. }))
    {
        RealtimePreviewGraphSupport::Degraded
    } else {
        RealtimePreviewGraphSupport::Supported
    }
}
