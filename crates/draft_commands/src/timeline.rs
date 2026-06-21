//! Timeline command validation helpers.

use draft_model::{
    CommandDelta, CommandEvent, CommandName, CommandPayload, CommandState, Draft, Material,
    MaterialId, MaterialKind, Microseconds, Segment, SegmentId, SourceTimerange, TargetTimerange,
    TextSegment, TimelineCommandResponse, TimelineSelection, Track, TrackId, TrackKind,
    TrimSegmentDirection, validate_draft,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    audio::{add_audio_segment, set_segment_volume, set_track_mute, update_segment_audio},
    canvas::update_draft_canvas_config,
    delta::{
        current_range, moved_segment_delta, previous_range, segment_delta,
        segment_with_canvas_delta, split_segment_delta, track_delta, track_visibility_delta,
    },
    history::{push_undo_snapshot, redo_timeline_edit, undo_timeline_edit},
    keyframe::{remove_segment_keyframe, set_segment_keyframe},
    snapping::{apply_main_track_magnet, apply_snapping, snap_trim_boundary},
    text::{add_text_segment, edit_text_segment, import_subtitle_srt, import_subtitle_srt_intent},
    visual::update_segment_visual,
};

const DEFAULT_INTENT_SEGMENT_DURATION_US: u64 = 3_000_000;

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
        .filter(|track| is_visual_track(track.kind) && track.visible)
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
        CommandPayload::AddTimelineSegmentIntent(payload) => add_timeline_segment_intent(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.material_id,
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
        CommandPayload::MoveSelectedSegmentIntent(payload) => move_selected_segment_intent(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.delta,
        ),
        CommandPayload::SplitSegment(payload) => split_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.right_segment_id,
            payload.split_at,
        ),
        CommandPayload::SplitSelectedSegmentIntent(payload) => split_selected_segment_intent(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
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
        CommandPayload::TrimSelectedSegmentIntent(payload) => trim_selected_segment_intent(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.direction,
            payload.delta,
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
        CommandPayload::AddTextSegmentIntent(payload) => add_text_segment_intent(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.text,
            payload.duration,
        ),
        CommandPayload::EditTextSegment(payload) => edit_text_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.text,
        ),
        CommandPayload::ImportSubtitleSrt(payload) => import_subtitle_srt(payload),
        CommandPayload::ImportSubtitleSrtIntent(payload) => import_subtitle_srt_intent(payload),
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
        CommandPayload::AddAudioSegmentIntent(payload) => add_audio_segment_intent(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.material_id,
            payload.duration,
        ),
        CommandPayload::SetSegmentVolume(payload) => set_segment_volume(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.volume,
        ),
        CommandPayload::UpdateSegmentAudio(payload) => update_segment_audio(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.gain_millis,
            payload
                .pan_balance_millis
                .map(|pan_balance| pan_balance.balance_millis),
            payload
                .fade_in_duration
                .map(|fade_in_duration| fade_in_duration.duration),
            payload
                .fade_out_duration
                .map(|fade_out_duration| fade_out_duration.duration),
            payload.effect_slots,
        ),
        CommandPayload::AddTrack(payload) => add_track(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.track_kind,
            payload.name,
        ),
        CommandPayload::AddTrackIntent(payload) => add_track_intent(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_kind,
        ),
        CommandPayload::RenameTrack(payload) => rename_track(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.name,
        ),
        CommandPayload::SetTrackLock(payload) => set_track_lock(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.locked,
        ),
        CommandPayload::SetTrackVisibility(payload) => set_track_visibility(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.visible,
        ),
        CommandPayload::SetTrackMute(payload) => set_track_mute(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.muted,
        ),
        CommandPayload::UpdateDraftCanvasConfig(payload) => update_draft_canvas_config(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.canvas_config,
        ),
        CommandPayload::UpdateSegmentVisual(payload) => update_segment_visual(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.visual,
        ),
        CommandPayload::SetSegmentKeyframe(payload) => set_segment_keyframe(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.keyframe,
        ),
        CommandPayload::RemoveSegmentKeyframe(payload) => remove_segment_keyframe(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.segment_id,
            payload.property,
            payload.at,
        ),
        other => Err(TimelineCommandError::new(
            TimelineCommandErrorKind::UnsupportedCommand {
                command: format!("{:?}", other.command_name()),
            },
        )),
    }
}

