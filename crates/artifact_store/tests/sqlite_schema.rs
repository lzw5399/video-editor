use std::fs;

use artifact_store::schema::{
    ARTIFACT_STORE_DB_FILE_NAME, ARTIFACT_STORE_SCHEMA_VERSION, open_artifact_store, run_migrations,
};
use rusqlite::Connection;
use serde_json::json;

#[test]
fn sqlite_schema_creates_store_with_required_pragmas() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");

    let store = open_artifact_store(&bundle_path).expect("store should open");

    assert_eq!(
        store.db_path,
        bundle_path
            .join("derived")
            .join(ARTIFACT_STORE_DB_FILE_NAME)
    );
    assert!(store.db_path.exists());
    assert_eq!(pragma_i64(store.connection(), "foreign_keys"), 1);
    assert_eq!(
        pragma_string(store.connection(), "journal_mode").to_ascii_lowercase(),
        "wal"
    );
    assert!(pragma_i64(store.connection(), "busy_timeout") >= 5_000);
    assert_eq!(
        pragma_i64(store.connection(), "user_version"),
        i64::from(ARTIFACT_STORE_SCHEMA_VERSION)
    );
}

#[test]
fn sqlite_schema_migration_is_idempotent_and_enforces_foreign_keys() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let store = open_artifact_store(&bundle_path).expect("store should open");

    run_migrations(store.connection()).expect("second migration should be idempotent");

    let tables = required_tables(store.connection());
    for table in [
        "store_metadata",
        "resource",
        "artifact",
        "artifact_dependency",
        "generation_job",
        "generation_chunk",
        "quota_state",
        "artifact_tombstone",
        "sync_manifest_entry",
    ] {
        assert!(tables.contains(&table.to_owned()), "missing table {table}");
    }

    let orphan_dependency = store.connection().execute(
        "INSERT INTO artifact_dependency (
            artifact_id, dependency_kind, dependency_key, created_at_unix_ms
        ) VALUES (?1, ?2, ?3, ?4)",
        ("missing-artifact", "material", "material-001", 1_i64),
    );
    assert!(
        matches!(
            orphan_dependency,
            Err(rusqlite::Error::SqliteFailure(ref error, _))
                if error.code == rusqlite::ErrorCode::ConstraintViolation
        ),
        "orphan dependency should be rejected: {orphan_dependency:?}"
    );

    let orphan_chunk = store.connection().execute(
        "INSERT INTO generation_chunk (
            job_id, chunk_index, status, target_start_us, target_duration_us, created_at_unix_ms, updated_at_unix_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        ("missing-job", 0_i64, "pending", 0_i64, 1_000_i64, 1_i64, 1_i64),
    );
    assert!(
        matches!(
            orphan_chunk,
            Err(rusqlite::Error::SqliteFailure(ref error, _))
                if error.code == rusqlite::ErrorCode::ConstraintViolation
        ),
        "orphan chunk should be rejected: {orphan_chunk:?}"
    );
}

#[test]
fn sqlite_schema_accepts_artifact_metadata_without_project_json_leakage() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    fs::create_dir_all(&bundle_path).expect("bundle directory should be created");
    let project_json_path = bundle_path.join("project.json");
    fs::write(
        &project_json_path,
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "draftId": "draft-001",
            "metadata": { "name": "Clean draft" },
            "materials": [],
            "tracks": []
        }))
        .expect("project JSON should serialize"),
    )
    .expect("project JSON should be written");
    let before = fs::read_to_string(&project_json_path).expect("project JSON should read");

    let store = open_artifact_store(&bundle_path).expect("store should open");
    store
        .connection()
        .execute(
            "INSERT INTO artifact (
                artifact_id, artifact_kind, stable_key, schema_fingerprint, generator_fingerprint,
                runtime_capability_fingerprint, source_fingerprint, graph_fingerprint,
                generation_parameters_json, status, dirty, byte_count, created_at_unix_ms, updated_at_unix_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            (
                "artifact-001",
                "thumbnail",
                "material:material-001:thumbnail",
                "schema:v1",
                "generator:v1",
                "runtime:v1",
                "source:v1",
                "graph:v1",
                "{}",
                "ready",
                0_i64,
                128_i64,
                1_i64,
                1_i64,
            ),
        )
        .expect("artifact metadata row should insert");

    let after = fs::read_to_string(&project_json_path).expect("project JSON should still read");
    assert_eq!(after, before);
    assert!(!after.contains("artifact"));
    assert!(!after.contains("previewCaches"));
}

fn pragma_i64(conn: &Connection, name: &str) -> i64 {
    conn.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
        .expect("pragma should read")
}

fn pragma_string(conn: &Connection, name: &str) -> String {
    conn.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
        .expect("pragma should read")
}

fn required_tables(conn: &Connection) -> Vec<String> {
    let mut statement = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .expect("table query should prepare");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("table query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("table names should collect")
}
