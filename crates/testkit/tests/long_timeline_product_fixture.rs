use draft_model::{MaterialKind, Microseconds, TrackKind, validate_draft};
use testkit::large_timeline::{
    PHASE20_BLOCKING_SEGMENTS_PER_TRACK, PHASE20_DIAGNOSTIC_SEGMENTS_PER_TRACK,
    PHASE20_PRODUCT_SEGMENTS_PER_TRACK, PHASE20_SEGMENT_DURATION_US, Phase20ProductMediaUris,
    assert_no_track_overlaps, build_phase20_product_timeline, phase20_product_timeline_config,
};

#[test]
fn phase20_product_fixture_config_matches_locked_scale() {
    let config = phase20_product_timeline_config();

    assert_eq!(PHASE20_PRODUCT_SEGMENTS_PER_TRACK, 180);
    assert_eq!(PHASE20_BLOCKING_SEGMENTS_PER_TRACK, 1_000);
    assert_eq!(PHASE20_DIAGNOSTIC_SEGMENTS_PER_TRACK, 3_000);
    assert_eq!(PHASE20_SEGMENT_DURATION_US, 1_000_000);
    assert_eq!(config.segments_per_track, 180);
    assert_eq!(config.track_count(), 3);
    assert_eq!(config.total_segment_count(), 540);
    assert_eq!(
        config.segment_duration,
        Microseconds::new(PHASE20_SEGMENT_DURATION_US)
    );
    assert_eq!(
        config.target_stride,
        Microseconds::new(PHASE20_SEGMENT_DURATION_US)
    );
}

#[test]
fn phase20_product_fixture_uses_real_video_and_audio_uris() {
    let media = Phase20ProductMediaUris::new(
        "/repo/apps/desktop-electron/tests/fixtures/media/p0-long-av-tone-testsrc.mp4",
        "/repo/apps/desktop-electron/tests/fixtures/media/p0-long-tone.wav",
    );
    let fixture = build_phase20_product_timeline(media.clone())
        .expect("phase 20 product fixture should build");

    let video_materials = fixture
        .draft
        .materials
        .iter()
        .filter(|material| material.kind == MaterialKind::Video)
        .collect::<Vec<_>>();
    let audio_materials = fixture
        .draft
        .materials
        .iter()
        .filter(|material| material.kind == MaterialKind::Audio)
        .collect::<Vec<_>>();

    assert_eq!(video_materials.len(), PHASE20_PRODUCT_SEGMENTS_PER_TRACK);
    assert_eq!(audio_materials.len(), PHASE20_PRODUCT_SEGMENTS_PER_TRACK);
    assert!(
        video_materials
            .iter()
            .all(|material| material.uri == media.video_uri),
        "video materials should use the supplied product media URI"
    );
    assert!(
        audio_materials
            .iter()
            .all(|material| material.uri == media.audio_uri),
        "audio materials should use the supplied product media URI"
    );
    assert!(
        fixture
            .draft
            .materials
            .iter()
            .filter(|material| material.kind != MaterialKind::Text)
            .all(|material| {
                !material.uri.starts_with("video://phase13/")
                    && !material.uri.starts_with("audio://phase13/")
            }),
        "product media materials must not keep synthetic Phase 13 media URIs"
    );
}

#[test]
fn phase20_product_fixture_is_valid_and_overlap_free() {
    let fixture = build_phase20_product_timeline(Phase20ProductMediaUris::new(
        "/repo/apps/desktop-electron/tests/fixtures/media/p0-long-av-tone-testsrc.mp4",
        "/repo/apps/desktop-electron/tests/fixtures/media/p0-long-tone.wav",
    ))
    .expect("phase 20 product fixture should build");

    validate_draft(&fixture.draft).expect("phase 20 product draft should validate");
    assert_no_track_overlaps(&fixture.draft).expect("phase 20 product tracks should not overlap");
    assert_eq!(fixture.draft.tracks.len(), 3);
    assert_eq!(fixture.draft.materials.len(), 540);
    assert_eq!(
        fixture
            .draft
            .tracks
            .iter()
            .map(|track| track.kind)
            .collect::<Vec<_>>(),
        vec![TrackKind::Video, TrackKind::Audio, TrackKind::Text]
    );
}
