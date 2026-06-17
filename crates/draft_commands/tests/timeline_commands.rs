use draft_commands::{
    TimelineCommandErrorKind,
    timeline,
};
use draft_model::{
    AddSegmentCommandPayload, CommandState, DeleteSegmentCommandPayload, Draft, Material,
    MaterialKind, Microseconds, MoveSegmentCommandPayload, SelectTimelineSegmentsCommandPayload,
    SourceTimerange, SplitSegmentCommandPayload, TargetTimerange, TimelineSelection, Track,
    TrackKind, TrimSegmentCommandPayload, TrimSegmentDirection,
};

#[test]
fn add_segment() {
    let draft = draft_with_empty_tracks();
    let command_state = CommandState::empty();
    let selection = TimelineSelection::empty();

    let response = timeline::add_segment(AddSegmentCommandPayload {
        draft: draft.clone(),
        command_state: command_state.clone(),
        selection: selection.clone(),
        track_id: "video-track".into(),
        segment_id: "segment-a".into(),
        material_id: "video-material".into(),
        source_timerange: SourceTimerange::new(250_000, 500_000),
        target_timerange: TargetTimerange::new(1_000_000, 500_000),
    })
    .expect("valid add should commit");

    assert_eq!(draft.tracks[0].segments.len(), 0, "input draft is immutable");
    assert_eq!(response.draft.tracks[0].segments.len(), 1);
    let segment = &response.draft.tracks[0].segments[0];
    assert_eq!(segment.segment_id.as_str(), "segment-a");
    assert_eq!(segment.material_id.as_str(), "video-material");
    assert_eq!(segment.source_timerange, SourceTimerange::new(250_000, 500_000));
    assert_eq!(segment.target_timerange, TargetTimerange::new(1_000_000, 500_000));
    assert_eq!(response.selection.segment_ids, vec!["segment-a".into()]);
    assert_eq!(response.command_state.undo_stack.len(), 1);
    assert_eq!(response.command_state.redo_stack.len(), 0);
    assert_eq!(response.events[0].kind, "segmentAdded");
}

#[test]
fn timeline_edits() {
    let response = timeline::add_segment(AddSegmentCommandPayload {
        draft: draft_with_empty_tracks(),
        command_state: CommandState::empty(),
        selection: TimelineSelection::empty(),
        track_id: "video-track".into(),
        segment_id: "segment-a".into(),
        material_id: "video-material".into(),
        source_timerange: SourceTimerange::new(100_000, 700_000),
        target_timerange: TargetTimerange::new(1_000_000, 700_000),
    })
    .expect("add should commit");

    let selected = timeline::select_timeline_segments(SelectTimelineSegmentsCommandPayload {
        draft: response.draft.clone(),
        command_state: response.command_state.clone(),
        selection: response.selection.clone(),
        segment_ids: vec!["segment-a".into()],
        track_ids: vec!["video-track".into()],
    })
    .expect("select should commit");
    assert_eq!(selected.draft, response.draft, "selection does not mutate draft");
    assert_eq!(selected.selection.segment_ids, vec!["segment-a".into()]);
    assert_eq!(selected.selection.track_ids, vec!["video-track".into()]);
    assert_eq!(selected.events[0].kind, "timelineSelectionChanged");

    let moved = timeline::move_segment(MoveSegmentCommandPayload {
        draft: selected.draft.clone(),
        command_state: selected.command_state.clone(),
        selection: selected.selection.clone(),
        segment_id: "segment-a".into(),
        target_track_id: "video-track".into(),
        target_start: Microseconds::new(2_000_000),
    })
    .expect("move should commit");
    let segment = &moved.draft.tracks[0].segments[0];
    assert_eq!(segment.source_timerange, SourceTimerange::new(100_000, 700_000));
    assert_eq!(segment.target_timerange, TargetTimerange::new(2_000_000, 700_000));
    assert_eq!(moved.events[0].kind, "segmentMoved");

    let split = timeline::split_segment(SplitSegmentCommandPayload {
        draft: moved.draft.clone(),
        command_state: moved.command_state.clone(),
        selection: moved.selection.clone(),
        segment_id: "segment-a".into(),
        right_segment_id: "segment-b".into(),
        split_at: Microseconds::new(2_300_000),
    })
    .expect("split should commit");
    assert_eq!(split.draft.tracks[0].segments.len(), 2);
    assert_eq!(
        split.draft.tracks[0].segments[0].source_timerange,
        SourceTimerange::new(100_000, 300_000)
    );
    assert_eq!(
        split.draft.tracks[0].segments[0].target_timerange,
        TargetTimerange::new(2_000_000, 300_000)
    );
    assert_eq!(
        split.draft.tracks[0].segments[1].source_timerange,
        SourceTimerange::new(400_000, 400_000)
    );
    assert_eq!(
        split.draft.tracks[0].segments[1].target_timerange,
        TargetTimerange::new(2_300_000, 400_000)
    );
    assert_eq!(split.events[0].kind, "segmentSplit");

    let left_trimmed = timeline::trim_segment(TrimSegmentCommandPayload {
        draft: split.draft.clone(),
        command_state: split.command_state.clone(),
        selection: split.selection.clone(),
        segment_id: "segment-a".into(),
        direction: TrimSegmentDirection::Left,
        target_timerange: TargetTimerange::new(2_100_000, 200_000),
    })
    .expect("left trim should commit");
    assert_eq!(
        left_trimmed.draft.tracks[0].segments[0].source_timerange,
        SourceTimerange::new(200_000, 200_000)
    );
    assert_eq!(
        left_trimmed.draft.tracks[0].segments[0].target_timerange,
        TargetTimerange::new(2_100_000, 200_000)
    );
    assert_eq!(left_trimmed.events[0].kind, "segmentTrimmed");

    let right_trimmed = timeline::trim_segment(TrimSegmentCommandPayload {
        draft: left_trimmed.draft.clone(),
        command_state: left_trimmed.command_state.clone(),
        selection: left_trimmed.selection.clone(),
        segment_id: "segment-b".into(),
        direction: TrimSegmentDirection::Right,
        target_timerange: TargetTimerange::new(2_300_000, 250_000),
    })
    .expect("right trim should commit");
    assert_eq!(
        right_trimmed.draft.tracks[0].segments[1].source_timerange,
        SourceTimerange::new(400_000, 250_000)
    );
    assert_eq!(
        right_trimmed.draft.tracks[0].segments[1].target_timerange,
        TargetTimerange::new(2_300_000, 250_000)
    );

    let deleted = timeline::delete_segment(DeleteSegmentCommandPayload {
        draft: right_trimmed.draft.clone(),
        command_state: right_trimmed.command_state.clone(),
        selection: TimelineSelection {
            segment_ids: vec!["segment-a".into(), "segment-b".into()],
            track_ids: vec!["video-track".into()],
        },
        segment_id: "segment-a".into(),
    })
    .expect("delete should commit");
    assert_eq!(deleted.draft.tracks[0].segments.len(), 1);
    assert_eq!(deleted.draft.tracks[0].segments[0].segment_id.as_str(), "segment-b");
    assert_eq!(deleted.selection.segment_ids, vec!["segment-b".into()]);
    assert_eq!(deleted.events[0].kind, "segmentDeleted");
}

