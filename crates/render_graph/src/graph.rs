use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use draft_model::{
    CanvasBackground, DraftId, Filter, Keyframe, KeyframeProperty, MaterialId, MaterialKind,
    Microseconds, RationalFrameRate, SegmentBackgroundFilling, SegmentBlendMode, SegmentId,
    SegmentMask, SegmentVisual, SourceTimerange, TargetTimerange, TrackId, TrackKind, Transition,
};
use engine_core::{
    FrameTextOverlay, MaterialRenderableState, NormalizedDraft, NormalizedMaterialRef,
    NormalizedSegment, NormalizedTrack, RenderRangeState,
};
use serde::{Deserialize, Serialize};

use crate::incremental::RenderGraphNodeId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraph {
    pub draft_id: DraftId,
    pub canvas: RenderCanvas,
    pub target_timerange: TargetTimerange,
    pub frame_rate: RationalFrameRate,
    pub materials: Vec<RenderMaterial>,
    pub video_layers: Vec<RenderVideoLayer>,
    pub audio_mixes: Vec<RenderAudioMix>,
    pub text_overlays: Vec<RenderTextOverlay>,
    pub sampled_frames: Vec<RenderSampledFrame>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sampled_animation_states: Vec<RenderSampledAnimationState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_diagnostics: Vec<RenderVisualDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderCanvas {
    pub node_id: RenderGraphNodeId,
    pub width: u32,
    pub height: u32,
    #[serde(
        default,
        skip_serializing_if = "RenderCanvasBackground::is_default_black"
    )]
    pub background: RenderCanvasBackground,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<RenderCanvasDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderCanvasBackground {
    pub mode: RenderCanvasBackgroundMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_id: Option<MaterialId>,
    pub support: RenderIntentSupport,
    pub reason: String,
}

impl RenderCanvasBackground {
    fn is_default_black(&self) -> bool {
        self == &Self::default()
    }
}

