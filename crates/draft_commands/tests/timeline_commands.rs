use draft_commands::{
    TimelineCommandErrorKind,
    timeline::{
        add_audio_segment_intent, add_segment as command_add_segment, add_text_segment_intent,
        add_timeline_segment_intent, add_track_intent, delete_segment as command_delete_segment,
        move_segment as command_move_segment, move_selected_segment_intent,
        select_timeline_segments as command_select_timeline_segments,
        split_segment as command_split_segment, split_selected_segment_intent,
        trim_segment as command_trim_segment, trim_selected_segment_intent,
    },
};
use draft_model::{
    CommandState, Draft, Material, MaterialKind, Microseconds, Segment, SourceTimerange,
    TargetTimerange, TextSegment, TextSegmentSource, TimelineSelection, Track, TrackKind,
    TrimSegmentDirection,
};

#[test]
fn add_segment() {
    let draft = draft_with_tracks_and_materials();
    let response = command_add_segment(
        &draft,
        &CommandState::empty(),
        &TimelineSelection::empty(),
        "video-track".into(),
        "intro-segment".into(),
        "video-material".into(),
        SourceTimerange::new(100_000, 400_000),
        TargetTimerange::new(1_000_000, 400_000),
    )
    .expect("add segment should commit on a valid unlocked compatible track");

    assert!(
        draft.tracks[0].segments.is_empty(),
        "input draft stays unchanged"
    );
    assert_eq!(response.draft.tracks[0].segments.len(), 1);
    let segment = &response.draft.tracks[0].segments[0];
    assert_eq!(segment.segment_id.as_str(), "intro-segment");
    assert_eq!(segment.material_id.as_str(), "video-material");
    assert_eq!(
        segment.source_timerange,
        SourceTimerange::new(100_000, 400_000)
    );
    assert_eq!(
        segment.target_timerange,
        TargetTimerange::new(1_000_000, 400_000)
    );
    assert_eq!(response.selection.segment_ids, vec!["intro-segment".into()]);
    assert_eq!(response.events[0].kind, "segmentAdded");
}

#[test]
fn timeline_edits() {
    let (draft, state, selection) = draft_with_existing_video_segment();

    let selected = command_select_timeline_segments(
        &draft,
        &state,
        &selection,
        vec!["segment-a".into()],
        vec!["video-track".into()],
    )
    .expect("selection command should not mutate draft");
    assert_eq!(selected.draft, draft);
    assert_eq!(selected.selection.segment_ids, vec!["segment-a".into()]);
    assert_eq!(selected.selection.track_ids, vec!["video-track".into()]);
    assert_eq!(selected.events[0].kind, "timelineSelectionChanged");

    let moved = command_move_segment(
        &draft,
        &state,
        &selected.selection,
        "segment-a".into(),
        "video-track".into(),
        Microseconds::new(500_000),
    )
    .expect("move should change target start only");
    let moved_segment = &moved.draft.tracks[0].segments[0];
    assert_eq!(
        moved_segment.source_timerange,
        SourceTimerange::new(100_000, 400_000)
    );
    assert_eq!(
        moved_segment.target_timerange,
        TargetTimerange::new(500_000, 400_000)
    );
    assert_eq!(moved.events[0].kind, "segmentMoved");

    let split = command_split_segment(
        &draft,
        &state,
        &selected.selection,
        "segment-a".into(),
        "segment-b".into(),
        Microseconds::new(250_000),
    )
    .expect("split should create adjacent segments with adjusted source ranges");
    assert_eq!(split.draft.tracks[0].segments.len(), 2);
    assert_eq!(
        split.draft.tracks[0].segments[0].target_timerange,
        TargetTimerange::new(0, 250_000)
    );
    assert_eq!(
        split.draft.tracks[0].segments[0].source_timerange,
        SourceTimerange::new(100_000, 250_000)
    );
    assert_eq!(
        split.draft.tracks[0].segments[1].target_timerange,
        TargetTimerange::new(250_000, 150_000)
    );
    assert_eq!(
        split.draft.tracks[0].segments[1].source_timerange,
        SourceTimerange::new(350_000, 150_000)
    );
    assert_eq!(split.events[0].kind, "segmentSplit");

    let left_trimmed = command_trim_segment(
        &draft,
        &state,
        &selected.selection,
        "segment-a".into(),
        TrimSegmentDirection::Left,
        TargetTimerange::new(150_000, 250_000),
    )
    .expect("left trim should advance source start and shrink target");
    assert_eq!(
        left_trimmed.draft.tracks[0].segments[0].source_timerange,
        SourceTimerange::new(250_000, 250_000)
    );
    assert_eq!(
        left_trimmed.draft.tracks[0].segments[0].target_timerange,
        TargetTimerange::new(150_000, 250_000)
    );

    let right_trimmed = command_trim_segment(
        &draft,
        &state,
        &selected.selection,
        "segment-a".into(),
        TrimSegmentDirection::Right,
        TargetTimerange::new(0, 250_000),
    )
    .expect("right trim should preserve source start and shrink duration");
    assert_eq!(
        right_trimmed.draft.tracks[0].segments[0].source_timerange,
        SourceTimerange::new(100_000, 250_000)
    );
    assert_eq!(
        right_trimmed.draft.tracks[0].segments[0].target_timerange,
        TargetTimerange::new(0, 250_000)
    );
    assert_eq!(right_trimmed.events[0].kind, "segmentTrimmed");

    let deleted = command_delete_segment(&draft, &state, &selected.selection, "segment-a".into())
        .expect("delete should remove the segment and clean selection");
    assert!(deleted.draft.tracks[0].segments.is_empty());
    assert!(deleted.selection.segment_ids.is_empty());
    assert_eq!(deleted.events[0].kind, "segmentDeleted");
}

