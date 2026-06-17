//! Test harness shell for fixtures, goldens, and render smoke checks.
//!
//! Later Phase 1 plans add deterministic schema fixtures, tiny media generation,
//! and FFmpeg render smoke helpers here. This shell exists so downstream plans
//! can depend on a stable testkit crate without introducing media behavior early.

use std::ffi::OsString;
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
const TINY_DURATION_MIN_MICROS: u64 = 900_000;
const TINY_DURATION_MAX_MICROS: u64 = 1_200_000;

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

/// ffprobe metadata used by the Phase 1 render smoke harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeMetadata {
    pub duration_microseconds: u64,
    pub width: u32,
    pub height: u32,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub has_video_stream: bool,
    pub has_audio_stream: bool,
}

/// Tiny render smoke output and parsed metadata.
#[derive(Debug)]
pub struct TinyRenderSmoke {
    media: TinyLavfiMedia,
    metadata: SmokeMetadata,
}

impl TinyRenderSmoke {
    /// Path to the generated MP4 output. The file is removed when this value is dropped.
    pub fn output_path(&self) -> &Path {
        self.media.output_path()
    }

    /// Parsed ffprobe metadata for the generated output.
    pub fn metadata(&self) -> &SmokeMetadata {
        &self.metadata
    }
}

/// Generate a tiny deterministic MP4 using FFmpeg lavfi sources.
pub fn generate_tiny_lavfi_media() -> SmokeResult<TinyLavfiMedia> {
    let runtime = discover_runtime_config()?;
    let executor = DesktopFfmpegExecutor::default();
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

/// Generate tiny media and assert it through ffprobe metadata.
pub fn run_tiny_render_smoke() -> SmokeResult<TinyRenderSmoke> {
    let media = generate_tiny_lavfi_media()?;
    let metadata = probe_media_metadata(media.output_path())?;
    assert_tiny_smoke_metadata(&metadata)?;

    Ok(TinyRenderSmoke { media, metadata })
}

/// Probe an existing media file with ffprobe and return metadata needed by the smoke gate.
pub fn probe_media_metadata(path: impl AsRef<Path>) -> SmokeResult<SmokeMetadata> {
    let path = path.as_ref();
    if !path.is_file() {
        return Err(SmokeError::new(format!(
            "cannot probe missing media file {}",
            path.display()
        )));
    }

    let runtime = discover_runtime_config()?;
    let executor = DesktopFfmpegExecutor::default();
    let args = vec![
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-output_format"),
        OsString::from("json"),
        OsString::from("-show_entries"),
        OsString::from("stream=codec_type,width,height,r_frame_rate,duration:format=duration"),
        path.as_os_str().to_owned(),
    ];
    let output = executor
        .run(&runtime.ffprobe.path, &args)
        .map_err(|error| {
            SmokeError::new(format!(
                "failed to launch ffprobe at {}: {error}",
                runtime.ffprobe.path.display()
            ))
        })?;

    if !output.status.success() {
        return Err(SmokeError::new(format!(
            "ffprobe metadata probe failed: stdout=`{}` stderr=`{}`",
            bounded_summary(&output.stdout),
            bounded_summary(&output.stderr)
        )));
    }

    parse_ffprobe_metadata(&output.stdout)
}

/// Assert that smoke metadata matches the Phase 1 tiny lavfi contract.
pub fn assert_tiny_smoke_metadata(metadata: &SmokeMetadata) -> SmokeResult<()> {
    if !metadata.has_video_stream {
        return Err(SmokeError::new("expected a video stream"));
    }

    if !metadata.has_audio_stream {
        return Err(SmokeError::new("expected an audio stream"));
    }

    if metadata.width != TINY_WIDTH || metadata.height != TINY_HEIGHT {
        return Err(SmokeError::new(format!(
            "expected {TINY_WIDTH}x{TINY_HEIGHT}, got {}x{}",
            metadata.width, metadata.height
        )));
    }

    if metadata.frame_rate_denominator == 0
        || metadata.frame_rate_numerator != TINY_FPS * metadata.frame_rate_denominator
    {
        return Err(SmokeError::new(format!(
            "expected {TINY_FPS} fps, got {}/{}",
            metadata.frame_rate_numerator, metadata.frame_rate_denominator
        )));
    }

    if !(TINY_DURATION_MIN_MICROS..=TINY_DURATION_MAX_MICROS)
        .contains(&metadata.duration_microseconds)
    {
        return Err(SmokeError::new(format!(
            "expected about one second, got {} microseconds",
            metadata.duration_microseconds
        )));
    }

    Ok(())
}

fn run_ffmpeg_generate(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
) -> SmokeResult<()> {
    let args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-y"),
        OsString::from("-f"),
        OsString::from("lavfi"),
        OsString::from("-i"),
        OsString::from(format!(
            "testsrc2=size={TINY_WIDTH}x{TINY_HEIGHT}:rate={TINY_FPS}:duration={TINY_DURATION_SECONDS}"
        )),
        OsString::from("-f"),
        OsString::from("lavfi"),
        OsString::from("-i"),
        OsString::from(format!(
            "sine=frequency=440:duration={TINY_DURATION_SECONDS}"
        )),
        OsString::from("-shortest"),
        OsString::from("-c:v"),
        OsString::from("libx264"),
        OsString::from("-pix_fmt"),
        OsString::from("yuv420p"),
        OsString::from("-c:a"),
        OsString::from("aac"),
        output_path.as_os_str().to_owned(),
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

fn parse_ffprobe_metadata(bytes: &[u8]) -> SmokeResult<SmokeMetadata> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        SmokeError::new(format!("failed to parse ffprobe JSON metadata: {error}"))
    })?;
    let streams = value
        .get("streams")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| SmokeError::new("ffprobe JSON metadata did not include streams"))?;
    let video_stream = streams.iter().find(|stream| {
        stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("video")
    });
    let audio_stream = streams.iter().find(|stream| {
        stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("audio")
    });
    let video_stream =
        video_stream.ok_or_else(|| SmokeError::new("ffprobe did not report a video stream"))?;
    let format_duration = value
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(serde_json::Value::as_str);
    let stream_duration = video_stream
        .get("duration")
        .and_then(serde_json::Value::as_str);
    let duration_microseconds = format_duration
        .or(stream_duration)
        .ok_or_else(|| SmokeError::new("ffprobe did not report media duration"))
        .and_then(parse_decimal_seconds_to_microseconds)?;
    let frame_rate = video_stream
        .get("r_frame_rate")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| SmokeError::new("ffprobe did not report video frame rate"))
        .and_then(parse_rational_frame_rate)?;

    Ok(SmokeMetadata {
        duration_microseconds,
        width: json_u32(video_stream, "width")?,
        height: json_u32(video_stream, "height")?,
        frame_rate_numerator: frame_rate.0,
        frame_rate_denominator: frame_rate.1,
        has_video_stream: true,
        has_audio_stream: audio_stream.is_some(),
    })
}

