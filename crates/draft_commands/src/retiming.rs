//! Segment retiming command semantics and integer source mapping helpers.

use draft_model::{
    AudioRetimePolicy, CommandDeltaName, CommandEvent, CommandState, Draft, Material, Microseconds,
    RetimeMode, Segment, SegmentId, SegmentRetiming, SpeedCurvePoint, SpeedRatio,
    TimelineCommandResponse, TimelineSelection, TrackKind,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    delta::retime_segment_delta,
    history::push_undo_snapshot,
    timeline::{
        checked_source_end, find_segment_location, validate_timeline_rules, validate_track_unlocked,
    },
};

pub const MAX_SPEED_CURVE_POINTS: usize = 64;

pub fn set_segment_retime(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    retiming: SegmentRetiming,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    let track_kind = next_draft.tracks[track_index].kind;
    let segment = next_draft.tracks[track_index].segments[segment_index].clone();
    let material = find_material(&next_draft, &segment.material_id)?;
    validate_segment_retime(&segment, &retiming, track_kind, material)?;

    next_draft.tracks[track_index].segments[segment_index].retiming = retiming;
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let segment = &next_draft.tracks[track_index].segments[segment_index];
    let delta = retime_segment_delta(
        CommandDeltaName::SetSegmentRetime,
        &track_id,
        segment,
        "segment retime set",
    );
    let (command_state, pruned) =
        push_undo_snapshot(command_state, draft, selection, "setSegmentRetime");
    let mut events = vec![CommandEvent {
        kind: "segmentRetimeSet".to_owned(),
        message: None,
    }];
    if pruned {
        events.push(CommandEvent {
            kind: "historyLimitPruned".to_owned(),
            message: None,
        });
    }

    Ok(TimelineCommandResponse {
        draft: next_draft,
        command_state,
        selection: selection.clone(),
        events,
        delta,
    })
}

pub fn clear_segment_retime(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    next_draft.tracks[track_index].segments[segment_index].retiming = SegmentRetiming::default();
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let segment = &next_draft.tracks[track_index].segments[segment_index];
    let delta = retime_segment_delta(
        CommandDeltaName::ClearSegmentRetime,
        &track_id,
        segment,
        "segment retime cleared",
    );
    let (command_state, pruned) =
        push_undo_snapshot(command_state, draft, selection, "clearSegmentRetime");
    let mut events = vec![CommandEvent {
        kind: "segmentRetimeCleared".to_owned(),
        message: None,
    }];
    if pruned {
        events.push(CommandEvent {
            kind: "historyLimitPruned".to_owned(),
            message: None,
        });
    }

    Ok(TimelineCommandResponse {
        draft: next_draft,
        command_state,
        selection: selection.clone(),
        events,
        delta,
    })
}

