use artifact_store::dependencies::{
    ArtifactDependency, DependencyFingerprint, DependencyUpsert, upsert_artifact_dependencies,
};
use artifact_store::invalidation::{
    InvalidationFallbackReason, SourceChange, SourceChangeKind, mark_dirty_for_source_change,
};
use artifact_store::schema::open_artifact_store;
use draft_model::MaterialId;
use rusqlite::params;

#[test]
fn invalidation_source_replacement_dirties_only_matching_material_and_source_fingerprint() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    insert_artifact(store.connection(), "artifact-material");
    insert_artifact(store.connection(), "artifact-source-fingerprint");
    insert_artifact(store.connection(), "artifact-unrelated");

    upsert_artifact_dependencies(
        &mut store,
        "artifact-material",
        vec![DependencyUpsert::new(ArtifactDependency::material(
            "video-001",
        ))],
    )
    .expect("material dependency should insert");
    upsert_artifact_dependencies(
        &mut store,
        "artifact-source-fingerprint",
        vec![DependencyUpsert::new(
            ArtifactDependency::source_fingerprint(DependencyFingerprint::new(
                "source:video-001",
                "blake3:old-source",
            )),
        )],
    )
    .expect("source fingerprint dependency should insert");
    upsert_artifact_dependencies(
        &mut store,
        "artifact-unrelated",
        vec![DependencyUpsert::new(ArtifactDependency::material(
            "video-002",
        ))],
    )
    .expect("unrelated dependency should insert");

    let result = mark_dirty_for_source_change(
        &mut store,
        SourceChange {
            kind: SourceChangeKind::Replaced,
            material_id: Some(MaterialId::new("video-001")),
            resource_id: Some("material:video-001".to_owned()),
            old_project_relative_ref: Some("media/old.mp4".to_owned()),
            new_project_relative_ref: Some("media/new.mp4".to_owned()),
            old_source_fingerprint: Some("blake3:old-source".to_owned()),
            new_source_fingerprint: Some("blake3:new-source".to_owned()),
            reason: "source replaced".to_owned(),
        },
    )
    .expect("source replacement should invalidate");

    assert_eq!(
        dirty_ids(&result),
        vec!["artifact-material", "artifact-source-fingerprint"]
    );
    assert!(result.fallbacks.is_empty());
    assert_artifact_status(store.connection(), "artifact-material", "dirty");
    assert_artifact_status(store.connection(), "artifact-source-fingerprint", "dirty");
    assert_artifact_status(store.connection(), "artifact-unrelated", "ready");
    assert_dirty_reason(
        store.connection(),
        "artifact-material",
        "sourceChange:replaced:source replaced",
    );
}

#[test]
fn invalidation_source_relink_and_rename_update_resource_refs_and_keep_unrelated_ready() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    insert_resource(store.connection(), "material:video-001", "media/old.mp4");
    insert_artifact(store.connection(), "artifact-resource");
    insert_artifact(store.connection(), "artifact-other");

    upsert_artifact_dependencies(
        &mut store,
        "artifact-resource",
        vec![DependencyUpsert::new(ArtifactDependency::resource(
            "material:video-001",
        ))],
    )
    .expect("resource dependency should insert");
    upsert_artifact_dependencies(
        &mut store,
        "artifact-other",
        vec![DependencyUpsert::new(ArtifactDependency::resource(
            "material:video-002",
        ))],
    )
    .expect("other dependency should insert");

    let relink = mark_dirty_for_source_change(
        &mut store,
        SourceChange {
            kind: SourceChangeKind::Relinked,
            material_id: Some(MaterialId::new("video-001")),
            resource_id: Some("material:video-001".to_owned()),
            old_project_relative_ref: Some("media/old.mp4".to_owned()),
            new_project_relative_ref: Some("media/relinked.mp4".to_owned()),
            old_source_fingerprint: None,
            new_source_fingerprint: Some("blake3:relinked".to_owned()),
            reason: "source relinked".to_owned(),
        },
    )
    .expect("source relink should invalidate");
    assert_eq!(dirty_ids(&relink), vec!["artifact-resource"]);
    assert_resource_ref(
        store.connection(),
        "material:video-001",
        "media/relinked.mp4",
        Some("blake3:relinked"),
    );
    assert_artifact_status(store.connection(), "artifact-other", "ready");

    reset_artifact(store.connection(), "artifact-resource");
    let rename = mark_dirty_for_source_change(
        &mut store,
        SourceChange {
            kind: SourceChangeKind::Renamed,
            material_id: None,
            resource_id: Some("material:video-001".to_owned()),
            old_project_relative_ref: Some("media/relinked.mp4".to_owned()),
            new_project_relative_ref: Some("media/renamed.mp4".to_owned()),
            old_source_fingerprint: None,
            new_source_fingerprint: None,
            reason: "source renamed".to_owned(),
        },
    )
    .expect("source rename should invalidate");
    assert_eq!(dirty_ids(&rename), vec!["artifact-resource"]);
    assert_resource_ref(
        store.connection(),
        "material:video-001",
        "media/renamed.mp4",
        Some("blake3:relinked"),
    );
}