#[test]
fn intent_timeline_edits_are_rust_owned() {
    let mut draft = draft_with_tracks_and_materials();
    draft
        .tracks
        .push(Track::new("video-overlay", TrackKind::Video, "Overlay"));
    let state = CommandState::empty();
    let selected_overlay = TimelineSelection {
        segment_ids: Vec::new(),
        track_ids: vec!["video-overlay".into()],
    };

    let added =
        add_timeline_segment_intent(&draft, &state, &selected_overlay, "video-material".into())
            .expect("add intent should resolve selected compatible track and allocate segment ID");
    assert_eq!(added.selection.track_ids, vec!["video-overlay".into()]);
    assert_eq!(added.selection.segment_ids, vec!["segment-1".into()]);
    let added_segment = &added.draft.tracks[2].segments[0];
    assert_eq!(added_segment.segment_id.as_str(), "segment-1");
    assert_eq!(added_segment.material_id.as_str(), "video-material");
    assert_eq!(
        added_segment.source_timerange,
        SourceTimerange::new(0, 1_000_000)
    );
    assert_eq!(
        added_segment.target_timerange,
        TargetTimerange::new(0, 1_000_000)
    );

    let moved = move_selected_segment_intent(
        &added.draft,
        &added.command_state,
        &added.selection,
        Microseconds::new(250_000),
    )
    .expect("move intent should derive the selected segment and target start");
    assert_eq!(
        moved.draft.tracks[2].segments[0].target_timerange,
        TargetTimerange::new(250_000, 1_000_000)
    );

    let split = split_selected_segment_intent(
        &moved.draft,
        &moved.command_state,
        &moved.selection,
        Microseconds::new(750_000),
    )
    .expect("split intent should derive selected segment and allocate right segment ID");
    assert_eq!(
        split.selection.segment_ids,
        vec!["segment-1".into(), "segment-right-2".into()]
    );
    assert_eq!(
        split.draft.tracks[2].segments[0].target_timerange,
        TargetTimerange::new(250_000, 500_000)
    );
    assert_eq!(
        split.draft.tracks[2].segments[1].target_timerange,
        TargetTimerange::new(750_000, 500_000)
    );

    let left_selected = TimelineSelection {
        segment_ids: vec!["segment-1".into()],
        track_ids: vec!["video-overlay".into()],
    };
    let trimmed = trim_selected_segment_intent(
        &split.draft,
        &split.command_state,
        &left_selected,
        TrimSegmentDirection::Left,
        Microseconds::new(900_000),
    )
    .expect("trim intent should clamp oversized deltas to a valid one-frame-equivalent range");
    assert_eq!(
        trimmed.draft.tracks[2].segments[0].target_timerange,
        TargetTimerange::new(749_999, 1)
    );
    assert_eq!(
        trimmed.draft.tracks[2].segments[0].source_timerange,
        SourceTimerange::new(499_999, 1)
    );

    let track_added = add_track_intent(
        &trimmed.draft,
        &trimmed.command_state,
        &trimmed.selection,
        TrackKind::Text,
    )
    .expect("track intent should allocate track ID and name in Rust");
    let text_track = track_added
        .draft
        .tracks
        .last()
        .expect("track intent should add a track");
    assert_eq!(text_track.track_id.as_str(), "track-text-4");
    assert_eq!(text_track.name, "文字轨道 1");

    let text_added = add_text_segment_intent(
        &track_added.draft,
        &track_added.command_state,
        &TimelineSelection {
            segment_ids: Vec::new(),
            track_ids: vec![text_track.track_id.clone()],
        },
        text_segment("Rust owned text"),
        Microseconds::ZERO,
    )
    .expect("text intent should allocate text material and clamp duration");
    let text_segment = text_added
        .draft
        .tracks
        .last()
        .and_then(|track| track.segments.first())
        .expect("text intent should append a segment");
    assert_eq!(text_segment.segment_id.as_str(), "text-segment-3");
    assert_eq!(text_segment.material_id.as_str(), "text-material-3");
    let text_material = text_added
        .draft
        .materials
        .iter()
        .find(|material| material.material_id == text_segment.material_id)
        .expect("text intent should create a text material");
    assert_eq!(text_material.display_name, "Rust owned text");
    assert_eq!(text_segment.target_timerange, TargetTimerange::new(0, 1));
    assert_eq!(
        text_segment.text.as_ref().map(|text| text.content.as_str()),
        Some("Rust owned text")
    );

    let audio_added = add_audio_segment_intent(
        &text_added.draft,
        &text_added.command_state,
        &TimelineSelection {
            segment_ids: Vec::new(),
            track_ids: vec!["audio-track".into()],
        },
        None,
        None,
    )
    .expect("audio intent should choose the selected audio track and first audio material");
    let audio_segment = audio_added.draft.tracks[1]
        .segments
        .first()
        .expect("audio intent should append a segment");
    assert_eq!(audio_segment.segment_id.as_str(), "audio-segment-4");
    assert_eq!(audio_segment.material_id.as_str(), "audio-material");
    assert_eq!(
        audio_segment.target_timerange,
        TargetTimerange::new(0, 2_000_000)
    );
}

