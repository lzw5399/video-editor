//! FFmpeg process runtime boundary.
//!
//! This crate owns the service boundary for FFmpeg and ffprobe execution. Later
//! plans add env/PATH discovery, version probes, progress reporting, and
//! structured runtime errors here. Pure draft and timeline semantic crates must
//! not depend on this trait.

use std::path::Path;

/// Service-boundary trait for executing FFmpeg-family binaries.
///
/// Implementations decide how to launch processes for a given platform. The
/// trait is intentionally narrow in Phase 1: it establishes ownership of the
/// runtime boundary without implementing discovery or render behavior.
pub trait FfmpegExecutor {
    /// Stable label for diagnostics and future compatibility reports.
    fn executor_name(&self) -> &'static str;

    /// Returns whether this executor can attempt to run a binary at `binary`.
    ///
    /// Phase 1 implementations may answer conservatively. Later discovery work
    /// will probe `ffmpeg -version` and `ffprobe -version` through this boundary.
    fn can_execute(&self, binary: &Path) -> bool;
}
