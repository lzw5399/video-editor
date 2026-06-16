//! Desktop implementation shell for the FFmpeg runtime boundary.
//!
//! This crate is the desktop backend for `media_runtime::FfmpegExecutor`.
//! Electron can inject this service at the app shell boundary. It does not
//! download, bundle, or redistribute FFmpeg in this plan.

use std::path::Path;
use std::process::{Command, Output};

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

    fn run_version_probe(&self, binary: &Path) -> std::io::Result<Output> {
        Command::new(binary).args(["-version"]).output()
    }

    fn run(&self, binary: &Path, args: &[String]) -> std::io::Result<Output> {
        Command::new(binary).args(args).output()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use media_runtime::FfmpegExecutor;

    use super::DesktopFfmpegExecutor;

    #[test]
    fn run_times_out_for_hung_processes() {
        let sandbox = std::env::temp_dir().join(format!(
            "video-editor-desktop-runtime-timeout-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&sandbox).unwrap();
        let binary = sandbox.join("ffmpeg");
        fs::write(&binary, "#!/bin/sh\nsleep 2\n").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let executor = DesktopFfmpegExecutor::with_timeout(Duration::from_millis(100));
        let error = executor
            .run(&binary, &[])
            .expect_err("hung process should time out");

        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(error.to_string().contains("timed out"));

        let _ = fs::remove_dir_all(sandbox);
    }
}
