use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArtifactStoreError {
    #[error("artifact store IO failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("artifact store SQLite operation failed at {path}: {source}")]
    Sqlite {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error("invalid derived relative path `{path}`: {reason}")]
    InvalidDerivedPath { path: String, reason: String },

    #[error(
        "artifact blob fingerprint mismatch for {artifact_id}: expected {expected}, actual {actual}"
    )]
    FingerprintMismatch {
        artifact_id: String,
        expected: String,
        actual: String,
    },

    #[error("invalid resource reference for {resource_id}: {reason}")]
    InvalidResourceRef { resource_id: String, reason: String },

    #[error("dependency range overflow for start {start_us} duration {duration_us}")]
    RangeOverflow { start_us: u64, duration_us: u64 },

    #[error("invalid artifact dependency `{dependency_key}`: {reason}")]
    InvalidDependency {
        dependency_key: String,
        reason: String,
    },
}
