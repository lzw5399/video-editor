use std::collections::BTreeMap;

use draft_commands::{
    TimelineCommandErrorKind,
    history::{redo_timeline_edit, undo_timeline_edit},
    timeline::{delete_segment, execute_timeline_edit, move_segment, split_segment, trim_segment},
    transition::{add_transition, remove_transition, validate_transition_relationships},
};
use draft_model::{
    AddTransitionCommandPayload, CommandState, DirtyDomain, Draft, MainTrackMagnet, Material,
    MaterialKind, Microseconds, RemoveTransitionCommandPayload, Segment, SegmentId,
    SourceTimerange, TargetTimerange, TimelineEditPayload, TimelineSelection, Track, TrackId,
    TrackKind, TrackTransition, TransitionKind, TransitionReference,
    UpdateTransitionDurationCommandPayload,
};

#[test]
fn transition_commands_phase19_relationship_model_is_adjacent_and_typed() {
    let mut parameters = BTreeMap::new();
    parameters.insert("curve".to_owned(), "linear".to_owned());

    let relationship = TrackTransition {
        from_segment_id: SegmentId::from("left-segment"),
        to_segment_id: SegmentId::from("right-segment"),
        reference: TransitionReference::FirstParty {
            transition: TransitionKind::Dissolve,
        },
        duration: Microseconds::new(500_000),
        parameters: parameters.clone(),
    };

    assert_eq!(
        relationship.from_segment_id,
        SegmentId::from("left-segment")
    );
    assert_eq!(relationship.to_segment_id, SegmentId::from("right-segment"));
    assert!(matches!(
        relationship.reference,
        TransitionReference::FirstParty {
            transition: TransitionKind::Dissolve
        }
    ));
    assert_eq!(relationship.duration, Microseconds::new(500_000));
    assert_eq!(relationship.parameters, parameters);
}

#[test]
fn transition_commands_phase19_track_owns_relationships_not_segment_local_deltas() {
    let mut track = Track::new(TrackId::from("video-1"), TrackKind::Video, "main video");

    track.transitions.push(TrackTransition::dissolve(
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        Microseconds::new(300_000),
    ));

    assert_eq!(track.transitions.len(), 1);
    assert_eq!(
        track.transitions[0].capability_id(),
        TransitionKind::Dissolve.capability_id()
    );
}

#[test]
fn transition_commands_phase19_external_references_are_report_only_not_first_party_kinds() {
    let relationship = TrackTransition::external_reference(
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        "jianying",
        "private-transition-id",
        Microseconds::new(400_000),
    );

    let external = relationship
        .external()
        .expect("provider transition must remain an external reference");
    assert_eq!(external.provider, "jianying");
    assert_eq!(external.effect_id, "private-transition-id");
    assert_eq!(
        relationship.capability_id(),
        "external:jianying:private-transition-id"
    );
    assert!(!matches!(
        relationship.reference,
        TransitionReference::FirstParty { .. }
    ));
}

#[test]
fn transition_commands_phase19_add_update_remove_route_and_commit_once() {
    let draft = draft_with_adjacent_video_segments(1_000_000, 1_000_000);
    let state = CommandState::empty();
    let selection = timeline_selection();

    let added = add_transition(
        &draft,
        &state,
        &selection,
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        TransitionReference::dissolve(),
        Microseconds::new(300_000),
        BTreeMap::new(),
    )
    .expect("adjacent visual segments should accept a dissolve transition");

    assert_eq!(added.events[0].kind, "transitionAdded");
    assert_eq!(added.command_state.undo_stack.len(), 1);
    assert_eq!(
        added.command_state.undo_stack[0].label.as_deref(),
        Some("addTransition")
    );
    assert!(added.delta.changed_domains.contains(&DirtyDomain::Timing));
    assert_eq!(added.draft.tracks[0].transitions.len(), 1);
    assert_eq!(
        added.draft.tracks[0].transitions[0].duration,
        Microseconds::new(300_000)
    );

    let updated = execute_timeline_edit(TimelineEditPayload::UpdateTransitionDuration(
        UpdateTransitionDurationCommandPayload {
            draft: added.draft,
            command_state: added.command_state,
            selection: added.selection,
            from_segment_id: SegmentId::from("left-segment"),
            to_segment_id: SegmentId::from("right-segment"),
            duration: Microseconds::new(400_000),
        },
    ))
    .expect("timeline dispatcher should route transition duration updates");
    assert_eq!(updated.events[0].kind, "transitionDurationUpdated");
    assert_eq!(updated.command_state.undo_stack.len(), 2);
    assert_eq!(
        updated.draft.tracks[0].transitions[0].duration,
        Microseconds::new(400_000)
    );

    let undone = undo_timeline_edit(&updated.draft, &updated.command_state, &updated.selection)
        .expect("transition update should be undoable");
    assert_eq!(
        undone.draft.tracks[0].transitions[0].duration,
        Microseconds::new(300_000)
    );
    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("transition update should be redoable");
    assert_eq!(redone.draft, updated.draft);

    let removed = execute_timeline_edit(TimelineEditPayload::RemoveTransition(
        RemoveTransitionCommandPayload {
            draft: redone.draft,
            command_state: redone.command_state,
            selection: redone.selection,
            from_segment_id: SegmentId::from("left-segment"),
            to_segment_id: SegmentId::from("right-segment"),
        },
    ))
    .expect("timeline dispatcher should route transition removal");
    assert_eq!(removed.events[0].kind, "transitionRemoved");
    assert!(removed.draft.tracks[0].transitions.is_empty());
    assert_eq!(removed.command_state.undo_stack.len(), 3);
}

