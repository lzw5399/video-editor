use draft_model::{MaterialId, Microseconds, TargetTimerange};
use preview_service::{
    PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile,
    accepted_audio_edit_invalidation, accepted_text_edit_invalidation,
    accepted_timeline_edit_invalidation, invalidate_preview_cache,
};

#[test]
fn dirty_propagation_target_uses_half_open_integer_ranges() {
    let entries = vec![
        entry("before", 0, 100_000, PreviewCacheProfile::FramePng),
        entry("hit", 149_999, 10_000, PreviewCacheProfile::FramePng),
        entry("adjacent", 200_000, 50_000, PreviewCacheProfile::FramePng),
    ];
    let request = accepted_timeline_edit_invalidation([range(100_000, 100_000)]);

    let result = invalidate_preview_cache(&entries, &request);

    assert_eq!(
        result
            .invalidated
            .iter()
            .map(|entry| entry.key.key_id.as_str())
            .collect::<Vec<_>>(),
        vec!["hit"],
        "half-open [100000, 200000) invalidates overlap but not adjacent ranges"
    );
    assert_eq!(
        result
            .retained
            .iter()
            .map(|entry| entry.key.key_id.as_str())
            .collect::<Vec<_>>(),
        vec!["before", "adjacent"]
    );
}

#[test]
fn dirty_propagation_target_preserves_consumer_specific_reasons_and_ranges() {
    let timeline = accepted_timeline_edit_invalidation([range(0, 100_000)]);
    let text = accepted_text_edit_invalidation([range(200_000, 100_000)]);
    let audio = accepted_audio_edit_invalidation([range(400_000, 100_000)]);

    assert_eq!(timeline.reason, "timeline edit");
    assert_eq!(timeline.changed_ranges, vec![range(0, 100_000)]);
    assert_eq!(text.reason, "text edit");
    assert_eq!(text.changed_ranges, vec![range(200_000, 100_000)]);
    assert_eq!(audio.reason, "audio edit");
    assert_eq!(audio.changed_ranges, vec![range(400_000, 100_000)]);
}

#[test]
fn dirty_propagation_target_keeps_unrelated_material_dependencies_retained() {
    let entries = vec![
        entry_with_material("video-hit", 0, 100_000, "video-material"),
        entry_with_material("audio-retained", 0, 100_000, "audio-material"),
    ];
    let request = preview_service::changed_material_invalidation(
        MaterialId::new("video-material"),
        "material relinked",
    );

    let result = invalidate_preview_cache(&entries, &request);

    assert_eq!(result.invalidated[0].key.key_id, "video-hit");
    assert_eq!(result.retained[0].key.key_id, "audio-retained");
}

fn entry(id: &str, start: u64, duration: u64, profile: PreviewCacheProfile) -> PreviewCacheEntry {
    PreviewCacheEntry {
        key: PreviewCacheKey {
            key_id: id.to_owned(),
            profile,
            target_timerange: range(start, duration),
            semantic_fingerprint: format!("fingerprint-{id}"),
            material_dependencies: Vec::new(),
        },
        artifact: PreviewArtifact {
            profile,
            path: format!("/cache/{id}.{}", profile.extension()),
            mime_type: profile.mime_type().to_owned(),
        },
    }
}

fn entry_with_material(
    id: &str,
    start: u64,
    duration: u64,
    material_id: &str,
) -> PreviewCacheEntry {
    let mut entry = entry(id, start, duration, PreviewCacheProfile::FramePng);
    entry
        .key
        .material_dependencies
        .push(MaterialId::new(material_id));
    entry
}

fn range(start: u64, duration: u64) -> TargetTimerange {
    TargetTimerange::new(Microseconds::new(start), Microseconds::new(duration))
}
