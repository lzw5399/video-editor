use draft_commands::{
    TimelineCommandErrorKind,
    history::{DEFAULT_HISTORY_LIMIT, redo_timeline_edit, undo_timeline_edit},
    snapping::DEFAULT_SNAP_THRESHOLD_US,
    timeline::{add_segment, delete_segment, move_segment, trim_segment},
};
use draft_model::{
    CommandHistorySnapshot, CommandState, Draft, MainTrackMagnet, Material, MaterialKind,
    Microseconds, Segment, SourceTimerange, TargetTimerange, TimelineSelection, Track, TrackKind,
    TrimSegmentDirection,
};

#[test]
fn undo_redo() {
    assert_eq!(DEFAULT_HISTORY_LIMIT, 100);

    let draft = draft_with_tracks_and_materials();
    let initial_selection = TimelineSelection::empty();
    let state = CommandState::empty();

    let added = add_segment(
        &draft,
        &state,
        &initial_selection,
        "video-track".into(),
        "segment-a".into(),
        "video-material".into(),
        SourceTimerange::new(0, 400_000),
        TargetTimerange::new(0, 400_000),
    )
    .expect("add should push undo history after commit");
    assert_eq!(added.command_state.undo_stack.len(), 1);
    assert!(added.command_state.redo_stack.is_empty());
    assert_eq!(added.command_state.undo_stack[0].draft, draft);
    assert_eq!(
        added.command_state.undo_stack[0].selection,
        initial_selection
    );

    let moved = move_segment(
        &added.draft,
        &added.command_state,
        &added.selection,
        "segment-a".into(),
        "video-track".into(),
        Microseconds::new(500_000),
    )
    .expect("move should push another undo snapshot");
    assert_eq!(moved.command_state.undo_stack.len(), 2);
    assert!(moved.command_state.redo_stack.is_empty());

    let undone = undo_timeline_edit(&moved.draft, &moved.command_state, &moved.selection)
        .expect("undo should restore previous draft and move current to redo");
    assert_eq!(undone.draft, added.draft);
    assert_eq!(undone.selection, added.selection);
    assert_eq!(undone.command_state.undo_stack.len(), 1);
    assert_eq!(undone.command_state.redo_stack.len(), 1);
    assert_eq!(undone.events[0].kind, "undoCommitted");

    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("redo should restore the redone draft and move current to undo");
    assert_eq!(redone.draft, moved.draft);
    assert_eq!(redone.selection, moved.selection);
    assert_eq!(redone.command_state.undo_stack.len(), 2);
    assert!(redone.command_state.redo_stack.is_empty());
    assert_eq!(redone.events[0].kind, "redoCommitted");

    let mut limited_state = CommandState::empty();
    limited_state.max_history_entries = 2;
    let first = add_segment(
        &draft,
        &limited_state,
        &TimelineSelection::empty(),
        "video-track".into(),
        "limit-a".into(),
        "video-material".into(),
        SourceTimerange::new(0, 100_000),
        TargetTimerange::new(0, 100_000),
    )
    .expect("first limited edit");
    let second = add_segment(
        &first.draft,
        &first.command_state,
        &first.selection,
        "video-track".into(),
        "limit-b".into(),
        "video-material".into(),
        SourceTimerange::new(100_000, 100_000),
        TargetTimerange::new(100_000, 100_000),
    )
    .expect("second limited edit");
    let third = add_segment(
        &second.draft,
        &second.command_state,
        &second.selection,
        "video-track".into(),
        "limit-c".into(),
        "video-material".into(),
        SourceTimerange::new(200_000, 100_000),
        TargetTimerange::new(200_000, 100_000),
    )
    .expect("third limited edit should prune oldest history");
    assert_eq!(third.command_state.undo_stack.len(), 2);
    assert!(
        third
            .events
            .iter()
            .any(|event| event.kind == "historyLimitPruned")
    );

    let after_undo = undo_timeline_edit(&moved.draft, &moved.command_state, &moved.selection)
        .expect("undo should create redo stack");
    assert_eq!(after_undo.command_state.redo_stack.len(), 1);
    let after_new_edit = delete_segment(
        &after_undo.draft,
        &after_undo.command_state,
        &after_undo.selection,
        "segment-a".into(),
    )
    .expect("new edit after undo should clear redo");
    assert!(after_new_edit.command_state.redo_stack.is_empty());

    let empty_undo =
        undo_timeline_edit(&draft, &CommandState::empty(), &TimelineSelection::empty())
            .expect_err("empty undo should reject");
    assert_eq!(
        empty_undo.kind,
        TimelineCommandErrorKind::HistoryEmpty {
            direction: "undo".to_owned(),
        }
    );

    let empty_redo =
        redo_timeline_edit(&draft, &CommandState::empty(), &TimelineSelection::empty())
            .expect_err("empty redo should reject");
    assert_eq!(
        empty_redo.kind,
        TimelineCommandErrorKind::HistoryEmpty {
            direction: "redo".to_owned(),
        }
    );
}

#[test]
fn invalid_edits_are_atomic() {
    let (draft, state, selection) = draft_with_existing_segment_and_history();

    let error = add_segment(
        &draft,
        &state,
        &selection,
        "video-track".into(),
        "overlap".into(),
        "video-material".into(),
        SourceTimerange::new(500_000, 100_000),
        TargetTimerange::new(100_000, 100_000),
    )
    .expect_err("rejected edits should not mutate history state");

    assert!(matches!(
        error.kind,
        TimelineCommandErrorKind::OverlappingSegment { .. }
    ));
    assert_eq!(state, draft_with_existing_segment_and_history().1);
    assert_eq!(selection, draft_with_existing_segment_and_history().2);
}

