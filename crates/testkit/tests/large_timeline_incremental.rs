use draft_model::{Microseconds, TargetTimerange, TrackKind, validate_draft};
use testkit::large_timeline::{
    LargeTimelineConfig, MAX_SEGMENTS_PER_TRACK, assert_no_track_overlaps, build_large_timeline,
};

#[test]
fn large_timeline_fixture_is_deterministic_and_valid() {
    let config = LargeTimelineConfig::new(240).with_localized_edit_index(120);

    let first = build_large_timeline(config.clone()).expect("first fixture should build");
    let second = build_large_timeline(config).expect("second fixture should build");

    validate_draft(&first.draft).expect("large draft should validate");
    assert_no_track_overlaps(&first.draft).expect("large draft tracks should not overlap");
    assert_eq!(
        serde_json::to_value(&first.draft).expect("first draft serializes"),
        serde_json::to_value(&second.draft).expect("second draft serializes"),
        "large timeline fixtures should be deterministic"
    );
    assert_eq!(first.draft.tracks.len(), 3);
    assert_eq!(first.draft.materials.len(), 720);
    assert_eq!(first.localized_edit, second.localized_edit);
}

#[test]
fn large_timeline_fixture_exposes_stable_localized_edit_coordinates() {
    let fixture = build_large_timeline(
        LargeTimelineConfig::new(360)
            .with_localized_edit_index(123)
            .with_segment_duration(Microseconds::new(80_000)),
    )
    .expect("fixture should build");

    assert_eq!(fixture.localized_edit.track_kind, TrackKind::Video);
    assert_eq!(fixture.localized_edit.track_id.as_str(), "video-track-000");
    assert_eq!(
        fixture.localized_edit.segment_id.as_str(),
        "video-segment-000123"
    );
    assert_eq!(
        fixture.localized_edit.material_id.as_str(),
        "video-material-000123"
    );
    assert_eq!(fixture.localized_edit.segment_index, 123);
    assert_eq!(
        fixture.localized_edit.target_timerange,
        TargetTimerange::new(9_840_000, 80_000)
    );
}

#[test]
fn large_timeline_fixture_supports_track_mix_knobs_without_runtime_dependencies() {
    let fixture = build_large_timeline(
        LargeTimelineConfig::new(128)
            .with_track_mix(false, true, true)
            .with_localized_edit_index(64),
    )
    .expect("audio/text fixture should build");

    assert_eq!(fixture.draft.tracks.len(), 2);
    assert_eq!(fixture.draft.materials.len(), 256);
    assert_eq!(fixture.localized_edit.track_kind, TrackKind::Audio);
    assert_eq!(fixture.localized_edit.track_id.as_str(), "audio-track-000");
    assert_eq!(
        fixture.localized_edit.target_timerange,
        TargetTimerange::new(6_400_000, 100_000)
    );
}

#[test]
fn large_timeline_fixture_bounds_segment_counts() {
    let zero = build_large_timeline(LargeTimelineConfig::new(0))
        .expect_err("zero segments should be rejected");
    assert!(zero.to_string().contains("greater than zero"));

    let too_many = build_large_timeline(LargeTimelineConfig::new(MAX_SEGMENTS_PER_TRACK + 1))
        .expect_err("unbounded large timelines should be rejected");
    assert!(too_many.to_string().contains("segments_per_track"));
}