pub fn validate_segment_retime(
    segment: &Segment,
    retiming: &SegmentRetiming,
    track_kind: TrackKind,
    material: &Material,
) -> Result<(), TimelineCommandError> {
    validate_retime_mode(
        &segment.segment_id,
        &retiming.mode,
        segment.target_timerange.duration,
    )?;
    validate_audio_policy(&segment.segment_id, retiming, track_kind)?;
    let required_source_duration =
        source_offset_for_target_duration(&retiming.mode, segment.target_timerange.duration)?;
    if required_source_duration.get() > segment.source_timerange.duration.get() {
        return Err(invalid_retime(
            &segment.segment_id,
            format!(
                "retime requires {}us of source for {}us target, but segment source duration is {}us",
                required_source_duration.get(),
                segment.target_timerange.duration.get(),
                segment.source_timerange.duration.get()
            ),
        ));
    }

    if let Some(material_duration) = material.metadata.duration {
        let source_end = checked_source_end(&segment.source_timerange)?;
        if source_end.get() > material_duration.get() {
            return Err(invalid_retime(
                &segment.segment_id,
                format!(
                    "retime source range ends at {}us beyond material duration {}us",
                    source_end.get(),
                    material_duration.get()
                ),
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_draft_retime_source_ranges(
    draft: &Draft,
) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        for segment in &track.segments {
            let material = find_material(draft, &segment.material_id)?;
            validate_segment_retime(segment, &segment.retiming, track.kind, material)?;
        }
    }
    Ok(())
}

pub(crate) fn source_offset_for_target_duration(
    mode: &RetimeMode,
    target_duration: Microseconds,
) -> Result<Microseconds, TimelineCommandError> {
    match mode {
        RetimeMode::Constant { speed } => checked_ratio_duration(target_duration, speed, true),
        RetimeMode::SpeedCurve { points } => {
            validate_speed_curve_points(&SegmentId::from(""), points, target_duration, true)?;
            integrate_speed_curve(points, target_duration)
        }
    }
}

fn validate_retime_mode(
    segment_id: &SegmentId,
    mode: &RetimeMode,
    target_duration: Microseconds,
) -> Result<(), TimelineCommandError> {
    match mode {
        RetimeMode::Constant { speed } => validate_speed_ratio(segment_id, speed),
        RetimeMode::SpeedCurve { points } => {
            validate_speed_curve_points(segment_id, points, target_duration, false)
        }
    }
}

fn validate_speed_curve_points(
    segment_id: &SegmentId,
    points: &[SpeedCurvePoint],
    target_duration: Microseconds,
    allow_empty_segment_id: bool,
) -> Result<(), TimelineCommandError> {
    if points.is_empty() {
        return Err(invalid_retime(
            segment_id,
            "speed curve requires at least one point",
        ));
    }
    if points.len() > MAX_SPEED_CURVE_POINTS {
        return Err(invalid_retime(
            segment_id,
            format!(
                "speed curve has {} points; maximum is {}",
                points.len(),
                MAX_SPEED_CURVE_POINTS
            ),
        ));
    }
    if points[0].target_time != Microseconds::ZERO {
        return Err(invalid_retime(
            segment_id,
            "speed curve first point must start at target time 0us",
        ));
    }

    let mut previous = None;
    for point in points {
        validate_speed_ratio(segment_id, &point.speed)?;
        if point.target_time.get() > target_duration.get() {
            return Err(invalid_retime(
                segment_id,
                format!(
                    "speed curve point {}us exceeds segment target duration {}us",
                    point.target_time.get(),
                    target_duration.get()
                ),
            ));
        }
        if let Some(previous_time) = previous {
            if point.target_time.get() <= previous_time {
                return Err(invalid_retime(
                    segment_id,
                    "speed curve target points must be strictly increasing",
                ));
            }
        }
        previous = Some(point.target_time.get());
    }

    if allow_empty_segment_id || !segment_id.as_str().is_empty() {
        return Ok(());
    }
    Err(invalid_retime(
        segment_id,
        "segment id is required for retime validation",
    ))
}

fn validate_speed_ratio(
    segment_id: &SegmentId,
    speed: &SpeedRatio,
) -> Result<(), TimelineCommandError> {
    if speed.numerator == 0 || speed.denominator == 0 {
        return Err(invalid_retime(
            segment_id,
            "speed ratios must use nonzero numerator and denominator",
        ));
    }
    Ok(())
}

fn validate_audio_policy(
    segment_id: &SegmentId,
    retiming: &SegmentRetiming,
    track_kind: TrackKind,
) -> Result<(), TimelineCommandError> {
    if retiming.audio_policy != AudioRetimePolicy::PreservePitch
        || is_effectively_1x(&retiming.mode)
    {
        return Ok(());
    }

    Err(TimelineCommandError::new(
        TimelineCommandErrorKind::UnsupportedAudioRetime {
            segment_id: segment_id.clone(),
            policy: format!("{:?}", retiming.audio_policy),
            reason: format!(
                "pitch preservation for non-1x retiming is not supported on {:?} segments",
                track_kind
            ),
        },
    ))
}

fn is_effectively_1x(mode: &RetimeMode) -> bool {
    match mode {
        RetimeMode::Constant { speed } => speed.numerator == speed.denominator,
        RetimeMode::SpeedCurve { points } => points
            .iter()
            .all(|point| point.speed.numerator == point.speed.denominator),
    }
}

fn integrate_speed_curve(
    points: &[SpeedCurvePoint],
    target_duration: Microseconds,
) -> Result<Microseconds, TimelineCommandError> {
    let mut source_us = 0_u64;
    for (index, point) in points.iter().enumerate() {
        let segment_start = point.target_time;
        let segment_end = points
            .get(index + 1)
            .map(|next| next.target_time)
            .unwrap_or(target_duration);
        if segment_end.get() <= segment_start.get() {
            continue;
        }
        let span = Microseconds::new(segment_end.get() - segment_start.get());
        let mapped = checked_ratio_duration(span, &point.speed, true)?;
        source_us = source_us.checked_add(mapped.get()).ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                field: "retiming.sourceDuration".to_owned(),
            })
        })?;
    }
    Ok(Microseconds::new(source_us))
}

fn checked_ratio_duration(
    duration: Microseconds,
    speed: &SpeedRatio,
    ceil: bool,
) -> Result<Microseconds, TimelineCommandError> {
    let numerator = u128::from(duration.get()) * u128::from(speed.numerator);
    let denominator = u128::from(speed.denominator);
    let mapped = if ceil {
        numerator
            .checked_add(denominator.saturating_sub(1))
            .ok_or_else(|| {
                TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                    field: "retiming.sourceDuration".to_owned(),
                })
            })?
            / denominator
    } else {
        numerator / denominator
    };
    let mapped = u64::try_from(mapped).map_err(|_| {
        TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
            field: "retiming.sourceDuration".to_owned(),
        })
    })?;
    Ok(Microseconds::new(mapped))
}

fn find_material<'a>(
    draft: &'a Draft,
    material_id: &draft_model::MaterialId,
) -> Result<&'a Material, TimelineCommandError> {
    draft
        .materials
        .iter()
        .find(|material| &material.material_id == material_id)
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::MaterialNotFound {
                material_id: material_id.clone(),
            })
        })
}

fn invalid_retime(segment_id: &SegmentId, reason: impl Into<String>) -> TimelineCommandError {
    TimelineCommandError::new(TimelineCommandErrorKind::InvalidRetime {
        segment_id: segment_id.clone(),
        reason: reason.into(),
    })
}
