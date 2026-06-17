use draft_model::{
    Draft, Material, MaterialKind, MaterialStatus, Microseconds, RationalFrameRate, Segment,
    SourceTimerange, TargetTimerange, TextAlignment, TextSegment, TextStyle, Track, TrackKind,
};
use engine_core::{EngineErrorKind, EngineProfile, MaterialRenderableState, normalize_draft};

#[test]
fn normalization_available_material_segments_are_sorted_by_track_and_target_timerange() {
    let draft = complete_draft();

    let normalized = normalize_draft(&draft, &EngineProfile::mvp_default())
        .expect("available draft should normalize");

    assert_eq!(normalized.draft_id.as_str(), "draft-normalize");
    assert_eq!(normalized.tracks.len(), 4);
    assert_eq!(
        normalized
            .tracks
            .iter()
            .map(|track| (track.track_id.as_str(), track.kind))
            .collect::<Vec<_>>(),
        vec![
            ("video-track", TrackKind::Video),
            ("image-track", TrackKind::Video),
            ("text-track", TrackKind::Text),
            ("audio-track", TrackKind::Audio),
        ]
    );
    assert_eq!(
        normalized.tracks[0]
            .segments
            .iter()
            .map(|segment| segment.segment_id.as_str())
            .collect::<Vec<_>>(),
        vec!["video-a", "video-b"]
    );
    assert_eq!(normalized.tracks[0].stack_index, Some(0));
    assert_eq!(normalized.tracks[1].stack_index, Some(1));
    assert_eq!(normalized.tracks[2].stack_index, Some(2));
    assert_eq!(normalized.tracks[3].stack_index, None);
    assert_eq!(
        normalized.tracks[2].segments[0]
            .text
            .as_ref()
            .expect("text segment should keep Jianying text semantics")
            .content,
        "字幕"
    );
    assert!(normalized.diagnostics.is_empty());
}

#[test]
fn normalization_classifies_muted_tracks_and_unavailable_materials_without_mutating_draft() {
    let mut draft = complete_draft();
    draft.tracks[0].muted = true;
    draft.materials[1].status = MaterialStatus::Missing;
    let before = serde_json::to_value(&draft).expect("draft should serialize");

    let normalized = normalize_draft(&draft, &EngineProfile::mvp_default())
        .expect("muted and missing material diagnostics are renderable-state, not mutation");

    let after = serde_json::to_value(&draft).expect("draft should serialize");
    assert_eq!(before, after);
    assert_eq!(normalized.diagnostics.len(), 3);
    assert!(normalized.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == EngineErrorKind::MutedTrack
            && diagnostic.track_id.as_ref().map(|id| id.as_str()) == Some("video-track")
    }));
    assert!(normalized.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == EngineErrorKind::UnavailableMaterial
            && diagnostic.material_id.as_ref().map(|id| id.as_str()) == Some("image-material")
    }));
    assert_eq!(
        normalized.tracks[0].segments[0].renderable,
        MaterialRenderableState::MutedTrack
    );
    assert_eq!(
        normalized.tracks[1].segments[0].renderable,
        MaterialRenderableState::UnavailableMaterial
    );
}

#[test]
fn normalization_rejects_checked_timerange_overflow_and_out_of_bounds_material_ranges() {
    let mut overflow = complete_draft();
    overflow.tracks[0].segments[0].source_timerange = SourceTimerange::new(u64::MAX, 1);

    let overflow_error = normalize_draft(&overflow, &EngineProfile::mvp_default())
        .expect_err("overflowing sourceTimerange should be rejected");
    assert_eq!(overflow_error.kind, EngineErrorKind::TimerangeOverflow);

    let mut out_of_bounds = complete_draft();
    out_of_bounds.tracks[0].segments[0].source_timerange = SourceTimerange::new(2_900_000, 200_000);

    let bounds_error = normalize_draft(&out_of_bounds, &EngineProfile::mvp_default())
        .expect_err("sourceTimerange beyond material duration should be rejected");
    assert_eq!(
        bounds_error.kind,
        EngineErrorKind::SourceRangeExceedsMaterialDuration
    );
}

#[test]
fn normalization_visual_track_stacking_uses_existing_rust_timeline_order() {
    let mut draft = complete_draft();
    draft.tracks.swap(0, 2);

    let normalized = normalize_draft(&draft, &EngineProfile::mvp_default())
        .expect("reordered draft should normalize");

    let visual_stack = normalized
        .tracks
        .iter()
        .filter_map(|track| {
            track
                .stack_index
                .map(|stack| (stack, track.track_id.as_str()))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        visual_stack,
        vec![(0, "text-track"), (1, "image-track"), (2, "video-track"),]
    );
}

fn complete_draft() -> Draft {
    let mut draft = Draft::new("draft-normalize", "Normalize");
    draft.materials = vec![
        material(
            "video-material",
            MaterialKind::Video,
            "file://video.mp4",
            3_000_000,
        ),
        material(
            "image-material",
            MaterialKind::Image,
            "file://image.png",
            3_000_000,
        ),
        material(
            "audio-material",
            MaterialKind::Audio,
            "file://audio.wav",
            3_000_000,
        ),
        material(
            "text-material",
            MaterialKind::Text,
            "text://caption",
            3_000_000,
        ),
    ];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track.segments = vec![
        segment("video-b", "video-material", 1_000_000, 500_000, 1_000_000),
        segment("video-a", "video-material", 0, 0, 1_000_000),
    ];

    let mut image_track = Track::new("image-track", TrackKind::Video, "图片");
    image_track
        .segments
        .push(segment("image-a", "image-material", 0, 250_000, 1_000_000));

    let mut text_track = Track::new("text-track", TrackKind::Text, "文字");
    let mut text = segment("text-a", "text-material", 0, 0, 1_000_000);
    text.text = Some(TextSegment {
        content: "字幕".to_owned(),
        style: TextStyle {
            font_size: 42,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            stroke: None,
            shadow: None,
            background: None,
        },
    });
    text_track.segments.push(text);

    let mut audio_track = Track::new("audio-track", TrackKind::Audio, "音频");
    audio_track
        .segments
        .push(segment("audio-a", "audio-material", 0, 0, 1_000_000));

    draft.tracks = vec![video_track, image_track, text_track, audio_track];
    draft
}

fn material(material_id: &str, kind: MaterialKind, uri: &str, duration: u64) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(duration));
    material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    material.metadata.width = Some(1920);
    material.metadata.height = Some(1080);
    material.metadata.has_video = matches!(kind, MaterialKind::Video | MaterialKind::Image);
    material.metadata.has_audio = matches!(kind, MaterialKind::Audio | MaterialKind::Video);
    material
}

fn segment(
    segment_id: &str,
    material_id: &str,
    source_start: u64,
    target_start: u64,
    duration: u64,
) -> Segment {
    Segment::new(
        segment_id,
        material_id,
        SourceTimerange::new(source_start, duration),
        TargetTimerange::new(target_start, duration),
    )
}
