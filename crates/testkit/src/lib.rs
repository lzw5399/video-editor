//! Test harness shell for fixtures, goldens, and render smoke checks.
//!
//! Later Phase 1 plans add deterministic schema fixtures, tiny media generation,
//! and FFmpeg render smoke helpers here. This shell exists so downstream plans
//! can depend on a stable testkit crate without introducing media behavior early.

use std::fmt;
use std::path::{Path, PathBuf};

use media_runtime::{
    FfmpegExecutor, MAX_STDERR_SUMMARY_BYTES, RuntimeConfig, discover_runtime_config,
};
use media_runtime_desktop::DesktopFfmpegExecutor;

/// Boundary marker for Phase 1 test harness helpers.
pub const TESTKIT_BOUNDARY: &str = "fixtures-goldens-render-smoke-shell";

const TINY_WIDTH: u32 = 160;
const TINY_HEIGHT: u32 = 90;
const TINY_FPS: u32 = 10;
const TINY_DURATION_SECONDS: f64 = 1.0;

/// Result type for deterministic smoke helpers.
pub type SmokeResult<T> = Result<T, SmokeError>;

/// Error raised by deterministic smoke helpers.
#[derive(Debug)]
pub struct SmokeError {
    message: String,
}

impl SmokeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SmokeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SmokeError {}

impl From<media_runtime::DiscoveryError> for SmokeError {
    fn from(error: media_runtime::DiscoveryError) -> Self {
        Self::new(format!("{error}: {}", error.remediation))
    }
}

impl From<std::io::Error> for SmokeError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

/// Temporary media generated from FFmpeg lavfi sources.
#[derive(Debug)]
pub struct TinyLavfiMedia {
    _temp_dir: tempfile::TempDir,
    output_path: PathBuf,
}

impl TinyLavfiMedia {
    /// Path to the generated MP4 output. The file is removed when this value is dropped.
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

/// Generate a tiny deterministic MP4 using FFmpeg lavfi sources.
pub fn generate_tiny_lavfi_media() -> SmokeResult<TinyLavfiMedia> {
    let runtime = discover_runtime_config()?;
    let executor = DesktopFfmpegExecutor;
    let temp_dir = tempfile::Builder::new()
        .prefix("media-generated-")
        .tempdir()?;
    let media_dir = temp_dir.path().join("media-generated");
    std::fs::create_dir_all(&media_dir)?;
    let output_path = media_dir.join("tiny-render-smoke.mp4");

    run_ffmpeg_generate(&executor, &runtime, &output_path)?;

    if !output_path.is_file() {
        return Err(SmokeError::new(format!(
            "ffmpeg completed but did not create {}",
            output_path.display()
        )));
    }

    Ok(TinyLavfiMedia {
        _temp_dir: temp_dir,
        output_path,
    })
}

fn run_ffmpeg_generate(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
) -> SmokeResult<()> {
    let args = vec![
        "-hide_banner".to_string(),
        "-y".to_string(),
        "-f".to_string(),
        "lavfi".to_string(),
        "-i".to_string(),
        format!(
            "testsrc2=size={TINY_WIDTH}x{TINY_HEIGHT}:rate={TINY_FPS}:duration={TINY_DURATION_SECONDS}"
        ),
        "-f".to_string(),
        "lavfi".to_string(),
        "-i".to_string(),
        format!("sine=frequency=440:duration={TINY_DURATION_SECONDS}"),
        "-shortest".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        output_path.display().to_string(),
    ];

    let output = executor.run(&runtime.ffmpeg.path, &args).map_err(|error| {
        SmokeError::new(format!(
            "failed to launch ffmpeg at {}: {error}",
            runtime.ffmpeg.path.display()
        ))
    })?;

    if !output.status.success() {
        return Err(SmokeError::new(format!(
            "ffmpeg lavfi generation failed: stdout=`{}` stderr=`{}`",
            bounded_summary(&output.stdout),
            bounded_summary(&output.stderr)
        )));
    }

    Ok(())
}

fn bounded_summary(bytes: &[u8]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_tiny_lavfi_media_creates_temporary_mp4() {
        let media = generate_tiny_lavfi_media().expect(
            "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
        );

        assert!(
            media.output_path().is_file(),
            "tiny lavfi smoke output should exist"
        );
        assert_eq!(
            media
                .output_path()
                .extension()
                .and_then(|value| value.to_str()),
            Some("mp4")
        );
        assert!(
            media
                .output_path()
                .ancestors()
                .any(|path| path.file_name().and_then(|value| value.to_str())
                    == Some("media-generated")),
            "generated media should live under a media-generated temp directory"
        );
    }
}
