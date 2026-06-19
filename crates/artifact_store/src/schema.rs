use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::ArtifactStoreError;
use crate::paths::{artifact_store_db_path, derived_root_path};

pub const ARTIFACT_STORE_DB_FILE_NAME: &str = "artifact-store.sqlite";
pub const ARTIFACT_STORE_SCHEMA_VERSION: u32 = 1;
pub const ARTIFACT_STORE_BUSY_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactStoreConfig {
    pub bundle_path: PathBuf,
    pub derived_path: PathBuf,
    pub db_path: PathBuf,
}

impl ArtifactStoreConfig {
    pub fn for_bundle(bundle_path: impl AsRef<Path>) -> Self {
        let bundle_path = bundle_path.as_ref().to_path_buf();
        let derived_path = derived_root_path(&bundle_path);
        let db_path = artifact_store_db_path(&bundle_path);
        Self {
            bundle_path,
            derived_path,
            db_path,
        }
    }
}

#[derive(Debug)]
pub struct ArtifactStore {
    pub config: ArtifactStoreConfig,
    pub db_path: PathBuf,
    conn: Connection,
}

impl ArtifactStore {
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub(crate) fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

pub fn open_artifact_store(
    bundle_path: impl AsRef<Path>,
) -> Result<ArtifactStore, ArtifactStoreError> {
    let config = ArtifactStoreConfig::for_bundle(bundle_path);
    std::fs::create_dir_all(&config.derived_path).map_err(|source| ArtifactStoreError::Io {
        path: config.derived_path.clone(),
        source,
    })?;

    let conn = Connection::open(&config.db_path).map_err(|source| ArtifactStoreError::Sqlite {
        path: config.db_path.clone(),
        source,
    })?;
    apply_connection_pragmas_at(&conn, &config.db_path)?;
    run_migrations_at(&conn, &config.db_path)?;

    Ok(ArtifactStore {
        db_path: config.db_path.clone(),
        config,
        conn,
    })
}

pub fn apply_connection_pragmas(conn: &Connection) -> Result<(), ArtifactStoreError> {
    apply_connection_pragmas_at(conn, Path::new(ARTIFACT_STORE_DB_FILE_NAME))
}

pub fn run_migrations(conn: &Connection) -> Result<(), ArtifactStoreError> {
    run_migrations_at(conn, Path::new(ARTIFACT_STORE_DB_FILE_NAME))
}

fn apply_connection_pragmas_at(
    conn: &Connection,
    db_path: &Path,
) -> Result<(), ArtifactStoreError> {
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|source| sqlite_error(db_path, source))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|source| sqlite_error(db_path, source))?;
    conn.busy_timeout(std::time::Duration::from_millis(
        ARTIFACT_STORE_BUSY_TIMEOUT_MS,
    ))
    .map_err(|source| sqlite_error(db_path, source))?;
    Ok(())
}

fn run_migrations_at(conn: &Connection, db_path: &Path) -> Result<(), ArtifactStoreError> {
    conn.execute_batch(INITIAL_SCHEMA_SQL)
        .map_err(|source| sqlite_error(db_path, source))?;
    conn.pragma_update(None, "user_version", ARTIFACT_STORE_SCHEMA_VERSION)
        .map_err(|source| sqlite_error(db_path, source))?;
    conn.execute(
        "INSERT OR IGNORE INTO store_metadata (
            key, value, updated_at_unix_ms
        ) VALUES (?1, ?2, ?3)",
        (
            "schema_version",
            ARTIFACT_STORE_SCHEMA_VERSION.to_string(),
            0_i64,
        ),
    )
    .map_err(|source| sqlite_error(db_path, source))?;
    Ok(())
}

fn sqlite_error(path: &Path, source: rusqlite::Error) -> ArtifactStoreError {
    ArtifactStoreError::Sqlite {
        path: path.to_path_buf(),
        source,
    }
}

