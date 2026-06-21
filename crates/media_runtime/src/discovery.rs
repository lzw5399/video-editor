use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{DEFAULT_PROCESS_TIMEOUT, DiscoveryError, run_process_with_timeout};

/// Maximum bytes retained from external process stdout/stderr summaries.
pub const MAX_STDERR_SUMMARY_BYTES: usize = 4096;
/// Test/development override for packaged FFmpeg-family resources.
pub const BUNDLED_FFMPEG_DIR_ENV: &str = "VE_BUNDLED_FFMPEG_DIR";

static CONFIGURED_BUNDLED_RUNTIME_DIRECTORY: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

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
}

/// Source used to resolve an FFmpeg-family binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DiscoverySource {
    Bundled { directory: PathBuf },
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
    discover_bundled_runtime_config()
}

/// Configure the packaged FFmpeg-family runtime directory from the app shell.
pub fn configure_bundled_runtime_directory(directory: PathBuf) {
    *configured_bundled_runtime_directory_slot().lock().unwrap() = Some(directory);
}

/// Discover only packaged FFmpeg-family resources.
pub fn discover_bundled_runtime_config() -> Result<RuntimeConfig, DiscoveryError> {
    let ffmpeg = resolve_bundled_binary(BinaryKind::Ffmpeg)?;
    let ffprobe = resolve_bundled_binary(BinaryKind::Ffprobe)?;

    Ok(RuntimeConfig { ffmpeg, ffprobe })
}

pub fn resolve_binary(kind: BinaryKind) -> Result<DiscoveredBinary, DiscoveryError> {
    resolve_bundled_binary(kind)
}

fn resolve_bundled_binary(kind: BinaryKind) -> Result<DiscoveredBinary, DiscoveryError> {
    let directory = bundled_runtime_directory();
    let path = directory.join(platform_binary_filename(kind.binary_name()));
    if !path.is_file() {
        return Err(DiscoveryError::missing_binary(kind, vec![path]));
    }

    probe_binary_version(
        kind,
        path,
        DiscoverySource::Bundled {
            directory: directory.clone(),
        },
    )
}

/// Run a `-version` probe for a resolved binary using process argument arrays.
pub fn probe_binary_version(
    kind: BinaryKind,
    path: PathBuf,
    source: DiscoverySource,
) -> Result<DiscoveredBinary, DiscoveryError> {
    probe_binary_version_with_timeout(kind, path, source, DEFAULT_PROCESS_TIMEOUT)
}

/// Run a `-version` probe with an explicit timeout.
pub fn probe_binary_version_with_timeout(
    kind: BinaryKind,
    path: PathBuf,
    source: DiscoverySource,
    timeout: Duration,
) -> Result<DiscoveredBinary, DiscoveryError> {
    let args = vec![OsString::from("-version")];
    let output = run_process_with_timeout(&path, &args, timeout).map_err(|error| {
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

fn bundled_runtime_directory() -> PathBuf {
    if let Some(directory) = configured_bundled_runtime_directory() {
        return directory;
    }

    if let Some(directory) = debug_env_bundled_runtime_directory() {
        return directory;
    }

    default_development_bundled_runtime_directory()
}

fn configured_bundled_runtime_directory() -> Option<PathBuf> {
    configured_bundled_runtime_directory_slot()
        .lock()
        .unwrap()
        .clone()
}

fn configured_bundled_runtime_directory_slot() -> &'static Mutex<Option<PathBuf>> {
    CONFIGURED_BUNDLED_RUNTIME_DIRECTORY.get_or_init(|| Mutex::new(None))
}

#[cfg(debug_assertions)]
fn debug_env_bundled_runtime_directory() -> Option<PathBuf> {
    env::var_os(BUNDLED_FFMPEG_DIR_ENV).map(PathBuf::from)
}

#[cfg(not(debug_assertions))]
fn debug_env_bundled_runtime_directory() -> Option<PathBuf> {
    None
}

fn default_development_bundled_runtime_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop-electron/runtime/ffmpeg")
        .join(platform_arch_segment())
}

fn platform_binary_filename(binary_name: &str) -> String {
    if cfg!(windows) {
        format!("{binary_name}.exe")
    } else {
        binary_name.to_owned()
    }
}

fn platform_arch_segment() -> String {
    let platform = match env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        value => value,
    };
    let arch = match env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        value => value,
    };
    format!("{platform}-{arch}")
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
