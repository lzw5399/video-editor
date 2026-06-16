//! FFmpeg process runtime boundary.
//!
//! This crate owns the service boundary for FFmpeg and ffprobe execution. Pure
//! draft and timeline semantic crates must not depend on this trait.

use std::path::Path;
use std::process::Output;

mod discovery;
mod error;

pub use discovery::{
    BinaryKind, DiscoveredBinary, DiscoverySource, MAX_STDERR_SUMMARY_BYTES, RuntimeConfig,
    discover_runtime_config, probe_binary_version, resolve_binary,
};
pub use error::{DiscoveryError, DiscoveryErrorKind};

/// Service-boundary trait for executing FFmpeg-family binaries.
///
/// Implementations decide how to launch processes for a given platform. The
/// trait is intentionally narrow in Phase 1: it establishes ownership of the
/// runtime boundary without implementing discovery or render behavior.
pub trait FfmpegExecutor {
    /// Stable label for diagnostics and future compatibility reports.
    fn executor_name(&self) -> &'static str;

    /// Returns whether this executor can attempt to run a binary at `binary`.
    fn can_execute(&self, binary: &Path) -> bool;

    /// Run a version probe with explicit process arguments.
    fn run_version_probe(&self, binary: &Path) -> std::io::Result<Output>;
}