pub fn add_track(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    track_id: TrackId,
    track_kind: TrackKind,
    name: String,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    next_draft
        .tracks
        .push(Track::new(track_id.clone(), track_kind, name));
    validate_timeline_rules(&next_draft)?;

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "addTrack"),
        TimelineSelection {
            segment_ids: Vec::new(),
            track_ids: vec![track_id.clone()],
        },
        "trackAdded",
        track_delta(CommandName::AddTrack, &track_id, "track added"),
    ))
}

pub fn add_track_intent(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    track_kind: TrackKind,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let same_kind_count = draft
        .tracks
        .iter()
        .filter(|track| track.kind == track_kind)
        .count();
    add_track(
        draft,
        command_state,
        selection,
        next_track_id(draft, track_kind),
        track_kind,
        default_track_name(track_kind, same_kind_count.saturating_add(1)),
    )
}

pub fn rename_track(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    track_id: TrackId,
    name: String,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let track_index = find_track_index(&next_draft, &track_id)?;
    next_draft.tracks[track_index].name = name;
    validate_timeline_rules(&next_draft)?;

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "renameTrack"),
        TimelineSelection {
            segment_ids: selection.segment_ids.clone(),
            track_ids: vec![track_id.clone()],
        },
        "trackRenamed",
        track_delta(CommandName::RenameTrack, &track_id, "track renamed"),
    ))
}

pub fn set_track_lock(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    track_id: TrackId,
    locked: bool,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let track_index = find_track_index(&next_draft, &track_id)?;
    next_draft.tracks[track_index].locked = locked;
    validate_timeline_rules(&next_draft)?;

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "setTrackLock"),
        TimelineSelection {
            segment_ids: selection.segment_ids.clone(),
            track_ids: vec![track_id.clone()],
        },
        "trackLockChanged",
        track_delta(CommandName::SetTrackLock, &track_id, "track lock changed"),
    ))
}

pub fn set_track_visibility(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    track_id: TrackId,
    visible: bool,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let track_index = find_track_index(&next_draft, &track_id)?;
    if !is_visual_track(next_draft.tracks[track_index].kind) {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::InvalidTrackOperation {
                track_id,
                reason: "visibility is only supported for visual tracks".to_owned(),
            },
        ));
    }
    next_draft.tracks[track_index].visible = visible;
    validate_timeline_rules(&next_draft)?;
    let track_segments = next_draft.tracks[track_index].segments.clone();

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "setTrackVisibility"),
        TimelineSelection {
            segment_ids: selection.segment_ids.clone(),
            track_ids: vec![track_id.clone()],
        },
        "trackVisibilityChanged",
        track_visibility_delta(&track_id, &track_segments),
    ))
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

    let auto_adapted_canvas = should_auto_adapt_canvas(&next_draft)
        .then(|| {
            crate::canvas::first_visual_material_canvas_config(&next_draft.canvas_config, &material)
        })
        .flatten();
    if let Some(canvas_config) = auto_adapted_canvas.clone() {
        next_draft.canvas_config = canvas_config;
    }

    let segment = Segment::new(
        segment_id.clone(),
        material_id.clone(),
        source_timerange,
        target_timerange.clone(),
    );
    next_draft.tracks[track_index].segments.push(segment);
    validate_timeline_rules(&next_draft)?;
    let segment = next_draft.tracks[track_index]
        .segments
        .last()
        .expect("segment was just pushed");
    let delta = if auto_adapted_canvas.is_some() {
        segment_with_canvas_delta(
            &track_id,
            segment,
            &next_draft,
            "segment added and draft canvas auto adapted",
        )
    } else {
        segment_delta(
            CommandName::AddSegment,
            &track_id,
            segment,
            vec![current_range(target_timerange)],
            "segment added",
        )
    };
    let extra_events = if auto_adapted_canvas.is_some() {
        vec![CommandEvent {
            kind: "draftCanvasAutoAdapted".to_owned(),
            message: None,
        }]
    } else {
        Vec::new()
    };

    Ok(response_with_events(
        next_draft,
        command_state_after_commit(command_state, draft, _selection, "addSegment"),
        TimelineSelection {
            segment_ids: vec![segment_id],
            track_ids: vec![track_id],
        },
        "segmentAdded",
        extra_events,
        delta,
    ))
}

