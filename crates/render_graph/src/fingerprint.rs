use std::fmt;

use draft_model::{DraftId, RationalFrameRate, TargetTimerange};
use serde::{Deserialize, Serialize};

use crate::{
    RenderAudioMix, RenderCanvas, RenderFilterIntent, RenderGraph, RenderGraphNodeId,
    RenderMaterial, RenderOutputProfile, RenderSampledFrame, RenderTextOverlay,
    RenderTransitionIntent, RenderVideoLayer,
};

pub const GRAPH_SCHEMA_VERSION: u32 = 1;
pub const GRAPH_GENERATOR_VERSION: &str = "render-graph-generator-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraphNodeFingerprint {
    pub node_id: RenderGraphNodeId,
    pub semantic_fingerprint: String,
    pub input_fingerprint: String,
    pub output_profile_fingerprint: String,
    pub runtime_capability_fingerprint: String,
    pub graph_schema_version: u32,
    pub generator_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraphSnapshot {
    pub draft_id: DraftId,
    pub target_timerange: TargetTimerange,
    pub frame_rate: RationalFrameRate,
    pub graph_schema_version: u32,
    pub generator_version: String,
    pub output_profile_fingerprint: String,
    pub runtime_capability_fingerprint: String,
    pub node_fingerprints: Vec<RenderGraphNodeFingerprint>,
}

impl RenderGraphSnapshot {
    pub fn from_graph(
        graph: &RenderGraph,
        output_profile: &RenderOutputProfile,
        runtime_capability_fingerprint: &str,
    ) -> Self {
        let output_profile_fingerprint =
            deterministic_fingerprint("output-profile", output_profile);
        let runtime_capability_fingerprint = deterministic_fingerprint(
            "runtime-capability",
            &RuntimeFingerprintInput {
                capability_fingerprint: runtime_capability_fingerprint,
            },
        );
        let mut node_fingerprints = node_fingerprints(
            graph,
            &output_profile_fingerprint,
            &runtime_capability_fingerprint,
        );
        node_fingerprints.sort_by_key(|fingerprint| fingerprint.node_id.stable_key());

        Self {
            draft_id: graph.draft_id.clone(),
            target_timerange: graph.target_timerange.clone(),
            frame_rate: graph.frame_rate.clone(),
            graph_schema_version: GRAPH_SCHEMA_VERSION,
            generator_version: GRAPH_GENERATOR_VERSION.to_owned(),
            output_profile_fingerprint,
            runtime_capability_fingerprint,
            node_fingerprints,
        }
    }

    pub fn node_fingerprint_by_key(&self, stable_key: &str) -> Option<&RenderGraphNodeFingerprint> {
        self.node_fingerprints
            .iter()
            .find(|fingerprint| fingerprint.node_id.stable_key() == stable_key)
    }
}

fn node_fingerprints(
    graph: &RenderGraph,
    output_profile_fingerprint: &str,
    runtime_capability_fingerprint: &str,
) -> Vec<RenderGraphNodeFingerprint> {
    let mut fingerprints = Vec::new();
    fingerprints.push(canvas_fingerprint(
        &graph.canvas,
        output_profile_fingerprint,
        runtime_capability_fingerprint,
    ));
    fingerprints.extend(graph.materials.iter().map(|material| {
        material_fingerprint(
            material,
            output_profile_fingerprint,
            runtime_capability_fingerprint,
        )
    }));
    fingerprints.extend(graph.video_layers.iter().map(|layer| {
        video_layer_fingerprint(
            layer,
            output_profile_fingerprint,
            runtime_capability_fingerprint,
        )
    }));
    fingerprints.extend(graph.audio_mixes.iter().map(|mix| {
        audio_mix_fingerprint(
            mix,
            output_profile_fingerprint,
            runtime_capability_fingerprint,
        )
    }));
    fingerprints.extend(graph.text_overlays.iter().map(|overlay| {
        text_overlay_fingerprint(
            overlay,
            output_profile_fingerprint,
            runtime_capability_fingerprint,
        )
    }));
    fingerprints.extend(graph.sampled_frames.iter().map(|frame| {
        sampled_frame_fingerprint(
            frame,
            output_profile_fingerprint,
            runtime_capability_fingerprint,
        )
    }));
    fingerprints
}

