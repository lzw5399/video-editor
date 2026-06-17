use draft_commands::{
    TimelineCommandErrorKind,
    timeline::{
        audio_track_mix_order, checked_source_end, checked_target_end, main_video_track_id,
        target_ranges_overlap, validate_segment_material_bounds, validate_timeline_rules,
        validate_track_material_compatibility, validate_track_unlocked, visual_track_stack_order,
    },
};
use draft_model::{
    Draft, Material, MaterialKind, Microseconds, Segment, SourceTimerange, TargetTimerange, Track,
    TrackKind, validate_draft,
};

#[test]
fn timeline_tracks() {
    let draft = draft_with_root_video_audio_text_tracks();

    validate_draft(&draft).expect("root Draft.tracks fixture should be valid draft state");
    validate_timeline_rules(&draft).expect("root Draft.tracks should satisfy timeline rules");

    assert_eq!(
        visual_track_stack_order(&draft),
        vec!["video-track".into(), "text-track".into()]
    );
    assert_eq!(audio_track_mix_order(&draft), vec!["audio-track".into()]);
    assert_eq!(main_video_track_id(&draft), Some("video-track".into()));
}

#[test]
fn track_rules() {
    let mut draft = draft_with_root_video_audio_text_tracks();
    draft.tracks[0].segments.push(segment(
        "video-segment-a",
        "video-material",
        0,
        1_000_000,
        0,
        1_000_000,
    ));
    draft.tracks[0].segments.push(segment(
        "video-segment-b",
        "video-material",
        500_000,
        500_000,
        500_000,
        500_000,
    ));

    let error = validate_timeline_rules(&draft).expect_err("same-track overlap should be rejected");
    assert_eq!(
        error.kind,
        TimelineCommandErrorKind::OverlappingSegment {
            track_id: "video-track".into(),
            first_segment_id: "video-segment-a".into(),
            second_segment_id: "video-segment-b".into(),
        }
    );

    let mut locked_track = Track::new("locked-video-track", TrackKind::Video, "Locked Video");
    locked_track.locked = true;
    let error =
        validate_track_unlocked(&locked_track).expect_err("locked track mutation should reject");
    assert_eq!(
        error.kind,
        TimelineCommandErrorKind::LockedTrack {
            track_id: "locked-video-track".into(),
        }
    );

    let video_track = Track::new("compatible-video-track", TrackKind::Video, "Video");
    let audio_material =
        Material::new("audio-material-only", MaterialKind::Audio, "bgm.wav", "BGM");
    let error = validate_track_material_compatibility(&video_track, &audio_material)
        .expect_err("audio material should not be compatible with a video track");
    assert_eq!(
        error.kind,
        TimelineCommandErrorKind::IncompatibleTrackMaterialKind {
            track_id: "compatible-video-track".into(),
            track_kind: TrackKind::Video,
            material_id: "audio-material-only".into(),
            material_kind: MaterialKind::Audio,
        }
    );

    let mut overlay_draft = draft_with_root_video_audio_text_tracks();
    overlay_draft.tracks[0].segments.push(segment(
        "video-overlay-time",
        "video-material",
        0,
        1_000_000,
        0,
        1_000_000,
    ));
    overlay_draft.tracks[2].segments.push(segment(
        "text-overlay-time",
        "text-material",
        0,
        1_000_000,
        0,
        1_000_000,
    ));
    overlay_draft.tracks[1].muted = true;

    validate_timeline_rules(&overlay_draft)
        .expect("simultaneous segments on separate tracks should be accepted");
    assert!(
        overlay_draft.tracks[1].muted,
        "track mute state should remain persisted draft state"
    );
}

#[test]
fn timerange_rules() {
    assert_eq!(
        checked_source_end(&SourceTimerange::new(250_000, 750_000)).expect("valid source end"),
        Microseconds::new(1_000_000)
    );
    assert_eq!(
        checked_target_end(&TargetTimerange::new(1_000_000, 500_000)).expect("valid target end"),
        Microseconds::new(1_500_000)
    );
    assert!(
        target_ranges_overlap(
            &TargetTimerange::new(0, 1_000_000),
            &TargetTimerange::new(1_000_000, 500_000),
        )
        .expect("adjacent ranges should be comparable")
            == false
    );

    let error = checked_source_end(&SourceTimerange::new(u64::MAX, 1))
        .expect_err("source end overflow should reject");
    assert_eq!(
        error.kind,
        TimelineCommandErrorKind::TimerangeOverflow {
            field: "sourceTimerange".to_owned(),
        }
    );

    let mut zero_duration = draft_with_root_video_audio_text_tracks();
    zero_duration.tracks[0]
        .segments
        .push(segment("zero-duration", "video-material", 0, 0, 0, 0));
    let error =
        validate_timeline_rules(&zero_duration).expect_err("zero-duration ranges should reject");
    assert_eq!(
        error.kind,
        TimelineCommandErrorKind::ZeroDuration {
            field: "sourceTimerange.duration".to_owned(),
        }
    );

    let mut source_overrun = draft_with_root_video_audio_text_tracks();
    source_overrun.tracks[0].segments.push(segment(
        "source-overrun",
        "video-material",
        750_000,
        500_000,
        0,
        500_000,
    ));
    let error = validate_segment_material_bounds(&source_overrun)
        .expect_err("source range beyond known material duration should reject");
    assert_eq!(
        error.kind,
        TimelineCommandErrorKind::SourceRangeExceedsMaterialDuration {
            segment_id: "source-overrun".into(),
            material_id: "video-material".into(),
            source_end: Microseconds::new(1_250_000),
            material_duration: Microseconds::new(1_000_000),
        }
    );
}

fn draft_with_root_video_audio_text_tracks() -> Draft {
    let mut draft = Draft::new("timeline-root-draft", "Timeline Root Draft");
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
    draft.materials.push(material_with_duration(
        "text-material",
        MaterialKind::Text,
        "internal://text/title",
        1_000_000,
    ));
    draft
        .tracks
        .push(Track::new("video-track", TrackKind::Video, "Video"));
    draft
        .tracks
        .push(Track::new("audio-track", TrackKind::Audio, "Audio"));
    draft
        .tracks
        .push(Track::new("text-track", TrackKind::Text, "Text"));
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
