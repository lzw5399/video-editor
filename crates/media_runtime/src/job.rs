use std::ffi::OsString;
use std::fmt;
use std::io::{self, BufRead, Read};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::MAX_STDERR_SUMMARY_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegJobId(pub String);

impl FfmpegJobId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegRuntimeJob {
    pub job_id: FfmpegJobId,
    pub ffmpeg_path: PathBuf,
    pub args: Vec<OsString>,
    pub output_path: PathBuf,
    pub expected_duration_microseconds: Option<u64>,
    pub timeout: Duration,
}

impl FfmpegRuntimeJob {
    pub fn new(
        job_id: impl Into<String>,
        ffmpeg_path: impl Into<PathBuf>,
        args: Vec<OsString>,
        output_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            job_id: FfmpegJobId::new(job_id),
            ffmpeg_path: ffmpeg_path.into(),
            args,
            output_path: output_path.into(),
            expected_duration_microseconds: None,
            timeout: Duration::from_secs(60 * 30),
        }
    }

    pub fn with_expected_duration_microseconds(mut self, duration: u64) -> Self {
        self.expected_duration_microseconds = Some(duration);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FfmpegJobState {
    Started,
    Running,
    Completed,
    Cancelled,
    TimedOut,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegProgress {
    pub out_time_microseconds: u64,
    pub expected_duration_microseconds: Option<u64>,
    pub progress_per_mille: Option<u16>,
}

impl FfmpegProgress {
    fn new(out_time_microseconds: u64, expected_duration_microseconds: Option<u64>) -> Self {
        let progress_per_mille = expected_duration_microseconds
            .filter(|duration| *duration > 0)
            .map(|duration| {
                let per_mille = out_time_microseconds.saturating_mul(1000) / duration;
                u16::try_from(per_mille.min(1000)).unwrap_or(1000)
            });
        Self {
            out_time_microseconds,
            expected_duration_microseconds,
            progress_per_mille,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum FfmpegJobEvent {
    Started { job_id: FfmpegJobId },
    Progress { progress: FfmpegProgress },
    Completed { state: FfmpegJobState },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegJobResult {
    pub job_id: FfmpegJobId,
    pub state: FfmpegJobState,
    pub output_path: PathBuf,
    pub final_progress: Option<FfmpegProgress>,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FfmpegRuntimeErrorKind {
    RuntimeUnavailable,
    ProcessLaunchFailed,
    Timeout,
    NonZeroExit,
    MissingEncoder,
    MissingFilter,
    MalformedProgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegRuntimeError {
    pub kind: FfmpegRuntimeErrorKind,
    pub job_id: FfmpegJobId,
    pub ffmpeg_path: PathBuf,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
    pub exit_code: Option<i32>,
    pub message: String,
}

impl FfmpegRuntimeError {
    fn new(
        kind: FfmpegRuntimeErrorKind,
        job: &FfmpegRuntimeJob,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            job_id: job.job_id.clone(),
            ffmpeg_path: job.ffmpeg_path.clone(),
            stdout_summary: None,
            stderr_summary: None,
            exit_code: None,
            message: message.into(),
        }
    }

    fn with_output(mut self, stdout: &[u8], stderr: &[u8], status: Option<ExitStatus>) -> Self {
        self.stdout_summary = optional_summary(stdout);
        self.stderr_summary = optional_summary(stderr);
        self.exit_code = status.and_then(|status| status.code());
        if matches!(self.kind, FfmpegRuntimeErrorKind::NonZeroExit) {
            self.kind = classify_non_zero_stderr(stderr);
        }
        self
    }
}

impl fmt::Display for FfmpegRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FFmpeg runtime job failed: {}", self.message)
    }
}

impl std::error::Error for FfmpegRuntimeError {}

#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub fn parse_progress_lines(
    lines: &str,
    expected_duration_microseconds: Option<u64>,
) -> Result<Vec<FfmpegProgress>, String> {
    let mut progress = Vec::new();
    for line in lines.lines() {
        if let Some(item) = parse_progress_line(line, expected_duration_microseconds)? {
            progress.push(item);
        }
    }
    Ok(progress)
}

pub fn run_export_job<F>(
    job: &FfmpegRuntimeJob,
    cancel_token: &CancelToken,
    mut on_event: F,
) -> Result<FfmpegJobResult, FfmpegRuntimeError>
where
    F: FnMut(FfmpegJobEvent),
{
    if !job.ffmpeg_path.is_file() {
        return Err(FfmpegRuntimeError::new(
            FfmpegRuntimeErrorKind::RuntimeUnavailable,
            job,
            format!("FFmpeg binary is not a file: {}", job.ffmpeg_path.display()),
        ));
    }

    let mut child = Command::new(&job.ffmpeg_path)
        .args(progress_args(&job.args))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            FfmpegRuntimeError::new(
                FfmpegRuntimeErrorKind::ProcessLaunchFailed,
                job,
                format!("failed to launch FFmpeg: {error}"),
            )
            .with_output(&[], error.to_string().as_bytes(), None)
        })?;

    let stdout = child.stdout.take().expect("stdout pipe configured");
    let stderr = child.stderr.take().expect("stderr pipe configured");
    let expected_duration = job.expected_duration_microseconds;
    let (progress_tx, progress_rx) = mpsc::channel();
    let stdout_handle =
        thread::spawn(move || read_progress(stdout, expected_duration, progress_tx));
    let stderr_handle = thread::spawn(move || read_all(stderr));
    let started = Instant::now();
    let mut stdout_bytes = Vec::new();
    let mut final_progress = None;

    on_event(FfmpegJobEvent::Started {
        job_id: job.job_id.clone(),
    });

    loop {
        loop {
            match progress_rx.try_recv() {
                Ok(ProgressMessage::Bytes(bytes)) => stdout_bytes.extend(bytes),
                Ok(ProgressMessage::Progress(progress)) => {
                    final_progress = Some(progress);
                    on_event(FfmpegJobEvent::Progress { progress });
                }
                Ok(ProgressMessage::Malformed(message)) => {
                    let _ = child.kill();
                    let status = child.wait().ok();
                    let stderr_bytes = join_bytes(stderr_handle);
                    stdout_bytes.extend(join_progress(stdout_handle).unwrap_or_default());
                    return Err(FfmpegRuntimeError::new(
                        FfmpegRuntimeErrorKind::MalformedProgress,
                        job,
                        message,
                    )
                    .with_output(&stdout_bytes, &stderr_bytes, status));
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }

        if cancel_token.is_cancelled() {
            let _ = child.kill();
            let status = child.wait().ok();
            let joined_stdout = join_progress(stdout_handle).unwrap_or_default();
            if final_progress.is_none() {
                if let Ok(progress_items) = parse_progress_lines(
                    &String::from_utf8_lossy(&joined_stdout),
                    expected_duration,
                ) {
                    for progress in progress_items {
                        final_progress = Some(progress);
                        on_event(FfmpegJobEvent::Progress { progress });
                    }
                }
            }
            stdout_bytes.extend(joined_stdout);
            let stderr_bytes = join_bytes(stderr_handle);
            let result = FfmpegJobResult {
                job_id: job.job_id.clone(),
                state: FfmpegJobState::Cancelled,
                output_path: job.output_path.clone(),
                final_progress,
                stdout_summary: optional_summary(&stdout_bytes),
                stderr_summary: optional_summary(&stderr_bytes),
                exit_code: status.and_then(|status| status.code()),
            };
            on_event(FfmpegJobEvent::Completed {
                state: FfmpegJobState::Cancelled,
            });
            return Ok(result);
        }

        if started.elapsed() >= job.timeout {
            let _ = child.kill();
            let status = child.wait().ok();
            stdout_bytes.extend(join_progress(stdout_handle).unwrap_or_default());
            let stderr_bytes = join_bytes(stderr_handle);
            return Err(FfmpegRuntimeError::new(
                FfmpegRuntimeErrorKind::Timeout,
                job,
                format!("FFmpeg timed out after {} ms", job.timeout.as_millis()),
            )
            .with_output(&stdout_bytes, &stderr_bytes, status));
        }

        if let Some(status) = child.try_wait().map_err(|error| {
            FfmpegRuntimeError::new(
                FfmpegRuntimeErrorKind::ProcessLaunchFailed,
                job,
                format!("failed to wait for FFmpeg: {error}"),
            )
        })? {
            stdout_bytes.extend(join_progress(stdout_handle).unwrap_or_default());
            let stderr_bytes = join_bytes(stderr_handle);
            if status.success() {
                let result = FfmpegJobResult {
                    job_id: job.job_id.clone(),
                    state: FfmpegJobState::Completed,
                    output_path: job.output_path.clone(),
                    final_progress,
                    stdout_summary: optional_summary(&stdout_bytes),
                    stderr_summary: optional_summary(&stderr_bytes),
                    exit_code: status.code(),
                };
                on_event(FfmpegJobEvent::Completed {
                    state: FfmpegJobState::Completed,
                });
                return Ok(result);
            }
            return Err(FfmpegRuntimeError::new(
                FfmpegRuntimeErrorKind::NonZeroExit,
                job,
                "FFmpeg returned a non-zero exit status",
            )
            .with_output(&stdout_bytes, &stderr_bytes, Some(status)));
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn progress_args(args: &[OsString]) -> Vec<OsString> {
    let mut with_progress = vec![
        OsString::from("-hide_banner"),
        OsString::from("-nostats"),
        OsString::from("-progress"),
        OsString::from("pipe:1"),
    ];
    with_progress.extend(args.iter().cloned());
    with_progress
}

enum ProgressMessage {
    Bytes(Vec<u8>),
    Progress(FfmpegProgress),
    Malformed(String),
}

fn read_progress<R: Read + Send + 'static>(
    reader: R,
    expected_duration_microseconds: Option<u64>,
    tx: mpsc::Sender<ProgressMessage>,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    let reader = io::BufReader::new(reader);
    for line in reader.split(b'\n') {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                let _ = tx.send(ProgressMessage::Malformed(format!(
                    "failed to read FFmpeg progress: {error}"
                )));
                break;
            }
        };
        bytes.extend(&line);
        bytes.push(b'\n');
        let _ = tx.send(ProgressMessage::Bytes({
            let mut owned = line.clone();
            owned.push(b'\n');
            owned
        }));
        let text = String::from_utf8_lossy(&line);
        match parse_progress_line(text.trim(), expected_duration_microseconds) {
            Ok(Some(progress)) => {
                let _ = tx.send(ProgressMessage::Progress(progress));
            }
            Ok(None) => {}
            Err(message) => {
                let _ = tx.send(ProgressMessage::Malformed(message));
                break;
            }
        }
    }
    bytes
}

fn parse_progress_line(
    line: &str,
    expected_duration_microseconds: Option<u64>,
) -> Result<Option<FfmpegProgress>, String> {
    let Some((key, value)) = line.split_once('=') else {
        return Ok(None);
    };

    match key {
        "out_time_us" | "out_time_ms" => {
            let value = value.trim();
            if is_unavailable_progress_timestamp(value) {
                return Ok(None);
            }
            let micros = value
                .parse::<u64>()
                .map_err(|error| format!("malformed FFmpeg progress `{line}`: {error}"))?;
            Ok(Some(FfmpegProgress::new(
                micros,
                expected_duration_microseconds,
            )))
        }
        "out_time" => {
            let value = value.trim();
            if is_unavailable_progress_timestamp(value) {
                return Ok(None);
            }
            let micros = parse_hhmmss_microseconds(value)
                .map_err(|message| format!("malformed FFmpeg progress `{line}`: {message}"))?;
            Ok(Some(FfmpegProgress::new(
                micros,
                expected_duration_microseconds,
            )))
        }
        _ => Ok(None),
    }
}

fn is_unavailable_progress_timestamp(value: &str) -> bool {
    value.eq_ignore_ascii_case("N/A")
}

fn parse_hhmmss_microseconds(value: &str) -> Result<u64, String> {
    let mut pieces = value.split(':');
    let hours = pieces
        .next()
        .ok_or_else(|| "missing hours".to_string())?
        .parse::<u64>()
        .map_err(|error| format!("invalid hours: {error}"))?;
    let minutes = pieces
        .next()
        .ok_or_else(|| "missing minutes".to_string())?
        .parse::<u64>()
        .map_err(|error| format!("invalid minutes: {error}"))?;
    let seconds = pieces.next().ok_or_else(|| "missing seconds".to_string())?;
    if pieces.next().is_some() {
        return Err("too many time components".to_string());
    }
    let (whole_seconds, fractional) = seconds
        .split_once('.')
        .map_or((seconds, ""), |(whole, fractional)| (whole, fractional));
    let whole_seconds = whole_seconds
        .parse::<u64>()
        .map_err(|error| format!("invalid seconds: {error}"))?;
    if minutes >= 60 || whole_seconds >= 60 {
        return Err("minutes and seconds must be below 60".to_string());
    }
    if !fractional.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("invalid fractional seconds".to_string());
    }
    let mut fraction = fractional.chars().take(6).collect::<String>();
    while fraction.len() < 6 {
        fraction.push('0');
    }
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<u64>()
            .map_err(|error| format!("invalid fraction: {error}"))?
    };
    hours
        .checked_mul(3_600_000_000)
        .and_then(|value| value.checked_add(minutes * 60_000_000))
        .and_then(|value| value.checked_add(whole_seconds * 1_000_000))
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| "progress timestamp overflow".to_string())
}

fn read_all<R: Read>(mut reader: R) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = reader.read_to_end(&mut bytes);
    bytes
}

fn join_progress(handle: thread::JoinHandle<Vec<u8>>) -> Option<Vec<u8>> {
    handle.join().ok()
}

fn join_bytes(handle: thread::JoinHandle<Vec<u8>>) -> Vec<u8> {
    handle.join().unwrap_or_default()
}

fn classify_non_zero_stderr(stderr: &[u8]) -> FfmpegRuntimeErrorKind {
    let stderr = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    if stderr.contains("unknown encoder")
        || stderr.contains("encoder not found")
        || stderr.contains("cannot find encoder")
    {
        FfmpegRuntimeErrorKind::MissingEncoder
    } else if stderr.contains("no such filter")
        || stderr.contains("filter not found")
        || stderr.contains("unknown filter")
    {
        FfmpegRuntimeErrorKind::MissingFilter
    } else {
        FfmpegRuntimeErrorKind::NonZeroExit
    }
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
