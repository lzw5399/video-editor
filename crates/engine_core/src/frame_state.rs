use draft_model::{
    Filter, Keyframe, KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue,
    MaterialId, MaterialKind, Microseconds, RationalFrameRate, SegmentId, SegmentRetiming,
    SegmentVisual, TargetTimerange, TextSegment, TrackId, Transition,
};
use serde::{Deserialize, Serialize};

use crate::{
    EngineError, EngineErrorKind, MaterialRenderableState, NormalizedDraft, NormalizedSegment,
    ResolvedTextOverlay,
    text_layout::resolve_text_overlay,
    time_mapping::{AudioRetimeDiagnostic, SegmentTimeMap, audio_retime_diagnostic},
};

const MICROSECONDS_PER_SECOND: u128 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameState {
    pub at: Microseconds,
    pub visual_layers: Vec<FrameVisualLayer>,
    pub audio_segments: Vec<FrameAudioSegment>,
    pub text_overlays: Vec<ResolvedTextOverlay>,
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
    pub retiming: SegmentRetiming,
    pub filters: Vec<Filter>,
    pub transition: Option<Transition>,
    pub visual: SegmentVisual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameAudioSegment {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_position: Microseconds,
    pub target_timerange: TargetTimerange,
    pub retiming: SegmentRetiming,
    pub audio_retime_diagnostic: Option<AudioRetimeDiagnostic>,
    pub volume_level_millis: u32,
}

pub type FrameTextOverlay = ResolvedTextOverlay;

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

            let segment_time = segment_relative_time_at(segment, at)?;
            let source_position = source_position_at(segment, at)?;
            match track.kind {
                draft_model::TrackKind::Audio => audio_segments.push(FrameAudioSegment {
                    track_id: track.track_id.clone(),
                    segment_id: segment.segment_id.clone(),
                    material_id: segment.material.material_id.clone(),
                    source_position,
                    target_timerange: segment.target_timerange.clone(),
                    retiming: segment.retiming.clone(),
                    audio_retime_diagnostic: audio_retime_diagnostic(&segment.retiming),
                    volume_level_millis: resolve_segment_volume(segment, segment_time),
                }),
                draft_model::TrackKind::Video | draft_model::TrackKind::Sticker => {
                    if !segment.visual.visible {
                        continue;
                    }
                    if let Some(stack_index) = track.stack_index {
                        visual_layers.push(FrameVisualLayer {
                            track_id: track.track_id.clone(),
                            segment_id: segment.segment_id.clone(),
                            material_id: segment.material.material_id.clone(),
                            material_kind: segment.material.kind,
                            stack_index,
                            source_position,
                            target_timerange: segment.target_timerange.clone(),
                            retiming: segment.retiming.clone(),
                            filters: segment.filters.clone(),
                            transition: segment.transition.clone(),
                            visual: resolve_segment_visual(segment, segment_time),
                        });
                    }
                }
                draft_model::TrackKind::Text => {
                    if !segment.visual.visible {
                        continue;
                    }
                    if let Some(stack_index) = track.stack_index {
                        if let Some(text) = &segment.text {
                            let text_layout =
                                normalized.profile.text_layout.as_ref().ok_or_else(|| {
                                    EngineError::new(
                                        EngineErrorKind::MissingTextLayoutProfile,
                                        "active text segment requires a deterministic text layout profile",
                                    )
                                    .with_segment_id(segment.segment_id.clone())
                                    .with_material_id(segment.material.material_id.clone())
                                })?;
                            let resolved_text = resolve_segment_text(segment, text, segment_time);
                            text_overlays.push(resolve_text_overlay(
                                &track.track_id,
                                segment,
                                &resolved_text,
                                stack_index,
                                source_position,
                                text_layout,
                                normalized.profile.canvas_width,
                                normalized.profile.canvas_height,
                            )?);
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
    let offset = segment_relative_time_at(segment, at)?;
    SegmentTimeMap::new(&segment.source_timerange, &segment.retiming)
        .source_at_target(offset)
        .map_err(|error| {
            error
                .with_segment_id(segment.segment_id.clone())
                .with_material_id(segment.material.material_id.clone())
        })
}

fn segment_relative_time_at(
    segment: &NormalizedSegment,
    at: Microseconds,
) -> Result<Microseconds, EngineError> {
    at.get()
        .checked_sub(segment.target_timerange.start.get())
        .map(Microseconds::new)
        .ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                "frame position precedes segment targetTimerange start",
            )
            .with_segment_id(segment.segment_id.clone())
            .with_material_id(segment.material.material_id.clone())
        })
}

fn resolve_segment_visual(segment: &NormalizedSegment, at: Microseconds) -> SegmentVisual {
    let mut visual = segment.visual.clone();
    visual.transform.position.x = resolve_int_keyframe(
        segment,
        KeyframeProperty::VisualPositionX,
        visual.transform.position.x,
        at,
    );
    visual.transform.position.y = resolve_int_keyframe(
        segment,
        KeyframeProperty::VisualPositionY,
        visual.transform.position.y,
        at,
    );
    visual.transform.scale.x_millis = resolve_uint_keyframe(
        segment,
        KeyframeProperty::VisualScaleX,
        visual.transform.scale.x_millis,
        at,
    );
    visual.transform.scale.y_millis = resolve_uint_keyframe(
        segment,
        KeyframeProperty::VisualScaleY,
        visual.transform.scale.y_millis,
        at,
    );
    visual.transform.rotation.degrees = resolve_int_keyframe(
        segment,
        KeyframeProperty::VisualRotation,
        visual.transform.rotation.degrees,
        at,
    );
    visual.transform.opacity.value_millis = resolve_uint_keyframe(
        segment,
        KeyframeProperty::VisualOpacity,
        visual.transform.opacity.value_millis,
        at,
    );
    visual
}

fn resolve_segment_text(
    segment: &NormalizedSegment,
    text: &TextSegment,
    at: Microseconds,
) -> TextSegment {
    let mut resolved = text.clone();
    resolved.style.font_size = resolve_uint_keyframe(
        segment,
        KeyframeProperty::TextFontSize,
        resolved.style.font_size,
        at,
    );
    resolved.style.color = resolve_color_keyframe(
        segment,
        KeyframeProperty::TextColor,
        &resolved.style.color,
        at,
    );
    resolved.style.line_height_millis = resolve_uint_keyframe(
        segment,
        KeyframeProperty::TextLineHeight,
        resolved.style.line_height_millis,
        at,
    );
    resolved.style.letter_spacing_millis = resolve_uint_keyframe(
        segment,
        KeyframeProperty::TextLetterSpacing,
        resolved.style.letter_spacing_millis,
        at,
    );
    resolved.layout_region.x_millis = resolve_uint_keyframe(
        segment,
        KeyframeProperty::TextLayoutX,
        resolved.layout_region.x_millis,
        at,
    );
    resolved.layout_region.y_millis = resolve_uint_keyframe(
        segment,
        KeyframeProperty::TextLayoutY,
        resolved.layout_region.y_millis,
        at,
    );
    resolved.layout_region.width_millis = resolve_uint_keyframe(
        segment,
        KeyframeProperty::TextLayoutWidth,
        resolved.layout_region.width_millis,
        at,
    );
    resolved.layout_region.height_millis = resolve_uint_keyframe(
        segment,
        KeyframeProperty::TextLayoutHeight,
        resolved.layout_region.height_millis,
        at,
    );
    resolved
}

fn resolve_segment_volume(segment: &NormalizedSegment, at: Microseconds) -> u32 {
    resolve_uint_keyframe(
        segment,
        KeyframeProperty::Volume,
        segment.volume_level_millis,
        at,
    )
}

fn resolve_int_keyframe(
    segment: &NormalizedSegment,
    property: KeyframeProperty,
    base: i32,
    at: Microseconds,
) -> i32 {
    resolve_numeric_keyframe(
        segment,
        property,
        i128::from(base),
        at,
        |keyframe| match &keyframe.value {
            KeyframeValue::Int { value } => Some(i128::from(*value)),
            _ => None,
        },
    )
    .and_then(|value| i32::try_from(value).ok())
    .unwrap_or(base)
}

fn resolve_uint_keyframe(
    segment: &NormalizedSegment,
    property: KeyframeProperty,
    base: u32,
    at: Microseconds,
) -> u32 {
    resolve_numeric_keyframe(
        segment,
        property,
        i128::from(base),
        at,
        |keyframe| match &keyframe.value {
            KeyframeValue::Uint { value } => Some(i128::from(*value)),
            _ => None,
        },
    )
    .and_then(|value| u32::try_from(value).ok())
    .unwrap_or(base)
}

fn resolve_numeric_keyframe(
    segment: &NormalizedSegment,
    property: KeyframeProperty,
    base: i128,
    at: Microseconds,
    value_of: impl Fn(&Keyframe) -> Option<i128>,
) -> Option<i128> {
    let mut keyframes = keyframes_for_property(segment, property);
    if keyframes.is_empty() || at < keyframes[0].at {
        return Some(base);
    }

    for index in 0..keyframes.len() {
        let current = keyframes[index];
        if at == current.at {
            return value_of(current);
        }
        if at < current.at {
            let previous = keyframes[index - 1];
            let previous_value = value_of(previous)?;
            if previous.interpolation == KeyframeInterpolation::Hold {
                return Some(previous_value);
            }
            let current_value = value_of(current)?;
            let progress = eased_progress_per_mille(previous, at, current.at)?;
            return Some(interpolate_i128(previous_value, current_value, progress));
        }
    }

    keyframes.pop().and_then(value_of)
}

fn resolve_color_keyframe(
    segment: &NormalizedSegment,
    property: KeyframeProperty,
    base: &str,
    at: Microseconds,
) -> String {
    let keyframes = keyframes_for_property(segment, property);
    if keyframes.is_empty() || at < keyframes[0].at {
        return base.to_owned();
    }

    let mut last_color = base.to_owned();
    for keyframe in keyframes {
        if at < keyframe.at {
            return last_color;
        }
        if let KeyframeValue::Color { value } = &keyframe.value {
            last_color = value.clone();
        }
        if at == keyframe.at {
            return last_color;
        }
    }
    last_color
}

fn keyframes_for_property(
    segment: &NormalizedSegment,
    property: KeyframeProperty,
) -> Vec<&Keyframe> {
    let mut keyframes = segment
        .keyframes
        .iter()
        .filter(|keyframe| keyframe.property == property)
        .collect::<Vec<_>>();
    keyframes.sort_by(|first, second| first.at.cmp(&second.at));
    keyframes
}

fn eased_progress_per_mille(start: &Keyframe, at: Microseconds, end: Microseconds) -> Option<u32> {
    let span = end.get().checked_sub(start.at.get())?;
    if span == 0 {
        return Some(1_000);
    }
    let elapsed = at.get().checked_sub(start.at.get())?;
    let raw = (u128::from(elapsed) * 1_000_u128 / u128::from(span)).min(1_000) as u32;
    Some(match start.easing {
        KeyframeEasing::None => raw,
        KeyframeEasing::EaseIn => raw.saturating_mul(raw) / 1_000,
        KeyframeEasing::EaseOut => {
            let remaining = 1_000_u32.saturating_sub(raw);
            1_000_u32.saturating_sub(remaining.saturating_mul(remaining) / 1_000)
        }
        KeyframeEasing::EaseInOut => {
            if raw < 500 {
                2_u32.saturating_mul(raw).saturating_mul(raw) / 1_000
            } else {
                let remaining = 1_000_u32.saturating_sub(raw);
                1_000_u32.saturating_sub(
                    2_u32.saturating_mul(remaining).saturating_mul(remaining) / 1_000,
                )
            }
        }
    })
}

fn interpolate_i128(start: i128, end: i128, progress_per_mille: u32) -> i128 {
    let delta = end - start;
    start + (delta * i128::from(progress_per_mille)) / 1_000
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
