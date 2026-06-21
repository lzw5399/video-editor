//! Preview/export render comparison helpers.
//!
//! These helpers keep Phase 5 golden checks in Rust-owned test/runtime
//! boundaries. They probe FFmpeg capabilities, extract comparable RGB frames,
//! and report classified setup errors instead of silently skipping parity.

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use draft_model::{
    BUNDLED_TEXT_FONT_FAMILY, BUNDLED_TEXT_FONT_LICENSE_SPDX, BUNDLED_TEXT_FONT_REF,
    bundled_text_font_path,
};
use ffmpeg_compiler::{CompilerCapabilities, TextRenderCapability};
use media_runtime::{FfmpegExecutor, MAX_STDERR_SUMMARY_BYTES, RuntimeConfig};

pub const PHASE5_MEAN_RGB_DELTA_MAX: f64 = 8.0;
pub const PHASE5_P99_RGB_DELTA_MAX: u8 = 24;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelTolerance {
    pub mean_rgb_delta_max: f64,
    pub p99_rgb_delta_max: u8,
}

impl PixelTolerance {
    pub const fn phase5() -> Self {
        Self {
            mean_rgb_delta_max: PHASE5_MEAN_RGB_DELTA_MAX,
            p99_rgb_delta_max: PHASE5_P99_RGB_DELTA_MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparableFrame {
    pub metadata: FrameMetadata,
    pub rgb24: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameMetadata {
    pub width: u32,
    pub height: u32,
    pub frame_index: u64,
    pub timestamp_microseconds: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameComparison {
    pub width: u32,
    pub height: u32,
    pub mean_rgb_delta: f64,
    pub p99_rgb_delta: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderSetupErrorKind {
    FfmpegProbeFailed,
    MissingEncoder,
    MissingFilter,
    MissingFont,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSetupError {
    pub kind: RenderSetupErrorKind,
    pub message: String,
    pub remediation: String,
}

impl RenderSetupError {
    fn new(
        kind: RenderSetupErrorKind,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            remediation: remediation.into(),
        }
    }
}

impl fmt::Display for RenderSetupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?}: {} Remediation: {}",
            self.kind, self.message, self.remediation
        )
    }
}

impl Error for RenderSetupError {}

#[derive(Debug)]
pub enum RenderCompareError {
    Setup(RenderSetupError),
    Runtime(String),
    Assertion(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl RenderCompareError {
    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }

    fn assertion(message: impl Into<String>) -> Self {
        Self::Assertion(message.into())
    }
}

impl fmt::Display for RenderCompareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(error) => write!(formatter, "{error}"),
            Self::Runtime(message) | Self::Assertion(message) => formatter.write_str(message),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for RenderCompareError {}

impl From<std::io::Error> for RenderCompareError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for RenderCompareError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub type RenderCompareResult<T> = Result<T, RenderCompareError>;

pub fn probe_phase5_render_capabilities(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
) -> RenderCompareResult<CompilerCapabilities> {
    let encoders = run_ffmpeg_probe(executor, runtime, &["-hide_banner", "-encoders"])?;
    let filters = run_ffmpeg_probe(executor, runtime, &["-hide_banner", "-filters"])?;
    compiler_capabilities_from_probe_outputs(
        &encoders,
        &filters,
        {
            let mut paths = resolved_text_font_paths();
            paths.push(bundled_text_font_path());
            paths
        },
        env::var_os("VE_TEXT_FONT_PATH").map(PathBuf::from),
    )
}

pub fn compiler_capabilities_from_probe_outputs(
    encoders: &str,
    filters: &str,
    available_font_paths: Vec<PathBuf>,
    env_text_font_path: Option<PathBuf>,
) -> RenderCompareResult<CompilerCapabilities> {
    if !probe_output_has_feature(encoders, "libx264") {
        return Err(RenderCompareError::Setup(RenderSetupError::new(
            RenderSetupErrorKind::MissingEncoder,
            "Phase 5 render parity requires the libx264 encoder.",
            "Install/select an FFmpeg build with libx264 before running preview/export parity.",
        )));
    }
    if !probe_output_has_feature(encoders, "aac") {
        return Err(RenderCompareError::Setup(RenderSetupError::new(
            RenderSetupErrorKind::MissingEncoder,
            "Phase 5 render parity requires the AAC encoder.",
            "Install/select an FFmpeg build with AAC encoder support before running export parity.",
        )));
    }
    if !probe_output_has_feature(filters, "ass") {
        return Err(RenderCompareError::Setup(RenderSetupError::new(
            RenderSetupErrorKind::MissingFilter,
            "Phase 5 text parity requires the ASS filter.",
            "Install/select an FFmpeg build with libass/ASS filter support.",
        )));
    }
    if !probe_output_has_feature(filters, "subtitles") {
        return Err(RenderCompareError::Setup(RenderSetupError::new(
            RenderSetupErrorKind::MissingFilter,
            "Phase 5 text parity requires the subtitles filter.",
            "Install/select an FFmpeg build with subtitles filter support.",
        )));
    }

    let bundled_font_path = bundled_text_font_path().to_string_lossy().into_owned();
    let available_font_paths = available_font_paths
        .into_iter()
        .filter(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if available_font_paths.is_empty() {
        return Err(RenderCompareError::Setup(RenderSetupError::new(
            RenderSetupErrorKind::MissingFont,
            "Phase 5 text parity requires one pinned deterministic text font.",
            "Set VE_TEXT_FONT_PATH or install PingFang SC, Arial Unicode, Noto Sans CJK, or DejaVu Sans.",
        )));
    }

    let env_text_font_path = env_text_font_path
        .filter(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned());

    Ok(CompilerCapabilities {
        supports_h264_encoder: true,
        supports_aac_encoder: true,
        text: TextRenderCapability {
            supports_ass_filter: true,
            supports_subtitles_filter: true,
            env_text_font_path,
            available_font_paths,
            bundled_font_ref: Some(BUNDLED_TEXT_FONT_REF.to_owned()),
            bundled_font_family: Some(BUNDLED_TEXT_FONT_FAMILY.to_owned()),
            bundled_font_path: Some(bundled_font_path),
            bundled_font_license: Some(BUNDLED_TEXT_FONT_LICENSE_SPDX.to_owned()),
        },
    })
}

pub fn extract_rgb_frame_at(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    media_path: impl AsRef<Path>,
    timestamp_microseconds: u64,
    frame_index: u64,
    width: u32,
    height: u32,
) -> RenderCompareResult<ComparableFrame> {
    let media_path = media_path.as_ref();
    let args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-i"),
        media_path.as_os_str().to_owned(),
        OsString::from("-ss"),
        OsString::from(format_seconds(timestamp_microseconds)),
        OsString::from("-frames:v"),
        OsString::from("1"),
        OsString::from("-f"),
        OsString::from("rawvideo"),
        OsString::from("-pix_fmt"),
        OsString::from("rgb24"),
        OsString::from("pipe:1"),
    ];
    let output = executor.run(&runtime.ffmpeg.path, &args).map_err(|error| {
        RenderCompareError::runtime(format!(
            "failed to launch FFmpeg frame extraction at {}: {error}",
            runtime.ffmpeg.path.display()
        ))
    })?;
    if !output.status.success() {
        return Err(RenderCompareError::runtime(format!(
            "FFmpeg frame extraction failed for {}: stdout=`{}` stderr=`{}`",
            media_path.display(),
            bounded_summary(&output.stdout),
            bounded_summary(&output.stderr)
        )));
    }

    let expected_len = frame_byte_len(width, height)?;
    if output.stdout.len() != expected_len {
        return Err(RenderCompareError::runtime(format!(
            "expected raw RGB frame length {expected_len}, got {} for {}",
            output.stdout.len(),
            media_path.display()
        )));
    }

    Ok(ComparableFrame {
        metadata: FrameMetadata {
            width,
            height,
            frame_index,
            timestamp_microseconds,
        },
        rgb24: output.stdout,
    })
}

pub fn extract_rgb_frame_index(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    media_path: impl AsRef<Path>,
    frame_index: u64,
    timestamp_microseconds: u64,
    width: u32,
    height: u32,
) -> RenderCompareResult<ComparableFrame> {
    let media_path = media_path.as_ref();
    let args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-i"),
        media_path.as_os_str().to_owned(),
        OsString::from("-vf"),
        OsString::from(format!("select=eq(n\\,{frame_index})")),
        OsString::from("-frames:v"),
        OsString::from("1"),
        OsString::from("-vsync"),
        OsString::from("0"),
        OsString::from("-f"),
        OsString::from("rawvideo"),
        OsString::from("-pix_fmt"),
        OsString::from("rgb24"),
        OsString::from("pipe:1"),
    ];
    let output = executor.run(&runtime.ffmpeg.path, &args).map_err(|error| {
        RenderCompareError::runtime(format!(
            "failed to launch FFmpeg frame-index extraction at {}: {error}",
            runtime.ffmpeg.path.display()
        ))
    })?;
    if !output.status.success() {
        return Err(RenderCompareError::runtime(format!(
            "FFmpeg frame-index extraction failed for {}: stdout=`{}` stderr=`{}`",
            media_path.display(),
            bounded_summary(&output.stdout),
            bounded_summary(&output.stderr)
        )));
    }

    let expected_len = frame_byte_len(width, height)?;
    if output.stdout.len() != expected_len {
        return Err(RenderCompareError::runtime(format!(
            "expected raw RGB frame length {expected_len}, got {} for frame {frame_index} in {}",
            output.stdout.len(),
            media_path.display()
        )));
    }

    Ok(ComparableFrame {
        metadata: FrameMetadata {
            width,
            height,
            frame_index,
            timestamp_microseconds,
        },
        rgb24: output.stdout,
    })
}

pub fn probe_video_frame_metadata(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    media_path: impl AsRef<Path>,
) -> RenderCompareResult<Vec<FrameMetadata>> {
    let media_path = media_path.as_ref();
    let args = vec![
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-select_streams"),
        OsString::from("v:0"),
        OsString::from("-show_entries"),
        OsString::from(
            "stream=width,height:frame=best_effort_timestamp_time,pkt_pts_time,pts_time",
        ),
        OsString::from("-of"),
        OsString::from("json"),
        media_path.as_os_str().to_owned(),
    ];
    let output = executor
        .run(&runtime.ffprobe.path, &args)
        .map_err(|error| {
            RenderCompareError::runtime(format!(
                "failed to launch ffprobe frame metadata at {}: {error}",
                runtime.ffprobe.path.display()
            ))
        })?;
    if !output.status.success() {
        return Err(RenderCompareError::runtime(format!(
            "ffprobe frame metadata failed for {}: stdout=`{}` stderr=`{}`",
            media_path.display(),
            bounded_summary(&output.stdout),
            bounded_summary(&output.stderr)
        )));
    }

    parse_frame_metadata(&output.stdout)
}

pub fn assert_expected_frame_metadata(
    metadata: &FrameMetadata,
    expected_frame_index: u64,
    expected_timestamp_microseconds: u64,
    tolerance_microseconds: u64,
) -> RenderCompareResult<()> {
    if metadata.frame_index != expected_frame_index {
        return Err(RenderCompareError::assertion(format!(
            "expected frame index {expected_frame_index}, got {}",
            metadata.frame_index
        )));
    }
    let delta = metadata
        .timestamp_microseconds
        .abs_diff(expected_timestamp_microseconds);
    if delta > tolerance_microseconds {
        return Err(RenderCompareError::assertion(format!(
            "expected frame timestamp {expected_timestamp_microseconds} us +/- {tolerance_microseconds} us, got {} us",
            metadata.timestamp_microseconds
        )));
    }
    Ok(())
}

pub fn compare_rgb_frames(
    preview: &ComparableFrame,
    export: &ComparableFrame,
    tolerance: PixelTolerance,
) -> RenderCompareResult<FrameComparison> {
    if preview.metadata.width != export.metadata.width
        || preview.metadata.height != export.metadata.height
    {
        return Err(RenderCompareError::assertion(format!(
            "frame dimension mismatch: preview {}x{}, export {}x{}",
            preview.metadata.width,
            preview.metadata.height,
            export.metadata.width,
            export.metadata.height
        )));
    }
    if preview.rgb24.len() != export.rgb24.len() {
        return Err(RenderCompareError::assertion(format!(
            "raw RGB length mismatch: preview {}, export {}",
            preview.rgb24.len(),
            export.rgb24.len()
        )));
    }

    let mut deltas = preview
        .rgb24
        .iter()
        .zip(&export.rgb24)
        .map(|(left, right)| left.abs_diff(*right))
        .collect::<Vec<_>>();
    let sum = deltas.iter().map(|delta| u64::from(*delta)).sum::<u64>();
    deltas.sort_unstable();
    let mean_rgb_delta = sum as f64 / deltas.len() as f64;
    let p99_index = ((deltas.len() as f64 * 0.99).ceil() as usize)
        .saturating_sub(1)
        .min(deltas.len().saturating_sub(1));
    let p99_rgb_delta = deltas[p99_index];

    if mean_rgb_delta > tolerance.mean_rgb_delta_max || p99_rgb_delta > tolerance.p99_rgb_delta_max
    {
        return Err(RenderCompareError::assertion(format!(
            "preview/export RGB delta exceeded tolerance: mean {:.2} <= {:.2}, p99 {} <= {}",
            mean_rgb_delta,
            tolerance.mean_rgb_delta_max,
            p99_rgb_delta,
            tolerance.p99_rgb_delta_max
        )));
    }

    Ok(FrameComparison {
        width: preview.metadata.width,
        height: preview.metadata.height,
        mean_rgb_delta,
        p99_rgb_delta,
    })
}

fn run_ffmpeg_probe(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    args: &[&str],
) -> RenderCompareResult<String> {
    let args = args.iter().map(OsString::from).collect::<Vec<_>>();
    let output = executor.run(&runtime.ffmpeg.path, &args).map_err(|error| {
        RenderCompareError::Setup(RenderSetupError::new(
            RenderSetupErrorKind::FfmpegProbeFailed,
            format!("failed to launch FFmpeg capability probe: {error}"),
            "Check VE_FFMPEG_PATH or install FFmpeg on PATH.",
        ))
    })?;
    if !output.status.success() {
        return Err(RenderCompareError::Setup(RenderSetupError::new(
            RenderSetupErrorKind::FfmpegProbeFailed,
            format!(
                "FFmpeg capability probe returned a non-zero exit status: stdout=`{}` stderr=`{}`",
                bounded_summary(&output.stdout),
                bounded_summary(&output.stderr)
            ),
            "Use an FFmpeg build that supports -encoders and -filters probes.",
        )));
    }

    Ok(format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn resolved_text_font_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = env::var_os("VE_TEXT_FONT_PATH").map(PathBuf::from) {
        paths.push(path);
    }
    paths.extend([
        PathBuf::from("/System/Library/Fonts/PingFang.ttc"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
    ]);
    paths
}

fn probe_output_has_feature(output: &str, feature: &str) -> bool {
    output.lines().any(|line| {
        line.split_whitespace()
            .any(|field| field == feature || field == format!("{feature},"))
    })
}

fn parse_frame_metadata(bytes: &[u8]) -> RenderCompareResult<Vec<FrameMetadata>> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let stream = value
        .get("streams")
        .and_then(serde_json::Value::as_array)
        .and_then(|streams| streams.first())
        .ok_or_else(|| {
            RenderCompareError::runtime("ffprobe JSON did not include a video stream")
        })?;
    let width = json_u32(stream, "width")?;
    let height = json_u32(stream, "height")?;
    let frames = value
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| RenderCompareError::runtime("ffprobe JSON did not include frames"))?;

    frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let timestamp = frame_timestamp(frame)?;
            Ok(FrameMetadata {
                width,
                height,
                frame_index: index as u64,
                timestamp_microseconds: timestamp,
            })
        })
        .collect()
}

