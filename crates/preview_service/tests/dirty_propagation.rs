use draft_model::{
    CommandDelta, CommandName, DirtyDomain, DirtyRange, DirtyRangeSource, InvalidationScope,
    MaterialId, Microseconds, TargetTimerange,
};
use preview_service::{
    ExportPrepDirtyFacts, PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile,
    PreviewInvalidationRequest,
    accepted_audio_edit_invalidation, accepted_text_edit_invalidation,
    accepted_timeline_edit_invalidation, consumer_domains_for_dirty_domains,
    invalidate_preview_cache,
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
    assert_eq!(
        timeline.dirty_ranges,
        vec![dirty_range(0, 100_000, DirtyRangeSource::Current)]
    );
    assert_eq!(text.reason, "text edit");
    assert_eq!(
        text.dirty_ranges,
        vec![dirty_range(200_000, 100_000, DirtyRangeSource::Current)]
    );
    assert_eq!(audio.reason, "audio edit");
    assert_eq!(
        audio.dirty_ranges,
        vec![dirty_range(400_000, 100_000, DirtyRangeSource::Current)]
    );
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

#[test]
fn consumer_domain_expansion_covers_phase13_dirty_targets() {
    assert_eq!(
        consumer_domains_for_dirty_domains([
            DirtyDomain::Timing,
            DirtyDomain::Text,
            DirtyDomain::Audio,
            DirtyDomain::Material,
            DirtyDomain::Canvas,
            DirtyDomain::OutputProfile,
            DirtyDomain::RuntimeCapabilities,
        ]),
        vec![
            DirtyDomain::Preview,
            DirtyDomain::ExportPrep,
            DirtyDomain::Audio,
            DirtyDomain::Thumbnail,
            DirtyDomain::Waveform,
            DirtyDomain::Proxy,
            DirtyDomain::GraphSnapshot,
            DirtyDomain::PreviewCache,
        ]
    );

    assert_eq!(
        consumer_domains_for_dirty_domains([DirtyDomain::Visual]),
        vec![
            DirtyDomain::Preview,
            DirtyDomain::ExportPrep,
            DirtyDomain::Thumbnail,
            DirtyDomain::Proxy,
            DirtyDomain::GraphSnapshot,
            DirtyDomain::PreviewCache,
        ]
    );
}

#[test]
fn export_prep_dirty_facts_match_preview_invalidation_facts() {
    let delta = CommandDelta::targeted(
        CommandName::MoveSegment,
        Vec::new(),
        vec![DirtyDomain::Timing, DirtyDomain::Visual],
        vec![
            dirty_range(0, 100_000, DirtyRangeSource::Previous),
            dirty_range(500_000, 100_000, DirtyRangeSource::Current),
        ],
        InvalidationScope {
            full_draft: false,
            material_ids: vec![MaterialId::new("video-material")],
            graph_node_ids: vec!["draft:draft-1:track:track-1:segment:segment-1:video".to_owned()],
            consumer_domains: Vec::new(),
        },
        "segment moved",
    );

    let request = PreviewInvalidationRequest::from_command_delta(&delta)
        .with_runtime_capability_fingerprint("runtime-software")
        .with_output_profile_fingerprint("output-profile-preview");
    let export_facts = ExportPrepDirtyFacts::from_invalidation_request(&request);

    assert_eq!(export_facts.dirty_ranges, request.dirty_ranges);
    assert_eq!(export_facts.changed_material_ids, request.changed_material_ids);
    assert_eq!(export_facts.changed_graph_node_keys, request.changed_graph_node_keys);
    assert_eq!(export_facts.changed_domains, request.changed_domains);
    assert_eq!(
        export_facts.runtime_capability_fingerprint.as_deref(),
        Some("runtime-software")
    );
    assert_eq!(
        export_facts.output_profile_fingerprint.as_deref(),
        Some("output-profile-preview")
    );
    assert!(!export_facts.full_draft);
    assert_eq!(export_facts.reason, "segment moved");
    assert!(
        export_facts
            .changed_domains
            .contains(&DirtyDomain::ExportPrep)
    );
}

#[test]
fn dirty_domain_range_invalidation_requires_preview_cache_consumer_domain() {
    let entries = vec![
        entry("hit", 0, 100_000, PreviewCacheProfile::FramePng),
        entry("retained", 300_000, 100_000, PreviewCacheProfile::FramePng),
    ];
    let audio_only = PreviewInvalidationRequest::new(
        [dirty_range(0, 100_000, DirtyRangeSource::Current)],
        [],
        [],
        [DirtyDomain::Waveform],
        "waveform only",
    );
    let preview_cache = PreviewInvalidationRequest::new(
        [dirty_range(0, 100_000, DirtyRangeSource::Current)],
        [],
        [],
        [DirtyDomain::PreviewCache],
        "preview cache",
    );

    assert!(invalidate_preview_cache(&entries, &audio_only).invalidated.is_empty());
    assert_eq!(
        invalidate_preview_cache(&entries, &preview_cache).invalidated[0]
            .key
            .key_id,
        "hit"
    );
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

fn dirty_range(start: u64, duration: u64, source: DirtyRangeSource) -> DirtyRange {
    DirtyRange {
        target_timerange: range(start, duration),
        source,
    }
}