#[test]
fn transition_commands_phase19_invalid_relationships_reject_atomically() {
    let draft = draft_with_adjacent_video_segments(1_000_000, 1_000_000);
    let state = CommandState::empty();
    let selection = timeline_selection();

    let gap_draft = draft_with_gap_between_video_segments();
    let gap = add_transition(
        &gap_draft,
        &state,
        &selection,
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        TransitionReference::dissolve(),
        Microseconds::new(300_000),
        BTreeMap::new(),
    )
    .expect_err("non-adjacent segments should reject transitions");
    assert!(matches!(
        gap.kind,
        TimelineCommandErrorKind::InvalidTransitionRelationship { .. }
    ));

    let too_long = add_transition(
        &draft,
        &state,
        &selection,
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        TransitionReference::dissolve(),
        Microseconds::new(1_500_000),
        BTreeMap::new(),
    )
    .expect_err("duration beyond available segment windows should reject");
    assert!(matches!(
        too_long.kind,
        TimelineCommandErrorKind::InvalidTransitionRelationship { .. }
    ));

    let mut locked = draft.clone();
    locked.tracks[0].locked = true;
    let locked_error = add_transition(
        &locked,
        &state,
        &selection,
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        TransitionReference::dissolve(),
        Microseconds::new(300_000),
        BTreeMap::new(),
    )
    .expect_err("locked tracks should reject transition edits");
    assert!(matches!(
        locked_error.kind,
        TimelineCommandErrorKind::LockedTrack { .. }
    ));

    assert_eq!(
        draft,
        draft_with_adjacent_video_segments(1_000_000, 1_000_000)
    );
    assert_eq!(state, CommandState::empty());
}

#[test]
fn transition_commands_phase19_segment_edits_preserve_or_reject_relationships_atomically() {
    let draft = draft_with_transition(Microseconds::new(300_000));
    let state = CommandState::empty();
    let selection = timeline_selection();

    let snapped = move_segment(
        &draft,
        &state,
        &selection,
        SegmentId::from("right-segment"),
        TrackId::from("video-track"),
        Microseconds::new(1_050_000),
    )
    .expect("snapping should preserve a transition-adjacent boundary");
    assert_eq!(
        segment_by_id(&snapped.draft, "right-segment")
            .target_timerange
            .start,
        Microseconds::new(1_000_000)
    );
    validate_transition_relationships(&snapped.draft)
        .expect("snapped edit should keep transition windows valid");

    let moved = move_segment(
        &draft,
        &state,
        &selection,
        SegmentId::from("right-segment"),
        TrackId::from("video-track"),
        Microseconds::new(1_250_000),
    )
    .expect_err("moving away from the boundary should reject atomically");
    assert!(matches!(
        moved.kind,
        TimelineCommandErrorKind::InvalidTransitionRelationship { .. }
    ));

    let trimmed = trim_segment(
        &draft,
        &state,
        &selection,
        SegmentId::from("left-segment"),
        draft_model::TrimSegmentDirection::Right,
        TargetTimerange::new(0, 800_000),
    )
    .expect_err("trim that breaks transition adjacency should reject");
    assert!(matches!(
        trimmed.kind,
        TimelineCommandErrorKind::InvalidTransitionRelationship { .. }
    ));

    let split = split_segment(
        &draft,
        &state,
        &selection,
        SegmentId::from("left-segment"),
        SegmentId::from("split-right"),
        Microseconds::new(500_000),
    )
    .expect_err("split that leaves relationship boundary impossible should reject");
    assert!(matches!(
        split.kind,
        TimelineCommandErrorKind::InvalidTransitionRelationship { .. }
    ));

    let deleted = delete_segment(&draft, &state, &selection, SegmentId::from("left-segment"))
        .expect_err("delete that dangles a transition should reject");
    assert!(matches!(
        deleted.kind,
        TimelineCommandErrorKind::InvalidTransitionRelationship { .. }
    ));

    let mut magnet = draft_with_transition(Microseconds::new(300_000));
    for segment in &mut magnet.tracks[0].segments {
        segment.main_track_magnet = MainTrackMagnet::enabled();
    }
    let magnet_delete =
        delete_segment(&magnet, &state, &selection, SegmentId::from("left-segment"))
            .expect_err("main-track magnet cannot silently delete a transition endpoint");
    assert!(matches!(
        magnet_delete.kind,
        TimelineCommandErrorKind::InvalidTransitionRelationship { .. }
    ));

    let removed = remove_transition(
        &draft,
        &state,
        &selection,
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
    )
    .expect("explicit remove should clean the relationship before segment edits");
    let moved_after_remove = move_segment(
        &removed.draft,
        &removed.command_state,
        &removed.selection,
        SegmentId::from("right-segment"),
        TrackId::from("video-track"),
        Microseconds::new(1_250_000),
    )
    .expect("moving after explicit transition removal should be valid");
    assert_eq!(
        segment_by_id(&moved_after_remove.draft, "right-segment")
            .target_timerange
            .start,
        Microseconds::new(1_250_000)
    );

    assert_eq!(draft, draft_with_transition(Microseconds::new(300_000)));
}