pub fn add_timeline_segment_intent(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    material_id: MaterialId,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let material = find_material(draft, &material_id)?.clone();
    let track_id = choose_compatible_track(draft, selection, &material)?;
    let track_index = find_track_index(draft, &track_id)?;
    let duration = material
        .metadata
        .duration
        .unwrap_or_else(|| Microseconds::new(DEFAULT_INTENT_SEGMENT_DURATION_US));
    let target_start = track_end(&draft.tracks[track_index])?;
    let segment_id = next_segment_id(draft, "segment");

    add_segment(
        draft,
        command_state,
        selection,
        track_id,
        segment_id,
        material_id,
        SourceTimerange {
            start: Microseconds::ZERO,
            duration,
        },
        TargetTimerange {
            start: target_start,
            duration,
        },
    )
}

pub fn add_text_segment_intent(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    text: TextSegment,
    duration: Microseconds,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let track_id = choose_track_by_kind(draft, selection, TrackKind::Text)?;
    let track_index = find_track_index(draft, &track_id)?;
    let duration = positive_duration(duration);
    add_text_segment(
        draft,
        command_state,
        selection,
        track_id,
        next_segment_id(draft, "text-segment"),
        next_material_id(draft, "text-material"),
        SourceTimerange {
            start: Microseconds::ZERO,
            duration,
        },
        TargetTimerange {
            start: track_end(&draft.tracks[track_index])?,
            duration,
        },
        text,
    )
}

pub fn add_audio_segment_intent(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    material_id: Option<MaterialId>,
    duration: Option<Microseconds>,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let material = match material_id {
        Some(material_id) => find_material(draft, &material_id)?,
        None => draft
            .materials
            .iter()
            .find(|material| material.kind == MaterialKind::Audio)
            .ok_or_else(|| {
                TimelineCommandError::new(TimelineCommandErrorKind::InvalidTrackOperation {
                    track_id: TrackId::from(""),
                    reason: "no audio material for add audio intent".to_owned(),
                })
            })?,
    };
    if material.kind != MaterialKind::Audio {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::IncompatibleTrackMaterialKind {
                track_id: TrackId::from(""),
                track_kind: TrackKind::Audio,
                material_id: material.material_id.clone(),
                material_kind: material.kind,
            },
        ));
    }

    let track_id = choose_track_by_kind(draft, selection, TrackKind::Audio)?;
    let track_index = find_track_index(draft, &track_id)?;
    let duration = positive_duration(
        duration
            .or(material.metadata.duration)
            .unwrap_or_else(|| Microseconds::new(DEFAULT_INTENT_SEGMENT_DURATION_US)),
    );
    add_audio_segment(
        draft,
        command_state,
        selection,
        track_id,
        next_segment_id(draft, "audio-segment"),
        material.material_id.clone(),
        SourceTimerange {
            start: Microseconds::ZERO,
            duration,
        },
        TargetTimerange {
            start: track_end(&draft.tracks[track_index])?,
            duration,
        },
    )
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
        CommandDelta::none(CommandName::SelectTimelineSegments, "selection only"),
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
    let previous_target_timerange = segment.target_timerange.clone();
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
    let current_target_timerange = segment.target_timerange.clone();
    let delta = moved_segment_delta(
        &source_track_id,
        &target_track_id,
        &segment,
        previous_target_timerange,
        current_target_timerange,
    );

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
        delta,
    )
    .with_selection_fallback(selection))
}

