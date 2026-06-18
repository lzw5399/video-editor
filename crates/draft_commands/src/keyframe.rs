//! Segment-level keyframe command semantics.

use draft_model::{
    CommandDelta, CommandEvent, CommandName, CommandState, Draft, Keyframe, KeyframeProperty,
    Microseconds, SegmentId, TimelineCommandResponse, TimelineSelection,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    delta::keyframe_delta,
    history::push_undo_snapshot,
    timeline::{find_segment_location, validate_timeline_rules, validate_track_unlocked},
};

pub fn set_segment_keyframe(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    keyframe: Keyframe,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    let property = keyframe.property.clone();
    let at = keyframe.at;

    let segment_keyframes = &mut next_draft.tracks[track_index].segments[segment_index].keyframes;
    if let Some(existing) = segment_keyframes
        .iter_mut()
        .find(|existing| existing.property == keyframe.property && existing.at == keyframe.at)
    {
        *existing = keyframe;
    } else {
        segment_keyframes.push(keyframe);
    }
    sort_keyframes(segment_keyframes);
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let delta = keyframe_delta(
        CommandName::SetSegmentKeyframe,
        &track_id,
        &next_draft.tracks[track_index].segments[segment_index],
        property,
        at,
        "segment keyframe set",
    );

    Ok(response(
        next_draft,
        command_state,
        draft,
        selection,
        "setSegmentKeyframe",
        "segmentKeyframeSet",
        CommandName::SetSegmentKeyframe,
        delta,
    ))
}

pub fn remove_segment_keyframe(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    property: KeyframeProperty,
    at: Microseconds,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    let segment_keyframes = &mut next_draft.tracks[track_index].segments[segment_index].keyframes;
    let Some(index) = segment_keyframes
        .iter()
        .position(|existing| existing.property == property && existing.at == at)
    else {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::DraftValidationFailed {
                message: format!(
                    "keyframe {:?} at {} not found on segment {:?}",
                    property,
                    at.get(),
                    segment_id
                ),
            },
        ));
    };

    segment_keyframes.remove(index);
    sort_keyframes(segment_keyframes);
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let delta = keyframe_delta(
        CommandName::RemoveSegmentKeyframe,
        &track_id,
        &next_draft.tracks[track_index].segments[segment_index],
        property.clone(),
        at,
        "segment keyframe removed",
    );

    Ok(response(
        next_draft,
        command_state,
        draft,
        selection,
        "removeSegmentKeyframe",
        "segmentKeyframeRemoved",
        CommandName::RemoveSegmentKeyframe,
        delta,
    ))
}

fn sort_keyframes(keyframes: &mut [Keyframe]) {
    keyframes.sort_by(|left, right| {
        left.property
            .cmp(&right.property)
            .then(left.at.cmp(&right.at))
    });
}

fn response(
    draft: Draft,
    command_state: &CommandState,
    previous_draft: &Draft,
    previous_selection: &TimelineSelection,
    history_label: &str,
    event_kind: &str,
    _command: CommandName,
    delta: CommandDelta,
) -> TimelineCommandResponse {
    let (command_state, pruned) = push_undo_snapshot(
        command_state,
        previous_draft,
        previous_selection,
        history_label,
    );
    let mut events = vec![CommandEvent {
        kind: event_kind.to_owned(),
        message: None,
    }];
    if pruned {
        events.push(CommandEvent {
            kind: "historyLimitPruned".to_owned(),
            message: None,
        });
    }

    TimelineCommandResponse {
        draft,
        command_state,
        selection: previous_selection.clone(),
        events,
        delta,
    }
}
