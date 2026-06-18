use draft_commands::timeline::{
    add_segment as command_add_segment, delete_segment as command_delete_segment,
    move_segment as command_move_segment,
    select_timeline_segments as command_select_timeline_segments,
    split_segment as command_split_segment, trim_segment as command_trim_segment,
};
use draft_model::{
    ChangedEntity, CommandDelta, CommandName, CommandState, DirtyDomain, DirtyRange,
    DirtyRangeSource, Draft, InvalidationScope, Material, MaterialKind, Microseconds, Segment,
    SourceTimerange, TargetTimerange, TimelineSelection, Track, TrackKind, TrimSegmentDirection,
};

#[test]
fn simple_timeline_add_emits_semantic_delta() {
    let draft = draft_with_tracks_and_materials();

    let added = command_add_segment(
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

    assert_eq!(added.events[0].kind, "segmentAdded");
    assert_delta_eq(
        &added.delta,
        expected_segment_delta(
            CommandName::AddSegment,
            "video-track",
            "segment-new",
            "video-material",
            vec![dirty_range(1_000_000, 250_000, DirtyRangeSource::Current)],
            "segment added",
        ),
    );
}

#[test]
fn simple_timeline_move_emits_previous_and_current_ranges() {
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
    assert_delta_eq(
        &moved.delta,
        expected_segment_delta(
            CommandName::MoveSegment,
            "video-track",
            "segment-a",
            "video-material",
            vec![
                dirty_range(0, 400_000, DirtyRangeSource::Previous),
                dirty_range(600_000, 400_000, DirtyRangeSource::Current),
            ],
            "segment moved",
        ),
    );
}

#[test]
fn simple_timeline_split_emits_original_range_and_both_segments() {
    let (draft, state, selection) = draft_with_existing_segment();

    let split = command_split_segment(
        &draft,
        &state,
        &selection,
        "segment-a".into(),
        "segment-b".into(),
        Microseconds::new(250_000),
    )
    .expect("split should commit");

    assert_eq!(split.events[0].kind, "segmentSplit");
    assert!(
        split
            .delta
            .changed_entities
            .contains(&ChangedEntity::Segment {
                track_id: "video-track".into(),
                segment_id: "segment-a".into(),
            })
    );
    assert!(
        split
            .delta
            .changed_entities
            .contains(&ChangedEntity::Segment {
                track_id: "video-track".into(),
                segment_id: "segment-b".into(),
            })
    );
    assert_eq!(
        split.delta.changed_ranges,
        vec![dirty_range(
            0,
            400_000,
            DirtyRangeSource::PreviousAndCurrent
        )]
    );
    assert!(split.delta.changed_domains.contains(&DirtyDomain::Timing));
    assert!(split.delta.changed_domains.contains(&DirtyDomain::Visual));
    assert!(!split.delta.invalidation.full_draft);
}

#[test]
fn simple_timeline_trim_emits_previous_and_current_ranges() {
    let (draft, state, selection) = draft_with_existing_segment();

    let trimmed = command_trim_segment(
        &draft,
        &state,
        &selection,
        "segment-a".into(),
        TrimSegmentDirection::Right,
        TargetTimerange::new(0, 250_000),
    )
    .expect("trim should commit");

    assert_eq!(trimmed.events[0].kind, "segmentTrimmed");
    assert_delta_eq(
        &trimmed.delta,
        expected_segment_delta(
            CommandName::TrimSegment,
            "video-track",
            "segment-a",
            "video-material",
            vec![
                dirty_range(0, 400_000, DirtyRangeSource::Previous),
                dirty_range(0, 250_000, DirtyRangeSource::Current),
            ],
            "segment trimmed",
        ),
    );
}

#[test]
fn simple_timeline_delete_emits_previous_range() {
    let (draft, state, selection) = draft_with_existing_segment();

    let deleted = command_delete_segment(&draft, &state, &selection, "segment-a".into())
        .expect("delete should commit");

    assert_eq!(deleted.events[0].kind, "segmentDeleted");
    assert_delta_eq(
        &deleted.delta,
        expected_segment_delta(
            CommandName::DeleteSegment,
            "video-track",
            "segment-a",
            "video-material",
            vec![dirty_range(0, 400_000, DirtyRangeSource::Previous)],
            "segment deleted",
        ),
    );
}

#[test]
fn simple_timeline_selection_emits_noop_delta() {
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
    assert_eq!(
        selected.delta,
        CommandDelta::none(CommandName::SelectTimelineSegments, "selection only")
    );
    assert!(
        selected.command_state.undo_stack.is_empty(),
        "selection-only commands must not create semantic undo snapshots"
    );
}

#[test]
fn simple_timeline_all_accepted_responses_include_command_delta() {
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
    assert_eq!(response.delta.command, CommandName::AddSegment);
}

fn assert_delta_eq(actual: &CommandDelta, expected: CommandDelta) {
    assert_eq!(actual, &expected);
    assert!(
        !actual.invalidation.full_draft,
        "simple timeline commands must use targeted invalidation"
    );
}

fn expected_segment_delta(
    command: CommandName,
    track_id: &str,
    segment_id: &str,
    material_id: &str,
    changed_ranges: Vec<DirtyRange>,
    reason: &str,
) -> CommandDelta {
    CommandDelta {
        command,
        changed_entities: vec![
            ChangedEntity::Track {
                track_id: track_id.into(),
            },
            ChangedEntity::Segment {
                track_id: track_id.into(),
                segment_id: segment_id.into(),
            },
            ChangedEntity::Material {
                material_id: material_id.into(),
            },
        ],
        changed_domains: vec![
            DirtyDomain::Timing,
            DirtyDomain::Visual,
            DirtyDomain::Material,
            DirtyDomain::Preview,
            DirtyDomain::ExportPrep,
            DirtyDomain::Thumbnail,
            DirtyDomain::Proxy,
            DirtyDomain::GraphSnapshot,
            DirtyDomain::PreviewCache,
        ],
        changed_ranges,
        invalidation: InvalidationScope::targeted(
            vec![material_id.into()],
            vec![
                DirtyDomain::Preview,
                DirtyDomain::ExportPrep,
                DirtyDomain::Thumbnail,
                DirtyDomain::Proxy,
                DirtyDomain::GraphSnapshot,
                DirtyDomain::PreviewCache,
            ],
        ),
        reason: reason.to_owned(),
    }
}

fn dirty_range(start: u64, duration: u64, source: DirtyRangeSource) -> DirtyRange {
    DirtyRange {
        target_timerange: TargetTimerange::new(start, duration),
        source,
    }
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
