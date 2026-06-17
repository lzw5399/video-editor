use std::path::PathBuf;

use draft_model::{DraftSchemaVersion, DraftValidationError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectStoreError {
    #[error("project store IO failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid project JSON at {path}: {message}")]
    InvalidProjectJson { path: PathBuf, message: String },

    #[error(
        "unsupported draft schema version at {path}: found {found}, expected {}",
        DraftSchemaVersion::CURRENT_VALUE
    )]
    UnsupportedSchemaVersion { path: PathBuf, found: String },

    #[error("draft semantic validation failed at {path}: {source}")]
    SemanticValidation {
        path: PathBuf,
        #[source]
        source: DraftValidationError,
    },

    #[error("invalid material URI `{uri}`: {reason}")]
    InvalidMaterialUri { uri: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectStoreWarning {
    MissingMaterial {
        material_id: String,
        uri: String,
        resolved_path: Option<PathBuf>,
    },
}
