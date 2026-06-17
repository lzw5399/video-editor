//! Timeline command validation helpers.

use draft_model::{
    CommandEvent, CommandPayload, CommandState, Draft, Material, MaterialId, MaterialKind,
    Microseconds, Segment, SegmentId, SourceTimerange, TargetTimerange, TimelineCommandResponse,
    TimelineSelection, Track, TrackId, TrackKind, TrimSegmentDirection, validate_draft,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    audio::{add_audio_segment, set_segment_volume, set_track_mute},
    history::{push_undo_snapshot, redo_timeline_edit, undo_timeline_edit},
    snapping::{apply_main_track_magnet, apply_snapping, snap_trim_boundary},
    text::{add_text_segment, edit_text_segment},
};

pub fn checked_source_end(
    timerange: &SourceTimerange,
) -> Result<Microseconds, TimelineCommandError> {
    checked_timerange_end(
        "sourceTimerange",
        "sourceTimerange.duration",
        timerange.start,
        timerange.duration,
    )
}

pub fn checked_target_end(
    timerange: &TargetTimerange,
) -> Result<Microseconds, TimelineCommandError> {
    checked_timerange_end(
        "targetTimerange",
        "targetTimerange.duration",
        timerange.start,
        timerange.duration,
    )
}

pub fn target_ranges_overlap(
    first: &TargetTimerange,
    second: &TargetTimerange,
) -> Result<bool, TimelineCommandError> {
    let first_end = checked_target_end(first)?;
    let second_end = checked_target_end(second)?;
    Ok(first.start.get() < second_end.get() && second.start.get() < first_end.get())
}

pub fn validate_timeline_rules(draft: &Draft) -> Result<(), TimelineCommandError> {
    validate_timeranges(draft)?;
    validate_track_material_rules(draft)?;
    validate_segment_material_bounds(draft)?;
    validate_track_overlaps(draft)?;
    validate_draft(draft)?;
    Ok(())
}

pub fn validate_segment_material_bounds(draft: &Draft) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        for segment in &track.segments {
            let material = find_material(draft, &segment.material_id)?;
            if let Some(material_duration) = material.metadata.duration {
                let source_end = checked_source_end(&segment.source_timerange)?;
                if source_end.get() > material_duration.get() {
                    return Err(TimelineCommandError::new(
                        TimelineCommandErrorKind::SourceRangeExceedsMaterialDuration {
                            segment_id: segment.segment_id.clone(),
                            material_id: material.material_id.clone(),
                            source_end,
                            material_duration,
                        },
                    ));
                }
            }
        }
    }

    Ok(())
}

pub fn validate_track_material_compatibility(
    track: &Track,
    material: &Material,
) -> Result<(), TimelineCommandError> {
    if track_accepts_material(track.kind, material.kind) {
        return Ok(());
    }

    Err(TimelineCommandError::new(
        TimelineCommandErrorKind::IncompatibleTrackMaterialKind {
            track_id: track.track_id.clone(),
            track_kind: track.kind,
            material_id: material.material_id.clone(),
            material_kind: material.kind,
        },
    ))
}

pub fn validate_track_unlocked(track: &Track) -> Result<(), TimelineCommandError> {
    if track.locked {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::LockedTrack {
                track_id: track.track_id.clone(),
            },
        ));
    }

    Ok(())
}

pub fn visual_track_stack_order(draft: &Draft) -> Vec<TrackId> {
    draft
        .tracks
        .iter()
        .filter(|track| is_visual_track(track.kind))
        .map(|track| track.track_id.clone())
        .collect()
}

pub fn audio_track_mix_order(draft: &Draft) -> Vec<TrackId> {
    draft
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio)
        .map(|track| track.track_id.clone())
        .collect()
}

pub fn main_video_track_id(draft: &Draft) -> Option<TrackId> {
    draft
        .tracks
        .iter()
        .find(|track| track.kind == TrackKind::Video)
        .map(|track| track.track_id.clone())
}

