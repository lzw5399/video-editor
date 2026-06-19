use std::fs;
use std::path::Path;

use artifact_store::blob_store::{BlobStore, BlobWriteIntent};
use artifact_store::gc::{
    GcMode, TombstoneReason, collect_garbage, plan_garbage_collection, sweep_temporary_blobs,
};
use artifact_store::jobs::{
    ArtifactGenerationRequest, ArtifactKind, GenerationProgress, create_generation_job,
    start_generation_chunk,
};
use artifact_store::paths::{blob_tmp_path, derived_root_path};
use artifact_store::schema::open_artifact_store;
use rusqlite::params;
use serde_json::json;

#[test]
fn gc_quota_manifest_gc_dry_run_preserves_live_artifacts_jobs_and_source_media() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    fs::create_dir_all(&bundle_path).expect("bundle should be created");
    fs::write(bundle_path.join("project.json"), "{}").expect("project json should be written");
    fs::create_dir_all(bundle_path.join("media")).expect("media dir should be created");
    fs::write(bundle_path.join("media/source.mp4"), b"source media")
        .expect("source media should be written");

    let ready = write_artifact(&bundle_path, "artifact-ready", "thumbnail", b"ready bytes");
    let dirty_live = write_artifact(
        &bundle_path,
        "artifact-dirty-live",
        "waveform",
        b"dirty-but-live bytes",
    );
    let job_live = write_artifact(
        &bundle_path,
        "artifact-job-live",
        "proxy",
        b"active job bytes",
    );
    let stale = write_artifact(&bundle_path, "artifact-stale", "proxy", b"stale bytes");

    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    store
        .connection()
        .execute(
            "UPDATE artifact SET dirty = 1, status = 'dirty' WHERE artifact_id = ?1",
            ["artifact-dirty-live"],
        )
        .expect("dirty row should update");
    create_generation_job(
        &mut store,
        ArtifactGenerationRequest {
            job_id: "job-active".to_owned(),
            artifact_id: Some("artifact-job-live".to_owned()),
            kind: ArtifactKind::Proxy,
            stable_key: "material:source:job-live".to_owned(),
            generation_parameters_json: json!({ "kind": "proxy" }),
            source_fingerprint: Some("source:v1".to_owned()),
            runtime_capability_fingerprint: Some("runtime:v1".to_owned()),
            output_profile_fingerprint: Some("output:v1".to_owned()),
            graph_fingerprint: Some("graph:v1".to_owned()),
            chunks: vec![GenerationProgress::new(Some(0), Some(1_000_000), Some(0))],
        },
    )
    .expect("active job should persist");
    start_generation_chunk(&mut store, "job-active", 0).expect("chunk should be active");
    store
        .connection()
        .execute(
            "UPDATE artifact
             SET status = 'dirty', dirty = 1, updated_at_unix_ms = updated_at_unix_ms + 1
             WHERE artifact_id = ?1",
            ["artifact-stale"],
        )
        .expect("stale artifact should be marked dirty");

    let plan = plan_garbage_collection(&store).expect("gc plan should be built");

    assert_candidate_ids(&plan.candidates, &["artifact-stale"]);
    assert!(path_exists(&bundle_path, &ready));
    assert!(path_exists(&bundle_path, &dirty_live));
    assert!(path_exists(&bundle_path, &job_live));
    assert!(bundle_path.join("project.json").exists());
    assert!(bundle_path.join("media/source.mp4").exists());

    let outcome =
        collect_garbage(&mut store, GcMode::DryRun).expect("dry run should not delete files");
    assert_eq!(outcome.deleted_blob_count, 0);
    assert_eq!(outcome.reclaimable_bytes, stale.byte_count);
    assert!(path_exists(&bundle_path, &stale));
    assert_eq!(artifact_status(&store, "artifact-stale"), "dirty");
}

#[test]
fn gc_quota_manifest_gc_apply_tombstones_and_deletes_only_contained_candidates() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let stale = write_artifact(&bundle_path, "artifact-stale-apply", "proxy", b"delete me");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    store
        .connection()
        .execute(
            "UPDATE artifact SET dirty = 1, status = 'dirty' WHERE artifact_id = ?1",
            ["artifact-stale-apply"],
        )
        .expect("stale artifact should be dirty");
    insert_artifact_row(
        &store,
        "artifact-absolute-path",
        "/tmp/should-not-delete.bin",
        "blake3:v1:absolute",
        3,
        "dirty",
    );
    insert_artifact_row(
        &store,
        "artifact-traversal-path",
        "../escape.bin",
        "blake3:v1:traversal",
        4,
        "dirty",
    );
    #[cfg(unix)]
    {
        let outside = sandbox.path().join("outside");
        fs::create_dir_all(&outside).expect("outside dir should be created");
        std::os::unix::fs::symlink(&outside, derived_root_path(&bundle_path).join("escape-link"))
            .expect("escape symlink should be created");
        insert_artifact_row(
            &store,
            "artifact-symlink-path",
            "escape-link/owned.bin",
            "blake3:v1:symlink",
            5,
            "dirty",
        );
    }

    let outcome = collect_garbage(&mut store, GcMode::Apply).expect("apply gc should succeed");

    assert_eq!(outcome.deleted_artifact_ids, vec!["artifact-stale-apply"]);
    assert_eq!(outcome.deleted_blob_count, 1);
    assert_eq!(outcome.released_bytes, stale.byte_count);
    assert!(!path_exists(&bundle_path, &stale));
    assert_eq!(artifact_status(&store, "artifact-stale-apply"), "tombstoned");

    let tombstone = tombstone_for(&store, "artifact-stale-apply");
    assert_eq!(tombstone.0, stale.blob_relative_path);
    assert_eq!(tombstone.1, stale.blob_fingerprint.to_string());
    assert_eq!(tombstone.2, stale.byte_count);
    assert_eq!(tombstone.3, TombstoneReason::GarbageCollected.as_str());
    assert_eq!(
        artifact_status(&store, "artifact-absolute-path"),
        "dirty",
        "absolute path rows must fail closed"
    );
    assert_eq!(
        artifact_status(&store, "artifact-traversal-path"),
        "dirty",
        "traversal rows must fail closed"
    );
    #[cfg(unix)]
    assert_eq!(
        artifact_status(&store, "artifact-symlink-path"),
        "dirty",
        "symlink escape rows must fail closed"
    );
}

