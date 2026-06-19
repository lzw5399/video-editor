use artifact_store::dependencies::{
    ArtifactDependency, ArtifactDependencyKind, DependencyFingerprint, DependencyRange,
    DependencyUpsert, artifact_ids_for_dependency, dependencies_for_artifact,
    normalize_dependency_ranges, upsert_artifact_dependencies,
};
use artifact_store::resource_index::{
    ResourceKind, ResourceRef, ResourceStatus, index_draft_resources, list_resources_for_material,
    resource_ref_for_effect, resource_ref_for_font, resource_ref_for_material, upsert_resource,
};
use artifact_store::schema::open_artifact_store;
use draft_model::DirtyDomain;
use draft_model::{
    Draft, Filter, Material, MaterialKind, Segment, SourceTimerange, TargetTimerange,
    TextAlignment, TextFont, TextSegment, TextSegmentSource, TextStyle, TextWrapping, Track,
    TrackKind, Transition,
};
use rusqlite::params;
use serde_json::json;

#[test]
fn resource_index_indexes_material_font_effect_filter_transition_rows() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let draft = draft_with_resource_refs();

    let index = index_draft_resources(&bundle_path, &draft).expect("resources should index");

    let material = index
        .resource("material:video-001")
        .expect("material resource should exist");
    assert_eq!(material.resource_id.as_str(), "material:video-001");
    assert_eq!(material.kind, ResourceKind::Material);
    assert_eq!(material.stable_key, "material:video-001");
    assert_eq!(material.source_ref.as_deref(), Some("media/source.mp4"));
    assert_eq!(
        material.project_relative_ref.as_deref(),
        Some("media/source.mp4")
    );
    assert_eq!(material.status, ResourceStatus::Ready);
    assert!(
        !material
            .project_relative_ref
            .as_deref()
            .unwrap_or_default()
            .contains("derived"),
        "material refs must not point at derived blob paths"
    );

    assert_eq!(
        index
            .resource("font:fonts/PingFangSC.ttf")
            .expect("font resource should exist")
            .kind,
        ResourceKind::Font
    );
    assert_eq!(
        index
            .resource("filter:lut-cinematic")
            .expect("filter resource should exist")
            .kind,
        ResourceKind::Filter
    );
    assert_eq!(
        index
            .resource("transition:crossfade")
            .expect("transition resource should exist")
            .kind,
        ResourceKind::Transition
    );
    assert_eq!(
        index
            .resource("effect:text-shadow")
            .expect("text effect resource should exist")
            .kind,
        ResourceKind::Effect
    );

    let material_ref = resource_ref_for_material("video-001");
    assert_eq!(material_ref.kind, ResourceKind::Material);
    assert_eq!(material_ref.resource_id.as_str(), "material:video-001");
    assert_eq!(material_ref.stable_key, "material:video-001");
    assert_eq!(
        resource_ref_for_font("fonts/PingFangSC.ttf"),
        ResourceRef::new(
            ResourceKind::Font,
            "font:fonts/PingFangSC.ttf",
            "fonts/PingFangSC.ttf"
        )
    );
    assert_eq!(
        resource_ref_for_effect(ResourceKind::Filter, "lut-cinematic"),
        ResourceRef::new(
            ResourceKind::Filter,
            "filter:lut-cinematic",
            "lut-cinematic"
        )
    );
}

#[test]
fn resource_index_links_proxy_thumbnail_and_waveform_refs_to_material_resources() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let draft = draft_with_resource_refs();
    let mut index = index_draft_resources(&bundle_path, &draft).expect("resources should index");

    upsert_resource(
        &mut index,
        resource_ref_for_material("video-001").derived_role(ResourceKind::Proxy, "proxy"),
        Some("derived/blobs/proxy/video-001.mp4"),
    )
    .expect("proxy resource should upsert");
    upsert_resource(
        &mut index,
        resource_ref_for_material("video-001").derived_role(ResourceKind::Thumbnail, "thumbnail"),
        Some("derived/blobs/thumb/video-001.png"),
    )
    .expect("thumbnail resource should upsert");
    upsert_resource(
        &mut index,
        resource_ref_for_material("video-001").derived_role(ResourceKind::Waveform, "waveform"),
        Some("derived/blobs/waveform/video-001.json"),
    )
    .expect("waveform resource should upsert");

    let resources = list_resources_for_material(&index, "video-001");
    let kinds = resources
        .iter()
        .map(|resource| resource.kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&ResourceKind::Material));
    assert!(kinds.contains(&ResourceKind::Proxy));
    assert!(kinds.contains(&ResourceKind::Thumbnail));
    assert!(kinds.contains(&ResourceKind::Waveform));
    for resource in resources {
        assert!(
            resource
                .project_relative_ref
                .as_deref()
                .map(|path| !path.starts_with('/') && !path.starts_with(".."))
                .unwrap_or(true),
            "derived artifact refs stay project-relative: {resource:?}"
        );
    }
}

