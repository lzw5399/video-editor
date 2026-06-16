use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::DiscoveryError;

/// Maximum bytes retained from external process stdout/stderr summaries.
pub const MAX_STDERR_SUMMARY_BYTES: usize = 4096;

/// FFmpeg-family binary kind discovered by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BinaryKind {
    Ffmpeg,
    Ffprobe,
}

impl BinaryKind {
    pub fn binary_name(self) -> &'static str {
        match self {
            Self::Ffmpeg => "ffmpeg",
            Self::Ffprobe => "ffprobe",
        }
    }

    pub fn env_var(self) -> &'static str {
        match self {
            Self::Ffmpeg => "VE_FFMPEG_PATH",
            Self::Ffprobe => "VE_FFPROBE_PATH",
        }
    }
}

/// Source used to resolve an FFmpeg-family binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DiscoverySource {
    Env { variable: String },
    Path,
}

/// Version-probed binary metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredBinary {
    pub kind: BinaryKind,
    pub path: PathBuf,
    pub source: DiscoverySource,
    pub version: String,
}

/// Runtime configuration needed by later preview/export jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfig {
    pub ffmpeg: DiscoveredBinary,
    pub ffprobe: DiscoveredBinary,
}

/// Discover and version-probe both FFmpeg and ffprobe.
pub fn discover_runtime_config() -> Result<RuntimeConfig, DiscoveryError> {
    let ffmpeg = resolve_binary(BinaryKind::Ffmpeg)?;
    let ffprobe = resolve_binary(BinaryKind::Ffprobe)?;

    Ok(RuntimeConfig { ffmpeg, ffprobe })
}

/// Resolve a binary through its explicit env var first, then PATH.
pub fn resolve_binary(kind: BinaryKind) -> Result<DiscoveredBinary, DiscoveryError> {
    let env_var = kind.env_var();

    if let Some(explicit_path) = env::var_os(env_var) {
        let path = PathBuf::from(explicit_path);
        if !path.is_file() {
            return Err(DiscoveryError::missing_binary(kind, vec![path]));
        }

        return probe_binary_version(
            kind,
            path.clone(),
            DiscoverySource::Env {
                variable: env_var.to_string(),
            },
        );
    }

    let binary_name = kind.binary_name();
    match which::which(binary_name) {
        Ok(path) => probe_binary_version(kind, path, DiscoverySource::Path),
        Err(_) => Err(DiscoveryError::missing_binary(
            kind,
            checked_path_candidates(binary_name),
        )),
    }
}

/// Run a `-version` probe for a resolved binary using process argument arrays.
pub fn probe_binary_version(
    kind: BinaryKind,
    path: PathBuf,
    source: DiscoverySource,
) -> Result<DiscoveredBinary, DiscoveryError> {
    let output = Command::new(&path)
        .args(["-version"])
        .output()
        .map_err(|error| {
            DiscoveryError::version_probe_failed(
                kind,
                vec![path.clone()],
                None,
                Some(summarize_output(error.to_string().as_bytes())),
            )
        })?;

    let stdout_summary = optional_summary(&output.stdout);
    let stderr_summary = optional_summary(&output.stderr);

    if !output.status.success() {
        return Err(DiscoveryError::version_probe_failed(
            kind,
            vec![path],
            stdout_summary,
            stderr_summary,
        ));
    }

    let version = stdout_summary
        .as_deref()
        .and_then(first_non_empty_line)
        .map(str::to_string)
        .ok_or_else(|| {
            DiscoveryError::unsupported_version(
                kind,
                vec![path.clone()],
                None,
                stderr_summary.clone(),
            )
        })?;

    let expected_prefix = format!("{} version", kind.binary_name());
    if !version.starts_with(&expected_prefix) {
        return Err(DiscoveryError::unsupported_version(
            kind,
            vec![path],
            Some(version),
            stderr_summary,
        ));
    }

    Ok(DiscoveredBinary {
        kind,
        path,
        source,
        version,
    })
}

fn checked_path_candidates(binary_name: &str) -> Vec<PathBuf> {
    env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .map(|path| path.join(binary_name))
        .collect()
}

fn optional_summary(bytes: &[u8]) -> Option<String> {
    let summary = summarize_output(bytes);
    if summary.is_empty() {
        None
    } else {
        Some(summary)
    }
}

fn summarize_output(bytes: &[u8]) -> String {
    let value = String::from_utf8_lossy(bytes);
    let trimmed = value.trim();
    let mut summary = String::new();

    for character in trimmed.chars() {
        if summary.len() + character.len_utf8() > MAX_STDERR_SUMMARY_BYTES {
            break;
        }
        summary.push(character);
    }

    summary
}

fn first_non_empty_line(value: &str) -> Option<&str> {
    value.lines().find(|line| !line.trim().is_empty())
}