fn canvas_fingerprint(
    canvas: &RenderCanvas,
    output_profile_fingerprint: &str,
    runtime_capability_fingerprint: &str,
) -> RenderGraphNodeFingerprint {
    fingerprint_parts(
        canvas.node_id.clone(),
        &CanvasSemanticInput {
            width: canvas.width,
            height: canvas.height,
            background: &canvas.background,
            diagnostics: &canvas.diagnostics,
        },
        &CanvasInputFacts {
            background_material_id: canvas.background.material_id.as_ref(),
        },
        output_profile_fingerprint,
        runtime_capability_fingerprint,
    )
}

fn material_fingerprint(
    material: &RenderMaterial,
    output_profile_fingerprint: &str,
    runtime_capability_fingerprint: &str,
) -> RenderGraphNodeFingerprint {
    fingerprint_parts(
        material.node_id.clone(),
        &MaterialSemanticInput {
            material_id: &material.material_id,
            kind: material.kind,
            display_name: &material.display_name,
        },
        &MaterialInputFacts {
            uri: &material.uri,
            duration: material.duration,
            frame_rate: material.frame_rate.as_ref(),
            width: material.width,
            height: material.height,
            has_video: material.has_video,
            has_audio: material.has_audio,
        },
        output_profile_fingerprint,
        runtime_capability_fingerprint,
    )
}

fn video_layer_fingerprint(
    layer: &RenderVideoLayer,
    output_profile_fingerprint: &str,
    runtime_capability_fingerprint: &str,
) -> RenderGraphNodeFingerprint {
    fingerprint_parts(
        layer.node_id.clone(),
        &VideoLayerSemanticInput {
            stack_index: layer.stack_index,
            source_timerange: &layer.source_timerange,
            target_timerange: &layer.target_timerange,
            keyframes: &layer.keyframes,
            filters: &layer.filters,
            transition: layer.transition.as_ref(),
            visual: &layer.visual,
        },
        &SegmentInputFacts {
            material_id: &layer.material_id,
            material_kind: layer.material_kind,
            source_timerange: &layer.source_timerange,
        },
        output_profile_fingerprint,
        runtime_capability_fingerprint,
    )
}

fn audio_mix_fingerprint(
    mix: &RenderAudioMix,
    output_profile_fingerprint: &str,
    runtime_capability_fingerprint: &str,
) -> RenderGraphNodeFingerprint {
    fingerprint_parts(
        mix.node_id.clone(),
        &AudioMixSemanticInput {
            target_timerange: &mix.target_timerange,
            keyframes: &mix.keyframes,
            volume_level_millis: mix.volume_level_millis,
            filters: &mix.filters,
        },
        &SegmentInputFacts {
            material_id: &mix.material_id,
            material_kind: draft_model::MaterialKind::Audio,
            source_timerange: &mix.source_timerange,
        },
        output_profile_fingerprint,
        runtime_capability_fingerprint,
    )
}

fn text_overlay_fingerprint(
    overlay: &RenderTextOverlay,
    output_profile_fingerprint: &str,
    runtime_capability_fingerprint: &str,
) -> RenderGraphNodeFingerprint {
    fingerprint_parts(
        overlay.node_id.clone(),
        &TextOverlaySemanticInput {
            overlay: &overlay.overlay,
            keyframes: &overlay.keyframes,
            filters: &overlay.filters,
            transition: overlay.transition.as_ref(),
            visual: &overlay.visual,
        },
        &TextOverlayInputFacts {
            material_id: &overlay.material_id,
            font_ref: overlay.overlay.font_ref.as_deref(),
            font_candidate: &overlay.overlay.font_candidate,
            fallback_candidates: &overlay.overlay.fallback_candidates,
        },
        output_profile_fingerprint,
        runtime_capability_fingerprint,
    )
}

fn sampled_frame_fingerprint(
    frame: &RenderSampledFrame,
    output_profile_fingerprint: &str,
    runtime_capability_fingerprint: &str,
) -> RenderGraphNodeFingerprint {
    fingerprint_parts(
        frame.node_id.clone(),
        &SampledFrameSemanticInput {
            frame_index: frame.frame_index,
            at: frame.at,
        },
        &SampledFrameInputFacts {
            frame_index: frame.frame_index,
            at: frame.at,
        },
        output_profile_fingerprint,
        runtime_capability_fingerprint,
    )
}