pub fn execute_timeline_edit(
    payload: CommandPayload,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    match payload {
        CommandPayload::AddSegment(payload) => add_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.segment_id,
            payload.material_id,
            payload.source_timerange,
            payload.target_timerange,
        ),
        CommandPayload::SelectTimelineSegments(payload) => select_timeline_segments(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_ids,
            payload.track_ids,
        ),
        CommandPayload::MoveSegment(payload) => move_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.target_track_id,
            payload.target_start,
        ),
        CommandPayload::SplitSegment(payload) => split_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.right_segment_id,
            payload.split_at,
        ),
        CommandPayload::TrimSegment(payload) => trim_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.direction,
            payload.target_timerange,
        ),
        CommandPayload::DeleteSegment(payload) => delete_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
        ),
        CommandPayload::UndoTimelineEdit(payload) => {
            undo_timeline_edit(&payload.draft, &payload.command_state, &payload.selection)
        }
        CommandPayload::RedoTimelineEdit(payload) => {
            redo_timeline_edit(&payload.draft, &payload.command_state, &payload.selection)
        }
        CommandPayload::AddTextSegment(payload) => add_text_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.segment_id,
            payload.material_id,
            payload.source_timerange,
            payload.target_timerange,
            payload.text,
        ),
        CommandPayload::EditTextSegment(payload) => edit_text_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.text,
        ),
        CommandPayload::AddAudioSegment(payload) => add_audio_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.segment_id,
            payload.material_id,
            payload.source_timerange,
            payload.target_timerange,
        ),
        CommandPayload::SetSegmentVolume(payload) => set_segment_volume(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.volume,
        ),
        CommandPayload::SetTrackMute(payload) => set_track_mute(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.muted,
        ),
        other => Err(TimelineCommandError::new(
            TimelineCommandErrorKind::UnsupportedCommand {
                command: format!("{:?}", other.command_name()),
            },
        )),
    }
}

pub fn add_segment(
    draft: &Draft,
    command_state: &CommandState,
    _selection: &TimelineSelection,
    track_id: TrackId,
    segment_id: SegmentId,
    material_id: MaterialId,
    source_timerange: SourceTimerange,
    target_timerange: TargetTimerange,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let track_index = find_track_index(&next_draft, &track_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    let material = find_material(&next_draft, &material_id)?.clone();
    validate_track_material_compatibility(&next_draft.tracks[track_index], &material)?;

    next_draft.tracks[track_index].segments.push(Segment::new(
        segment_id.clone(),
        material_id,
        source_timerange,
        target_timerange,
    ));
    validate_timeline_rules(&next_draft)?;

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, _selection, "addSegment"),
        TimelineSelection {
            segment_ids: vec![segment_id],
            track_ids: vec![track_id],
        },
        "segmentAdded",
    ))
}

pub fn select_timeline_segments(
    draft: &Draft,
    command_state: &CommandState,
    _selection: &TimelineSelection,
    segment_ids: Vec<SegmentId>,
    track_ids: Vec<TrackId>,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    for track_id in &track_ids {
        find_track_index(draft, track_id)?;
    }
    for segment_id in &segment_ids {
        find_segment_location(draft, segment_id)?;
    }

    Ok(response(
        draft.clone(),
        command_state.clone(),
        TimelineSelection {
            segment_ids,
            track_ids,
        },
        "timelineSelectionChanged",
    ))
}