fn frame_timestamp(frame: &serde_json::Value) -> RenderCompareResult<u64> {
    for key in ["best_effort_timestamp_time", "pkt_pts_time", "pts_time"] {
        if let Some(value) = frame.get(key).and_then(serde_json::Value::as_str) {
            return parse_decimal_seconds_to_microseconds(value);
        }
    }
    Err(RenderCompareError::runtime(
        "ffprobe frame did not include timestamp metadata",
    ))
}

fn json_u32(value: &serde_json::Value, key: &str) -> RenderCompareResult<u32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| RenderCompareError::runtime(format!("ffprobe did not report numeric {key}")))
}

fn parse_decimal_seconds_to_microseconds(value: &str) -> RenderCompareResult<u64> {
    let (whole, fractional) = value
        .split_once('.')
        .map_or((value, ""), |(whole, fractional)| (whole, fractional));
    let whole = whole.parse::<u64>().map_err(|error| {
        RenderCompareError::runtime(format!("invalid seconds `{value}`: {error}"))
    })?;
    let mut micros = fractional.chars().take(6).collect::<String>();
    while micros.len() < 6 {
        micros.push('0');
    }
    let micros = if micros.is_empty() {
        0
    } else {
        micros.parse::<u64>().map_err(|error| {
            RenderCompareError::runtime(format!("invalid fractional seconds `{value}`: {error}"))
        })?
    };
    whole
        .checked_mul(1_000_000)
        .and_then(|value| value.checked_add(micros))
        .ok_or_else(|| RenderCompareError::runtime("timestamp overflow"))
}

fn format_seconds(value: u64) -> String {
    let whole = value / 1_000_000;
    let micros = value % 1_000_000;
    format!("{whole}.{micros:06}")
}

fn frame_byte_len(width: u32, height: u32) -> RenderCompareResult<usize> {
    let pixels = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or_else(|| RenderCompareError::runtime("RGB frame dimensions overflowed"))?;
    Ok(pixels)
}

fn bounded_summary(bytes: &[u8]) -> String {
    let limit = MAX_STDERR_SUMMARY_BYTES.min(bytes.len());
    String::from_utf8_lossy(&bytes[..limit]).into_owned()
}
