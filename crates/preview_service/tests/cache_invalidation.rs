use draft_model::{MaterialId, Microseconds, TargetTimerange};
use preview_service::{
    PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile,
    accepted_audio_edit_invalidation, accepted_text_edit_invalidation,
    accepted_timeline_edit_invalidation, changed_material_invalidation, changed_range_invalidation,
    invalidate_preview_cache,
};

#[test]
fn cache_entry_snapshot_includes_range_profile_fingerprint_materials_and_artifact() {
    let entry = entry(
        "frame",
        0,
        100_000,
        &["video-material"],
        PreviewCacheProfile::FramePng,
    );

    assert_eq!(
        serde_json::to_value(&entry).expect("entry should serialize"),
        serde_json::json!({
            "key": {
                "keyId": "frame",
                "profile": "framePng",
                "targetTimerange": { "start": 0, "duration": 100000 },
                "semanticFingerprint": "fingerprint-frame",
                "materialDependencies": ["video-material"]
            },
            "artifact": {
                "profile": "framePng",
                "path": "/cache/frame.png",
                "mimeType": "image/png"
            }
        })
    );
}

#[test]
fn invalidation_removes_overlapping_ranges_and_keeps_unrelated_entries() {
    let entries = vec![
        entry(
            "before",
            0,
            100_000,
            &["video-material"],
            PreviewCacheProfile::FramePng,
        ),
        entry(
            "overlap",
            120_000,
            100_000,
            &["video-material"],
            PreviewCacheProfile::FramePng,
        ),
        entry(
            "after",
            400_000,
            100_000,
            &["video-material"],
            PreviewCacheProfile::FramePng,
        ),
    ];
    let request = changed_range_invalidation(
        TargetTimerange::new(Microseconds::new(150_000), Microseconds::new(100_000)),
        "timeline edit",
    );

    let result = invalidate_preview_cache(&entries, &request);

    assert_eq!(
        result
            .invalidated
            .iter()
            .map(|entry| entry.key.key_id.as_str())
            .collect::<Vec<_>>(),
        vec!["overlap"]
    );
    assert_eq!(
        result
            .retained
            .iter()
            .map(|entry| entry.key.key_id.as_str())
            .collect::<Vec<_>>(),
        vec!["before", "after"]
    );
}

#[test]
fn invalidation_removes_entries_with_changed_material_dependency() {
    let entries = vec![
        entry(
            "video",
            0,
            100_000,
            &["video-material"],
            PreviewCacheProfile::FramePng,
        ),
        entry(
            "audio",
            0,
            100_000,
            &["audio-material"],
            PreviewCacheProfile::SegmentMp4,
        ),
    ];
    let request =
        changed_material_invalidation(MaterialId::new("audio-material"), "material path changed");

    let result = invalidate_preview_cache(&entries, &request);

    assert_eq!(result.invalidated[0].key.key_id, "audio");
    assert_eq!(result.retained[0].key.key_id, "video");
}

#[test]
fn accepted_timeline_text_and_audio_edit_ranges_create_invalidation_requests() {
    let timeline =
        accepted_timeline_edit_invalidation([range(100_000, 200_000), range(600_000, 100_000)]);
    let text = accepted_text_edit_invalidation([range(300_000, 200_000)]);
    let audio = accepted_audio_edit_invalidation([range(500_000, 300_000)]);

    assert_eq!(timeline.reason, "timeline edit");
    assert_eq!(
        timeline.changed_ranges,
        vec![range(100_000, 200_000), range(600_000, 100_000)]
    );
    assert!(timeline.changed_material_ids.is_empty());

    assert_eq!(text.reason, "text edit");
    assert_eq!(text.changed_ranges, vec![range(300_000, 200_000)]);

    assert_eq!(audio.reason, "audio edit");
    assert_eq!(audio.changed_ranges, vec![range(500_000, 300_000)]);
}

#[test]
fn accepted_text_and_audio_edits_invalidate_only_overlapping_preview_ranges() {
    let entries = vec![
        entry("before", 0, 100_000, &[], PreviewCacheProfile::FramePng),
        entry(
            "text-hit",
            300_000,
            100_000,
            &[],
            PreviewCacheProfile::FramePng,
        ),
        entry(
            "audio-hit",
            650_000,
            100_000,
            &[],
            PreviewCacheProfile::SegmentMp4,
        ),
        entry(
            "after",
            900_000,
            100_000,
            &[],
            PreviewCacheProfile::SegmentMp4,
        ),
    ];

    let text_result = invalidate_preview_cache(
        &entries,
        &accepted_text_edit_invalidation([range(250_000, 200_000)]),
    );
    let audio_result = invalidate_preview_cache(
        &entries,
        &accepted_audio_edit_invalidation([range(600_000, 200_000)]),
    );

    assert_eq!(
        text_result
            .invalidated
            .iter()
            .map(|entry| entry.key.key_id.as_str())
            .collect::<Vec<_>>(),
        vec!["text-hit"]
    );
    assert_eq!(
        audio_result
            .invalidated
            .iter()
            .map(|entry| entry.key.key_id.as_str())
            .collect::<Vec<_>>(),
        vec!["audio-hit"]
    );
    assert_eq!(text_result.retained.len(), 3);
    assert_eq!(audio_result.retained.len(), 3);
}

#[test]
fn cache_metadata_is_absent_from_draft_schema_and_fixtures() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let draft_schema = std::fs::read_to_string(repo_root.join("schemas/draft.schema.json"))
        .expect("draft schema should exist");

    for forbidden in [
        "previewCache",
        "previewCaches",
        "previewArtifacts",
        "ffmpegScripts",
    ] {
        assert!(!draft_schema.contains(forbidden));
    }
}

fn entry(
    id: &str,
    start: u64,
    duration: u64,
    material_ids: &[&str],
    profile: PreviewCacheProfile,
) -> PreviewCacheEntry {
    PreviewCacheEntry {
        key: PreviewCacheKey {
            key_id: id.to_owned(),
            profile,
            target_timerange: TargetTimerange::new(
                Microseconds::new(start),
                Microseconds::new(duration),
            ),
            semantic_fingerprint: format!("fingerprint-{id}"),
            material_dependencies: material_ids
                .iter()
                .map(|value| MaterialId::new(*value))
                .collect(),
        },
        artifact: PreviewArtifact {
            profile,
            path: format!("/cache/{id}.{}", profile.extension()),
            mime_type: profile.mime_type().to_owned(),
        },
    }
}

fn range(start: u64, duration: u64) -> TargetTimerange {
    TargetTimerange::new(Microseconds::new(start), Microseconds::new(duration))
}
