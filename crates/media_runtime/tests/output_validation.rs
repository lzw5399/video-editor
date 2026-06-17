use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

use media_runtime::{
    BinaryKind, DiscoveredBinary, DiscoverySource, FfmpegExecutor, OutputValidationErrorKind,
    OutputValidationExpectation, RationalFrameRate, RuntimeConfig, validate_rendered_output,
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[test]
fn output_validation_checks_missing_and_empty_file_before_ffprobe() {
    let temp = tempfile::tempdir().expect("temp dir");
    let runtime = fake_runtime(temp.path().join("ffprobe"));
    let executor = FakeExecutor::successful(video_probe_json());

    let missing = temp.path().join("missing.mp4");
    let missing_error = validate_rendered_output(
        &executor,
        &runtime,
        &missing,
        &OutputValidationExpectation::new(),
    )
    .expect_err("missing output should fail");
    assert_eq!(missing_error.kind, OutputValidationErrorKind::MissingOutput);

    let empty = temp.path().join("empty.mp4");
    fs::write(&empty, []).expect("empty output should write");
    let empty_error = validate_rendered_output(
        &executor,
        &runtime,
        &empty,
        &OutputValidationExpectation::new(),
    )
    .expect_err("empty output should fail");
    assert_eq!(empty_error.kind, OutputValidationErrorKind::EmptyOutput);
}

#[test]
fn output_validation_accepts_expected_duration_fps_resolution_and_audio() {
    let temp = tempfile::tempdir().expect("temp dir");
    let output = temp.path().join("out.mp4");
    fs::write(&output, b"not-empty").expect("output should write");
    let runtime = fake_runtime(temp.path().join("ffprobe"));
    let executor = FakeExecutor::successful(video_probe_json());
    let expectation = OutputValidationExpectation::new()
        .with_expected_duration_microseconds(1_000_000, 1_000)
        .with_expected_frame_rate(RationalFrameRate {
            numerator: 30,
            denominator: 1,
        })
        .with_expected_dimensions(1920, 1080)
        .with_audio_stream(true);

    let report = validate_rendered_output(&executor, &runtime, &output, &expectation)
        .expect("matching metadata should validate");

    assert_eq!(report.file_size_bytes, 9);
    assert_eq!(report.metadata.width, Some(1920));
    assert!(report.metadata.has_audio_stream);
}

#[test]
fn output_validation_classifies_duration_fps_resolution_and_audio_mismatches() {
    let temp = tempfile::tempdir().expect("temp dir");
    let output = temp.path().join("out.mp4");
    fs::write(&output, b"not-empty").expect("output should write");
    let runtime = fake_runtime(temp.path().join("ffprobe"));
    let executor = FakeExecutor::successful(video_probe_json());

    let duration_error = validate_rendered_output(
        &executor,
        &runtime,
        &output,
        &OutputValidationExpectation::new().with_expected_duration_microseconds(2_000_000, 1_000),
    )
    .expect_err("duration mismatch should fail");
    assert_eq!(
        duration_error.kind,
        OutputValidationErrorKind::DurationMismatch
    );

    let fps_error = validate_rendered_output(
        &executor,
        &runtime,
        &output,
        &OutputValidationExpectation::new().with_expected_frame_rate(RationalFrameRate {
            numerator: 24,
            denominator: 1,
        }),
    )
    .expect_err("fps mismatch should fail");
    assert_eq!(fps_error.kind, OutputValidationErrorKind::FrameRateMismatch);

    let resolution_error = validate_rendered_output(
        &executor,
        &runtime,
        &output,
        &OutputValidationExpectation::new().with_expected_dimensions(1280, 720),
    )
    .expect_err("resolution mismatch should fail");
    assert_eq!(
        resolution_error.kind,
        OutputValidationErrorKind::ResolutionMismatch
    );

    let audio_error = validate_rendered_output(
        &executor,
        &runtime,
        &output,
        &OutputValidationExpectation::new().with_audio_stream(false),
    )
    .expect_err("unexpected audio should fail");
    assert_eq!(
        audio_error.kind,
        OutputValidationErrorKind::UnexpectedAudioStream
    );
}

#[test]
fn output_validation_classifies_malformed_json_missing_streams_timeout_and_process_failure() {
    let temp = tempfile::tempdir().expect("temp dir");
    let output = temp.path().join("out.mp4");
    fs::write(&output, b"not-empty").expect("output should write");
    let runtime = fake_runtime(temp.path().join("ffprobe"));

    let malformed = validate_rendered_output(
        &FakeExecutor::successful(b"not-json".to_vec()),
        &runtime,
        &output,
        &OutputValidationExpectation::new(),
    )
    .expect_err("malformed JSON should fail");
    assert_eq!(
        malformed.kind,
        OutputValidationErrorKind::MalformedProbeJson
    );

    let missing_streams = validate_rendered_output(
        &FakeExecutor::successful(br#"{"streams":[]}"#.to_vec()),
        &runtime,
        &output,
        &OutputValidationExpectation::new(),
    )
    .expect_err("missing streams should fail");
    assert_eq!(
        missing_streams.kind,
        OutputValidationErrorKind::MissingStreams
    );

    let timeout = validate_rendered_output(
        &FakeExecutor::timeout(),
        &runtime,
        &output,
        &OutputValidationExpectation::new(),
    )
    .expect_err("timeout should fail");
    assert_eq!(timeout.kind, OutputValidationErrorKind::Timeout);

    let failed = validate_rendered_output(
        &FakeExecutor::failed("probe failed"),
        &runtime,
        &output,
        &OutputValidationExpectation::new(),
    )
    .expect_err("process failure should fail");
    assert_eq!(failed.kind, OutputValidationErrorKind::ProbeFailed);
}

#[derive(Clone)]
enum FakeBehavior {
    Successful(Vec<u8>),
    Failed(String),
    Timeout,
}

struct FakeExecutor {
    behavior: FakeBehavior,
}

impl FakeExecutor {
    fn successful(stdout: Vec<u8>) -> Self {
        Self {
            behavior: FakeBehavior::Successful(stdout),
        }
    }

    fn failed(stderr: &str) -> Self {
        Self {
            behavior: FakeBehavior::Failed(stderr.to_owned()),
        }
    }

    fn timeout() -> Self {
        Self {
            behavior: FakeBehavior::Timeout,
        }
    }
}

impl FfmpegExecutor for FakeExecutor {
    fn executor_name(&self) -> &'static str {
        "fake-output-validation-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, binary: &Path) -> io::Result<Output> {
        self.run(binary, &[])
    }

    fn run(&self, _binary: &Path, _args: &[OsString]) -> io::Result<Output> {
        match &self.behavior {
            FakeBehavior::Successful(stdout) => Ok(Output {
                status: success_status(),
                stdout: stdout.clone(),
                stderr: Vec::new(),
            }),
            FakeBehavior::Failed(stderr) => Ok(Output {
                status: failure_status(),
                stdout: Vec::new(),
                stderr: stderr.as_bytes().to_vec(),
            }),
            FakeBehavior::Timeout => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "ffprobe timed out after 100 ms",
            )),
        }
    }
}

fn video_probe_json() -> Vec<u8> {
    br#"{
      "streams": [
        {
          "codec_type": "video",
          "width": 1920,
          "height": 1080,
          "r_frame_rate": "30/1",
          "duration": "1.000000"
        },
        {
          "codec_type": "audio",
          "sample_rate": "48000",
          "channels": 2,
          "duration": "1.000000"
        }
      ],
      "format": { "duration": "1.000000" }
    }"#
    .to_vec()
}

fn fake_runtime(ffprobe_path: PathBuf) -> RuntimeConfig {
    RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: ffprobe_path.with_file_name("ffmpeg"),
            source: DiscoverySource::Path,
            version: "ffmpeg version fake".to_owned(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: ffprobe_path,
            source: DiscoverySource::Path,
            version: "ffprobe version fake".to_owned(),
        },
    }
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}

#[cfg(unix)]
fn failure_status() -> ExitStatus {
    ExitStatus::from_raw(1 << 8)
}