#[test]
fn snapping() {
    assert_eq!(DEFAULT_SNAP_THRESHOLD_US, 100_000);

    let (draft, state, selection) = draft_with_two_video_segments();
    let snapped = move_segment(
        &draft,
        &state,
        &selection,
        "segment-b".into(),
        "video-track".into(),
        Microseconds::new(410_000),
    )
    .expect("move within default threshold should snap to previous segment end");
    let moved = segment_by_id(&snapped.draft, "segment-b");
    assert_eq!(moved.target_timerange.start, Microseconds::new(400_000));
    assert!(snapped.events.iter().any(|event| event.kind == "snapped"));

    let not_snapped = move_segment(
        &draft,
        &state,
        &selection,
        "segment-b".into(),
        "video-track".into(),
        Microseconds::new(550_000),
    )
    .expect("move outside default threshold should not snap");
    let moved = segment_by_id(&not_snapped.draft, "segment-b");
    assert_eq!(moved.target_timerange.start, Microseconds::new(550_000));
    assert!(
        !not_snapped
            .events
            .iter()
            .any(|event| event.kind == "snapped")
    );

    let mut override_state = state.clone();
    override_state.snapping.threshold = Microseconds::new(200_000);
    let override_snapped = move_segment(
        &draft,
        &override_state,
        &selection,
        "segment-b".into(),
        "video-track".into(),
        Microseconds::new(550_000),
    )
    .expect("larger threshold should snap");
    let moved = segment_by_id(&override_snapped.draft, "segment-b");
    assert_eq!(moved.target_timerange.start, Microseconds::new(400_000));

    let trim_snapped = trim_segment(
        &draft,
        &state,
        &selection,
        "segment-a".into(),
        TrimSegmentDirection::Right,
        TargetTimerange::new(0, 650_000),
    )
    .expect("right trim near next segment start should snap boundary");
    let trimmed = segment_by_id(&trim_snapped.draft, "segment-a");
    assert_eq!(
        trimmed.target_timerange.duration,
        Microseconds::new(700_000)
    );
    assert!(
        trim_snapped
            .events
            .iter()
            .any(|event| event.kind == "snapped")
    );
}

#[test]
fn main_track_magnet() {
    let (draft, state, selection) = draft_with_main_track_magnet_segments();
    let deleted = delete_segment(&draft, &state, &selection, "segment-a".into())
        .expect("main track magnet should close the gap after deletion");

    let remaining = segment_by_id(&deleted.draft, "segment-b");
    assert_eq!(remaining.target_timerange.start, Microseconds::new(0));
    assert!(
        deleted
            .events
            .iter()
            .any(|event| event.kind == "mainTrackMagnetApplied")
    );
}

fn draft_with_existing_segment_and_history() -> (Draft, CommandState, TimelineSelection) {
    let mut draft = draft_with_tracks_and_materials();
    draft.tracks[0].segments.push(segment(
        "segment-a",
        "video-material",
        0,
        400_000,
        0,
        400_000,
    ));
    let selection = TimelineSelection {
        segment_ids: vec!["segment-a".into()],
        track_ids: vec!["video-track".into()],
    };
    let mut state = CommandState::empty();
    state.undo_stack.push(CommandHistorySnapshot {
        draft: draft_with_tracks_and_materials(),
        selection: TimelineSelection::empty(),
        label: Some("seed undo".to_owned()),
    });
    state.redo_stack.push(CommandHistorySnapshot {
        draft: draft.clone(),
        selection: selection.clone(),
        label: Some("seed redo".to_owned()),
    });
    (draft, state, selection)
}

fn draft_with_two_video_segments() -> (Draft, CommandState, TimelineSelection) {
    let mut draft = draft_with_tracks_and_materials();
    draft.tracks[0].segments.push(segment(
        "segment-a",
        "video-material",
        0,
        400_000,
        0,
        400_000,
    ));
    draft.tracks[0].segments.push(segment(
        "segment-b",
        "video-material",
        700_000,
        200_000,
        700_000,
        200_000,
    ));
    (
        draft,
        CommandState::empty(),
        TimelineSelection {
            segment_ids: vec!["segment-b".into()],
            track_ids: vec!["video-track".into()],
        },
    )
}

fn draft_with_main_track_magnet_segments() -> (Draft, CommandState, TimelineSelection) {
    let (mut draft, state, selection) = draft_with_two_video_segments();
    for segment in &mut draft.tracks[0].segments {
        segment.main_track_magnet = MainTrackMagnet::enabled();
    }
    (draft, state, selection)
}

fn segment_by_id<'a>(draft: &'a Draft, segment_id: &str) -> &'a Segment {
    draft
        .tracks
        .iter()
        .flat_map(|track| &track.segments)
        .find(|segment| segment.segment_id.as_str() == segment_id)
        .expect("segment should exist")
}

fn draft_with_tracks_and_materials() -> Draft {
    let mut draft = Draft::new("history-command-draft", "History Commands");
    draft.materials.push(material_with_duration(
        "video-material",
        MaterialKind::Video,
        "video.mp4",
        1_000_000,
    ));
    draft
        .tracks
        .push(Track::new("video-track", TrackKind::Video, "Video"));
    draft
}

fn material_with_duration(
    material_id: &str,
    kind: MaterialKind,
    uri: &str,
    duration: u64,
) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(duration));
    material
}

fn segment(
    segment_id: &str,
    material_id: &str,
    source_start: u64,
    source_duration: u64,
    target_start: u64,
    target_duration: u64,
) -> Segment {
    Segment::new(
        segment_id,
        material_id,
        SourceTimerange::new(source_start, source_duration),
        TargetTimerange::new(target_start, target_duration),
    )
}
