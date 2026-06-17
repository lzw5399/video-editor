use draft_commands::{
    TimelineCommandErrorKind,
    history::{DEFAULT_HISTORY_LIMIT, redo_timeline_edit, undo_timeline_edit},
    timeline::{add_segment, delete_segment, move_segment},
};
use draft_model::{
    CommandHistorySnapshot, CommandState, Draft, Material, MaterialKind, Microseconds, Segment,
    SourceTimerange, TargetTimerange, TimelineSelection, Track, TrackKind,
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
    assert_eq!(added.command_state.undo_stack[0].selection, initial_selection);

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

    let empty_undo = undo_timeline_edit(&draft, &CommandState::empty(), &TimelineSelection::empty())
        .expect_err("empty undo should reject");
    assert_eq!(
        empty_undo.kind,
        TimelineCommandErrorKind::HistoryEmpty {
            direction: "undo".to_owned(),
        }
    );

    let empty_redo = redo_timeline_edit(&draft, &CommandState::empty(), &TimelineSelection::empty())
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
