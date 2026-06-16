//! Desktop implementation shell for the FFmpeg runtime boundary.
//!
//! This crate is the desktop backend for `media_runtime::FfmpegExecutor`.
//! Electron can inject this service at the app shell boundary. It does not
//! download, bundle, redistribute, discover, or run FFmpeg in this plan.

use std::path::Path;

use media_runtime::FfmpegExecutor;

/// Desktop FFmpeg executor shell.
#[derive(Debug, Default, Clone, Copy)]
pub struct DesktopFfmpegExecutor;

impl FfmpegExecutor for DesktopFfmpegExecutor {
    fn executor_name(&self) -> &'static str {
        "desktop-ffmpeg-executor"
    }

    fn can_execute(&self, binary: &Path) -> bool {
        binary.is_file()
    }
}