#[test]
fn transition_commands_phase19_dispatcher_routes_add_transition_payload() {
    let response = execute_timeline_edit(TimelineEditPayload::AddTransition(
        AddTransitionCommandPayload {
            draft: draft_with_adjacent_video_segments(1_000_000, 1_000_000),
            command_state: CommandState::empty(),
            selection: timeline_selection(),
            from_segment_id: SegmentId::from("left-segment"),
            to_segment_id: SegmentId::from("right-segment"),
            reference: TransitionReference::dissolve(),
            duration: Microseconds::new(300_000),
            parameters: BTreeMap::new(),
        },
    ))
    .expect("timeline dispatcher should route add transition payloads");

    assert_eq!(response.events[0].kind, "transitionAdded");
    assert_eq!(response.draft.tracks[0].transitions.len(), 1);
}

fn draft_with_transition(duration: Microseconds) -> Draft {
    let mut draft = draft_with_adjacent_video_segments(1_000_000, 1_000_000);
    draft.tracks[0].transitions.push(TrackTransition::dissolve(
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        duration,
    ));
    draft
}

fn draft_with_gap_between_video_segments() -> Draft {
    let mut draft = draft_with_tracks_and_material();
    draft.tracks[0]
        .segments
        .push(segment("left-segment", 0, 1_000_000, 0, 1_000_000));
    draft.tracks[0].segments.push(segment(
        "right-segment",
        1_200_000,
        1_000_000,
        1_200_000,
        1_000_000,
    ));
    draft
}

fn draft_with_adjacent_video_segments(left_duration: u64, right_duration: u64) -> Draft {
    let mut draft = draft_with_tracks_and_material();
    draft.tracks[0]
        .segments
        .push(segment("left-segment", 0, left_duration, 0, left_duration));
    draft.tracks[0].segments.push(segment(
        "right-segment",
        left_duration,
        right_duration,
        left_duration,
        right_duration,
    ));
    draft
}

fn draft_with_tracks_and_material() -> Draft {
    let mut draft = Draft::new("transition-command-draft", "Transition Commands");
    draft.materials.push(Material::new(
        "video-material",
        MaterialKind::Video,
        "file://video.mp4",
        "video.mp4",
    ));
    draft
        .tracks
        .push(Track::new("video-track", TrackKind::Video, "Video"));
    draft
}

fn segment(
    segment_id: &str,
    target_start: u64,
    target_duration: u64,
    source_start: u64,
    source_duration: u64,
) -> Segment {
    Segment::new(
        segment_id,
        "video-material",
        SourceTimerange::new(source_start, source_duration),
        TargetTimerange::new(target_start, target_duration),
    )
}

fn segment_by_id<'a>(draft: &'a Draft, segment_id: &str) -> &'a Segment {
    draft
        .tracks
        .iter()
        .flat_map(|track| &track.segments)
        .find(|segment| segment.segment_id.as_str() == segment_id)
        .expect("segment should exist")
}

fn timeline_selection() -> TimelineSelection {
    TimelineSelection {
        segment_ids: vec![SegmentId::from("left-segment")],
        track_ids: vec![TrackId::from("video-track")],
    }
}
