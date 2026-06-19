//! Desktop implementation shell for the FFmpeg runtime boundary.
//!
//! This crate is the desktop backend for `media_runtime::FfmpegExecutor`.
//! Electron can inject this service at the app shell boundary. It does not
//! download, bundle, or redistribute FFmpeg in this plan.

use std::ffi::OsString;
use std::path::Path;
use std::process::Output;
use std::time::Duration;

use media_runtime::{DEFAULT_PROCESS_TIMEOUT, FfmpegExecutor, run_process_with_timeout};

mod capabilities;
mod ffmpeg_fallback;
mod platform;

pub use capabilities::probe_desktop_runtime_capabilities;
pub use ffmpeg_fallback::{
    FfmpegCpuFrameDecodeRequest, FfmpegCpuFrameFingerprint, FfmpegCpuFrameFingerprintError,
    FfmpegCpuFrameFingerprintRequest, FfmpegCpuVideoDecoder, FfmpegDecodeDiagnostic,
    FfmpegFallbackMediaReader, FfmpegFallbackMediaSession, decode_ffmpeg_cpu_frame_fingerprint,
};
#[cfg(target_os = "macos")]
pub use platform::macos_system_metal_device_id;
pub use platform::{
    MacosMediaReader, MacosMediaSession, MacosRegisteredTextureLease, MacosTextureInteropPolicy,
    MacosVideoDecoder, WindowsMediaReader, WindowsMediaSession, WindowsTextureInteropPolicy,
    WindowsVideoDecoder, select_macos_texture_interop_fallback,
    select_windows_texture_interop_fallback,
};

/// Desktop FFmpeg executor shell.
#[derive(Debug, Clone, Copy)]
pub struct DesktopFfmpegExecutor {
    timeout: Duration,
}

impl Default for DesktopFfmpegExecutor {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_PROCESS_TIMEOUT,
        }
    }
}

impl DesktopFfmpegExecutor {
    /// Create a desktop executor with a custom process timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl FfmpegExecutor for DesktopFfmpegExecutor {
    fn executor_name(&self) -> &'static str {
        "desktop-ffmpeg-executor"
    }

    fn can_execute(&self, binary: &Path) -> bool {
        binary.is_file()
    }

    fn run_version_probe(&self, binary: &Path) -> std::io::Result<Output> {
        let args = vec![OsString::from("-version")];
        run_process_with_timeout(binary, &args, self.timeout)
    }

    fn run(&self, binary: &Path, args: &[OsString]) -> std::io::Result<Output> {
        run_process_with_timeout(binary, args, self.timeout)
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
