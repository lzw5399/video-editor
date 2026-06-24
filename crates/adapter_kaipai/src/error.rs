//! Error types for the offline Kaipai adapter boundary.

use draft_import::{DraftImportPlanValidationError, ResourceLocalizationError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterKaipaiError {
    #[error("invalid Kaipai formula bundle JSON: {message}")]
    InvalidBundleJson { message: String },

    #[error("invalid Kaipai formula bundle at `{path}`: {reason}")]
    InvalidBundle { path: String, reason: &'static str },

    #[error("unsafe Kaipai formula evidence at `{path}`: {reason}")]
    UnsafeFormulaEvidence { path: String, reason: &'static str },

    #[error("Kaipai mapper failed at `{path}`: {reason}")]
    Mapper { path: String, reason: &'static str },

    #[error(transparent)]
    ResourceLocalization { source: ResourceLocalizationError },

    #[error(transparent)]
    ImportPlan {
        source: DraftImportPlanValidationError,
    },
}

impl From<ResourceLocalizationError> for AdapterKaipaiError {
    fn from(source: ResourceLocalizationError) -> Self {
        Self::ResourceLocalization { source }
    }
}

impl From<DraftImportPlanValidationError> for AdapterKaipaiError {
    fn from(source: DraftImportPlanValidationError) -> Self {
        Self::ImportPlan { source }
    }
}
