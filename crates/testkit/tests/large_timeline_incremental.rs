use draft_model::{
    CanvasAspectRatio, CanvasBackground, DirtyDomain, DirtyRange, DirtyRangeSource, Draft,
    DraftCanvasConfig, Microseconds, RationalFrameRate, SegmentOpacity, SegmentVolume,
    TargetTimerange, TextSegment, TrackKind, validate_draft,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use preview_service::{
    PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile,
    PreviewInvalidationRequest, invalidate_preview_cache,
};
use render_graph::{
    OutputDimensions, RenderGraphDiff, RenderGraphSnapshot, RenderOutputProfile, build_render_graph,
};
use testkit::large_timeline::{
    LargeTimelineConfig, MAX_SEGMENTS_PER_TRACK, assert_no_track_overlaps, build_large_timeline,
};

#[test]
fn large_timeline_incremental_fixture_is_deterministic_and_valid() {
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
fn large_timeline_incremental_fixture_exposes_stable_localized_edit_coordinates() {
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
fn large_timeline_incremental_fixture_supports_track_mix_knobs_without_runtime_dependencies() {
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
fn large_timeline_incremental_fixture_bounds_segment_counts() {
    let zero = build_large_timeline(LargeTimelineConfig::new(0))
        .expect_err("zero segments should be rejected");
    assert!(zero.to_string().contains("greater than zero"));

    let too_many = build_large_timeline(LargeTimelineConfig::new(MAX_SEGMENTS_PER_TRACK + 1))
        .expect_err("unbounded large timelines should be rejected");
    assert!(too_many.to_string().contains("segments_per_track"));
}

#[test]
fn large_timeline_incremental_localized_move_has_bounded_graph_diff_dirty_ranges_and_cache_scope() {
    let fixture = build_large_timeline(
        LargeTimelineConfig::new(360)
            .with_localized_edit_index(180)
            .with_target_stride(Microseconds::new(250_000)),
    )
    .expect("large timeline fixture should build");
    let full_range = full_draft_range(&fixture.draft);
    let previous = snapshot_for(&fixture.draft, full_range.clone());

    let mut edited = fixture.draft.clone();
    let moved_start = fixture.localized_edit.target_timerange.start.get() + 100_000;
    segment_mut(
        &mut edited,
        fixture.localized_edit.track_kind,
        fixture.localized_edit.segment_index,
    )
    .target_timerange = TargetTimerange::new(
        moved_start,
        fixture.localized_edit.target_timerange.duration,
    );
    assert_no_track_overlaps(&edited).expect("localized move should stay inside the segment gap");

    let current = snapshot_for(&edited, full_range);
    let dirty_ranges = vec![
        dirty_range(
            fixture.localized_edit.target_timerange.start.get(),
            fixture.localized_edit.target_timerange.duration.get(),
            DirtyRangeSource::Previous,
        ),
        dirty_range(
            moved_start,
            fixture.localized_edit.target_timerange.duration.get(),
            DirtyRangeSource::Current,
        ),
    ];
    let diff = RenderGraphDiff::between(
        &previous,
        &current,
        &dirty_ranges,
        &[DirtyDomain::Timing, DirtyDomain::GraphSnapshot],
    );
    let changed_keys = changed_keys(&diff);
    let edited_key = segment_node_key(
        &fixture.draft,
        fixture.localized_edit.track_id.as_str(),
        fixture.localized_edit.segment_id.as_str(),
        "video",
    );

    assert!(
        diff.added.is_empty(),
        "localized move should keep node identity stable"
    );
    assert!(
        diff.removed.is_empty(),
        "localized move should not remove semantic graph nodes"
    );
    assert!(
        changed_keys.contains(&edited_key),
        "localized segment node should be the primary changed node: {changed_keys:?}"
    );
    assert!(
        diff.changed.len() <= 16,
        "localized move should touch only the segment and nearby sampled frames, changed={}",
        diff.changed.len()
    );
    assert!(
        diff.unchanged.len() > diff.changed.len() * 100,
        "large graph should remain mostly unchanged: changed={} unchanged={}",
        diff.changed.len(),
        diff.unchanged.len()
    );
    assert_eq!(diff.dirty_ranges, dirty_ranges);

    let entries = vec![
        cache_entry_with_deps(
            "old-range",
            fixture.localized_edit.target_timerange.start.get(),
            100_000,
            &fixture.localized_edit.material_id,
            &edited_key,
        ),
        cache_entry_with_deps(
            "current-range",
            moved_start,
            100_000,
            &fixture.localized_edit.material_id,
            &edited_key,
        ),
        cache_entry_with_deps(
            "unrelated-before",
            1_000_000,
            100_000,
            &draft_model::MaterialId::new("stable-material"),
            "stable-node-before",
        ),
        cache_entry_with_deps(
            "unrelated-after",
            75_000_000,
            100_000,
            &draft_model::MaterialId::new("stable-material"),
            "stable-node-after",
        ),
    ];
    let request = PreviewInvalidationRequest::new(
        diff.dirty_ranges.clone(),
        [fixture.localized_edit.material_id.clone()],
        [edited_key],
        [DirtyDomain::PreviewCache],
        "large localized move",
    );
    let result = invalidate_preview_cache(&entries, &request);

    assert_eq!(invalidated_ids(&result), vec!["old-range", "current-range"]);
    assert_eq!(result.retained.len(), 2);
}

#[test]
fn large_timeline_incremental_localized_trim_text_volume_and_visual_edits_keep_graph_diff_bounded()
{
    let fixture = build_large_timeline(
        LargeTimelineConfig::new(420)
            .with_localized_edit_index(210)
            .with_target_stride(Microseconds::new(200_000)),
    )
    .expect("large timeline fixture should build");
    let full_range = full_draft_range(&fixture.draft);

    assert_localized_change_is_bounded(
        &fixture.draft,
        full_range.clone(),
        "trim",
        DirtyDomain::Timing,
        |draft| {
            let segment = segment_mut(draft, TrackKind::Video, 210);
            segment.target_timerange.duration = Microseconds::new(60_000);
        },
    );
    assert_localized_change_is_bounded(
        &fixture.draft,
        full_range.clone(),
        "visual",
        DirtyDomain::Visual,
        |draft| {
            segment_mut(draft, TrackKind::Video, 210)
                .visual
                .transform
                .opacity = SegmentOpacity { value_millis: 650 };
        },
    );
    assert_localized_change_is_bounded(
        &fixture.draft,
        full_range.clone(),
        "volume",
        DirtyDomain::Audio,
        |draft| {
            segment_mut(draft, TrackKind::Audio, 210).volume = SegmentVolume { level_millis: 400 };
        },
    );
    assert_localized_change_is_bounded(
        &fixture.draft,
        full_range,
        "text",
        DirtyDomain::Text,
        |draft| {
            let text = segment_mut(draft, TrackKind::Text, 210)
                .text
                .as_mut()
                .expect("text segment should have text payload");
            *text = TextSegment {
                content: "大型时间线局部改字".to_owned(),
                ..text.clone()
            };
        },
    );
}

#[test]
fn large_timeline_incremental_canvas_profile_change_uses_full_draft_invalidation_fallback() {
    let fixture = build_large_timeline(LargeTimelineConfig::new(240))
        .expect("large timeline fixture should build");
    let full_range = full_draft_range(&fixture.draft);
    let previous = snapshot_for(&fixture.draft, full_range.clone());
    let mut edited = fixture.draft.clone();
    edited.canvas_config = DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::custom(9, 16),
        width: 1080,
        height: 1920,
        frame_rate: RationalFrameRate::new(25, 1),
        background: CanvasBackground::SolidColor {
            color: "#101820".to_owned(),
        },
    };
    let current = snapshot_for(&edited, full_range);
    let diff = RenderGraphDiff::between(
        &previous,
        &current,
        &[dirty_range(
            0,
            full_draft_range(&fixture.draft).duration.get(),
            DirtyRangeSource::FullDraft,
        )],
        &[
            DirtyDomain::Canvas,
            DirtyDomain::OutputProfile,
            DirtyDomain::GraphSnapshot,
        ],
    );

    assert!(
        diff.changed.len() > diff.unchanged.len(),
        "profile/canvas changes are allowed to invalidate broadly: changed={} unchanged={}",
        diff.changed.len(),
        diff.unchanged.len()
    );

    let entries = (0..20)
        .map(|index| cache_entry(&format!("entry-{index:02}"), index * 500_000, 100_000))
        .collect::<Vec<_>>();
    let result = invalidate_preview_cache(
        &entries,
        &PreviewInvalidationRequest::full_draft("canvas/profile changed"),
    );

    assert_eq!(result.invalidated.len(), entries.len());
    assert!(result.retained.is_empty());
}

fn assert_localized_change_is_bounded(
    draft: &Draft,
    full_range: TargetTimerange,
    label: &str,
    domain: DirtyDomain,
    edit: impl FnOnce(&mut Draft),
) {
    let previous = snapshot_for(draft, full_range.clone());
    let mut edited = draft.clone();
    edit(&mut edited);
    validate_draft(&edited).expect("localized edit should keep the draft valid");
    let current = snapshot_for(&edited, full_range);
    let diff = RenderGraphDiff::between(
        &previous,
        &current,
        &[dirty_range(42_000_000, 100_000, DirtyRangeSource::Current)],
        &[domain, DirtyDomain::GraphSnapshot],
    );

    assert!(diff.added.is_empty(), "{label} should not add graph nodes");
    assert!(
        diff.removed.is_empty(),
        "{label} should not remove graph nodes"
    );
    assert!(
        diff.changed.len() <= 16,
        "{label} should have bounded graph changes, changed={}",
        diff.changed.len()
    );
    assert!(
        diff.unchanged.len() > diff.changed.len() * 100,
        "{label} should leave most large-timeline nodes unchanged"
    );
    assert!(
        diff.dirty_domains.contains(&domain),
        "{label} dirty facts should carry {domain:?}"
    );
}

fn snapshot_for(draft: &Draft, target_timerange: TargetTimerange) -> RenderGraphSnapshot {
    let profile = EngineProfile::from_draft_canvas(draft).expect("canvas profile should resolve");
    let normalized = normalize_draft(draft, &profile).expect("draft should normalize");
    let range =
        resolve_render_range(&normalized, target_timerange.clone()).expect("range should resolve");
    let graph = build_render_graph(&normalized, &range).expect("graph should build");
    let output_profile = RenderOutputProfile::preview_frame_png(
        OutputDimensions::new(profile.canvas_width, profile.canvas_height),
        range.frame_rate.clone(),
        target_timerange,
    );
    RenderGraphSnapshot::from_graph(&graph, &output_profile, "runtime:large-timeline")
}

fn full_draft_range(draft: &Draft) -> TargetTimerange {
    let end = draft
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .map(|segment| {
            segment.target_timerange.start.get() + segment.target_timerange.duration.get()
        })
        .max()
        .expect("large timeline should contain segments");
    TargetTimerange::new(0, end)
}

fn segment_mut(
    draft: &mut Draft,
    track_kind: TrackKind,
    segment_index: usize,
) -> &mut draft_model::Segment {
    draft
        .tracks
        .iter_mut()
        .find(|track| track.kind == track_kind)
        .and_then(|track| track.segments.get_mut(segment_index))
        .expect("large timeline segment should exist")
}

fn dirty_range(start: u64, duration: u64, source: DirtyRangeSource) -> DirtyRange {
    DirtyRange {
        target_timerange: TargetTimerange::new(start, duration),
        source,
    }
}

fn changed_keys(diff: &RenderGraphDiff) -> Vec<String> {
    diff.changed
        .iter()
        .map(|change| change.node_id.stable_key())
        .collect()
}

fn segment_node_key(draft: &Draft, track_id: &str, segment_id: &str, role: &str) -> String {
    format!(
        "draft:{}:track:{track_id}:segment:{segment_id}:{role}",
        draft.draft_id.as_str()
    )
}

fn cache_entry(id: &str, start: u64, duration: u64) -> PreviewCacheEntry {
    cache_entry_with_deps(
        id,
        start,
        duration,
        &draft_model::MaterialId::new("stable-material"),
        "stable-node",
    )
}

fn cache_entry_with_deps(
    id: &str,
    start: u64,
    duration: u64,
    material_id: &draft_model::MaterialId,
    graph_node_key: &str,
) -> PreviewCacheEntry {
    PreviewCacheEntry {
        key: PreviewCacheKey {
            key_id: id.to_owned(),
            profile: PreviewCacheProfile::FramePng,
            target_timerange: TargetTimerange::new(start, duration),
            graph_node_keys: vec![graph_node_key.to_owned()],
            semantic_fingerprint: format!("semantic-{id}"),
            input_fingerprint: format!("input-{id}"),
            output_profile_fingerprint: "output-profile-large".to_owned(),
            runtime_capability_fingerprint: "runtime-large".to_owned(),
            material_dependencies: vec![material_id.clone()],
            artifact_schema_version: 2,
            generator_version: "preview-cache-generator-v2".to_owned(),
        },
        artifact: PreviewArtifact {
            profile: PreviewCacheProfile::FramePng,
            path: format!("/cache/{id}.png"),
            mime_type: PreviewCacheProfile::FramePng.mime_type().to_owned(),
        },
    }
}

fn invalidated_ids(result: &preview_service::PreviewInvalidationResult) -> Vec<&str> {
    result
        .invalidated
        .iter()
        .map(|entry| entry.key.key_id.as_str())
        .collect()
}
