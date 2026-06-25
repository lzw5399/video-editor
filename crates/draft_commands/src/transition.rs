//! Rust-owned transition relationship commands.

use std::collections::{BTreeMap, BTreeSet};

use draft_model::{
    ChangedEntity, CommandDelta, CommandDeltaName, CommandEvent, CommandState, DirtyDomain,
    DirtyRange, DirtyRangeSource, Draft, InvalidationScope, Microseconds, Segment, SegmentId,
    TimelineCommandResponse, TimelineSelection, Track, TrackId, TrackKind, TrackTransition,
    TransitionReference,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    history::push_undo_snapshot,
    timeline::{checked_target_end, find_segment_location, validate_timeline_rules},
};

const TRANSITION_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Timing,
    DirtyDomain::Transition,
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const TRANSITION_CONSUMERS: &[DirtyDomain] = &[
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

pub fn add_transition(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    from_segment_id: SegmentId,
    to_segment_id: SegmentId,
    reference: TransitionReference,
    duration: Microseconds,
    parameters: BTreeMap<String, String>,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    reject_external_command_reference(
        draft,
        &from_segment_id,
        &to_segment_id,
        &reference,
        "transition command accepts only first-party transition references",
    )?;

    let mut next_draft = draft.clone();
    let (track_index, from_index, to_index) =
        transition_endpoint_indexes(&next_draft, &from_segment_id, &to_segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    if next_draft.tracks[track_index]
        .transitions
        .iter()
        .any(|transition| {
            transition.from_segment_id == from_segment_id
                && transition.to_segment_id == to_segment_id
        })
    {
        let track_id = next_draft.tracks[track_index].track_id.clone();
        return invalid_transition(
            track_id,
            from_segment_id,
            to_segment_id,
            "transition relationship already exists",
        );
    }

    next_draft.tracks[track_index]
        .transitions
        .push(TrackTransition {
            from_segment_id: from_segment_id.clone(),
            to_segment_id: to_segment_id.clone(),
            reference,
            duration,
            parameters,
        });
    validate_timeline_rules(&next_draft)?;

    let track = &next_draft.tracks[track_index];
    let delta = transition_delta(
        CommandDeltaName::AddTransition,
        track,
        &track.segments[from_index],
        &track.segments[to_index],
        "transition added",
    )?;

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "addTransition"),
        selection.clone(),
        "transitionAdded",
        delta,
    ))
}

pub fn update_transition_duration(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    from_segment_id: SegmentId,
    to_segment_id: SegmentId,
    duration: Microseconds,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, from_index, to_index) =
        transition_endpoint_indexes(&next_draft, &from_segment_id, &to_segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    let transition_index = transition_index(
        &next_draft.tracks[track_index],
        &from_segment_id,
        &to_segment_id,
    )?;

    next_draft.tracks[track_index].transitions[transition_index].duration = duration;
    validate_timeline_rules(&next_draft)?;

    let track = &next_draft.tracks[track_index];
    let delta = transition_delta(
        CommandDeltaName::UpdateTransitionDuration,
        track,
        &track.segments[from_index],
        &track.segments[to_index],
        "transition duration updated",
    )?;

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "updateTransitionDuration"),
        selection.clone(),
        "transitionDurationUpdated",
        delta,
    ))
}

pub fn remove_transition(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    from_segment_id: SegmentId,
    to_segment_id: SegmentId,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, from_index, to_index) =
        transition_endpoint_indexes(&next_draft, &from_segment_id, &to_segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    let transition_index = transition_index(
        &next_draft.tracks[track_index],
        &from_segment_id,
        &to_segment_id,
    )?;

    let track_for_delta = next_draft.tracks[track_index].clone();
    let delta = transition_delta(
        CommandDeltaName::RemoveTransition,
        &track_for_delta,
        &track_for_delta.segments[from_index],
        &track_for_delta.segments[to_index],
        "transition removed",
    )?;
    next_draft.tracks[track_index]
        .transitions
        .remove(transition_index);
    validate_timeline_rules(&next_draft)?;

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "removeTransition"),
        selection.clone(),
        "transitionRemoved",
        delta,
    ))
}

