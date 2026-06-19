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
}
