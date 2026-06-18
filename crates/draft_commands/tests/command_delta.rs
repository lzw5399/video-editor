use draft_commands::timeline::{
    add_segment as command_add_segment, move_segment as command_move_segment,
    select_timeline_segments as command_select_timeline_segments,
};
use draft_model::{
    CommandState, Draft, Material, MaterialKind, Microseconds, Segment, SourceTimerange,
    TargetTimerange, TimelineSelection, Track, TrackKind,
};

#[test]
fn command_delta_target_anchors_accepted_timeline_edit_ranges() {
    let (draft, state, selection) = draft_with_existing_segment();

    let moved = command_move_segment(
        &draft,
        &state,
        &selection,
        "segment-a".into(),
        "video-track".into(),
        Microseconds::new(600_000),
    )
    .expect("move should commit");

    assert_eq!(moved.events[0].kind, "segmentMoved");
    assert_eq!(
        moved.draft.tracks[0].segments[0].target_timerange,
        TargetTimerange::new(600_000, 400_000)
    );
    assert_dirty_ranges_cover_previous_and_current(
        &[
            draft.tracks[0].segments[0].target_timerange.clone(),
            moved.draft.tracks[0].segments[0].target_timerange.clone(),
        ],
        TargetTimerange::new(0, 1_000_000),
    );
}

#[test]
fn command_delta_target_keeps_selection_only_commands_non_semantic() {
    let (draft, state, selection) = draft_with_existing_segment();

    let selected = command_select_timeline_segments(
        &draft,
        &state,
        &selection,
        vec!["segment-a".into()],
        vec!["video-track".into()],
    )
    .expect("selection command should commit");

    assert_eq!(selected.draft, draft);
    assert_eq!(selected.events[0].kind, "timelineSelectionChanged");
    assert!(
        selected.command_state.undo_stack.is_empty(),
        "selection-only commands must not create semantic undo snapshots"
    );
}

#[test]
fn command_delta_target_keeps_current_response_rust_owned() {
    let draft = draft_with_tracks_and_materials();
    let response = command_add_segment(
        &draft,
        &CommandState::empty(),
        &TimelineSelection::empty(),
        "video-track".into(),
        "segment-new".into(),
        "video-material".into(),
        SourceTimerange::new(0, 250_000),
        TargetTimerange::new(1_000_000, 250_000),
    )
    .expect("add segment should commit");

    assert_eq!(response.events[0].kind, "segmentAdded");
    assert_eq!(
        response.draft.draft_id.as_str(),
        "phase13-command-delta-draft"
    );
    assert_eq!(response.command_state.undo_stack.len(), 1);
    assert_eq!(response.selection.segment_ids, vec!["segment-new".into()]);
}

fn assert_dirty_ranges_cover_previous_and_current(
    ranges: &[TargetTimerange],
    expected_cover: TargetTimerange,
) {
    assert_eq!(ranges.len(), 2);
    let start = ranges
        .iter()
        .map(|range| range.start.get())
        .min()
        .expect("at least one range");
    let end = ranges
        .iter()
        .map(|range| range.start.get() + range.duration.get())
        .max()
        .expect("at least one range");
    assert_eq!(start, expected_cover.start.get());
    assert_eq!(end - start, expected_cover.duration.get());
}

fn draft_with_existing_segment() -> (Draft, CommandState, TimelineSelection) {
    let mut draft = draft_with_tracks_and_materials();
    draft.tracks[0].segments.push(segment(
        "segment-a",
        "video-material",
        SourceTimerange::new(100_000, 400_000),
        TargetTimerange::new(0, 400_000),
    ));
    (draft, CommandState::empty(), TimelineSelection::empty())
}

fn draft_with_tracks_and_materials() -> Draft {
    let mut draft = Draft::new("phase13-command-delta-draft", "Phase 13 Command Delta");
    draft.materials.push(material(
        "video-material",
        MaterialKind::Video,
        "file://video.mp4",
        2_000_000,
    ));
    draft
        .tracks
        .push(Track::new("video-track", TrackKind::Video, "Video"));
    draft
}

fn material(id: &str, kind: MaterialKind, uri: &str, duration: u64) -> Material {
    let mut material = Material::new(id, kind, uri, id);
    material.metadata.duration = Some(Microseconds::new(duration));
    material
}

fn segment(
    id: &str,
    material_id: &str,
    source: SourceTimerange,
    target: TargetTimerange,
) -> Segment {
    Segment::new(id, material_id, source, target)
}