pub fn validate_transition_relationships(draft: &Draft) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        if track.transitions.is_empty() {
            continue;
        }
        if !is_visual_track(track.kind) {
            for transition in &track.transitions {
                return invalid_transition(
                    track.track_id.clone(),
                    transition.from_segment_id.clone(),
                    transition.to_segment_id.clone(),
                    "transitions require a visual track",
                );
            }
        }

        let mut seen = BTreeSet::new();
        for transition in &track.transitions {
            if !seen.insert((
                transition.from_segment_id.clone(),
                transition.to_segment_id.clone(),
            )) {
                return invalid_transition(
                    track.track_id.clone(),
                    transition.from_segment_id.clone(),
                    transition.to_segment_id.clone(),
                    "duplicate transition relationship",
                );
            }
            validate_transition_on_track(track, transition)?;
        }
    }

    Ok(())
}

fn validate_transition_on_track(
    track: &Track,
    transition: &TrackTransition,
) -> Result<(), TimelineCommandError> {
    let from_index = segment_index(track, &transition.from_segment_id).ok_or_else(|| {
        TimelineCommandError::new(TimelineCommandErrorKind::InvalidTransitionRelationship {
            track_id: track.track_id.clone(),
            from_segment_id: transition.from_segment_id.clone(),
            to_segment_id: transition.to_segment_id.clone(),
            reason: "from segment is not present on transition track".to_owned(),
        })
    })?;
    let to_index = segment_index(track, &transition.to_segment_id).ok_or_else(|| {
        TimelineCommandError::new(TimelineCommandErrorKind::InvalidTransitionRelationship {
            track_id: track.track_id.clone(),
            from_segment_id: transition.from_segment_id.clone(),
            to_segment_id: transition.to_segment_id.clone(),
            reason: "to segment is not present on transition track".to_owned(),
        })
    })?;
    let from = &track.segments[from_index];
    let to = &track.segments[to_index];
    let from_end = checked_target_end(&from.target_timerange)?;
    if from_end != to.target_timerange.start {
        return invalid_transition(
            track.track_id.clone(),
            transition.from_segment_id.clone(),
            transition.to_segment_id.clone(),
            format!(
                "segments must be adjacent: from end {} != to start {}",
                from_end.get(),
                to.target_timerange.start.get()
            ),
        );
    }
    if transition.duration.get() == 0 {
        return invalid_transition(
            track.track_id.clone(),
            transition.from_segment_id.clone(),
            transition.to_segment_id.clone(),
            "transition duration must be positive",
        );
    }

    let max_duration = from
        .target_timerange
        .duration
        .get()
        .min(to.target_timerange.duration.get())
        .min(from.source_timerange.duration.get())
        .min(to.source_timerange.duration.get());
    if transition.duration.get() > max_duration {
        return invalid_transition(
            track.track_id.clone(),
            transition.from_segment_id.clone(),
            transition.to_segment_id.clone(),
            format!(
                "transition duration {} exceeds available overlap window {}",
                transition.duration.get(),
                max_duration
            ),
        );
    }

    Ok(())
}

fn reject_external_command_reference(
    draft: &Draft,
    from_segment_id: &SegmentId,
    to_segment_id: &SegmentId,
    reference: &TransitionReference,
    reason: &str,
) -> Result<(), TimelineCommandError> {
    if !matches!(reference, TransitionReference::ExternalReference { .. }) {
        return Ok(());
    }
    let track_id = transition_track_id(draft, from_segment_id, to_segment_id)
        .unwrap_or_else(|| TrackId::from(""));
    invalid_transition(
        track_id,
        from_segment_id.clone(),
        to_segment_id.clone(),
        reason,
    )
}

fn transition_endpoint_indexes(
    draft: &Draft,
    from_segment_id: &SegmentId,
    to_segment_id: &SegmentId,
) -> Result<(usize, usize, usize), TimelineCommandError> {
    let (from_track_index, from_segment_index) = find_segment_location(draft, from_segment_id)?;
    let (to_track_index, to_segment_index) = find_segment_location(draft, to_segment_id)?;
    if from_track_index != to_track_index {
        return invalid_transition(
            draft.tracks[from_track_index].track_id.clone(),
            from_segment_id.clone(),
            to_segment_id.clone(),
            "transition endpoints must be on the same track",
        );
    }

    Ok((from_track_index, from_segment_index, to_segment_index))
}

