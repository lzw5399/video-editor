use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    FfmpegExecutor, MaterialProbeError, MaterialProbeErrorKind, MaterialProbeMetadata,
    RationalFrameRate, RuntimeConfig, probe_material_metadata,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputValidationExpectation {
    pub expected_duration_microseconds: Option<u64>,
    pub duration_tolerance_microseconds: u64,
    pub expected_frame_rate: Option<RationalFrameRate>,
    pub expected_width: Option<u32>,
    pub expected_height: Option<u32>,
    pub expect_audio_stream: Option<bool>,
}

impl Default for OutputValidationExpectation {
    fn default() -> Self {
        Self {
            expected_duration_microseconds: None,
            duration_tolerance_microseconds: 33_334,
            expected_frame_rate: None,
            expected_width: None,
            expected_height: None,
            expect_audio_stream: None,
        }
    }
}

impl OutputValidationExpectation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_expected_duration_microseconds(mut self, value: u64, tolerance: u64) -> Self {
        self.expected_duration_microseconds = Some(value);
        self.duration_tolerance_microseconds = tolerance;
        self
    }

    pub fn with_expected_frame_rate(mut self, value: RationalFrameRate) -> Self {
        self.expected_frame_rate = Some(value);
        self
    }

    pub fn with_expected_dimensions(mut self, width: u32, height: u32) -> Self {
        self.expected_width = Some(width);
        self.expected_height = Some(height);
        self
    }

    pub fn with_audio_stream(mut self, expected: bool) -> Self {
        self.expect_audio_stream = Some(expected);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputValidationReport {
    pub path: PathBuf,
    pub file_size_bytes: u64,
    pub metadata: MaterialProbeMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputValidationErrorKind {
    MissingOutput,
    EmptyOutput,
    RuntimeUnavailable,
    ProcessLaunchFailed,
    Timeout,
    ProbeFailed,
    MalformedProbeJson,
    MissingStreams,
    MissingDuration,
    DurationMismatch,
    MissingFrameRate,
    FrameRateMismatch,
    ResolutionMismatch,
    MissingAudioStream,
    UnexpectedAudioStream,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputValidationError {
    pub kind: OutputValidationErrorKind,
    pub path: PathBuf,
    pub message: String,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
}

impl OutputValidationError {
    fn new(
        kind: OutputValidationErrorKind,
        path: impl AsRef<Path>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            path: path.as_ref().to_path_buf(),
            message: message.into(),
            stdout_summary: None,
            stderr_summary: None,
        }
    }

    fn with_probe(mut self, error: MaterialProbeError) -> Self {
        self.stdout_summary = error.stdout_summary;
        self.stderr_summary = error.stderr_summary;
        self
    }
}

impl fmt::Display for OutputValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "output validation failed: {}", self.message)
    }
}

impl std::error::Error for OutputValidationError {}

pub fn validate_rendered_output(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    path: impl AsRef<Path>,
    expectation: &OutputValidationExpectation,
) -> Result<OutputValidationReport, OutputValidationError> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|_| {
        OutputValidationError::new(
            OutputValidationErrorKind::MissingOutput,
            path,
            format!("rendered output does not exist: {}", path.display()),
        )
    })?;
    if !metadata.is_file() {
        return Err(OutputValidationError::new(
            OutputValidationErrorKind::MissingOutput,
            path,
            format!("rendered output is not a file: {}", path.display()),
        ));
    }
    if metadata.len() == 0 {
        return Err(OutputValidationError::new(
            OutputValidationErrorKind::EmptyOutput,
            path,
            format!("rendered output is empty: {}", path.display()),
        ));
    }

    let probe = probe_material_metadata(executor, runtime, path).map_err(|error| {
        OutputValidationError::new(
            map_probe_error(error.kind),
            path,
            format!("ffprobe validation failed: {}", error.message),
        )
        .with_probe(error)
    })?;

    validate_duration(path, &probe, expectation)?;
    validate_frame_rate(path, &probe, expectation)?;
    validate_dimensions(path, &probe, expectation)?;
    validate_audio(path, &probe, expectation)?;

    Ok(OutputValidationReport {
        path: path.to_path_buf(),
        file_size_bytes: metadata.len(),
        metadata: probe,
    })
}