fn fingerprint_parts<T, U>(
    node_id: RenderGraphNodeId,
    semantic: &T,
    input: &U,
    output_profile_fingerprint: &str,
    runtime_capability_fingerprint: &str,
) -> RenderGraphNodeFingerprint
where
    T: Serialize,
    U: Serialize,
{
    RenderGraphNodeFingerprint {
        node_id,
        semantic_fingerprint: deterministic_fingerprint("semantic", semantic),
        input_fingerprint: deterministic_fingerprint("input", input),
        output_profile_fingerprint: output_profile_fingerprint.to_owned(),
        runtime_capability_fingerprint: runtime_capability_fingerprint.to_owned(),
        graph_schema_version: GRAPH_SCHEMA_VERSION,
        generator_version: GRAPH_GENERATOR_VERSION.to_owned(),
    }
}

pub fn deterministic_fingerprint<T>(namespace: &str, value: &T) -> String
where
    T: Serialize,
{
    let json = serde_json::to_string(value).unwrap_or_else(|error| {
        format!(
            "{{\"serializationError\":\"{}\"}}",
            EscapeDisplay(error.to_string())
        )
    });
    format!("rgfp:v1:{:016x}", fnv1a64(namespace, &json))
}

fn fnv1a64(namespace: &str, value: &str) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x00000100000001b3;

    let mut hash = OFFSET;
    for byte in namespace.bytes().chain([0xff]).chain(value.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

struct EscapeDisplay(String);

impl fmt::Display for EscapeDisplay {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for character in self.0.chars() {
            match character {
                '"' => formatter.write_str("\\\"")?,
                '\\' => formatter.write_str("\\\\")?,
                other => write!(formatter, "{other}")?,
            }
        }
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeFingerprintInput<'a> {
    capability_fingerprint: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanvasSemanticInput<'a> {
    width: u32,
    height: u32,
    background: &'a crate::RenderCanvasBackground,
    diagnostics: &'a [crate::RenderCanvasDiagnostic],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanvasInputFacts<'a> {
    background_material_id: Option<&'a draft_model::MaterialId>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MaterialSemanticInput<'a> {
    material_id: &'a draft_model::MaterialId,
    kind: draft_model::MaterialKind,
    display_name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MaterialInputFacts<'a> {
    uri: &'a str,
    duration: Option<draft_model::Microseconds>,
    frame_rate: Option<&'a RationalFrameRate>,
    width: Option<u32>,
    height: Option<u32>,
    has_video: bool,
    has_audio: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SegmentInputFacts<'a> {
    material_id: &'a draft_model::MaterialId,
    material_kind: draft_model::MaterialKind,
    source_timerange: &'a draft_model::SourceTimerange,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoLayerSemanticInput<'a> {
    stack_index: u32,
    source_timerange: &'a draft_model::SourceTimerange,
    target_timerange: &'a TargetTimerange,
    keyframes: &'a [draft_model::Keyframe],
    filters: &'a [RenderFilterIntent],
    transition: Option<&'a RenderTransitionIntent>,
    visual: &'a draft_model::SegmentVisual,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioMixSemanticInput<'a> {
    target_timerange: &'a TargetTimerange,
    keyframes: &'a [draft_model::Keyframe],
    volume_level_millis: u32,
    filters: &'a [RenderFilterIntent],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TextOverlaySemanticInput<'a> {
    overlay: &'a engine_core::FrameTextOverlay,
    keyframes: &'a [draft_model::Keyframe],
    filters: &'a [RenderFilterIntent],
    transition: Option<&'a RenderTransitionIntent>,
    visual: &'a draft_model::SegmentVisual,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TextOverlayInputFacts<'a> {
    material_id: &'a draft_model::MaterialId,
    font_ref: Option<&'a str>,
    font_candidate: &'a str,
    fallback_candidates: &'a [String],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SampledFrameSemanticInput {
    frame_index: u64,
    at: draft_model::Microseconds,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SampledFrameInputFacts {
    frame_index: u64,
    at: draft_model::Microseconds,
}