pub fn move_selected_segment_intent(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    delta: Microseconds,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let segment_id = selected_segment_id(selection)?;
    let (track_index, segment_index) = find_segment_location(draft, &segment_id)?;
    let segment = &draft.tracks[track_index].segments[segment_index];
    let target_start = Microseconds::new(
        segment
            .target_timerange
            .start
            .get()
            .saturating_add(delta.get()),
    );

    move_segment(
        draft,
        command_state,
        selection,
        segment_id,
        draft.tracks[track_index].track_id.clone(),
        target_start,
    )
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
    let original_target_timerange = original.target_timerange.clone();
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
    let delta = split_segment_delta(
        &track_id,
        &next_draft.tracks[track_index].segments[segment_index],
        &right_segment_id,
        original_target_timerange,
    );

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, _selection, "splitSegment"),
        TimelineSelection {
            segment_ids: vec![segment_id, right_segment_id],
            track_ids: vec![track_id],
        },
        "segmentSplit",
        delta,
    ))
}

pub fn split_selected_segment_intent(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    split_at: Microseconds,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let segment_id = selected_segment_id(selection)?;
    let right_segment_id = next_segment_id(draft, "segment-right");
    split_segment(
        draft,
        command_state,
        selection,
        segment_id,
        right_segment_id,
        split_at,
    )
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
    let previous_target_timerange = original.target_timerange.clone();
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
    let current_segment = next_draft.tracks[track_index].segments[segment_index].clone();
    let delta = segment_delta(
        CommandName::TrimSegment,
        &track_id,
        &current_segment,
        vec![
            previous_range(previous_target_timerange),
            current_range(current_segment.target_timerange.clone()),
        ],
        "segment trimmed",
    );
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
        delta,
    ))
}

pub fn trim_selected_segment_intent(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    direction: TrimSegmentDirection,
    delta: Microseconds,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let segment_id = selected_segment_id(selection)?;
    let (track_index, segment_index) = find_segment_location(draft, &segment_id)?;
    let segment = &draft.tracks[track_index].segments[segment_index];
    let current = &segment.target_timerange;
    let max_delta = current.duration.get().saturating_sub(1);
    let applied_delta = delta.get().min(max_delta);
    let target_timerange = match direction {
        TrimSegmentDirection::Left => {
            let old_end = checked_target_end(current)?;
            let new_start = Microseconds::new(current.start.get().saturating_add(applied_delta));
            TargetTimerange {
                start: new_start,
                duration: Microseconds::new(old_end.get().saturating_sub(new_start.get()).max(1)),
            }
        }
        TrimSegmentDirection::Right => TargetTimerange {
            start: current.start,
            duration: Microseconds::new(
                current.duration.get().saturating_sub(applied_delta).max(1),
            ),
        },
    };

    trim_segment(
        draft,
        command_state,
        selection,
        segment_id,
        direction,
        target_timerange,
    )
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
    let removed_segment = next_draft.tracks[track_index].segments[segment_index].clone();
    let delta = segment_delta(
        CommandName::DeleteSegment,
        &track_id,
        &removed_segment,
        vec![previous_range(removed_segment.target_timerange.clone())],
        "segment deleted",
    );

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
        delta,
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
    delta: CommandDelta,
) -> TimelineCommandResponse {
    response_with_events(
        draft,
        command_state,
        selection,
        event_kind,
        Vec::new(),
        delta,
    )
}

fn response_with_events(
    draft: Draft,
    command_state: impl Into<CommandStateWithEvents>,
    selection: TimelineSelection,
    event_kind: &str,
    extra_events: Vec<CommandEvent>,
    delta: CommandDelta,
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
        delta,
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

fn selected_segment_id(selection: &TimelineSelection) -> Result<SegmentId, TimelineCommandError> {
    selection.segment_ids.first().cloned().ok_or_else(|| {
        TimelineCommandError::new(TimelineCommandErrorKind::InvalidTrackOperation {
            track_id: TrackId::from(""),
            reason: "no selected segment for timeline intent".to_owned(),
        })
    })
}

fn choose_compatible_track(
    draft: &Draft,
    selection: &TimelineSelection,
    material: &Material,
) -> Result<TrackId, TimelineCommandError> {
    if let Some(track_id) = selection
        .track_ids
        .iter()
        .filter_map(|track_id| {
            draft.tracks.iter().find(|track| {
                &track.track_id == track_id && track_accepts_material(track.kind, material.kind)
            })
        })
        .map(|track| track.track_id.clone())
        .next()
    {
        return Ok(track_id);
    }

    draft
        .tracks
        .iter()
        .find(|track| track_accepts_material(track.kind, material.kind))
        .map(|track| track.track_id.clone())
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::InvalidTrackOperation {
                track_id: TrackId::from(""),
                reason: format!(
                    "no compatible track for material {} ({:?})",
                    material.material_id.as_str(),
                    material.kind
                ),
            })
        })
}