const INITIAL_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS store_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS resource (
    resource_id TEXT PRIMARY KEY,
    resource_kind TEXT NOT NULL,
    stable_key TEXT NOT NULL UNIQUE,
    source_uri TEXT,
    project_relative_ref TEXT,
    source_fingerprint TEXT,
    source_byte_count INTEGER,
    status TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS artifact (
    artifact_id TEXT PRIMARY KEY,
    artifact_kind TEXT NOT NULL,
    stable_key TEXT NOT NULL UNIQUE,
    blob_relative_path TEXT,
    blob_fingerprint TEXT,
    schema_fingerprint TEXT NOT NULL,
    generator_fingerprint TEXT NOT NULL,
    runtime_capability_fingerprint TEXT,
    source_fingerprint TEXT,
    graph_fingerprint TEXT,
    output_profile_fingerprint TEXT,
    generation_parameters_json TEXT NOT NULL,
    status TEXT NOT NULL,
    dirty INTEGER NOT NULL DEFAULT 0 CHECK (dirty IN (0, 1)),
    byte_count INTEGER NOT NULL DEFAULT 0 CHECK (byte_count >= 0),
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_artifact_kind_status
    ON artifact (artifact_kind, status, dirty);

CREATE TABLE IF NOT EXISTS artifact_dependency (
    artifact_id TEXT NOT NULL REFERENCES artifact(artifact_id) ON DELETE CASCADE,
    dependency_kind TEXT NOT NULL,
    dependency_key TEXT NOT NULL,
    target_start_us INTEGER,
    target_duration_us INTEGER,
    source_start_us INTEGER,
    source_duration_us INTEGER,
    dirty_domain TEXT,
    dependency_fingerprint TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY (
        artifact_id,
        dependency_kind,
        dependency_key,
        target_start_us,
        target_duration_us,
        source_start_us,
        source_duration_us,
        dirty_domain
    )
);

CREATE INDEX IF NOT EXISTS idx_artifact_dependency_lookup
    ON artifact_dependency (dependency_kind, dependency_key, dirty_domain);

CREATE TABLE IF NOT EXISTS generation_job (
    job_id TEXT PRIMARY KEY,
    artifact_id TEXT REFERENCES artifact(artifact_id) ON DELETE SET NULL,
    job_kind TEXT NOT NULL,
    generation_parameters_json TEXT NOT NULL,
    status TEXT NOT NULL,
    progress_per_mille INTEGER CHECK (progress_per_mille IS NULL OR (progress_per_mille >= 0 AND progress_per_mille <= 1000)),
    cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK (cancel_requested IN (0, 1)),
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS generation_chunk (
    job_id TEXT NOT NULL REFERENCES generation_job(job_id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    status TEXT NOT NULL,
    target_start_us INTEGER,
    target_duration_us INTEGER,
    blob_relative_path TEXT,
    blob_fingerprint TEXT,
    byte_count INTEGER NOT NULL DEFAULT 0 CHECK (byte_count >= 0),
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY (job_id, chunk_index)
);

CREATE TABLE IF NOT EXISTS quota_state (
    quota_id TEXT PRIMARY KEY,
    byte_limit INTEGER,
    current_bytes INTEGER NOT NULL DEFAULT 0 CHECK (current_bytes >= 0),
    last_gc_at_unix_ms INTEGER,
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS artifact_tombstone (
    tombstone_id TEXT PRIMARY KEY,
    artifact_id TEXT NOT NULL,
    blob_relative_path TEXT,
    blob_fingerprint TEXT,
    reason TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_manifest_entry (
    manifest_id TEXT NOT NULL,
    entry_key TEXT NOT NULL,
    artifact_id TEXT REFERENCES artifact(artifact_id) ON DELETE SET NULL,
    blob_relative_path TEXT,
    blob_fingerprint TEXT,
    byte_count INTEGER NOT NULL DEFAULT 0 CHECK (byte_count >= 0),
    dependency_fingerprint TEXT,
    status TEXT NOT NULL,
    tombstoned INTEGER NOT NULL DEFAULT 0 CHECK (tombstoned IN (0, 1)),
    updated_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY (manifest_id, entry_key)
);
"#;
