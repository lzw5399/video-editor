//! Segment-level visual command semantics.

use draft_model::{
    CommandDeltaName, CommandEvent, CommandState, Draft, SegmentId, SegmentVisual,
    TimelineCommandResponse, TimelineSelection,
};

use crate::{
    TimelineCommandError,
    delta::visual_segment_delta,
    history::push_undo_snapshot,
    timeline::{find_segment_location, validate_timeline_rules, validate_track_unlocked},
};

pub fn update_segment_visual(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    visual: SegmentVisual,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    next_draft.tracks[track_index].segments[segment_index].visual = visual;
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let delta = visual_segment_delta(
        CommandDeltaName::UpdateSegmentVisual,
        &track_id,
        &next_draft.tracks[track_index].segments[segment_index],
        "segment visual updated",
    );

    let (command_state, pruned) =
        push_undo_snapshot(command_state, draft, selection, "updateSegmentVisual");
    let mut events = vec![CommandEvent {
        kind: "segmentVisualUpdated".to_owned(),
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