fn choose_track_by_kind(
    draft: &Draft,
    selection: &TimelineSelection,
    track_kind: TrackKind,
) -> Result<TrackId, TimelineCommandError> {
    if let Some(track_id) = selection
        .track_ids
        .iter()
        .filter_map(|track_id| {
            draft
                .tracks
                .iter()
                .find(|track| &track.track_id == track_id && track.kind == track_kind)
        })
        .map(|track| track.track_id.clone())
        .next()
    {
        return Ok(track_id);
    }

    draft
        .tracks
        .iter()
        .find(|track| track.kind == track_kind)
        .map(|track| track.track_id.clone())
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::InvalidTrackOperation {
                track_id: TrackId::from(""),
                reason: format!("no {:?} track for timeline intent", track_kind),
            })
        })
}

fn track_end(track: &Track) -> Result<Microseconds, TimelineCommandError> {
    track
        .segments
        .iter()
        .try_fold(Microseconds::ZERO, |end, segment| {
            checked_target_end(&segment.target_timerange)
                .map(|segment_end| Microseconds::new(end.get().max(segment_end.get())))
        })
}

fn next_segment_id(draft: &Draft, prefix: &str) -> SegmentId {
    let mut ordinal = draft
        .tracks
        .iter()
        .map(|track| track.segments.len())
        .sum::<usize>()
        .saturating_add(1);
    loop {
        let candidate = SegmentId::from(format!("{prefix}-{ordinal}"));
        let exists = draft.tracks.iter().any(|track| {
            track
                .segments
                .iter()
                .any(|segment| segment.segment_id == candidate)
        });
        if !exists {
            return candidate;
        }
        ordinal = ordinal.saturating_add(1);
    }
}

fn next_material_id(draft: &Draft, prefix: &str) -> MaterialId {
    let mut ordinal = draft.materials.len().saturating_add(1);
    loop {
        let candidate = MaterialId::from(format!("{prefix}-{ordinal}"));
        if !draft
            .materials
            .iter()
            .any(|material| material.material_id == candidate)
        {
            return candidate;
        }
        ordinal = ordinal.saturating_add(1);
    }
}

fn next_track_id(draft: &Draft, track_kind: TrackKind) -> TrackId {
    let prefix = match track_kind {
        TrackKind::Video => "track-video",
        TrackKind::Audio => "track-audio",
        TrackKind::Text => "track-text",
        TrackKind::Sticker => "track-sticker",
        TrackKind::Filter => "track-filter",
    };
    let mut ordinal = draft.tracks.len().saturating_add(1);
    loop {
        let candidate = TrackId::from(format!("{prefix}-{ordinal}"));
        if !draft.tracks.iter().any(|track| track.track_id == candidate) {
            return candidate;
        }
        ordinal = ordinal.saturating_add(1);
    }
}

fn default_track_name(kind: TrackKind, index: usize) -> String {
    let label = match kind {
        TrackKind::Video => "视频轨道",
        TrackKind::Audio => "音频轨道",
        TrackKind::Text => "文字轨道",
        TrackKind::Sticker => "贴纸轨道",
        TrackKind::Filter => "滤镜轨道",
    };
    format!("{label} {index}")
}

fn positive_duration(duration: Microseconds) -> Microseconds {
    Microseconds::new(duration.get().max(1))
}

pub(crate) fn find_segment_location(
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

fn should_auto_adapt_canvas(draft: &Draft) -> bool {
    !draft
        .tracks
        .iter()
        .filter(|track| is_visual_track(track.kind))
        .any(|track| !track.segments.is_empty())
}
