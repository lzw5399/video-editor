use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterKaipaiError {
    #[error("invalid Kaipai formula bundle JSON: {message}")]
    InvalidBundleJson { message: String },

    #[error("unsupported Kaipai formula bundle schema version: found {found}, expected {expected}")]
    UnsupportedSchemaVersion { found: u32, expected: u32 },

    #[error("missing required Kaipai formula evidence `{field}`: {reason}")]
    MissingRequiredEvidence {
        field: &'static str,
        reason: &'static str,
    },

    #[error("unsafe Kaipai formula evidence at `{path}`: {reason}")]
    UnsafeFormulaEvidence { path: String, reason: &'static str },

    #[error("invalid Kaipai resource evidence `{resource_id}`: {reason}")]
    InvalidResourceEvidence {
        resource_id: String,
        reason: &'static str,
    },

    #[error("Kaipai resource localization IO failed at {path}: {source}")]
    ResourceLocalizationIo {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}