impl Default for RenderCanvasBackground {
    fn default() -> Self {
        Self {
            mode: RenderCanvasBackgroundMode::Black,
            color: None,
            material_id: None,
            support: RenderIntentSupport::Supported,
            reason: "black canvas background is directly supported".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderCanvasBackgroundMode {
    Black,
    SolidColor,
    BlurFill,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderCanvasDiagnostic {
    pub mode: RenderCanvasBackgroundMode,
    pub support: RenderIntentSupport,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderMaterial {
    pub node_id: RenderGraphNodeId,
    pub material_id: MaterialId,
    pub kind: MaterialKind,
    pub uri: String,
    pub display_name: String,
    pub duration: Option<Microseconds>,
    pub frame_rate: Option<RationalFrameRate>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_video: bool,
    pub has_audio: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderVideoLayer {
    pub node_id: RenderGraphNodeId,
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub material_kind: MaterialKind,
    pub stack_index: u32,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub keyframes: Vec<Keyframe>,
    pub filters: Vec<RenderFilterIntent>,
    pub transition: Option<RenderTransitionIntent>,
    pub visual: SegmentVisual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderAudioMix {
    pub node_id: RenderGraphNodeId,
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<Keyframe>,
    pub volume_level_millis: u32,
    pub filters: Vec<RenderFilterIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderTextOverlay {
    pub node_id: RenderGraphNodeId,
    pub overlay: FrameTextOverlay,
    pub material_id: MaterialId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<Keyframe>,
    pub filters: Vec<RenderFilterIntent>,
    pub transition: Option<RenderTransitionIntent>,
    pub visual: SegmentVisual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderVisualDiagnostic {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub property: String,
    pub support: RenderIntentSupport,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderFilterIntent {
    pub node_id: RenderGraphNodeId,
    pub name: String,
    pub parameters: BTreeMap<String, String>,
    pub support: RenderIntentSupport,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderTransitionIntent {
    pub node_id: RenderGraphNodeId,
    pub name: String,
    pub duration: Microseconds,
    pub support: RenderIntentSupport,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderIntentSupport {
    Supported,
    Degraded,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderSampledFrame {
    pub node_id: RenderGraphNodeId,
    pub frame_index: u64,
    pub at: Microseconds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderSampledAnimationState {
    pub frame_index: u64,
    pub at: Microseconds,
    pub visual_layers: Vec<RenderSampledVisualLayer>,
    pub audio_segments: Vec<RenderSampledAudioSegment>,
    pub text_overlays: Vec<RenderSampledTextOverlay>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderSampledVisualLayer {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub visual: SegmentVisual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderSampledAudioSegment {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub volume_level_millis: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderSampledTextOverlay {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub font_size: u32,
    pub color: String,
    pub line_height_millis: u32,
    pub letter_spacing_millis: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraphError {
    pub kind: RenderGraphErrorKind,
    pub track_id: Option<TrackId>,
    pub segment_id: Option<SegmentId>,
    pub material_id: Option<MaterialId>,
    pub message: String,
}

impl RenderGraphError {
    fn new(kind: RenderGraphErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            track_id: None,
            segment_id: None,
            material_id: None,
            message: message.into(),
        }
    }

    fn with_track_id(mut self, track_id: TrackId) -> Self {
        self.track_id = Some(track_id);
        self
    }

    fn with_segment_id(mut self, segment_id: SegmentId) -> Self {
        self.segment_id = Some(segment_id);
        self
    }

    fn with_material_id(mut self, material_id: MaterialId) -> Self {
        self.material_id = Some(material_id);
        self
    }
}

impl fmt::Display for RenderGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for RenderGraphError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderGraphErrorKind {
    EmptyRenderRange,
    UnknownSegmentInRangeState,
    UnknownMaterialInRangeState,
    UnsupportedProfileSetting,
}

pub fn build_render_graph(
    normalized: &NormalizedDraft,
    range: &RenderRangeState,
) -> Result<RenderGraph, RenderGraphError> {
    if range.frames.is_empty() {
        return Err(RenderGraphError::new(
            RenderGraphErrorKind::EmptyRenderRange,
            "render graph requires at least one sampled frame from engine_core",
        ));
    }

    let segments = segment_index(normalized);
    let active_visual_segments = active_visual_segments(range, &segments)?;
    let active_audio_segments = active_audio_segments(normalized, range, &segments)?;
    let active_text_segments = active_text_segments(range, &segments)?;
    let visual_diagnostics = render_visual_diagnostics(
        &active_visual_segments,
        &active_audio_segments,
        &active_text_segments,
        &segments,
    );
    let sampled_animation_states = render_sampled_animation_states(range, &segments);

    let mut material_ids = BTreeSet::new();
    let mut video_layers = Vec::new();
    for segment_key in active_visual_segments {
        let (track, segment) = segments.get(&segment_key).ok_or_else(|| {
            unknown_segment_error(&segment_key.0, &segment_key.1, "visual range state")
        })?;
        material_ids.insert(segment.material.material_id.clone());
        video_layers.push(render_video_layer(&normalized.draft_id, track, segment)?);
    }

    let mut audio_mixes = Vec::new();
    for segment_key in active_audio_segments {
        let (track, segment) = segments.get(&segment_key).ok_or_else(|| {
            unknown_segment_error(&segment_key.0, &segment_key.1, "audio range state")
        })?;
        material_ids.insert(segment.material.material_id.clone());
        audio_mixes.push(render_audio_mix(&normalized.draft_id, track, segment));
    }

    let mut text_overlays = Vec::new();
    for segment_key in active_text_segments {
        let (track, segment) = segments.get(&segment_key).ok_or_else(|| {
            unknown_segment_error(&segment_key.0, &segment_key.1, "text range state")
        })?;
        let overlay = first_text_overlay_for(range, &segment_key.0, &segment_key.1)
            .ok_or_else(|| unknown_segment_error(&segment_key.0, &segment_key.1, "text overlay"))?;
        material_ids.insert(segment.material.material_id.clone());
        text_overlays.push(render_text_overlay(
            &normalized.draft_id,
            track,
            segment,
            overlay,
        ));
    }

    let materials = render_materials(normalized, &material_ids)?;

    video_layers.sort_by(|first, second| {
        first
            .stack_index
            .cmp(&second.stack_index)
            .then_with(|| first.track_id.cmp(&second.track_id))
            .then_with(|| first.segment_id.cmp(&second.segment_id))
    });
    audio_mixes.sort_by(|first, second| {
        first
            .track_id
            .cmp(&second.track_id)
            .then_with(|| first.segment_id.cmp(&second.segment_id))
    });
    text_overlays.sort_by(|first, second| {
        first
            .overlay
            .stack_index
            .cmp(&second.overlay.stack_index)
            .then_with(|| first.overlay.track_id.cmp(&second.overlay.track_id))
            .then_with(|| first.overlay.segment_id.cmp(&second.overlay.segment_id))
    });

    Ok(RenderGraph {
        draft_id: normalized.draft_id.clone(),
        canvas: render_canvas(normalized),
        target_timerange: range.target_timerange.clone(),
        frame_rate: range.frame_rate.clone(),
        materials,
        video_layers,
        audio_mixes,
        text_overlays,
        sampled_frames: range
            .frames
            .iter()
            .enumerate()
            .map(|(frame_index, frame)| RenderSampledFrame {
                node_id: RenderGraphNodeId::sampled_frame(
                    &normalized.draft_id,
                    frame_index as u64,
                    frame.at.get(),
                ),
                frame_index: frame_index as u64,
                at: frame.at,
            })
            .collect(),
        sampled_animation_states,
        visual_diagnostics,
    })
}

fn render_canvas(normalized: &NormalizedDraft) -> RenderCanvas {
    let background = render_canvas_background(&normalized.profile.canvas_background);
    let diagnostics = canvas_background_diagnostics(&background);
    RenderCanvas {
        node_id: RenderGraphNodeId::canvas(&normalized.draft_id),
        width: normalized.profile.canvas_width,
        height: normalized.profile.canvas_height,
        background,
        diagnostics,
    }
}

fn render_canvas_background(background: &CanvasBackground) -> RenderCanvasBackground {
    match background {
        CanvasBackground::Black => RenderCanvasBackground::default(),
        CanvasBackground::SolidColor { color } => RenderCanvasBackground {
            mode: RenderCanvasBackgroundMode::SolidColor,
            color: Some(color.clone()),
            material_id: None,
            support: RenderIntentSupport::Supported,
            reason: "solid color canvas background is directly supported".to_owned(),
        },
        CanvasBackground::BlurFill => RenderCanvasBackground {
            mode: RenderCanvasBackgroundMode::BlurFill,
            color: None,
            material_id: None,
            support: RenderIntentSupport::Degraded,
            reason:
                "blur fill canvas background is preserved as degraded until render support is implemented"
                    .to_owned(),
        },
        CanvasBackground::Image { material_id } => RenderCanvasBackground {
            mode: RenderCanvasBackgroundMode::Image,
            color: None,
            material_id: material_id.clone(),
            support: RenderIntentSupport::Unsupported,
            reason:
                "image canvas background is unsupported until background material rendering is implemented"
                    .to_owned(),
        },
    }
}

fn canvas_background_diagnostics(
    background: &RenderCanvasBackground,
) -> Vec<RenderCanvasDiagnostic> {
    match background.support {
        RenderIntentSupport::Supported => Vec::new(),
        RenderIntentSupport::Degraded | RenderIntentSupport::Unsupported => {
            vec![RenderCanvasDiagnostic {
                mode: background.mode,
                support: background.support,
                reason: background.reason.clone(),
            }]
        }
    }
}

fn segment_index(
    normalized: &NormalizedDraft,
) -> BTreeMap<(TrackId, SegmentId), (&NormalizedTrack, &NormalizedSegment)> {
    let mut segments = BTreeMap::new();
    for track in &normalized.tracks {
        for segment in &track.segments {
            if segment.renderable == MaterialRenderableState::Renderable {
                segments.insert(
                    (track.track_id.clone(), segment.segment_id.clone()),
                    (track, segment),
                );
            }
        }
    }
    segments
}

fn active_visual_segments(
    range: &RenderRangeState,
    segments: &BTreeMap<(TrackId, SegmentId), (&NormalizedTrack, &NormalizedSegment)>,
) -> Result<BTreeSet<(TrackId, SegmentId)>, RenderGraphError> {
    let mut active = BTreeSet::new();
    for frame in &range.frames {
        for layer in &frame.visual_layers {
            let key = (layer.track_id.clone(), layer.segment_id.clone());
            if !segments.contains_key(&key) {
                return Err(unknown_segment_error(
                    &layer.track_id,
                    &layer.segment_id,
                    "visual range state",
                )
                .with_material_id(layer.material_id.clone()));
            }
            active.insert(key);
        }
    }
    Ok(active)
}

fn active_audio_segments(
    normalized: &NormalizedDraft,
    range: &RenderRangeState,
    segments: &BTreeMap<(TrackId, SegmentId), (&NormalizedTrack, &NormalizedSegment)>,
) -> Result<BTreeSet<(TrackId, SegmentId)>, RenderGraphError> {
    let mut active = BTreeSet::new();
    for frame in &range.frames {
        for audio in &frame.audio_segments {
            let key = (audio.track_id.clone(), audio.segment_id.clone());
            if !segments.contains_key(&key) {
                return Err(unknown_segment_error(
                    &audio.track_id,
                    &audio.segment_id,
                    "audio range state",
                )
                .with_material_id(audio.material_id.clone()));
            }
            active.insert(key);
        }
    }

    for track in &normalized.tracks {
        for segment in &track.segments {
            if segment.renderable == MaterialRenderableState::Renderable
                && segment.material.has_audio
                && segment_intersects_range(segment, &range.target_timerange)
            {
                active.insert((track.track_id.clone(), segment.segment_id.clone()));
            }
        }
    }

    Ok(active)
}

fn active_text_segments(
    range: &RenderRangeState,
    segments: &BTreeMap<(TrackId, SegmentId), (&NormalizedTrack, &NormalizedSegment)>,
) -> Result<BTreeSet<(TrackId, SegmentId)>, RenderGraphError> {
    let mut active = BTreeSet::new();
    for frame in &range.frames {
        for overlay in &frame.text_overlays {
            let key = (overlay.track_id.clone(), overlay.segment_id.clone());
            if !segments.contains_key(&key) {
                return Err(unknown_segment_error(
                    &overlay.track_id,
                    &overlay.segment_id,
                    "text range state",
                ));
            }
            active.insert(key);
        }
    }
    Ok(active)
}

fn render_video_layer(
    draft_id: &DraftId,
    track: &NormalizedTrack,
    segment: &NormalizedSegment,
) -> Result<RenderVideoLayer, RenderGraphError> {
    let stack_index = track.stack_index.ok_or_else(|| {
        RenderGraphError::new(
            RenderGraphErrorKind::UnknownSegmentInRangeState,
            "visual segment range state referenced a track without stackIndex",
        )
        .with_track_id(track.track_id.clone())
        .with_segment_id(segment.segment_id.clone())
        .with_material_id(segment.material.material_id.clone())
    })?;

    Ok(RenderVideoLayer {
        node_id: RenderGraphNodeId::video_segment(
            draft_id,
            &track.track_id,
            &segment.segment_id,
            &segment.material.material_id,
        ),
        track_id: track.track_id.clone(),
        segment_id: segment.segment_id.clone(),
        material_id: segment.material.material_id.clone(),
        material_kind: segment.material.kind,
        stack_index,
        source_timerange: segment.source_timerange.clone(),
        target_timerange: segment.target_timerange.clone(),
        keyframes: segment.keyframes.clone(),
        filters: render_filter_intents(draft_id, track, segment, &segment.filters),
        transition: segment
            .transition
            .as_ref()
            .map(|transition| render_transition_intent(draft_id, track, segment, transition)),
        visual: segment.visual.clone(),
    })
}

fn render_audio_mix(
    draft_id: &DraftId,
    track: &NormalizedTrack,
    segment: &NormalizedSegment,
) -> RenderAudioMix {
    RenderAudioMix {
        node_id: RenderGraphNodeId::audio_segment(
            draft_id,
            &track.track_id,
            &segment.segment_id,
            &segment.material.material_id,
        ),
        track_id: track.track_id.clone(),
        segment_id: segment.segment_id.clone(),
        material_id: segment.material.material_id.clone(),
        source_timerange: segment.source_timerange.clone(),
        target_timerange: segment.target_timerange.clone(),
        keyframes: segment.keyframes.clone(),
        volume_level_millis: segment.volume_level_millis,
        filters: render_filter_intents(draft_id, track, segment, &segment.filters),
    }
}

fn render_text_overlay(
    draft_id: &DraftId,
    track: &NormalizedTrack,
    segment: &NormalizedSegment,
    overlay: FrameTextOverlay,
) -> RenderTextOverlay {
    RenderTextOverlay {
        node_id: RenderGraphNodeId::text_overlay(
            draft_id,
            &track.track_id,
            &segment.segment_id,
            &segment.material.material_id,
        ),
        overlay,
        material_id: segment.material.material_id.clone(),
        keyframes: segment.keyframes.clone(),
        filters: render_filter_intents(draft_id, track, segment, &segment.filters),
        transition: segment
            .transition
            .as_ref()
            .map(|transition| render_transition_intent(draft_id, track, segment, transition)),
        visual: segment.visual.clone(),
    }
}

fn render_visual_diagnostics(
    active_visual_segments: &BTreeSet<(TrackId, SegmentId)>,
    active_audio_segments: &BTreeSet<(TrackId, SegmentId)>,
    active_text_segments: &BTreeSet<(TrackId, SegmentId)>,
    segments: &BTreeMap<(TrackId, SegmentId), (&NormalizedTrack, &NormalizedSegment)>,
) -> Vec<RenderVisualDiagnostic> {
    active_visual_segments
        .iter()
        .chain(active_audio_segments.iter())
        .chain(active_text_segments.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|segment_key| {
            segments
                .get(&segment_key)
                .map(|value| (segment_key, *value))
        })
        .flat_map(|((track_id, segment_id), (track, segment))| {
            let mut diagnostics = Vec::new();
            if matches!(
                track.kind,
                TrackKind::Video | TrackKind::Text | TrackKind::Sticker
            ) {
                diagnostics.extend(visual_diagnostics_for(
                    track_id.clone(),
                    segment_id.clone(),
                    segment.material.material_id.clone(),
                    &segment.visual,
                ));
            }
            diagnostics.extend(keyframe_diagnostics_for(
                &track_id,
                &segment_id,
                &segment.material.material_id,
                &segment.keyframes,
            ));
            diagnostics
        })
        .collect()
}

fn visual_diagnostics_for(
    track_id: TrackId,
    segment_id: SegmentId,
    material_id: MaterialId,
    visual: &SegmentVisual,
) -> Vec<RenderVisualDiagnostic> {
    let mut diagnostics = Vec::new();
    match &visual.background_filling {
        SegmentBackgroundFilling::None
        | SegmentBackgroundFilling::Black
        | SegmentBackgroundFilling::SolidColor { .. } => {}
        SegmentBackgroundFilling::Blur => diagnostics.push(render_visual_diagnostic(
            &track_id,
            &segment_id,
            &material_id,
            "backgroundFilling",
            RenderIntentSupport::Degraded,
            "segment backgroundFilling blur is preserved as degraded render intent",
        )),
        SegmentBackgroundFilling::Image { .. } => diagnostics.push(render_visual_diagnostic(
            &track_id,
            &segment_id,
            &material_id,
            "backgroundFilling",
            RenderIntentSupport::Unsupported,
            "segment backgroundFilling image is unsupported until segment background material rendering is implemented",
        )),
    }

    if visual.transform.rotation.degrees != 0 {
        diagnostics.push(render_visual_diagnostic(
            &track_id,
            &segment_id,
            &material_id,
            "rotation",
            RenderIntentSupport::Unsupported,
            "segment rotation is unsupported until anchor-aware FFmpeg rotation is implemented",
        ));
    }
    if let SegmentBlendMode::Unsupported { name } = &visual.blend_mode {
        diagnostics.push(render_visual_diagnostic(
            &track_id,
            &segment_id,
            &material_id,
            "blendMode",
            RenderIntentSupport::Unsupported,
            format!("segment blendMode {name} is unsupported"),
        ));
    }
    if let SegmentMask::Unsupported { name } = &visual.mask {
        diagnostics.push(render_visual_diagnostic(
            &track_id,
            &segment_id,
            &material_id,
            "mask",
            RenderIntentSupport::Unsupported,
            format!("segment mask {name} is unsupported"),
        ));
    }
    diagnostics
}

fn keyframe_diagnostics_for(
    track_id: &TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    keyframes: &[Keyframe],
) -> Vec<RenderVisualDiagnostic> {
    keyframes
        .iter()
        .map(|keyframe| keyframe.property.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|property| {
            let (support, reason) = keyframe_support(&property);
            render_visual_diagnostic(
                track_id,
                segment_id,
                material_id,
                keyframe_property_name(&property),
                support,
                reason,
            )
        })
        .collect()
}

fn keyframe_support(property: &KeyframeProperty) -> (RenderIntentSupport, &'static str) {
    match property {
        KeyframeProperty::VisualRotation => (
            RenderIntentSupport::Unsupported,
            "animated visual rotation is unsupported until anchor-aware animated rotation is implemented",
        ),
        KeyframeProperty::StickerPositionX
        | KeyframeProperty::StickerPositionY
        | KeyframeProperty::StickerScaleX
        | KeyframeProperty::StickerScaleY => (
            RenderIntentSupport::Unsupported,
            "sticker keyframe animation is deferred until sticker render semantics are implemented",
        ),
        KeyframeProperty::FilterParameterUnsupported => (
            RenderIntentSupport::Unsupported,
            "filter parameter keyframe animation is deferred until filter semantics are implemented",
        ),
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
        | KeyframeProperty::Volume => (
            RenderIntentSupport::Degraded,
            "keyframe animation is engine-resolved and preserved as sampled render intent; continuous compiler expressions are deferred",
        ),
    }
}

fn keyframe_property_name(property: &KeyframeProperty) -> &'static str {
    match property {
        KeyframeProperty::VisualPositionX => "keyframe.visualPositionX",
        KeyframeProperty::VisualPositionY => "keyframe.visualPositionY",
        KeyframeProperty::VisualScaleX => "keyframe.visualScaleX",
        KeyframeProperty::VisualScaleY => "keyframe.visualScaleY",
        KeyframeProperty::VisualRotation => "keyframe.visualRotation",
        KeyframeProperty::VisualOpacity => "keyframe.visualOpacity",
        KeyframeProperty::TextFontSize => "keyframe.textFontSize",
        KeyframeProperty::TextColor => "keyframe.textColor",
        KeyframeProperty::TextLineHeight => "keyframe.textLineHeight",
        KeyframeProperty::TextLetterSpacing => "keyframe.textLetterSpacing",
        KeyframeProperty::TextLayoutX => "keyframe.textLayoutX",
        KeyframeProperty::TextLayoutY => "keyframe.textLayoutY",
        KeyframeProperty::TextLayoutWidth => "keyframe.textLayoutWidth",
        KeyframeProperty::TextLayoutHeight => "keyframe.textLayoutHeight",
        KeyframeProperty::Volume => "keyframe.volume",
        KeyframeProperty::StickerPositionX => "keyframe.stickerPositionX",
        KeyframeProperty::StickerPositionY => "keyframe.stickerPositionY",
        KeyframeProperty::StickerScaleX => "keyframe.stickerScaleX",
        KeyframeProperty::StickerScaleY => "keyframe.stickerScaleY",
        KeyframeProperty::FilterParameterUnsupported => "keyframe.filterParameterUnsupported",
    }
}

fn render_visual_diagnostic(
    track_id: &TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    property: &str,
    support: RenderIntentSupport,
    reason: impl Into<String>,
) -> RenderVisualDiagnostic {
    RenderVisualDiagnostic {
        track_id: track_id.clone(),
        segment_id: segment_id.clone(),
        material_id: material_id.clone(),
        property: property.to_owned(),
        support,
        reason: reason.into(),
    }
}

fn render_filter_intents(
    draft_id: &DraftId,
    track: &NormalizedTrack,
    segment: &NormalizedSegment,
    filters: &[Filter],
) -> Vec<RenderFilterIntent> {
    filters
        .iter()
        .enumerate()
        .map(|(filter_index, filter)| RenderFilterIntent {
            node_id: RenderGraphNodeId::segment_filter(
                draft_id,
                &track.track_id,
                &segment.segment_id,
                &segment.material.material_id,
                filter_index,
            ),
            name: filter.name.clone(),
            parameters: filter.parameters.clone(),
            support: RenderIntentSupport::Degraded,
            reason: "filter intent is preserved for compiler/runtime capability handling"
                .to_owned(),
        })
        .collect()
}

fn render_transition_intent(
    draft_id: &DraftId,
    track: &NormalizedTrack,
    segment: &NormalizedSegment,
    transition: &Transition,
) -> RenderTransitionIntent {
    RenderTransitionIntent {
        node_id: RenderGraphNodeId::segment_transition(
            draft_id,
            &track.track_id,
            &segment.segment_id,
            &segment.material.material_id,
        ),
        name: transition.name.clone(),
        duration: transition.duration,
        support: RenderIntentSupport::Degraded,
        reason: "transition intent is preserved for compiler/runtime capability handling"
            .to_owned(),
    }
}

fn render_sampled_animation_states(
    range: &RenderRangeState,
    segments: &BTreeMap<(TrackId, SegmentId), (&NormalizedTrack, &NormalizedSegment)>,
) -> Vec<RenderSampledAnimationState> {
    range
        .frames
        .iter()
        .enumerate()
        .filter_map(|(frame_index, frame)| {
            let visual_layers = frame
                .visual_layers
                .iter()
                .filter(|layer| segment_has_keyframes(segments, &layer.track_id, &layer.segment_id))
                .map(|layer| RenderSampledVisualLayer {
                    track_id: layer.track_id.clone(),
                    segment_id: layer.segment_id.clone(),
                    material_id: layer.material_id.clone(),
                    visual: layer.visual.clone(),
                })
                .collect::<Vec<_>>();
            let audio_segments = frame
                .audio_segments
                .iter()
                .filter(|audio| segment_has_keyframes(segments, &audio.track_id, &audio.segment_id))
                .map(|audio| RenderSampledAudioSegment {
                    track_id: audio.track_id.clone(),
                    segment_id: audio.segment_id.clone(),
                    material_id: audio.material_id.clone(),
                    volume_level_millis: audio.volume_level_millis,
                })
                .collect::<Vec<_>>();
            let text_overlays = frame
                .text_overlays
                .iter()
                .filter(|overlay| {
                    segment_has_keyframes(segments, &overlay.track_id, &overlay.segment_id)
                })
                .filter_map(|overlay| {
                    let material_id = segments
                        .get(&(overlay.track_id.clone(), overlay.segment_id.clone()))
                        .map(|(_track, segment)| segment.material.material_id.clone())?;
                    Some(RenderSampledTextOverlay {
                        track_id: overlay.track_id.clone(),
                        segment_id: overlay.segment_id.clone(),
                        material_id,
                        font_size: overlay.font_size,
                        color: overlay.style.color.clone(),
                        line_height_millis: overlay.line_height_millis,
                        letter_spacing_millis: overlay.letter_spacing_millis,
                    })
                })
                .collect::<Vec<_>>();

            if visual_layers.is_empty() && audio_segments.is_empty() && text_overlays.is_empty() {
                return None;
            }

            Some(RenderSampledAnimationState {
                frame_index: frame_index as u64,
                at: frame.at,
                visual_layers,
                audio_segments,
                text_overlays,
            })
        })
        .collect()
}

fn segment_has_keyframes(
    segments: &BTreeMap<(TrackId, SegmentId), (&NormalizedTrack, &NormalizedSegment)>,
    track_id: &TrackId,
    segment_id: &SegmentId,
) -> bool {
    segments
        .get(&(track_id.clone(), segment_id.clone()))
        .map(|(_track, segment)| !segment.keyframes.is_empty())
        .unwrap_or(false)
}

fn render_materials(
    normalized: &NormalizedDraft,
    material_ids: &BTreeSet<MaterialId>,
) -> Result<Vec<RenderMaterial>, RenderGraphError> {
    let materials = normalized
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .map(|segment| {
            (
                segment.material.material_id.clone(),
                segment.material.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    material_ids
        .iter()
        .map(|material_id| {
            let material = materials.get(material_id).ok_or_else(|| {
                RenderGraphError::new(
                    RenderGraphErrorKind::UnknownMaterialInRangeState,
                    format!(
                        "render range state referenced unknown material {}",
                        material_id.as_str()
                    ),
                )
                .with_material_id(material_id.clone())
            })?;
            Ok(render_material(&normalized.draft_id, material))
        })
        .collect()
}

fn render_material(draft_id: &DraftId, material: &NormalizedMaterialRef) -> RenderMaterial {
    RenderMaterial {
        node_id: RenderGraphNodeId::material(draft_id, &material.material_id),
        material_id: material.material_id.clone(),
        kind: material.kind,
        uri: material.uri.clone(),
        display_name: material.display_name.clone(),
        duration: material.duration,
        frame_rate: material.frame_rate.clone(),
        width: material.width,
        height: material.height,
        has_video: material.has_video,
        has_audio: material.has_audio,
    }
}

fn first_text_overlay_for(
    range: &RenderRangeState,
    track_id: &TrackId,
    segment_id: &SegmentId,
) -> Option<FrameTextOverlay> {
    range.frames.iter().find_map(|frame| {
        frame
            .text_overlays
            .iter()
            .find(|overlay| &overlay.track_id == track_id && &overlay.segment_id == segment_id)
            .cloned()
    })
}

fn segment_intersects_range(segment: &NormalizedSegment, range: &TargetTimerange) -> bool {
    let segment_start = segment.target_timerange.start.get();
    let segment_end = segment.target_end.get();
    let range_start = range.start.get();
    let Some(range_end) = range_start.checked_add(range.duration.get()) else {
        return false;
    };
    segment_start < range_end && range_start < segment_end
}

fn unknown_segment_error(
    track_id: &TrackId,
    segment_id: &SegmentId,
    context: &str,
) -> RenderGraphError {
    RenderGraphError::new(
        RenderGraphErrorKind::UnknownSegmentInRangeState,
        format!(
            "{context} referenced segment {} on track {} that is not present in NormalizedDraft",
            segment_id.as_str(),
            track_id.as_str()
        ),
    )
    .with_track_id(track_id.clone())
    .with_segment_id(segment_id.clone())
}

#[allow(dead_code)]
fn _track_kind_is_visual(kind: TrackKind) -> bool {
    matches!(
        kind,
        TrackKind::Video | TrackKind::Sticker | TrackKind::Text
    )
}
