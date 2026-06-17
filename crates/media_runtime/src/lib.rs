//! FFmpeg process runtime boundary.
//!
//! This crate owns the service boundary for FFmpeg and ffprobe execution. Pure
//! draft and timeline semantic crates must not depend on this trait.

use std::path::Path;
use std::process::Output;

mod discovery;
mod error;
mod process;

pub use discovery::{
    BinaryKind, DiscoveredBinary, DiscoverySource, MAX_STDERR_SUMMARY_BYTES, RuntimeConfig,
    discover_runtime_config, probe_binary_version, probe_binary_version_with_timeout,
    resolve_binary,
};
pub use error::{DiscoveryError, DiscoveryErrorKind};
pub use process::{DEFAULT_PROCESS_TIMEOUT, run_process_with_timeout};

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

    /// Run an FFmpeg-family process with explicit process arguments.
    fn run(&self, binary: &Path, args: &[String]) -> std::io::Result<Output>;
}
