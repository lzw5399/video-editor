use draft_model::{
    CommandDelta, CommandName, DirtyDomain, DirtyRange, DirtyRangeSource, InvalidationScope,
    MaterialId, Microseconds, TargetTimerange,
};
use preview_service::{
    ExportPrepDirtyFacts, PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile,
    PreviewInvalidationRequest, accepted_audio_edit_invalidation, accepted_text_edit_invalidation,
    accepted_timeline_edit_invalidation, changed_material_invalidation, changed_range_invalidation,
    invalidate_preview_cache,
};
use render_graph::{
    GRAPH_GENERATOR_VERSION, GRAPH_SCHEMA_VERSION, RenderGraphNodeFingerprint, RenderGraphNodeId,
    RenderGraphNodeRole,
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
fn cache_key_v2_serializes_graph_fingerprint_runtime_profile_and_generator_facts() {
    let entry = entry_v2(
        "frame-v2",
        0,
        100_000,
        &["video-material"],
        &["draft:draft-1:track:track-1:segment:segment-1:video"],
        PreviewCacheProfile::FramePng,
    );

    assert_eq!(
        serde_json::to_value(&entry).expect("entry should serialize"),
        serde_json::json!({
            "key": {
                "keyId": "frame-v2",
                "profile": "framePng",
                "targetTimerange": { "start": 0, "duration": 100000 },
                "graphNodeKeys": ["draft:draft-1:track:track-1:segment:segment-1:video"],
                "semanticFingerprint": "semantic-frame-v2",
                "inputFingerprint": "input-frame-v2",
                "outputProfileFingerprint": "output-profile-preview",
                "runtimeCapabilityFingerprint": "runtime-software",
                "materialDependencies": ["video-material"],
                "artifactSchemaVersion": 2,
                "generatorVersion": "preview-cache-generator-v2"
            },
            "artifact": {
                "profile": "framePng",
                "path": "/cache/frame-v2.png",
                "mimeType": "image/png"
            }
        })
    );
}

#[test]
fn cache_key_v2_can_be_derived_from_render_graph_node_fingerprints() {
    let node_id = RenderGraphNodeId {
        role: RenderGraphNodeRole::VideoSegment,
        draft_id: draft_model::DraftId::new("draft-1"),
        track_id: Some(draft_model::TrackId::new("track-1")),
        segment_id: Some(draft_model::SegmentId::new("segment-1")),
        material_id: Some(MaterialId::new("video-material")),
        local_id: None,
    };
    let fingerprint = RenderGraphNodeFingerprint {
        node_id,
        semantic_fingerprint: "semantic-video".to_owned(),
        input_fingerprint: "input-video".to_owned(),
        output_profile_fingerprint: "output-profile-preview".to_owned(),
        runtime_capability_fingerprint: "runtime-software".to_owned(),
        graph_schema_version: GRAPH_SCHEMA_VERSION,
        generator_version: GRAPH_GENERATOR_VERSION.to_owned(),
    };

    let key = PreviewCacheKey::from_node_fingerprints(
        PreviewCacheProfile::FramePng,
        range(0, 100_000),
        &[fingerprint],
        [MaterialId::new("video-material")],
    );

    assert_eq!(
        key.graph_node_keys,
        vec!["draft:draft-1:track:track-1:segment:segment-1:video"]
    );
    assert_eq!(key.semantic_fingerprint, "semantic-video");
    assert_eq!(key.input_fingerprint, "input-video");
    assert_eq!(key.output_profile_fingerprint, "output-profile-preview");
    assert_eq!(key.runtime_capability_fingerprint, "runtime-software");
    assert_eq!(key.artifact_schema_version, 2);
    assert_eq!(key.generator_version, "preview-cache-generator-v2");
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
        timeline.dirty_ranges,
        vec![
            dirty_range(100_000, 200_000, DirtyRangeSource::Current),
            dirty_range(600_000, 100_000, DirtyRangeSource::Current)
        ]
    );
    assert!(timeline.changed_material_ids.is_empty());

    assert_eq!(text.reason, "text edit");
    assert_eq!(
        text.dirty_ranges,
        vec![dirty_range(300_000, 200_000, DirtyRangeSource::Current)]
    );

    assert_eq!(audio.reason, "audio edit");
    assert_eq!(
        audio.dirty_ranges,
        vec![dirty_range(500_000, 300_000, DirtyRangeSource::Current)]
    );
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

#[test]
fn invalidation_v2_predicates_cover_range_material_graph_runtime_profile_and_full_draft() {
    let entries = vec![
        entry_v2(
            "range-hit",
            100_000,
            100_000,
            &["video-material"],
            &["node-range"],
            PreviewCacheProfile::FramePng,
        ),
        entry_v2(
            "material-hit",
            500_000,
            100_000,
            &["changed-material"],
            &["node-material"],
            PreviewCacheProfile::FramePng,
        ),
        entry_v2(
            "graph-hit",
            700_000,
            100_000,
            &["other-material"],
            &["changed-node"],
            PreviewCacheProfile::SegmentMp4,
        ),
        entry_v2(
            "retained",
            900_000,
            100_000,
            &["other-material"],
            &["other-node"],
            PreviewCacheProfile::FramePng,
        ),
    ];

    let range_result = invalidate_preview_cache(
        &entries,
        &PreviewInvalidationRequest::new(
            [dirty_range(150_000, 10_000, DirtyRangeSource::Current)],
            [],
            [],
            [DirtyDomain::PreviewCache],
            "range edit",
        ),
    );
    assert_eq!(invalidated_ids(&range_result), vec!["range-hit"]);

    let material_result = invalidate_preview_cache(
        &entries,
        &PreviewInvalidationRequest::new(
            [],
            [MaterialId::new("changed-material")],
            [],
            [DirtyDomain::PreviewCache],
            "material edit",
        ),
    );
    assert_eq!(invalidated_ids(&material_result), vec!["material-hit"]);

    let node_result = invalidate_preview_cache(
        &entries,
        &PreviewInvalidationRequest::new(
            [],
            [],
            ["changed-node".to_owned()],
            [DirtyDomain::PreviewCache],
            "graph node edit",
        ),
    );
    assert_eq!(invalidated_ids(&node_result), vec!["graph-hit"]);

    let runtime_result = invalidate_preview_cache(
        &entries,
        &PreviewInvalidationRequest::new(
            [],
            [],
            [],
            [DirtyDomain::RuntimeCapabilities],
            "runtime changed",
        )
        .with_runtime_capability_fingerprint("runtime-hardware"),
    );
    assert_eq!(
        invalidated_ids(&runtime_result),
        vec!["range-hit", "material-hit", "graph-hit", "retained"]
    );

    let profile_result = invalidate_preview_cache(
        &entries,
        &PreviewInvalidationRequest::new(
            [],
            [],
            [],
            [DirtyDomain::OutputProfile],
            "profile changed",
        )
        .with_output_profile_fingerprint("output-profile-export"),
    );
    assert_eq!(
        invalidated_ids(&profile_result),
        vec!["range-hit", "material-hit", "graph-hit", "retained"]
    );

    let full_result = invalidate_preview_cache(
        &entries,
        &PreviewInvalidationRequest::full_draft("unknown command"),
    );
    assert_eq!(
        invalidated_ids(&full_result),
        vec!["range-hit", "material-hit", "graph-hit", "retained"]
    );
}

#[test]
fn invalidation_v2_invalidates_legacy_entries_when_v2_facts_are_required() {
    let legacy = entry(
        "legacy",
        0,
        100_000,
        &["video-material"],
        PreviewCacheProfile::FramePng,
    );
    let request = PreviewInvalidationRequest::new(
        [],
        [],
        [],
        [DirtyDomain::RuntimeCapabilities],
        "runtime changed",
    )
    .with_runtime_capability_fingerprint("runtime-software");

    let result = invalidate_preview_cache(&[legacy], &request);

    assert_eq!(invalidated_ids(&result), vec!["legacy"]);
}

#[test]
fn invalidation_v2_from_command_delta_expands_export_prep_and_preview_cache_facts() {
    let delta = CommandDelta::targeted(
        CommandName::UpdateSegmentVisual,
        Vec::new(),
        vec![DirtyDomain::Visual],
        vec![dirty_range(100_000, 200_000, DirtyRangeSource::Current)],
        InvalidationScope::targeted(
            vec![MaterialId::new("video-material")],
            vec![DirtyDomain::PreviewCache, DirtyDomain::ExportPrep],
        ),
        "visual edit",
    );

    let request = PreviewInvalidationRequest::from_command_delta(&delta);
    let export_facts = ExportPrepDirtyFacts::from_invalidation_request(&request);

    assert_eq!(
        request.dirty_ranges,
        vec![dirty_range(100_000, 200_000, DirtyRangeSource::Current)]
    );
    assert_eq!(
        request.changed_domains,
        vec![
            DirtyDomain::Preview,
            DirtyDomain::ExportPrep,
            DirtyDomain::Thumbnail,
            DirtyDomain::Proxy,
            DirtyDomain::GraphSnapshot,
            DirtyDomain::PreviewCache,
        ]
    );
    assert_eq!(
        request.changed_material_ids,
        vec![MaterialId::new("video-material")]
    );
    assert_eq!(export_facts.dirty_ranges, request.dirty_ranges);
    assert_eq!(export_facts.changed_domains, request.changed_domains);
    assert_eq!(export_facts.reason, "visual edit");
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
            graph_node_keys: Vec::new(),
            semantic_fingerprint: format!("fingerprint-{id}"),
            input_fingerprint: String::new(),
            output_profile_fingerprint: String::new(),
            runtime_capability_fingerprint: String::new(),
            material_dependencies: material_ids
                .iter()
                .map(|value| MaterialId::new(*value))
                .collect(),
            artifact_schema_version: 0,
            generator_version: String::new(),
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

fn dirty_range(start: u64, duration: u64, source: DirtyRangeSource) -> DirtyRange {
    DirtyRange {
        target_timerange: range(start, duration),
        source,
    }
}

fn entry_v2(
    id: &str,
    start: u64,
    duration: u64,
    material_ids: &[&str],
    graph_node_keys: &[&str],
    profile: PreviewCacheProfile,
) -> PreviewCacheEntry {
    PreviewCacheEntry {
        key: PreviewCacheKey {
            key_id: id.to_owned(),
            profile,
            target_timerange: range(start, duration),
            graph_node_keys: graph_node_keys
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            semantic_fingerprint: format!("semantic-{id}"),
            input_fingerprint: format!("input-{id}"),
            output_profile_fingerprint: "output-profile-preview".to_owned(),
            runtime_capability_fingerprint: "runtime-software".to_owned(),
            material_dependencies: material_ids
                .iter()
                .map(|value| MaterialId::new(*value))
                .collect(),
            artifact_schema_version: 2,
            generator_version: "preview-cache-generator-v2".to_owned(),
        },
        artifact: PreviewArtifact {
            profile,
            path: format!("/cache/{id}.{}", profile.extension()),
            mime_type: profile.mime_type().to_owned(),
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
