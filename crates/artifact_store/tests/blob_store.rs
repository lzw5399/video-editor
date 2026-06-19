use std::fs;

use artifact_store::blob_store::{BlobStore, BlobWriteIntent};
use artifact_store::fingerprint::{ArtifactFingerprint, fingerprint_bytes, fingerprint_file};
use artifact_store::paths::{blob_tmp_path, derived_root_path, validate_derived_relative_path};
use artifact_store::schema::open_artifact_store;
use rusqlite::Connection;
use serde_json::json;

#[test]
fn blob_store_rejects_invalid_derived_relative_paths() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let derived_root = derived_root_path(&bundle_path);
    fs::create_dir_all(&derived_root).expect("derived root should be created");

    for invalid in [
        "",
        "/absolute/blob.bin",
        "C:/absolute/blob.bin",
        "C:\\absolute\\blob.bin",
        "../escape.bin",
        "blobs/../../escape.bin",
        "/",
    ] {
        let error = validate_derived_relative_path(&derived_root, invalid)
            .expect_err("invalid path should be rejected");
        assert!(
            error.to_string().contains("derived relative path"),
            "unexpected error for {invalid}: {error}"
        );
    }

    #[cfg(unix)]
    {
        let outside = sandbox.path().join("outside");
        fs::create_dir_all(&outside).expect("outside directory should be created");
        std::os::unix::fs::symlink(&outside, derived_root.join("escape-link"))
            .expect("symlink should be created");

        let error = validate_derived_relative_path(&derived_root, "escape-link/file.bin")
            .expect_err("symlink escape should be rejected");
        assert!(
            error.to_string().contains("escapes derived root"),
            "unexpected symlink error: {error}"
        );
    }
}

#[test]
fn blob_store_writes_atomic_content_addressed_project_relative_blob() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = BlobStore::open(&bundle_path).expect("blob store should open");
    let bytes = b"thumbnail bytes";

    let record = store
        .write_blob_atomic(write_intent("artifact-thumb-001", None), bytes)
        .expect("blob should write");

    assert_eq!(record.artifact_id, "artifact-thumb-001");
    assert_eq!(record.byte_count, bytes.len() as u64);
    assert_eq!(record.blob_fingerprint, fingerprint_bytes(bytes));
    assert_project_relative_blob_path(&record.blob_relative_path);
    assert!(
        derived_root_path(&bundle_path)
            .join(&record.blob_relative_path)
            .exists()
    );
    assert_eq!(
        fingerprint_file(&derived_root_path(&bundle_path).join(&record.blob_relative_path))
            .expect("written blob should fingerprint"),
        record.blob_fingerprint
    );
    assert!(
        fs::read_dir(blob_tmp_path(&bundle_path))
            .expect("tmp dir should exist")
            .next()
            .is_none(),
        "tmp dir should be empty after successful write"
    );

    let reopened = open_artifact_store(&bundle_path).expect("store should open");
    let (status, dirty, path): (String, i64, String) = reopened
        .connection()
        .query_row(
            "SELECT status, dirty, blob_relative_path FROM artifact WHERE artifact_id = ?1",
            ["artifact-thumb-001"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("artifact row should exist");
    assert_eq!(status, "ready");
    assert_eq!(dirty, 0);
    assert_eq!(path, record.blob_relative_path);
}

#[test]
fn blob_store_rejects_fingerprint_mismatch_before_ready_row() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = BlobStore::open(&bundle_path).expect("blob store should open");
    let expected = ArtifactFingerprint::from_prefixed("blake3:v1:not-the-content")
        .expect("test fingerprint should parse");

    let error = store
        .write_blob_atomic(
            write_intent("artifact-mismatch-001", Some(expected)),
            b"actual bytes",
        )
        .expect_err("fingerprint mismatch should fail");

    assert!(
        error.to_string().contains("fingerprint mismatch"),
        "unexpected error: {error}"
    );
    assert_eq!(
        artifact_count(
            open_artifact_store(&bundle_path)
                .expect("store should open")
                .connection()
        ),
        0
    );
}

#[test]
fn blob_store_repair_demotes_missing_ready_blobs_and_cleans_tmp_only() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = BlobStore::open(&bundle_path).expect("blob store should open");
    let ready = store
        .write_blob_atomic(write_intent("artifact-ready-001", None), b"ready bytes")
        .expect("ready blob should write");
    let missing = store
        .write_blob_atomic(write_intent("artifact-missing-001", None), b"missing bytes")
        .expect("missing blob should write");
    let ready_path = derived_root_path(&bundle_path).join(&ready.blob_relative_path);
    let missing_path = derived_root_path(&bundle_path).join(&missing.blob_relative_path);
    fs::remove_file(&missing_path).expect("missing blob should be removed");
    fs::write(blob_tmp_path(&bundle_path).join("partial.tmp"), b"partial")
        .expect("partial temp should be written");

    let report = store
        .repair_blob_rows()
        .expect("repair should demote missing blobs");

    assert_eq!(report.demoted_artifact_ids, vec!["artifact-missing-001"]);
    assert_eq!(report.removed_temp_files, 1);
    assert!(ready_path.exists(), "live ready blob should remain");
    assert!(!missing_path.exists(), "missing blob remains absent");
    assert!(
        fs::read_dir(blob_tmp_path(&bundle_path))
            .expect("tmp dir should exist")
            .next()
            .is_none(),
        "tmp dir should be cleared"
    );

    let conn = open_artifact_store(&bundle_path).expect("store should open");
    assert_eq!(
        artifact_status(conn.connection(), "artifact-ready-001"),
        ("ready".to_owned(), 0)
    );
    assert_eq!(
        artifact_status(conn.connection(), "artifact-missing-001"),
        ("dirty".to_owned(), 1)
    );
}

fn write_intent(artifact_id: &str, expected: Option<ArtifactFingerprint>) -> BlobWriteIntent {
    BlobWriteIntent {
        artifact_id: artifact_id.to_owned(),
        artifact_kind: "thumbnail".to_owned(),
        stable_key: format!("material:material-001:{artifact_id}"),
        schema_fingerprint: "schema:v1".to_owned(),
        generator_fingerprint: "generator:v1".to_owned(),
        runtime_capability_fingerprint: Some("runtime:v1".to_owned()),
        source_fingerprint: Some("source:v1".to_owned()),
        graph_fingerprint: Some("graph:v1".to_owned()),
        output_profile_fingerprint: Some("output:v1".to_owned()),
        generation_parameters_json: json!({ "kind": "thumbnail" }),
        expected_fingerprint: expected,
    }
}

fn assert_project_relative_blob_path(path: &str) {
    assert!(
        !path.starts_with('/'),
        "path should not be absolute: {path}"
    );
    assert!(
        !path.starts_with("C:"),
        "path should not use drive prefix: {path}"
    );
    assert!(!path.starts_with(".."), "path should not traverse: {path}");
    assert!(
        !path.starts_with(".veproj/derived"),
        "path should be relative to derived root: {path}"
    );
    assert!(
        path.starts_with("blobs/"),
        "path should live under blobs/: {path}"
    );
}

fn artifact_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM artifact", [], |row| row.get(0))
        .expect("artifact count should read")
}

fn artifact_status(conn: &Connection, artifact_id: &str) -> (String, i64) {
    conn.query_row(
        "SELECT status, dirty FROM artifact WHERE artifact_id = ?1",
        [artifact_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .expect("artifact status should read")
}