#[test]
fn invalid_edits_are_atomic() {
    let (draft, state, selection) = draft_with_existing_video_segment();

    let overlap = command_add_segment(
        &draft,
        &state,
        &selection,
        "video-track".into(),
        "overlap".into(),
        "video-material".into(),
        SourceTimerange::new(500_000, 250_000),
        TargetTimerange::new(100_000, 250_000),
    )
    .expect_err("same-track overlap should reject");
    assert!(matches!(
        overlap.kind,
        TimelineCommandErrorKind::OverlappingSegment { .. }
    ));

    let locked = {
        let mut locked = draft.clone();
        locked.tracks[0].locked = true;
        command_move_segment(
            &locked,
            &state,
            &selection,
            "segment-a".into(),
            "video-track".into(),
            Microseconds::new(600_000),
        )
        .expect_err("locked track mutation should reject")
    };
    assert!(matches!(
        locked.kind,
        TimelineCommandErrorKind::LockedTrack { .. }
    ));

    let material_overrun = command_add_segment(
        &draft,
        &state,
        &selection,
        "video-track".into(),
        "overrun".into(),
        "video-material".into(),
        SourceTimerange::new(900_000, 200_000),
        TargetTimerange::new(600_000, 200_000),
    )
    .expect_err("source ranges beyond material duration should reject");
    assert!(matches!(
        material_overrun.kind,
        TimelineCommandErrorKind::SourceRangeExceedsMaterialDuration { .. }
    ));

    let invalid_split = command_split_segment(
        &draft,
        &state,
        &selection,
        "segment-a".into(),
        "right-invalid".into(),
        Microseconds::new(400_000),
    )
    .expect_err("split at segment end should reject");
    assert!(matches!(
        invalid_split.kind,
        TimelineCommandErrorKind::InvalidSplitPoint { .. }
    ));

    let zero_trim = command_trim_segment(
        &draft,
        &state,
        &selection,
        "segment-a".into(),
        TrimSegmentDirection::Right,
        TargetTimerange::new(0, 0),
    )
    .expect_err("zero-duration trim should reject");
    assert!(matches!(
        zero_trim.kind,
        TimelineCommandErrorKind::ZeroDuration { .. }
    ));

    let missing_material = command_add_segment(
        &draft,
        &state,
        &selection,
        "video-track".into(),
        "missing".into(),
        "missing-material".into(),
        SourceTimerange::new(0, 100_000),
        TargetTimerange::new(600_000, 100_000),
    )
    .expect_err("missing material should reject");
    assert!(matches!(
        missing_material.kind,
        TimelineCommandErrorKind::MaterialNotFound { .. }
    ));

    let incompatible = command_add_segment(
        &draft,
        &state,
        &selection,
        "video-track".into(),
        "audio-on-video".into(),
        "audio-material".into(),
        SourceTimerange::new(0, 100_000),
        TargetTimerange::new(600_000, 100_000),
    )
    .expect_err("audio material on video track should reject");
    assert!(matches!(
        incompatible.kind,
        TimelineCommandErrorKind::IncompatibleTrackMaterialKind { .. }
    ));

    assert_eq!(draft, draft_with_existing_video_segment().0);
    assert_eq!(state, CommandState::empty());
    assert_eq!(selection, TimelineSelection::empty());
}

fn draft_with_existing_video_segment() -> (Draft, CommandState, TimelineSelection) {
    let mut draft = draft_with_tracks_and_materials();
    draft.tracks[0].segments.push(segment(
        "segment-a",
        "video-material",
        100_000,
        400_000,
        0,
        400_000,
    ));
    (draft, CommandState::empty(), TimelineSelection::empty())
}

fn draft_with_tracks_and_materials() -> Draft {
    let mut draft = Draft::new("timeline-command-draft", "Timeline Commands");
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

fn text_segment(content: &str) -> TextSegment {
    TextSegment {
        content: content.to_owned(),
        source: TextSegmentSource::Text,
        style: Default::default(),
        text_box: Default::default(),
        layout_region: Default::default(),
        wrapping: Default::default(),
        bubble: None,
        effect: None,
    }
}