#[test]
fn dependency_rows_store_typed_invalidation_facts() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    insert_artifact(store.connection(), "artifact-preview-001");

    let dependencies = vec![
        DependencyUpsert::new(ArtifactDependency::material("video-001")),
        DependencyUpsert::new(ArtifactDependency::resource("material:video-001")),
        DependencyUpsert::new(ArtifactDependency::graph_node(
            "draft:draft-001:track:v1:segment:s1:video",
        )),
        DependencyUpsert::new(ArtifactDependency::dirty_domain(DirtyDomain::PreviewCache)),
        DependencyUpsert::new(ArtifactDependency::target_range(0, 1_000_000)),
        DependencyUpsert::new(ArtifactDependency::source_range(250_000, 500_000)),
        DependencyUpsert::new(ArtifactDependency::source_fingerprint(
            DependencyFingerprint::new("source-material", "blake3:v1:source"),
        )),
        DependencyUpsert::new(ArtifactDependency::graph_fingerprint(
            DependencyFingerprint::new("graph", "graph:v1"),
        )),
        DependencyUpsert::new(ArtifactDependency::runtime_capability_fingerprint(
            DependencyFingerprint::new("runtime", "runtime:v1"),
        )),
        DependencyUpsert::new(ArtifactDependency::output_profile_fingerprint(
            DependencyFingerprint::new("output", "output:v1"),
        )),
        DependencyUpsert::new(ArtifactDependency::generation_parameters(json!({
            "profile": "framePng",
            "width": 960,
            "height": 540
        }))),
        DependencyUpsert::new(ArtifactDependency::schema_version(2)),
        DependencyUpsert::new(ArtifactDependency::generator_version(
            "preview-cache-generator-v2",
        )),
    ];

    upsert_artifact_dependencies(&mut store, "artifact-preview-001", dependencies)
        .expect("dependencies should upsert");

    let rows = dependencies_for_artifact(&store, "artifact-preview-001")
        .expect("dependencies should query");
    assert_eq!(rows.len(), 13);
    assert!(rows.iter().any(|row| row.kind == ArtifactDependencyKind::Material && row.dependency_key == "video-001"));
    assert!(rows.iter().any(|row| row.kind == ArtifactDependencyKind::Resource && row.dependency_key == "material:video-001"));
    assert!(rows.iter().any(|row| row.kind == ArtifactDependencyKind::GraphNode && row.dependency_key.contains(":segment:s1:")));
    assert!(rows.iter().any(|row| row.kind == ArtifactDependencyKind::DirtyDomain && row.dirty_domain == Some(DirtyDomain::PreviewCache)));
    assert!(rows.iter().any(|row| row.kind == ArtifactDependencyKind::TargetRange && row.target_range == Some(DependencyRange::new(0, 1_000_000))));
    assert!(rows.iter().any(|row| row.kind == ArtifactDependencyKind::SourceRange && row.source_range == Some(DependencyRange::new(250_000, 500_000))));
    assert!(rows.iter().any(|row| row.kind == ArtifactDependencyKind::GenerationParameters && row.dependency_key.starts_with("generationParameters:")));

    let artifacts = artifact_ids_for_dependency(
        &store,
        ArtifactDependency::graph_node("draft:draft-001:track:v1:segment:s1:video"),
    )
    .expect("dependency lookup should query");
    assert_eq!(artifacts, vec!["artifact-preview-001"]);
}

#[test]
fn dependency_range_normalization_uses_checked_microseconds() {
    let normalized = normalize_dependency_ranges(vec![
        DependencyRange::new(0, 100),
        DependencyRange::new(100, 50),
        DependencyRange::new(200, 25),
    ])
    .expect("adjacent ranges should normalize");
    assert_eq!(
        normalized,
        vec![DependencyRange::new(0, 150), DependencyRange::new(200, 25)]
    );

    let overflow = normalize_dependency_ranges(vec![DependencyRange::new(u64::MAX, 1)])
        .expect_err("overflow should be rejected");
    assert!(
        overflow.to_string().contains("overflow"),
        "unexpected overflow error: {overflow}"
    );
}