#[test]
fn gc_quota_manifest_temp_blob_sweep_removes_abandoned_temp_files_only() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let ready = write_artifact(&bundle_path, "artifact-ready-temp", "thumbnail", b"ready");
    let temp_dir = blob_tmp_path(&bundle_path);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    fs::write(temp_dir.join("abandoned.tmp"), b"partial").expect("temp file should be written");

    let removed = sweep_temporary_blobs(&bundle_path).expect("temp sweep should succeed");

    assert_eq!(removed.removed_temp_files, 1);
    assert!(!temp_dir.join("abandoned.tmp").exists());
    assert!(path_exists(&bundle_path, &ready));
}

fn write_artifact(
    bundle_path: &Path,
    artifact_id: &str,
    artifact_kind: &str,
    bytes: &[u8],
) -> artifact_store::blob_store::BlobRecord {
    let mut blobs = BlobStore::open(bundle_path).expect("blob store should open");
    blobs
        .write_blob_atomic(
            BlobWriteIntent {
                artifact_id: artifact_id.to_owned(),
                artifact_kind: artifact_kind.to_owned(),
                stable_key: format!("material:source:{artifact_id}"),
                schema_fingerprint: "schema:v1".to_owned(),
                generator_fingerprint: "generator:v1".to_owned(),
                runtime_capability_fingerprint: Some("runtime:v1".to_owned()),
                source_fingerprint: Some("source:v1".to_owned()),
                graph_fingerprint: Some("graph:v1".to_owned()),
                output_profile_fingerprint: Some("output:v1".to_owned()),
                generation_parameters_json: json!({ "artifactId": artifact_id }),
                expected_fingerprint: None,
            },
            bytes,
        )
        .expect("blob should write")
}

fn path_exists(bundle_path: &Path, record: &artifact_store::blob_store::BlobRecord) -> bool {
    derived_root_path(bundle_path)
        .join(&record.blob_relative_path)
        .exists()
}

fn assert_candidate_ids(candidates: &[artifact_store::gc::GcCandidate], expected: &[&str]) {
    let actual = candidates
        .iter()
        .map(|candidate| candidate.artifact_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

fn insert_artifact_row(
    store: &artifact_store::schema::ArtifactStore,
    artifact_id: &str,
    blob_relative_path: &str,
    blob_fingerprint: &str,
    byte_count: u64,
    status: &str,
) {
    store
        .connection()
        .execute(
            "INSERT INTO artifact (
                artifact_id, artifact_kind, stable_key, blob_relative_path, blob_fingerprint,
                schema_fingerprint, generator_fingerprint, generation_parameters_json, status,
                dirty, byte_count, created_at_unix_ms, updated_at_unix_ms
            ) VALUES (?1, 'proxy', ?2, ?3, ?4, 'schema:v1', 'generator:v1', '{}', ?5, 1, ?6, 0, 0)",
            params![
                artifact_id,
                format!("material:source:{artifact_id}"),
                blob_relative_path,
                blob_fingerprint,
                status,
                byte_count as i64,
            ],
        )
        .expect("artifact row should insert");
}

fn artifact_status(store: &artifact_store::schema::ArtifactStore, artifact_id: &str) -> String {
    store
        .connection()
        .query_row(
            "SELECT status FROM artifact WHERE artifact_id = ?1",
            [artifact_id],
            |row| row.get(0),
        )
        .expect("artifact status should read")
}

fn tombstone_for(
    store: &artifact_store::schema::ArtifactStore,
    artifact_id: &str,
) -> (String, String, u64, String) {
    let (path, fingerprint, bytes, reason): (String, String, i64, String) = store
        .connection()
        .query_row(
            "SELECT blob_relative_path, blob_fingerprint, byte_count, reason
             FROM artifact_tombstone WHERE artifact_id = ?1",
            [artifact_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("tombstone should exist");
    (path, fingerprint, bytes as u64, reason)
}