#[test]
fn invalidation_source_delete_tombstones_dependent_artifacts_without_deleting_rows() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    insert_artifact(store.connection(), "artifact-deleted");
    insert_artifact(store.connection(), "artifact-kept");

    upsert_artifact_dependencies(
        &mut store,
        "artifact-deleted",
        vec![DependencyUpsert::new(ArtifactDependency::material(
            "video-001",
        ))],
    )
    .expect("material dependency should insert");
    upsert_artifact_dependencies(
        &mut store,
        "artifact-kept",
        vec![DependencyUpsert::new(ArtifactDependency::material(
            "video-002",
        ))],
    )
    .expect("kept dependency should insert");

    let result = mark_dirty_for_source_change(
        &mut store,
        SourceChange {
            kind: SourceChangeKind::Deleted,
            material_id: Some(MaterialId::new("video-001")),
            resource_id: Some("material:video-001".to_owned()),
            old_project_relative_ref: Some("media/deleted.mp4".to_owned()),
            new_project_relative_ref: None,
            old_source_fingerprint: None,
            new_source_fingerprint: None,
            reason: "source deleted".to_owned(),
        },
    )
    .expect("source delete should invalidate");

    assert_eq!(dirty_ids(&result), vec!["artifact-deleted"]);
    assert_artifact_status(store.connection(), "artifact-deleted", "tombstoned");
    assert_artifact_status(store.connection(), "artifact-kept", "ready");
    assert_eq!(artifact_count(store.connection()), 2);
}

#[test]
fn invalidation_source_change_without_stable_dependency_records_unknown_fallback() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    insert_artifact(store.connection(), "artifact-a");
    insert_artifact(store.connection(), "artifact-b");

    let result = mark_dirty_for_source_change(
        &mut store,
        SourceChange {
            kind: SourceChangeKind::Replaced,
            material_id: None,
            resource_id: None,
            old_project_relative_ref: None,
            new_project_relative_ref: Some("media/new.mp4".to_owned()),
            old_source_fingerprint: None,
            new_source_fingerprint: None,
            reason: "missing stable dependency".to_owned(),
        },
    )
    .expect("unknown dependency should fail closed");

    assert_eq!(dirty_ids(&result), vec!["artifact-a", "artifact-b"]);
    assert_eq!(
        result
            .fallbacks
            .iter()
            .map(|fallback| fallback.reason)
            .collect::<Vec<_>>(),
        vec![InvalidationFallbackReason::UnknownDependency]
    );
    assert_artifact_status(store.connection(), "artifact-a", "dirty");
    assert_artifact_status(store.connection(), "artifact-b", "dirty");
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

fn insert_resource(conn: &rusqlite::Connection, resource_id: &str, project_relative_ref: &str) {
    conn.execute(
        "INSERT INTO resource (
            resource_id, resource_kind, stable_key, source_uri, project_relative_ref,
            source_fingerprint, source_byte_count, status, created_at_unix_ms, updated_at_unix_ms
        ) VALUES (?1, 'material', ?1, ?2, ?2, 'blake3:old', NULL, 'ready', 0, 0)",
        params![resource_id, project_relative_ref],
    )
    .expect("resource row should insert");
}

fn reset_artifact(conn: &rusqlite::Connection, artifact_id: &str) {
    conn.execute(
        "UPDATE artifact SET status = 'ready', dirty = 0 WHERE artifact_id = ?1",
        [artifact_id],
    )
    .expect("artifact should reset");
}

fn dirty_ids(result: &artifact_store::invalidation::ArtifactInvalidationResult) -> Vec<&str> {
    result
        .dirty_artifacts
        .iter()
        .map(|artifact| artifact.artifact_id.as_str())
        .collect()
}

fn assert_artifact_status(conn: &rusqlite::Connection, artifact_id: &str, expected: &str) {
    let (status, dirty): (String, i64) = conn
        .query_row(
            "SELECT status, dirty FROM artifact WHERE artifact_id = ?1",
            [artifact_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("artifact status should query");
    assert_eq!(status, expected);
    assert_eq!(dirty, i64::from(expected != "ready"));
}

fn assert_dirty_reason(conn: &rusqlite::Connection, artifact_id: &str, expected: &str) {
    let reason: String = conn
        .query_row(
            "SELECT dirty_reason FROM artifact WHERE artifact_id = ?1",
            [artifact_id],
            |row| row.get(0),
        )
        .expect("dirty reason should query");
    assert_eq!(reason, expected);
}

fn assert_resource_ref(
    conn: &rusqlite::Connection,
    resource_id: &str,
    expected_ref: &str,
    expected_fingerprint: Option<&str>,
) {
    let (source_uri, project_relative_ref, fingerprint): (String, String, Option<String>) = conn
        .query_row(
            "SELECT source_uri, project_relative_ref, source_fingerprint FROM resource WHERE resource_id = ?1",
            [resource_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("resource ref should query");
    assert_eq!(source_uri, expected_ref);
    assert_eq!(project_relative_ref, expected_ref);
    assert_eq!(fingerprint.as_deref(), expected_fingerprint);
}

fn artifact_count(conn: &rusqlite::Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM artifact", [], |row| row.get(0))
        .expect("artifact count should query")
}
