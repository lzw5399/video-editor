use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use draft_model::{
    DraftId, Filter, Keyframe, MaterialId, MaterialKind, Microseconds, RationalFrameRate,
    SegmentId, SourceTimerange, TargetTimerange, TrackId, TrackKind, Transition,
};
use engine_core::{
    FrameTextOverlay, MaterialRenderableState, NormalizedDraft, NormalizedMaterialRef,
    NormalizedSegment, NormalizedTrack, RenderRangeState,
};
use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderCanvas {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderMaterial {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderAudioMix {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub volume_level_millis: u32,
    pub filters: Vec<RenderFilterIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderTextOverlay {
    pub overlay: FrameTextOverlay,
    pub material_id: MaterialId,
    pub filters: Vec<RenderFilterIntent>,
    pub transition: Option<RenderTransitionIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderFilterIntent {
    pub name: String,
    pub parameters: BTreeMap<String, String>,
    pub support: RenderIntentSupport,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderTransitionIntent {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderSampledFrame {
    pub frame_index: u64,
    pub at: Microseconds,
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

    let mut material_ids = BTreeSet::new();
    let mut video_layers = Vec::new();
    for segment_key in active_visual_segments {
        let (track, segment) = segments.get(&segment_key).ok_or_else(|| {
            unknown_segment_error(&segment_key.0, &segment_key.1, "visual range state")
        })?;
        material_ids.insert(segment.material.material_id.clone());
        video_layers.push(render_video_layer(track, segment)?);
    }

    let mut audio_mixes = Vec::new();
    for segment_key in active_audio_segments {
        let (track, segment) = segments.get(&segment_key).ok_or_else(|| {
            unknown_segment_error(&segment_key.0, &segment_key.1, "audio range state")
        })?;
        material_ids.insert(segment.material.material_id.clone());
        audio_mixes.push(render_audio_mix(track, segment));
    }

    let mut text_overlays = Vec::new();
    for segment_key in active_text_segments {
        let (track, segment) = segments.get(&segment_key).ok_or_else(|| {
            unknown_segment_error(&segment_key.0, &segment_key.1, "text range state")
        })?;
        let overlay = first_text_overlay_for(range, &segment_key.0, &segment_key.1)
            .ok_or_else(|| unknown_segment_error(&segment_key.0, &segment_key.1, "text overlay"))?;
        material_ids.insert(segment.material.material_id.clone());
        text_overlays.push(render_text_overlay(track, segment, overlay));
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
        canvas: RenderCanvas {
            width: normalized.profile.canvas_width,
            height: normalized.profile.canvas_height,
        },
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
                frame_index: frame_index as u64,
                at: frame.at,
            })
            .collect(),
    })
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
        track_id: track.track_id.clone(),
        segment_id: segment.segment_id.clone(),
        material_id: segment.material.material_id.clone(),
        material_kind: segment.material.kind,
        stack_index,
        source_timerange: segment.source_timerange.clone(),
        target_timerange: segment.target_timerange.clone(),
        keyframes: segment.keyframes.clone(),
        filters: render_filter_intents(&segment.filters),
        transition: segment.transition.as_ref().map(render_transition_intent),
    })
}

fn render_audio_mix(track: &NormalizedTrack, segment: &NormalizedSegment) -> RenderAudioMix {
    RenderAudioMix {
        track_id: track.track_id.clone(),
        segment_id: segment.segment_id.clone(),
        material_id: segment.material.material_id.clone(),
        source_timerange: segment.source_timerange.clone(),
        target_timerange: segment.target_timerange.clone(),
        volume_level_millis: segment.volume_level_millis,
        filters: render_filter_intents(&segment.filters),
    }
}

fn render_text_overlay(
    _track: &NormalizedTrack,
    segment: &NormalizedSegment,
    overlay: FrameTextOverlay,
) -> RenderTextOverlay {
    RenderTextOverlay {
        overlay,
        material_id: segment.material.material_id.clone(),
        filters: render_filter_intents(&segment.filters),
        transition: segment.transition.as_ref().map(render_transition_intent),
    }
}

fn render_filter_intents(filters: &[Filter]) -> Vec<RenderFilterIntent> {
    filters
        .iter()
        .map(|filter| RenderFilterIntent {
            name: filter.name.clone(),
            parameters: filter.parameters.clone(),
            support: RenderIntentSupport::Degraded,
            reason: "filter intent is preserved for compiler/runtime capability handling"
                .to_owned(),
        })
        .collect()
}

fn render_transition_intent(transition: &Transition) -> RenderTransitionIntent {
    RenderTransitionIntent {
        name: transition.name.clone(),
        duration: transition.duration,
        support: RenderIntentSupport::Degraded,
        reason: "transition intent is preserved for compiler/runtime capability handling"
            .to_owned(),
    }
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
            Ok(render_material(material))
        })
        .collect()
}

fn render_material(material: &NormalizedMaterialRef) -> RenderMaterial {
    RenderMaterial {
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