fn json_u32(value: &serde_json::Value, key: &str) -> SmokeResult<u32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| SmokeError::new(format!("ffprobe did not report numeric {key}")))
}

fn parse_decimal_seconds_to_microseconds(value: &str) -> SmokeResult<u64> {
    let (whole, fractional) = value
        .split_once('.')
        .map_or((value, ""), |(whole, fractional)| (whole, fractional));
    let whole_micros = whole
        .parse::<u64>()
        .map_err(|error| SmokeError::new(format!("invalid duration seconds `{value}`: {error}")))?
        .saturating_mul(1_000_000);
    let mut fraction = fractional
        .chars()
        .take(6)
        .filter(|character| character.is_ascii_digit())
        .collect::<String>();

    while fraction.len() < 6 {
        fraction.push('0');
    }

    let fraction_micros = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().map_err(|error| {
            SmokeError::new(format!("invalid duration fraction `{value}`: {error}"))
        })?
    };

    Ok(whole_micros.saturating_add(fraction_micros))
}

fn parse_rational_frame_rate(value: &str) -> SmokeResult<(u32, u32)> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or_else(|| SmokeError::new(format!("invalid frame rate `{value}`")))?;
    let numerator = numerator.parse::<u32>().map_err(|error| {
        SmokeError::new(format!("invalid frame rate numerator `{value}`: {error}"))
    })?;
    let denominator = denominator.parse::<u32>().map_err(|error| {
        SmokeError::new(format!("invalid frame rate denominator `{value}`: {error}"))
    })?;

    if denominator == 0 {
        return Err(SmokeError::new("frame rate denominator cannot be zero"));
    }

    Ok((numerator, denominator))
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