fn transition_track_id(
    draft: &Draft,
    from_segment_id: &SegmentId,
    to_segment_id: &SegmentId,
) -> Option<TrackId> {
    let (from_track_index, _) = find_segment_location(draft, from_segment_id).ok()?;
    let (to_track_index, _) = find_segment_location(draft, to_segment_id).ok()?;
    (from_track_index == to_track_index).then(|| draft.tracks[from_track_index].track_id.clone())
}

fn transition_index(
    track: &Track,
    from_segment_id: &SegmentId,
    to_segment_id: &SegmentId,
) -> Result<usize, TimelineCommandError> {
    track
        .transitions
        .iter()
        .position(|transition| {
            &transition.from_segment_id == from_segment_id
                && &transition.to_segment_id == to_segment_id
        })
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::InvalidTransitionRelationship {
                track_id: track.track_id.clone(),
                from_segment_id: from_segment_id.clone(),
                to_segment_id: to_segment_id.clone(),
                reason: "transition relationship not found".to_owned(),
            })
        })
}

fn segment_index(track: &Track, segment_id: &SegmentId) -> Option<usize> {
    track
        .segments
        .iter()
        .position(|segment| &segment.segment_id == segment_id)
}

fn transition_delta(
    command: CommandDeltaName,
    track: &Track,
    from: &Segment,
    to: &Segment,
    reason: &'static str,
) -> Result<CommandDelta, TimelineCommandError> {
    let changed_range = from
        .target_timerange
        .union(&to.target_timerange)
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                field: "transition.targetTimerange".to_owned(),
            })
        })?;
    let material_ids = unique_material_ids(from, to);

    Ok(CommandDelta::targeted(
        command,
        vec![
            ChangedEntity::Track {
                track_id: track.track_id.clone(),
            },
            ChangedEntity::Segment {
                track_id: track.track_id.clone(),
                segment_id: from.segment_id.clone(),
            },
            ChangedEntity::Segment {
                track_id: track.track_id.clone(),
                segment_id: to.segment_id.clone(),
            },
            ChangedEntity::Material {
                material_id: from.material_id.clone(),
            },
            ChangedEntity::Material {
                material_id: to.material_id.clone(),
            },
        ],
        TRANSITION_DOMAINS.to_vec(),
        vec![DirtyRange {
            target_timerange: changed_range,
            source: DirtyRangeSource::PreviousAndCurrent,
        }],
        InvalidationScope::targeted(material_ids, TRANSITION_CONSUMERS.to_vec()),
        reason,
    ))
}

fn unique_material_ids(from: &Segment, to: &Segment) -> Vec<draft_model::MaterialId> {
    let mut material_ids = Vec::new();
    material_ids.push(from.material_id.clone());
    if to.material_id != from.material_id {
        material_ids.push(to.material_id.clone());
    }
    material_ids
}

fn response(
    draft: Draft,
    command_state: impl Into<CommandStateWithEvents>,
    selection: TimelineSelection,
    event_kind: &str,
    delta: CommandDelta,
) -> TimelineCommandResponse {
    let command_state = command_state.into();
    let mut events = vec![CommandEvent {
        kind: event_kind.to_owned(),
        message: None,
    }];
    events.extend(command_state.events);

    TimelineCommandResponse {
        draft,
        command_state: command_state.state,
        selection,
        events,
        delta,
    }
}

struct CommandStateWithEvents {
    state: CommandState,
    events: Vec<CommandEvent>,
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

fn validate_track_unlocked(track: &Track) -> Result<(), TimelineCommandError> {
    if track.locked {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::LockedTrack {
                track_id: track.track_id.clone(),
            },
        ));
    }
    Ok(())
}

fn is_visual_track(kind: TrackKind) -> bool {
    matches!(
        kind,
        TrackKind::Video | TrackKind::Text | TrackKind::Sticker | TrackKind::Filter
    )
}

fn invalid_transition<T>(
    track_id: TrackId,
    from_segment_id: SegmentId,
    to_segment_id: SegmentId,
    reason: impl Into<String>,
) -> Result<T, TimelineCommandError> {
    Err(TimelineCommandError::new(
        TimelineCommandErrorKind::InvalidTransitionRelationship {
            track_id,
            from_segment_id,
            to_segment_id,
            reason: reason.into(),
        },
    ))
}
