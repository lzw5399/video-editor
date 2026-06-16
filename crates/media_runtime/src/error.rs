use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::BinaryKind;

/// Stable classes of FFmpeg/ffprobe discovery failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(rename_all = "camelCase")]
pub enum DiscoveryErrorKind {
    #[error("missing binary")]
    MissingBinary,
    #[error("version probe failed")]
    VersionProbeFailed,
    #[error("unsupported version")]
    UnsupportedVersion,
}

/// Structured runtime discovery error with UI-ready remediation details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryError {
    pub kind: DiscoveryErrorKind,
    pub binary: BinaryKind,
    pub checked_paths: Vec<PathBuf>,
    pub remediation: String,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
}

impl DiscoveryError {
    pub(crate) fn missing_binary(binary: BinaryKind, checked_paths: Vec<PathBuf>) -> Self {
        let env_var = binary.env_var();
        let binary_name = binary.binary_name();
        Self {
            kind: DiscoveryErrorKind::MissingBinary,
            binary,
            checked_paths,
            remediation: format!(
                "Set {env_var} to a valid {binary_name} binary or install {binary_name} on PATH."
            ),
            stdout_summary: None,
            stderr_summary: None,
        }
    }

    pub(crate) fn version_probe_failed(
        binary: BinaryKind,
        checked_paths: Vec<PathBuf>,
        stdout_summary: Option<String>,
        stderr_summary: Option<String>,
    ) -> Self {
        let env_var = binary.env_var();
        let binary_name = binary.binary_name();
        Self {
            kind: DiscoveryErrorKind::VersionProbeFailed,
            binary,
            checked_paths,
            remediation: format!(
                "Verify {env_var} points to a working {binary_name} binary or remove it to use PATH discovery."
            ),
            stdout_summary,
            stderr_summary,
        }
    }

    pub(crate) fn unsupported_version(
        binary: BinaryKind,
        checked_paths: Vec<PathBuf>,
        stdout_summary: Option<String>,
        stderr_summary: Option<String>,
    ) -> Self {
        let binary_name = binary.binary_name();
        Self {
            kind: DiscoveryErrorKind::UnsupportedVersion,
            binary,
            checked_paths,
            remediation: format!(
                "Install a supported {binary_name} binary whose -version output starts with `{binary_name} version`."
            ),
            stdout_summary,
            stderr_summary,
        }
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} discovery failed: {}",
            self.binary.binary_name(),
            self.kind
        )
    }
}

impl std::error::Error for DiscoveryError {}