fn validate_duration(
    path: &Path,
    metadata: &MaterialProbeMetadata,
    expectation: &OutputValidationExpectation,
) -> Result<(), OutputValidationError> {
    let Some(expected) = expectation.expected_duration_microseconds else {
        return Ok(());
    };
    let Some(actual) = metadata.duration_microseconds else {
        return Err(OutputValidationError::new(
            OutputValidationErrorKind::MissingDuration,
            path,
            "ffprobe did not report output duration",
        ));
    };
    let delta = actual.abs_diff(expected);
    if delta > expectation.duration_tolerance_microseconds {
        return Err(OutputValidationError::new(
            OutputValidationErrorKind::DurationMismatch,
            path,
            format!(
                "duration mismatch: expected {expected} us +/- {} us, got {actual} us",
                expectation.duration_tolerance_microseconds
            ),
        ));
    }
    Ok(())
}

fn validate_frame_rate(
    path: &Path,
    metadata: &MaterialProbeMetadata,
    expectation: &OutputValidationExpectation,
) -> Result<(), OutputValidationError> {
    let Some(expected) = expectation.expected_frame_rate else {
        return Ok(());
    };
    let Some(actual) = metadata.frame_rate else {
        return Err(OutputValidationError::new(
            OutputValidationErrorKind::MissingFrameRate,
            path,
            "ffprobe did not report output frame rate",
        ));
    };
    if u64::from(actual.numerator) * u64::from(expected.denominator)
        != u64::from(expected.numerator) * u64::from(actual.denominator)
    {
        return Err(OutputValidationError::new(
            OutputValidationErrorKind::FrameRateMismatch,
            path,
            format!(
                "frame rate mismatch: expected {}/{}, got {}/{}",
                expected.numerator, expected.denominator, actual.numerator, actual.denominator
            ),
        ));
    }
    Ok(())
}

fn validate_dimensions(
    path: &Path,
    metadata: &MaterialProbeMetadata,
    expectation: &OutputValidationExpectation,
) -> Result<(), OutputValidationError> {
    if let Some(expected_width) = expectation.expected_width {
        if metadata.width != Some(expected_width) {
            return Err(OutputValidationError::new(
                OutputValidationErrorKind::ResolutionMismatch,
                path,
                format!(
                    "width mismatch: expected {expected_width}, got {:?}",
                    metadata.width
                ),
            ));
        }
    }
    if let Some(expected_height) = expectation.expected_height {
        if metadata.height != Some(expected_height) {
            return Err(OutputValidationError::new(
                OutputValidationErrorKind::ResolutionMismatch,
                path,
                format!(
                    "height mismatch: expected {expected_height}, got {:?}",
                    metadata.height
                ),
            ));
        }
    }
    Ok(())
}

fn validate_audio(
    path: &Path,
    metadata: &MaterialProbeMetadata,
    expectation: &OutputValidationExpectation,
) -> Result<(), OutputValidationError> {
    match expectation.expect_audio_stream {
        Some(true) if !metadata.has_audio_stream => Err(OutputValidationError::new(
            OutputValidationErrorKind::MissingAudioStream,
            path,
            "expected an audio stream but ffprobe reported none",
        )),
        Some(false) if metadata.has_audio_stream => Err(OutputValidationError::new(
            OutputValidationErrorKind::UnexpectedAudioStream,
            path,
            "expected no audio stream but ffprobe reported one",
        )),
        _ => Ok(()),
    }
}

fn map_probe_error(kind: MaterialProbeErrorKind) -> OutputValidationErrorKind {
    match kind {
        MaterialProbeErrorKind::MissingInput => OutputValidationErrorKind::MissingOutput,
        MaterialProbeErrorKind::RuntimeUnavailable => OutputValidationErrorKind::RuntimeUnavailable,
        MaterialProbeErrorKind::ProcessLaunchFailed => {
            OutputValidationErrorKind::ProcessLaunchFailed
        }
        MaterialProbeErrorKind::Timeout => OutputValidationErrorKind::Timeout,
        MaterialProbeErrorKind::ProbeFailed => OutputValidationErrorKind::ProbeFailed,
        MaterialProbeErrorKind::MalformedJson => OutputValidationErrorKind::MalformedProbeJson,
        MaterialProbeErrorKind::MissingStreams => OutputValidationErrorKind::MissingStreams,
        MaterialProbeErrorKind::InvalidDuration => OutputValidationErrorKind::MissingDuration,
        MaterialProbeErrorKind::InvalidFrameRate => OutputValidationErrorKind::MissingFrameRate,
    }
}