#[test]
fn dependency_rows_cascade_with_artifact_and_survive_resource_delete_for_invalidation() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    insert_artifact(store.connection(), "artifact-preview-001");
    insert_resource(store.connection(), "material:video-001");

    upsert_artifact_dependencies(
        &mut store,
        "artifact-preview-001",
        vec![DependencyUpsert::new(ArtifactDependency::resource(
            "material:video-001",
        ))],
    )
    .expect("dependency should insert");
    store
        .connection()
        .execute("DELETE FROM resource WHERE resource_id = ?1", ["material:video-001"])
        .expect("resource row delete should not erase dependency facts");
    assert_eq!(
        artifact_ids_for_dependency(
            &store,
            ArtifactDependency::resource("material:video-001")
        )
        .expect("resource dependency lookup should still work"),
        vec!["artifact-preview-001"]
    );

    store
        .connection()
        .execute("DELETE FROM artifact WHERE artifact_id = ?1", ["artifact-preview-001"])
        .expect("artifact delete should cascade dependencies");
    assert!(
        dependencies_for_artifact(&store, "artifact-preview-001")
            .expect("dependency query should still run")
            .is_empty()
    );
}

fn draft_with_resource_refs() -> Draft {
    let mut draft = Draft::new("draft-resource-index", "Resource index draft");
    draft.materials.push(Material::new(
        "video-001",
        MaterialKind::Video,
        "media/source.mp4",
        "source",
    ));
    draft.materials.push(Material::new(
        "text-001",
        MaterialKind::Text,
        "text://title",
        "title",
    ));

    let mut segment = Segment::new(
        "segment-001",
        "video-001",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    segment.filters.push(Filter {
        name: "lut-cinematic".to_owned(),
        parameters: Default::default(),
    });
    segment.transition = Some(Transition {
        name: "crossfade".to_owned(),
        duration: 200_000.into(),
    });

    let mut text_segment = Segment::new(
        "segment-text-001",
        "text-001",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    text_segment.text = Some(TextSegment {
        content: "字幕".to_owned(),
        source: TextSegmentSource::Text,
        style: TextStyle {
            font: TextFont {
                family: "PingFang SC".to_owned(),
                font_ref: Some("fonts/PingFangSC.ttf".to_owned()),
            },
            font_size: 48,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            line_height_millis: 1_200,
            letter_spacing_millis: 0,
            stroke: None,
            shadow: None,
            background: None,
        },
        text_box: Default::default(),
        layout_region: Default::default(),
        wrapping: TextWrapping::Auto,
        bubble: None,
        effect: Some(draft_model::TextEffectRef::Unsupported {
            name: "text-shadow".to_owned(),
            external_ref: Some("effect://text-shadow".to_owned()),
        }),
    });

    let mut video_track = Track::new("track-video", TrackKind::Video, "视频");
    video_track.segments.push(segment);
    let mut text_track = Track::new("track-text", TrackKind::Text, "文字");
    text_track.segments.push(text_segment);
    draft.tracks.push(video_track);
    draft.tracks.push(text_track);
    draft
}

fn insert_artifact(conn: &rusqlite::Connection, artifact_id: &str) {
    conn.execute(
        "INSERT INTO artifact (
            artifact_id, artifact_kind, stable_key, schema_fingerprint, generator_fingerprint,
            runtime_capability_fingerprint, source_fingerprint, graph_fingerprint,
            output_profile_fingerprint, generation_parameters_json, status, dirty, byte_count,
            created_at_unix_ms, updated_at_unix_ms
        ) VALUES (?1, 'previewArtifact', ?2, 'schema:v2', 'generator:v2', 'runtime:v1',
            'source:v1', 'graph:v1', 'output:v1', '{}', 'ready', 0, 1, 0, 0)",
        params![artifact_id, format!("preview:{artifact_id}")],
    )
    .expect("artifact row should insert");
}

fn insert_resource(conn: &rusqlite::Connection, resource_id: &str) {
    conn.execute(
        "INSERT INTO resource (
            resource_id, resource_kind, stable_key, source_uri, project_relative_ref,
            source_fingerprint, source_byte_count, status, created_at_unix_ms, updated_at_unix_ms
        ) VALUES (?1, 'material', ?1, 'media/source.mp4', 'media/source.mp4', NULL, NULL, 'ready', 0, 0)",
        [resource_id],
    )
    .expect("resource row should insert");
}
