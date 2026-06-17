//! Semantic text/subtitle timeline commands.

use draft_model::{
    CommandEvent, CommandState, Draft, Material, MaterialId, MaterialKind, Segment, SegmentId,
    SourceTimerange, TargetTimerange, TextSegment, TimelineCommandResponse, TimelineSelection,
    TrackId,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    history::push_undo_snapshot,
    timeline::{validate_timeline_rules, validate_track_unlocked},
};

pub fn add_text_segment(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    track_id: TrackId,
    segment_id: SegmentId,
    material_id: MaterialId,
    source_timerange: SourceTimerange,
    target_timerange: TargetTimerange,
    text: TextSegment,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let track_index = find_track_index(&next_draft, &track_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    ensure_text_material(&mut next_draft, &material_id);

    let mut segment = Segment::new(
        segment_id.clone(),
        material_id,
        source_timerange,
        target_timerange,
    );
    segment.text = Some(text);
    next_draft.tracks[track_index].segments.push(segment);

    validate_timeline_rules(&next_draft)?;

    Ok(response(
        next_draft,
        command_state,
        draft,
        selection,
        TimelineSelection {
            segment_ids: vec![segment_id],
            track_ids: vec![track_id],
        },
        "addTextSegment",
        "textSegmentAdded",
    ))
}

pub fn edit_text_segment(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    text: TextSegment,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    if next_draft.tracks[track_index].segments[segment_index]
        .text
        .is_none()
    {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::DraftValidationFailed {
                message: format!("segment {} has no text semantic data", segment_id.as_str()),
            },
        ));
    }

    next_draft.tracks[track_index].segments[segment_index].text = Some(text);
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();

    Ok(response(
        next_draft,
        command_state,
        draft,
        selection,
        TimelineSelection {
            segment_ids: vec![segment_id],
            track_ids: vec![track_id],
        },
        "editTextSegment",
        "textSegmentEdited",
    ))
}

fn ensure_text_material(draft: &mut Draft, material_id: &MaterialId) {
    if draft
        .materials
        .iter()
        .any(|material| &material.material_id == material_id)
    {
        return;
    }

    draft.materials.push(Material::new(
        material_id.clone(),
        MaterialKind::Text,
        format!("text://{}", material_id.as_str()),
        material_id.as_str(),
    ));
}

fn response(
    draft: Draft,
    command_state: &CommandState,
    previous_draft: &Draft,
    previous_selection: &TimelineSelection,
    selection: TimelineSelection,
    history_label: &str,
    event_kind: &str,
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
        selection,
        events,
    }
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