#[test]
fn invalid_edits_are_atomic() {
    let valid = timeline::add_segment(AddSegmentCommandPayload {
        draft: draft_with_empty_tracks(),
        command_state: CommandState::empty(),
        selection: TimelineSelection::empty(),
        track_id: "video-track".into(),
        segment_id: "segment-a".into(),
        material_id: "video-material".into(),
        source_timerange: SourceTimerange::new(0, 500_000),
        target_timerange: TargetTimerange::new(0, 500_000),
    })
    .expect("seed add should commit");

    let base_draft = valid.draft.clone();
    let base_state = valid.command_state.clone();
    let base_selection = valid.selection.clone();

    assert_atomic_rejection(
        timeline::add_segment(AddSegmentCommandPayload {
            draft: base_draft.clone(),
            command_state: base_state.clone(),
            selection: base_selection.clone(),
            track_id: "video-track".into(),
            segment_id: "overlap".into(),
            material_id: "video-material".into(),
            source_timerange: SourceTimerange::new(0, 500_000),
            target_timerange: TargetTimerange::new(250_000, 500_000),
        }),
        TimelineCommandErrorKind::OverlappingSegment {
            track_id: "video-track".into(),
            first_segment_id: "segment-a".into(),
            second_segment_id: "overlap".into(),
        },
        &base_draft,
        &base_state,
        &base_selection,
    );

    let mut locked_draft = base_draft.clone();
    locked_draft.tracks[0].locked = true;
    assert_atomic_rejection(
        timeline::move_segment(MoveSegmentCommandPayload {
            draft: locked_draft.clone(),
            command_state: base_state.clone(),
            selection: base_selection.clone(),
            segment_id: "segment-a".into(),
            target_track_id: "video-track".into(),
            target_start: Microseconds::new(750_000),
        }),
        TimelineCommandErrorKind::LockedTrack {
            track_id: "video-track".into(),
        },
        &locked_draft,
        &base_state,
        &base_selection,
    );

    assert_atomic_rejection(
        timeline::add_segment(AddSegmentCommandPayload {
            draft: base_draft.clone(),
            command_state: base_state.clone(),
            selection: base_selection.clone(),
            track_id: "video-track".into(),
            segment_id: "overrun".into(),
            material_id: "video-material".into(),
            source_timerange: SourceTimerange::new(900_000, 200_000),
            target_timerange: TargetTimerange::new(750_000, 200_000),
        }),
        TimelineCommandErrorKind::SourceRangeExceedsMaterialDuration {
            segment_id: "overrun".into(),
            material_id: "video-material".into(),
            source_end: Microseconds::new(1_100_000),
            material_duration: Microseconds::new(1_000_000),
        },
        &base_draft,
        &base_state,
        &base_selection,
    );

    assert_atomic_rejection(
        timeline::split_segment(SplitSegmentCommandPayload {
            draft: base_draft.clone(),
            command_state: base_state.clone(),
            selection: base_selection.clone(),
            segment_id: "segment-a".into(),
            right_segment_id: "segment-b".into(),
            split_at: Microseconds::new(0),
        }),
        TimelineCommandErrorKind::InvalidSplitPoint {
            segment_id: "segment-a".into(),
            split_at: Microseconds::new(0),
        },
        &base_draft,
        &base_state,
        &base_selection,
    );

    assert_atomic_rejection(
        timeline::trim_segment(TrimSegmentCommandPayload {
            draft: base_draft.clone(),
            command_state: base_state.clone(),
            selection: base_selection.clone(),
            segment_id: "segment-a".into(),
            direction: TrimSegmentDirection::Right,
            target_timerange: TargetTimerange::new(0, 0),
        }),
        TimelineCommandErrorKind::ZeroDuration {
            field: "targetTimerange.duration".to_owned(),
        },
        &base_draft,
        &base_state,
        &base_selection,
    );

    assert_atomic_rejection(
        timeline::delete_segment(DeleteSegmentCommandPayload {
            draft: base_draft.clone(),
            command_state: base_state.clone(),
            selection: base_selection.clone(),
            segment_id: "missing-segment".into(),
        }),
        TimelineCommandErrorKind::SegmentNotFound {
            segment_id: "missing-segment".into(),
        },
        &base_draft,
        &base_state,
        &base_selection,
    );

    assert_atomic_rejection(
        timeline::add_segment(AddSegmentCommandPayload {
            draft: base_draft.clone(),
            command_state: base_state.clone(),
            selection: base_selection.clone(),
            track_id: "video-track".into(),
            segment_id: "missing-material".into(),
            material_id: "missing-material".into(),
            source_timerange: SourceTimerange::new(0, 100_000),
            target_timerange: TargetTimerange::new(750_000, 100_000),
        }),
        TimelineCommandErrorKind::MaterialNotFound {
            material_id: "missing-material".into(),
        },
        &base_draft,
        &base_state,
        &base_selection,
    );

    assert_atomic_rejection(
        timeline::add_segment(AddSegmentCommandPayload {
            draft: base_draft.clone(),
            command_state: base_state.clone(),
            selection: base_selection.clone(),
            track_id: "audio-track".into(),
            segment_id: "incompatible".into(),
            material_id: "video-material".into(),
            source_timerange: SourceTimerange::new(0, 100_000),
            target_timerange: TargetTimerange::new(750_000, 100_000),
        }),
        TimelineCommandErrorKind::IncompatibleTrackMaterialKind {
            track_id: "audio-track".into(),
            track_kind: TrackKind::Audio,
            material_id: "video-material".into(),
            material_kind: MaterialKind::Video,
        },
        &base_draft,
        &base_state,
        &base_selection,
    );
}

fn assert_atomic_rejection(
    result: Result<draft_model::TimelineCommandResponse, draft_commands::TimelineCommandError>,
    expected: TimelineCommandErrorKind,
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
) {
    let error = result.expect_err("command should reject");
    assert_eq!(error.kind, expected);
    assert_eq!(*draft, draft.clone(), "draft remains unchanged after rejection");
    assert_eq!(
        *command_state,
        command_state.clone(),
        "command state remains unchanged after rejection"
    );
    assert_eq!(
        *selection,
        selection.clone(),
        "selection remains unchanged after rejection"
    );
}

fn draft_with_empty_tracks() -> Draft {
    let mut draft = Draft::new("timeline-command-draft", "Timeline Command Draft");
    draft.materials.push(material_with_duration(
        "video-material",
        MaterialKind::Video,
        "video.mp4",
        1_000_000,
    ));
    draft.materials.push(material_with_duration(
        "audio-material",
        MaterialKind::Audio,
        "bgm.wav",
        2_000_000,
    ));
    draft
        .tracks
        .push(Track::new("video-track", TrackKind::Video, "Video"));
    draft
        .tracks
        .push(Track::new("audio-track", TrackKind::Audio, "Audio"));
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