pub fn move_segment(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    target_track_id: TrackId,
    target_start: Microseconds,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (source_track_index, source_segment_index) =
        find_segment_location(&next_draft, &segment_id)?;
    let target_track_index = find_track_index(&next_draft, &target_track_id)?;

    validate_track_unlocked(&next_draft.tracks[source_track_index])?;
    if target_track_index != source_track_index {
        validate_track_unlocked(&next_draft.tracks[target_track_index])?;
    }

    let mut segment = next_draft.tracks[source_track_index].segments[source_segment_index].clone();
    let (snapped_start, snap_event) = apply_snapping(
        &next_draft,
        &target_track_id,
        &segment_id,
        target_start,
        segment.target_timerange.duration,
        &command_state.snapping,
    )?;
    let source_track_id = next_draft.tracks[source_track_index].track_id.clone();
    let mut extra_events = optional_events([snap_event]);
    segment.target_timerange.start = snapped_start;

    if target_track_index == source_track_index {
        next_draft.tracks[source_track_index].segments[source_segment_index] = segment;
    } else {
        let material = find_material(&next_draft, &segment.material_id)?.clone();
        validate_track_material_compatibility(&next_draft.tracks[target_track_index], &material)?;
        next_draft.tracks[source_track_index]
            .segments
            .remove(source_segment_index);
        next_draft.tracks[target_track_index].segments.push(segment);
    }

    if let Some(event) = apply_main_track_magnet(&mut next_draft, &source_track_id)? {
        extra_events.push(event);
    }
    if source_track_id != target_track_id {
        if let Some(event) = apply_main_track_magnet(&mut next_draft, &target_track_id)? {
            extra_events.push(event);
        }
    }
    validate_timeline_rules(&next_draft)?;

    Ok(response_with_events(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "moveSegment"),
        TimelineSelection {
            segment_ids: vec![segment_id],
            track_ids: vec![target_track_id],
        },
        "segmentMoved",
        extra_events,
    )
    .with_selection_fallback(selection))
}

pub fn split_segment(
    draft: &Draft,
    command_state: &CommandState,
    _selection: &TimelineSelection,
    segment_id: SegmentId,
    right_segment_id: SegmentId,
    split_at: Microseconds,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    let original = next_draft.tracks[track_index].segments[segment_index].clone();
    let target_start = original.target_timerange.start.get();
    let target_end = checked_target_end(&original.target_timerange)?.get();
    let split = split_at.get();
    if split <= target_start || split >= target_end {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::InvalidSplitPoint {
                segment_id,
                split_at,
            },
        ));
    }

    let left_duration = Microseconds::new(split - target_start);
    let right_duration = Microseconds::new(target_end - split);
    let right_source_start = original
        .source_timerange
        .start
        .get()
        .checked_add(left_duration.get())
        .map(Microseconds::new)
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                field: "sourceTimerange".to_owned(),
            })
        })?;

    next_draft.tracks[track_index].segments[segment_index]
        .source_timerange
        .duration = left_duration;
    next_draft.tracks[track_index].segments[segment_index]
        .target_timerange
        .duration = left_duration;

    let mut right_segment = original;
    right_segment.segment_id = right_segment_id.clone();
    right_segment.source_timerange = SourceTimerange {
        start: right_source_start,
        duration: right_duration,
    };
    right_segment.target_timerange = TargetTimerange {
        start: split_at,
        duration: right_duration,
    };
    next_draft.tracks[track_index]
        .segments
        .insert(segment_index + 1, right_segment);

    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, _selection, "splitSegment"),
        TimelineSelection {
            segment_ids: vec![segment_id, right_segment_id],
            track_ids: vec![track_id],
        },
        "segmentSplit",
    ))
}

