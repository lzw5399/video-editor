use draft_commands::{
    TimelineCommandErrorKind,
    history::{redo_timeline_edit, undo_timeline_edit},
    retiming::set_segment_retime,
    timeline::{execute_timeline_edit, move_segment, split_segment, trim_segment},
};
use draft_model::{
    AudioRetimePolicy, ClearSegmentRetimeCommandPayload, CommandState, DirtyDomain, Draft,
    Material, MaterialKind, Microseconds, RetimeMode, Segment, SegmentRetiming,
    SetSegmentRetimeCommandPayload, SourceTimerange, SpeedCurvePoint, SpeedRatio, TargetTimerange,
    TimelineEditPayload, TimelineSelection, Track, TrackKind, TrimSegmentDirection,
};

#[test]
fn phase19_retiming_commands_set_and_clear_commit_once_and_route_through_timeline_payloads() {
    let draft = draft_with_video_segment(4_000_000, 2_000_000);
    let state = CommandState::empty();
    let selection = selected_segment_context();
    let retiming = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    };

    let updated = set_segment_retime(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        retiming.clone(),
    )
    .expect("valid 2x retime should commit");

    assert_eq!(updated.events[0].kind, "segmentRetimeSet");
    assert_eq!(updated.draft.tracks[0].segments[0].retiming, retiming);
    assert_eq!(
        draft.tracks[0].segments[0].retiming,
        SegmentRetiming::default(),
        "input draft stays unchanged"
    );
    assert_eq!(updated.selection, selection);
    assert_eq!(updated.command_state.undo_stack.len(), 1);
    assert_eq!(
        updated.command_state.undo_stack[0].label.as_deref(),
        Some("setSegmentRetime")
    );
    assert!(updated.delta.changed_domains.contains(&DirtyDomain::Timing));
    assert!(updated.delta.changed_domains.contains(&DirtyDomain::Audio));

    let undone = undo_timeline_edit(&updated.draft, &updated.command_state, &updated.selection)
        .expect("retime set should be undoable");
    assert_eq!(undone.draft, draft);

    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("retime set should be redoable");
    assert_eq!(redone.draft, updated.draft);

    let routed_clear = execute_timeline_edit(TimelineEditPayload::ClearSegmentRetime(
        ClearSegmentRetimeCommandPayload {
            draft: redone.draft,
            command_state: redone.command_state,
            selection: redone.selection,
            segment_id: "video-segment".into(),
        },
    ))
    .expect("timeline dispatcher should route clear retime");
    assert_eq!(
        routed_clear.draft.tracks[0].segments[0].retiming,
        SegmentRetiming::default()
    );
    assert_eq!(routed_clear.events[0].kind, "segmentRetimeCleared");

    let routed_set = execute_timeline_edit(TimelineEditPayload::SetSegmentRetime(
        SetSegmentRetimeCommandPayload {
            draft: routed_clear.draft,
            command_state: routed_clear.command_state,
            selection: routed_clear.selection,
            segment_id: "video-segment".into(),
            retiming: SegmentRetiming {
                mode: RetimeMode::SpeedCurve {
                    points: vec![
                        SpeedCurvePoint {
                            target_time: Microseconds::new(0),
                            speed: SpeedRatio::new(1, 1),
                        },
                        SpeedCurvePoint {
                            target_time: Microseconds::new(1_000_000),
                            speed: SpeedRatio::new(2, 1),
                        },
                    ],
                },
                audio_policy: AudioRetimePolicy::MuteUnsupported,
            },
        },
    ))
    .expect("timeline dispatcher should route speed curve retime");
    assert!(matches!(
        routed_set.draft.tracks[0].segments[0].retiming.mode,
        RetimeMode::SpeedCurve { .. }
    ));
}

