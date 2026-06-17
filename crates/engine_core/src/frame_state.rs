use draft_model::{
    MaterialId, MaterialKind, Microseconds, RationalFrameRate, SegmentId, TargetTimerange, TrackId,
};
use serde::{Deserialize, Serialize};

use crate::{
    EngineError, EngineErrorKind, MaterialRenderableState, NormalizedDraft, NormalizedSegment,
};

const MICROSECONDS_PER_SECOND: u128 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameState {
    pub at: Microseconds,
    pub visual_layers: Vec<FrameVisualLayer>,
    pub audio_segments: Vec<FrameAudioSegment>,
    pub text_overlays: Vec<FrameTextOverlay>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameVisualLayer {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub material_kind: MaterialKind,
    pub stack_index: u32,
    pub source_position: Microseconds,
    pub target_timerange: TargetTimerange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameAudioSegment {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_position: Microseconds,
    pub target_timerange: TargetTimerange,
    pub volume_level_millis: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameTextOverlay {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub content: String,
    pub stack_index: u32,
    pub source_position: Microseconds,
    pub target_timerange: TargetTimerange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderRange {
    pub target_timerange: TargetTimerange,
    pub frame_rate: RationalFrameRate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderRangeState {
    pub target_timerange: TargetTimerange,
    pub frame_rate: RationalFrameRate,
    pub frames: Vec<FrameState>,
}

pub fn resolve_frame_state(
    normalized: &NormalizedDraft,
    at: Microseconds,
) -> Result<FrameState, EngineError> {
    let mut visual_layers = Vec::new();
    let mut audio_segments = Vec::new();
    let mut text_overlays = Vec::new();

    for track in &normalized.tracks {
        for segment in &track.segments {
            if segment.renderable != MaterialRenderableState::Renderable {
                continue;
            }
            if !covers_timeline_position(segment, at) {
                continue;
            }

            let source_position = source_position_at(segment, at)?;
            match track.kind {
                draft_model::TrackKind::Audio => audio_segments.push(FrameAudioSegment {
                    track_id: track.track_id.clone(),
                    segment_id: segment.segment_id.clone(),
                    material_id: segment.material.material_id.clone(),
                    source_position,
                    target_timerange: segment.target_timerange.clone(),
                    volume_level_millis: segment.volume_level_millis,
                }),
                draft_model::TrackKind::Video | draft_model::TrackKind::Sticker => {
                    if let Some(stack_index) = track.stack_index {
                        visual_layers.push(FrameVisualLayer {
                            track_id: track.track_id.clone(),
                            segment_id: segment.segment_id.clone(),
                            material_id: segment.material.material_id.clone(),
                            material_kind: segment.material.kind,
                            stack_index,
                            source_position,
                            target_timerange: segment.target_timerange.clone(),
                        });
                    }
                }
                draft_model::TrackKind::Text => {
                    if let Some(stack_index) = track.stack_index {
                        if let Some(text) = &segment.text {
                            text_overlays.push(FrameTextOverlay {
                                track_id: track.track_id.clone(),
                                segment_id: segment.segment_id.clone(),
                                content: text.content.clone(),
                                stack_index,
                                source_position,
                                target_timerange: segment.target_timerange.clone(),
                            });
                        }
                    }
                }
                draft_model::TrackKind::Filter => {}
            }
        }
    }

    visual_layers.sort_by(|first, second| {
        first
            .stack_index
            .cmp(&second.stack_index)
            .then_with(|| first.track_id.cmp(&second.track_id))
            .then_with(|| first.segment_id.cmp(&second.segment_id))
    });
    audio_segments.sort_by(|first, second| {
        first
            .track_id
            .cmp(&second.track_id)
            .then_with(|| first.segment_id.cmp(&second.segment_id))
    });
    text_overlays.sort_by(|first, second| {
        first
            .stack_index
            .cmp(&second.stack_index)
            .then_with(|| first.track_id.cmp(&second.track_id))
            .then_with(|| first.segment_id.cmp(&second.segment_id))
    });

    Ok(FrameState {
        at,
        visual_layers,
        audio_segments,
        text_overlays,
    })
}

pub fn resolve_render_range(
    normalized: &NormalizedDraft,
    target_timerange: TargetTimerange,
) -> Result<RenderRangeState, EngineError> {
    let target_end = target_timerange
        .start
        .get()
        .checked_add(target_timerange.duration.get())
        .map(Microseconds::new)
        .ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                "render range targetTimerange start + duration overflowed u64 microseconds",
            )
        })?;

    let render_range = RenderRange {
        target_timerange,
        frame_rate: normalized.profile.frame_rate.clone(),
    };
    validate_frame_rate(&render_range.frame_rate)?;

    let mut frames = Vec::new();
    let mut frame_index = 0_u64;
    loop {
        let offset = frame_index_to_microseconds(frame_index, &render_range.frame_rate)?;
        if offset.get() >= render_range.target_timerange.duration.get() {
            break;
        }
        let at = render_range
            .target_timerange
            .start
            .get()
            .checked_add(offset.get())
            .map(Microseconds::new)
            .ok_or_else(|| {
                EngineError::new(
                    EngineErrorKind::TimerangeOverflow,
                    "render range frame position overflowed u64 microseconds",
                )
            })?;
        if at >= target_end {
            break;
        }
        frames.push(resolve_frame_state(normalized, at)?);
        frame_index = frame_index.checked_add(1).ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                "render range frame index overflowed u64",
            )
        })?;
    }

    Ok(RenderRangeState {
        target_timerange: render_range.target_timerange,
        frame_rate: render_range.frame_rate,
        frames,
    })
}

