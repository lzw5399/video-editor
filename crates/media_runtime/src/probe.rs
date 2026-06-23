use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{FfmpegExecutor, MAX_STDERR_SUMMARY_BYTES, RuntimeConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaterialProbeKind {
    Video,
    Image,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaterialProbeStatus {
    Queued,
    Running,
    Probed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RationalFrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialProbeAudio {
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialProbeMetadata {
    pub status: MaterialProbeStatus,
    pub kind: MaterialProbeKind,
    pub duration_microseconds: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<RationalFrameRate>,
    pub has_video_stream: bool,
    pub has_audio_stream: bool,
    pub audio: Option<MaterialProbeAudio>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaterialProbeErrorKind {
    MissingInput,
    RuntimeUnavailable,
    ProcessLaunchFailed,
    Timeout,
    ProbeFailed,
    MalformedJson,
    MissingStreams,
    InvalidDuration,
    InvalidFrameRate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialProbeError {
    pub kind: MaterialProbeErrorKind,
    pub path: PathBuf,
    pub ffprobe_path: PathBuf,
    pub executor: String,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
    pub message: String,
}

impl MaterialProbeError {
    fn new(
        kind: MaterialProbeErrorKind,
        path: &Path,
        runtime: &RuntimeConfig,
        executor: &impl FfmpegExecutor,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            path: path.to_path_buf(),
            ffprobe_path: runtime.ffprobe.path.clone(),
            executor: executor.executor_name().to_string(),
            stdout_summary: None,
            stderr_summary: None,
            message: message.into(),
        }
    }

    fn with_output(mut self, stdout: &[u8], stderr: &[u8]) -> Self {
        self.stdout_summary = optional_summary(stdout);
        self.stderr_summary = optional_summary(stderr);
        self
    }

    fn with_stderr(mut self, stderr: impl AsRef<[u8]>) -> Self {
        self.stderr_summary = optional_summary(stderr.as_ref());
        self
    }
}

impl fmt::Display for MaterialProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "material probe failed: {}", self.message)
    }
}

impl std::error::Error for MaterialProbeError {}

pub fn probe_material_metadata(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    path: impl AsRef<Path>,
) -> Result<MaterialProbeMetadata, MaterialProbeError> {
    let path = path.as_ref();

    if !path.is_file() {
        return Err(MaterialProbeError::new(
            MaterialProbeErrorKind::MissingInput,
            path,
            runtime,
            executor,
            format!(
                "material path does not exist or is not a file: {}",
                path.display()
            ),
        ));
    }

    if !executor.can_execute(&runtime.ffprobe.path) {
        return Err(MaterialProbeError::new(
            MaterialProbeErrorKind::RuntimeUnavailable,
            path,
            runtime,
            executor,
            format!(
                "{} cannot execute ffprobe at {}",
                executor.executor_name(),
                runtime.ffprobe.path.display()
            ),
        ));
    }

    let args = vec![
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-print_format"),
        OsString::from("json"),
        OsString::from("-show_entries"),
        OsString::from(
            "stream=codec_type,codec_name,width,height,r_frame_rate,avg_frame_rate,duration,sample_rate,channels:format=duration",
        ),
        path.as_os_str().to_owned(),
    ];

    let output = executor
        .run(&runtime.ffprobe.path, &args)
        .map_err(|error| process_error(error, path, runtime, executor))?;

    if !output.status.success() {
        return Err(MaterialProbeError::new(
            MaterialProbeErrorKind::ProbeFailed,
            path,
            runtime,
            executor,
            format!("ffprobe failed for {}", path.display()),
        )
        .with_output(&output.stdout, &output.stderr));
    }

    let parsed = serde_json::from_slice::<FfprobeOutput>(&output.stdout).map_err(|error| {
        MaterialProbeError::new(
            MaterialProbeErrorKind::MalformedJson,
            path,
            runtime,
            executor,
            format!("ffprobe returned malformed JSON: {error}"),
        )
        .with_output(&output.stdout, &output.stderr)
    })?;

    normalize_probe_output(parsed, path, runtime, executor)
}

pub fn run_scheduled_material_probe(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    path: impl AsRef<Path>,
) -> Result<MaterialProbeMetadata, MaterialProbeError> {
    probe_material_metadata(executor, runtime, path)
}

fn process_error(
    error: io::Error,
    path: &Path,
    runtime: &RuntimeConfig,
    executor: &impl FfmpegExecutor,
) -> MaterialProbeError {
    let kind = if error.kind() == io::ErrorKind::TimedOut {
        MaterialProbeErrorKind::Timeout
    } else {
        MaterialProbeErrorKind::ProcessLaunchFailed
    };

    MaterialProbeError::new(
        kind,
        path,
        runtime,
        executor,
        format!("failed to run ffprobe: {error}"),
    )
    .with_stderr(error.to_string())
}

fn normalize_probe_output(
    parsed: FfprobeOutput,
    path: &Path,
    runtime: &RuntimeConfig,
    executor: &impl FfmpegExecutor,
) -> Result<MaterialProbeMetadata, MaterialProbeError> {
    let video_stream = parsed
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"));
    let audio_stream = parsed
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"));

    if video_stream.is_none() && audio_stream.is_none() {
        return Err(MaterialProbeError::new(
            MaterialProbeErrorKind::MissingStreams,
            path,
            runtime,
            executor,
            "ffprobe reported no video or audio streams",
        ));
    }

    let duration_microseconds = parsed
        .format
        .as_ref()
        .and_then(|format| format.duration.as_deref())
        .or_else(|| video_stream.and_then(|stream| stream.duration.as_deref()))
        .or_else(|| audio_stream.and_then(|stream| stream.duration.as_deref()))
        .map(parse_decimal_seconds_to_microseconds)
        .transpose()
        .map_err(|message| {
            MaterialProbeError::new(
                MaterialProbeErrorKind::InvalidDuration,
                path,
                runtime,
                executor,
                message,
            )
        })?;

    let frame_rate = video_stream
        .and_then(|stream| {
            preferred_rate(stream.r_frame_rate.as_deref())
                .or_else(|| preferred_rate(stream.avg_frame_rate.as_deref()))
        })
        .map(parse_rational_frame_rate)
        .transpose()
        .map_err(|message| {
            MaterialProbeError::new(
                MaterialProbeErrorKind::InvalidFrameRate,
                path,
                runtime,
                executor,
                message,
            )
        })?;

    let width = video_stream.and_then(|stream| stream.width);
    let height = video_stream.and_then(|stream| stream.height);
    let audio = audio_stream.and_then(|stream| {
        let sample_rate = stream
            .sample_rate
            .as_deref()
            .and_then(|value| value.parse::<u32>().ok())?;
        let channels = stream
            .channels
            .and_then(|value| u16::try_from(value).ok())?;
        Some(MaterialProbeAudio {
            sample_rate,
            channels,
        })
    });

    let kind = match (
        video_stream.is_some(),
        audio_stream.is_some(),
        duration_microseconds,
    ) {
        (true, _, Some(_)) => MaterialProbeKind::Video,
        (true, _, None) => MaterialProbeKind::Image,
        (false, true, _) => MaterialProbeKind::Audio,
        (false, false, _) => unreachable!("missing streams handled above"),
    };

    Ok(MaterialProbeMetadata {
        status: MaterialProbeStatus::Probed,
        kind,
        duration_microseconds,
        width,
        height,
        frame_rate,
        has_video_stream: video_stream.is_some(),
        has_audio_stream: audio_stream.is_some(),
        audio,
    })
}

fn preferred_rate(value: Option<&str>) -> Option<&str> {
    value.filter(|rate| !rate.is_empty() && *rate != "0/0")
}

fn parse_decimal_seconds_to_microseconds(value: &str) -> Result<u64, String> {
    let (whole, fractional) = value
        .split_once('.')
        .map_or((value, ""), |(whole, fractional)| (whole, fractional));
    let whole_micros = whole
        .parse::<u64>()
        .map_err(|error| format!("invalid duration seconds `{value}`: {error}"))?
        .checked_mul(1_000_000)
        .ok_or_else(|| format!("duration is too large `{value}`"))?;
    if !fractional.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("invalid duration fraction `{value}`"));
    }

    let mut fraction = fractional.chars().take(6).collect::<String>();

    while fraction.len() < 6 {
        fraction.push('0');
    }

    let fraction_micros = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<u64>()
            .map_err(|error| format!("invalid duration fraction `{value}`: {error}"))?
    };

    whole_micros
        .checked_add(fraction_micros)
        .ok_or_else(|| format!("duration is too large `{value}`"))
}

fn parse_rational_frame_rate(value: &str) -> Result<RationalFrameRate, String> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or_else(|| format!("invalid frame rate `{value}`"))?;
    let numerator = numerator
        .parse::<u32>()
        .map_err(|error| format!("invalid frame rate numerator `{value}`: {error}"))?;
    let denominator = denominator
        .parse::<u32>()
        .map_err(|error| format!("invalid frame rate denominator `{value}`: {error}"))?;

    if numerator == 0 {
        return Err("frame rate numerator cannot be zero".to_string());
    }

    if denominator == 0 {
        return Err("frame rate denominator cannot be zero".to_string());
    }

    Ok(RationalFrameRate {
        numerator,
        denominator,
    })
}

fn optional_summary(bytes: &[u8]) -> Option<String> {
    let summary = bounded_summary(bytes);
    if summary.is_empty() {
        None
    } else {
        Some(summary)
    }
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

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    duration: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
}