pub fn trim_segment(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    direction: TrimSegmentDirection,
    target_timerange: TargetTimerange,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let (target_timerange, snap_event) = snap_trim_boundary(
        &next_draft,
        &track_id,
        &segment_id,
        direction,
        target_timerange,
        &command_state.snapping,
    )?;
    checked_target_end(&target_timerange)?;

    let original = next_draft.tracks[track_index].segments[segment_index].clone();
    let old_target_start = original.target_timerange.start.get();
    let old_target_end = checked_target_end(&original.target_timerange)?.get();
    let new_target_start = target_timerange.start.get();
    let new_target_end = checked_target_end(&target_timerange)?.get();

    match direction {
        TrimSegmentDirection::Left => {
            if new_target_end != old_target_end {
                return invalid_trim(&segment_id, target_timerange.start);
            }
            let new_source_start = if new_target_start >= old_target_start {
                let source_delta = new_target_start - old_target_start;
                original
                    .source_timerange
                    .start
                    .get()
                    .checked_add(source_delta)
                    .map(Microseconds::new)
                    .ok_or_else(|| {
                        TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                            field: "sourceTimerange".to_owned(),
                        })
                    })?
            } else {
                let source_delta = old_target_start - new_target_start;
                original
                    .source_timerange
                    .start
                    .get()
                    .checked_sub(source_delta)
                    .map(Microseconds::new)
                    .ok_or_else(|| {
                        TimelineCommandError::new(TimelineCommandErrorKind::InvalidSplitPoint {
                            segment_id: segment_id.clone(),
                            split_at: target_timerange.start,
                        })
                    })?
            };
            next_draft.tracks[track_index].segments[segment_index].source_timerange =
                SourceTimerange {
                    start: new_source_start,
                    duration: target_timerange.duration,
                };
        }
        TrimSegmentDirection::Right => {
            if new_target_start != old_target_start {
                return invalid_trim(&segment_id, target_timerange.start);
            }
            next_draft.tracks[track_index].segments[segment_index]
                .source_timerange
                .duration = target_timerange.duration;
        }
    }

    next_draft.tracks[track_index].segments[segment_index].target_timerange = target_timerange;
    let mut extra_events = optional_events([snap_event]);
    if let Some(event) = apply_main_track_magnet(&mut next_draft, &track_id)? {
        extra_events.push(event);
    }
    validate_timeline_rules(&next_draft)?;

    Ok(response_with_events(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "trimSegment"),
        TimelineSelection {
            segment_ids: vec![segment_id],
            track_ids: if selection.track_ids.is_empty() {
                vec![track_id]
            } else {
                selection.track_ids.clone()
            },
        },
        "segmentTrimmed",
        extra_events,
    ))
}

pub fn delete_segment(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    let track_id = next_draft.tracks[track_index].track_id.clone();

    next_draft.tracks[track_index]
        .segments
        .remove(segment_index);
    let extra_events = optional_events([apply_main_track_magnet(&mut next_draft, &track_id)?]);
    validate_timeline_rules(&next_draft)?;

    let mut next_selection = selection.clone();
    next_selection
        .segment_ids
        .retain(|selected| selected != &segment_id);

    Ok(response_with_events(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "deleteSegment"),
        next_selection,
        "segmentDeleted",
        extra_events,
    ))
}

fn checked_timerange_end(
    field: &str,
    duration_field: &str,
    start: Microseconds,
    duration: Microseconds,
) -> Result<Microseconds, TimelineCommandError> {
    if duration.get() == 0 {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::ZeroDuration {
                field: duration_field.to_owned(),
            },
        ));
    }

    start
        .get()
        .checked_add(duration.get())
        .map(Microseconds::new)
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                field: field.to_owned(),
            })
        })
}

fn response(
    draft: Draft,
    command_state: impl Into<CommandStateWithEvents>,
    selection: TimelineSelection,
    event_kind: &str,
) -> TimelineCommandResponse {
    response_with_events(draft, command_state, selection, event_kind, Vec::new())
}

fn response_with_events(
    draft: Draft,
    command_state: impl Into<CommandStateWithEvents>,
    selection: TimelineSelection,
    event_kind: &str,
    extra_events: Vec<CommandEvent>,
) -> TimelineCommandResponse {
    let command_state = command_state.into();
    let mut events = vec![CommandEvent {
        kind: event_kind.to_owned(),
        message: None,
    }];
    events.extend(extra_events);
    events.extend(command_state.events);
    TimelineCommandResponse {
        draft,
        command_state: command_state.state,
        selection,
        events,
    }
}

fn optional_events<const N: usize>(events: [Option<CommandEvent>; N]) -> Vec<CommandEvent> {
    events.into_iter().flatten().collect()
}

struct CommandStateWithEvents {
    state: CommandState,
    events: Vec<CommandEvent>,
}

impl From<CommandState> for CommandStateWithEvents {
    fn from(state: CommandState) -> Self {
        Self {
            state,
            events: Vec::new(),
        }
    }
}