pub fn frame_index_to_microseconds(
    frame_index: u64,
    frame_rate: &RationalFrameRate,
) -> Result<Microseconds, EngineError> {
    validate_frame_rate(frame_rate)?;

    let numerator = u128::from(frame_rate.numerator);
    let denominator = u128::from(frame_rate.denominator);
    let value = u128::from(frame_index)
        .checked_mul(denominator)
        .and_then(|value| value.checked_mul(MICROSECONDS_PER_SECOND))
        .ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                "frame index to microsecond conversion overflowed",
            )
        })?
        / numerator;

    u64::try_from(value).map(Microseconds::new).map_err(|_| {
        EngineError::new(
            EngineErrorKind::TimerangeOverflow,
            "frame index to microsecond conversion exceeded u64 microseconds",
        )
    })
}

fn covers_timeline_position(segment: &NormalizedSegment, at: Microseconds) -> bool {
    segment.target_timerange.start <= at && at < segment.target_end
}

fn source_position_at(
    segment: &NormalizedSegment,
    at: Microseconds,
) -> Result<Microseconds, EngineError> {
    let offset = at
        .get()
        .checked_sub(segment.target_timerange.start.get())
        .ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                "frame position precedes segment targetTimerange start",
            )
            .with_segment_id_public(segment.segment_id.clone())
            .with_material_id_public(segment.material.material_id.clone())
        })?;
    segment
        .source_timerange
        .start
        .get()
        .checked_add(offset)
        .map(Microseconds::new)
        .ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                "sourceTimerange start plus timeline offset overflowed",
            )
            .with_segment_id_public(segment.segment_id.clone())
            .with_material_id_public(segment.material.material_id.clone())
        })
}

fn validate_frame_rate(frame_rate: &RationalFrameRate) -> Result<(), EngineError> {
    if frame_rate.numerator == 0 || frame_rate.denominator == 0 {
        return Err(EngineError::new(
            EngineErrorKind::InvalidFrameRate,
            "rational frame rate numerator and denominator must be greater than zero",
        ));
    }
    Ok(())
}

trait EngineErrorPublicContext {
    fn with_segment_id_public(self, segment_id: SegmentId) -> Self;
    fn with_material_id_public(self, material_id: MaterialId) -> Self;
}

impl EngineErrorPublicContext for EngineError {
    fn with_segment_id_public(mut self, segment_id: SegmentId) -> Self {
        self.segment_id = Some(segment_id);
        self
    }

    fn with_material_id_public(mut self, material_id: MaterialId) -> Self {
        self.material_id = Some(material_id);
        self
    }
}