#[test]
fn phase19_retiming_commands_invalid_retime_rejects_atomically_with_structured_errors() {
    let draft = draft_with_video_segment(1_000_000, 1_000_000);
    let state = CommandState::empty();
    let selection = selected_segment_context();
    let mut locked_draft = draft.clone();
    locked_draft.tracks[0].locked = true;

    let invalid_ratio = set_segment_retime(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        SegmentRetiming {
            mode: RetimeMode::Constant {
                speed: SpeedRatio::new(0, 1),
            },
            audio_policy: AudioRetimePolicy::FollowVideoSpeed,
        },
    )
    .expect_err("zero speed numerator should reject before mutation");
    assert!(matches!(
        invalid_ratio.kind,
        TimelineCommandErrorKind::InvalidRetime { .. }
    ));

    let non_monotonic_curve = set_segment_retime(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        SegmentRetiming {
            mode: RetimeMode::SpeedCurve {
                points: vec![
                    SpeedCurvePoint {
                        target_time: Microseconds::new(800_000),
                        speed: SpeedRatio::new(1, 1),
                    },
                    SpeedCurvePoint {
                        target_time: Microseconds::new(200_000),
                        speed: SpeedRatio::new(1, 1),
                    },
                ],
            },
            audio_policy: AudioRetimePolicy::FollowVideoSpeed,
        },
    )
    .expect_err("non-monotonic speed curve should reject");
    assert!(matches!(
        non_monotonic_curve.kind,
        TimelineCommandErrorKind::InvalidRetime { .. }
    ));

    let source_overflow = set_segment_retime(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        SegmentRetiming {
            mode: RetimeMode::Constant {
                speed: SpeedRatio::new(2, 1),
            },
            audio_policy: AudioRetimePolicy::FollowVideoSpeed,
        },
    )
    .expect_err("2x retime should require enough source duration");
    assert!(matches!(
        source_overflow.kind,
        TimelineCommandErrorKind::InvalidRetime { .. }
    ));

    let unsupported_audio = set_segment_retime(
        &draft_with_video_segment(4_000_000, 2_000_000),
        &state,
        &selection,
        "video-segment".into(),
        SegmentRetiming {
            mode: RetimeMode::Constant {
                speed: SpeedRatio::new(2, 1),
            },
            audio_policy: AudioRetimePolicy::PreservePitch,
        },
    )
    .expect_err("pitch preservation for non-1x retime is unsupported");
    assert!(matches!(
        unsupported_audio.kind,
        TimelineCommandErrorKind::UnsupportedAudioRetime { .. }
    ));

    let locked = set_segment_retime(
        &locked_draft,
        &state,
        &selection,
        "video-segment".into(),
        SegmentRetiming::default(),
    )
    .expect_err("locked tracks reject retime commands");
    assert!(matches!(
        locked.kind,
        TimelineCommandErrorKind::LockedTrack { .. }
    ));

    let missing = set_segment_retime(
        &draft,
        &state,
        &selection,
        "missing-segment".into(),
        SegmentRetiming::default(),
    )
    .expect_err("missing segment should reject");
    assert!(matches!(
        missing.kind,
        TimelineCommandErrorKind::SegmentNotFound { .. }
    ));

    assert_eq!(draft, draft_with_video_segment(1_000_000, 1_000_000));
    assert_eq!(state, CommandState::empty());
    assert_eq!(selection, selected_segment_context());
}

#[test]
fn phase19_retiming_commands_split_and_trim_preserve_retimed_source_mapping() {
    let mut draft = draft_with_video_segment(4_000_000, 2_000_000);
    draft.tracks[0].segments[0].retiming = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    };
    let state = CommandState::empty();
    let selection = selected_segment_context();

    let split = split_segment(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        "video-segment-right".into(),
        Microseconds::new(500_000),
    )
    .expect("retimed split should commit");
    let left = &split.draft.tracks[0].segments[0];
    let right = &split.draft.tracks[0].segments[1];
    assert_eq!(left.target_timerange.duration, Microseconds::new(500_000));
    assert_eq!(left.source_timerange.duration, Microseconds::new(1_000_000));
    assert_eq!(right.target_timerange.start, Microseconds::new(500_000));
    assert_eq!(
        right.target_timerange.duration,
        Microseconds::new(1_500_000)
    );
    assert_eq!(right.source_timerange.start, Microseconds::new(1_000_000));
    assert_eq!(
        right.source_timerange.duration,
        Microseconds::new(3_000_000)
    );
    assert_eq!(left.retiming, right.retiming);

    let trimmed = trim_segment(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        TrimSegmentDirection::Left,
        TargetTimerange::new(500_000, 1_500_000),
    )
    .expect("retimed left trim should commit");
    let segment = &trimmed.draft.tracks[0].segments[0];
    assert_eq!(segment.target_timerange.start, Microseconds::new(500_000));
    assert_eq!(
        segment.target_timerange.duration,
        Microseconds::new(1_500_000)
    );
    assert_eq!(segment.source_timerange.start, Microseconds::new(1_000_000));
    assert_eq!(
        segment.source_timerange.duration,
        Microseconds::new(3_000_000)
    );
    assert_eq!(segment.retiming, draft.tracks[0].segments[0].retiming);

    let moved = move_segment(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        "video-track".into(),
        Microseconds::new(1_000_000),
    )
    .expect("retimed move should preserve source mapping");
    let moved_segment = &moved.draft.tracks[0].segments[0];
    assert_eq!(
        moved_segment.target_timerange.start,
        Microseconds::new(1_000_000)
    );
    assert_eq!(
        moved_segment.source_timerange,
        draft.tracks[0].segments[0].source_timerange
    );
    assert_eq!(moved_segment.retiming, draft.tracks[0].segments[0].retiming);
}

fn draft_with_video_segment(source_duration: u64, target_duration: u64) -> Draft {
    let mut draft = Draft::new("retime-command-draft", "Retime Command Draft");
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "media/video.mp4",
        "video.mp4",
    );
    material.metadata.duration = Some(Microseconds::new(4_000_000));
    draft.materials.push(material);

    let mut track = Track::new("video-track", TrackKind::Video, "Video");
    track.segments.push(Segment::new(
        "video-segment",
        "video-material",
        SourceTimerange::new(0, source_duration),
        TargetTimerange::new(0, target_duration),
    ));
    draft.tracks.push(track);
    draft
}

fn selected_segment_context() -> TimelineSelection {
    TimelineSelection {
        segment_ids: vec!["video-segment".into()],
        track_ids: vec!["video-track".into()],
    }
}