fn command_state_after_commit(
    command_state: &CommandState,
    draft: &Draft,
    selection: &TimelineSelection,
    label: &str,
) -> CommandStateWithEvents {
    let (state, pruned) = push_undo_snapshot(command_state, draft, selection, label);
    let events = if pruned {
        vec![CommandEvent {
            kind: "historyLimitPruned".to_owned(),
            message: None,
        }]
    } else {
        Vec::new()
    };
    CommandStateWithEvents { state, events }
}

trait ResponseSelectionFallback {
    fn with_selection_fallback(self, previous: &TimelineSelection) -> Self;
}

impl ResponseSelectionFallback for TimelineCommandResponse {
    fn with_selection_fallback(mut self, previous: &TimelineSelection) -> Self {
        if self.selection.track_ids.is_empty() {
            self.selection.track_ids = previous.track_ids.clone();
        }
        self
    }
}

fn invalid_trim<T>(
    segment_id: &SegmentId,
    split_at: Microseconds,
) -> Result<T, TimelineCommandError> {
    Err(TimelineCommandError::new(
        TimelineCommandErrorKind::InvalidSplitPoint {
            segment_id: segment_id.clone(),
            split_at,
        },
    ))
}

fn validate_timeranges(draft: &Draft) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        for segment in &track.segments {
            checked_source_end(&segment.source_timerange)?;
            checked_target_end(&segment.target_timerange)?;
        }
    }

    Ok(())
}

fn validate_track_material_rules(draft: &Draft) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        for segment in &track.segments {
            let material = find_material(draft, &segment.material_id)?;
            validate_track_material_compatibility(track, material)?;
        }
    }

    Ok(())
}

fn validate_track_overlaps(draft: &Draft) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        for first_index in 0..track.segments.len() {
            for second_index in (first_index + 1)..track.segments.len() {
                let first = &track.segments[first_index];
                let second = &track.segments[second_index];
                if target_ranges_overlap(&first.target_timerange, &second.target_timerange)? {
                    return Err(TimelineCommandError::new(
                        TimelineCommandErrorKind::OverlappingSegment {
                            track_id: track.track_id.clone(),
                            first_segment_id: first.segment_id.clone(),
                            second_segment_id: second.segment_id.clone(),
                        },
                    ));
                }
            }
        }
    }

    Ok(())
}

fn find_material<'a>(
    draft: &'a Draft,
    material_id: &MaterialId,
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

fn find_track_index(draft: &Draft, track_id: &TrackId) -> Result<usize, TimelineCommandError> {
    draft
        .tracks
        .iter()
        .position(|track| &track.track_id == track_id)
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::TrackNotFound {
                track_id: track_id.clone(),
            })
        })
}

fn find_segment_location(
    draft: &Draft,
    segment_id: &SegmentId,
) -> Result<(usize, usize), TimelineCommandError> {
    draft
        .tracks
        .iter()
        .enumerate()
        .find_map(|(track_index, track)| {
            track
                .segments
                .iter()
                .position(|segment| &segment.segment_id == segment_id)
                .map(|segment_index| (track_index, segment_index))
        })
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::SegmentNotFound {
                segment_id: segment_id.clone(),
            })
        })
}

fn track_accepts_material(track_kind: TrackKind, material_kind: MaterialKind) -> bool {
    match track_kind {
        TrackKind::Video => matches!(material_kind, MaterialKind::Video | MaterialKind::Image),
        TrackKind::Audio => material_kind == MaterialKind::Audio,
        TrackKind::Text => material_kind == MaterialKind::Text,
        TrackKind::Sticker => material_kind == MaterialKind::Sticker,
        TrackKind::Filter => matches!(material_kind, MaterialKind::Video | MaterialKind::Image),
    }
}

fn is_visual_track(kind: TrackKind) -> bool {
    matches!(
        kind,
        TrackKind::Video | TrackKind::Text | TrackKind::Sticker | TrackKind::Filter
    )
}
