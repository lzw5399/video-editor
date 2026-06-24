//! Error types for the offline Kaipai adapter boundary.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterKaipaiError {
    #[error("invalid Kaipai formula bundle JSON: {message}")]
    InvalidBundleJson { message: String },

    #[error("invalid Kaipai formula bundle at `{path}`: {reason}")]
    InvalidBundle { path: String, reason: &'static str },

    #[error("unsafe Kaipai formula evidence at `{path}`: {reason}")]
    UnsafeFormulaEvidence { path: String, reason: &'static str },
}
