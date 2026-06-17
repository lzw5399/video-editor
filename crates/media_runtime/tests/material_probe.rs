use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

use media_runtime::{
    BinaryKind, DiscoveredBinary, DiscoverySource, FfmpegExecutor, MAX_STDERR_SUMMARY_BYTES,
    MaterialProbeErrorKind, RuntimeConfig, discover_runtime_config, probe_material_metadata,
};
use media_runtime_desktop::DesktopFfmpegExecutor;
use testkit::{
    generate_audio_material_fixture, generate_image_material_fixture,
    generate_video_material_fixture,
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[test]
fn material_probe_normalizes_generated_video_metadata() {
    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );
    let executor = DesktopFfmpegExecutor::default();
    let media = generate_video_material_fixture(&executor, &runtime)
        .expect("video fixture should generate");

    let metadata = probe_material_metadata(&executor, &runtime, media.path())
        .expect("generated video should probe");

    media
        .assert_probe_metadata(&metadata)
        .expect("generated video metadata should match fixture expectations");
}

#[test]
fn material_probe_normalizes_generated_image_metadata() {
    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );
    let executor = DesktopFfmpegExecutor::default();
    let media = generate_image_material_fixture(&executor, &runtime)
        .expect("image fixture should generate");

    let metadata = probe_material_metadata(&executor, &runtime, media.path())
        .expect("generated image should probe");

    media
        .assert_probe_metadata(&metadata)
        .expect("generated image metadata should match fixture expectations");
}

#[test]
fn material_probe_normalizes_generated_audio_metadata() {
    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );
    let executor = DesktopFfmpegExecutor::default();
    let media = generate_audio_material_fixture(&executor, &runtime)
        .expect("audio fixture should generate");

    let metadata = probe_material_metadata(&executor, &runtime, media.path())
        .expect("generated audio should probe");

    media
        .assert_probe_metadata(&metadata)
        .expect("generated audio metadata should match fixture expectations");
}

#[test]
fn material_probe_classifies_missing_and_corrupt_inputs() {
    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );
    let executor = DesktopFfmpegExecutor::default();
    let temp_dir = tempfile::tempdir().expect("temp dir should create");
    let missing = temp_dir.path().join("missing.mp4");

    let missing_error = probe_material_metadata(&executor, &runtime, &missing)
        .expect_err("missing input should fail before ffprobe");

    assert_eq!(missing_error.kind, MaterialProbeErrorKind::MissingInput);
    assert_eq!(missing_error.path, missing);

    let corrupt = temp_dir.path().join("corrupt.mp4");
    fs::write(&corrupt, b"not a media container").expect("corrupt fixture should write");
    let corrupt_error = probe_material_metadata(&executor, &runtime, &corrupt)
        .expect_err("corrupt input should fail with a classified probe error");

    assert_eq!(corrupt_error.kind, MaterialProbeErrorKind::ProbeFailed);
    assert!(corrupt_error.stderr_summary.is_some());
}

#[test]
fn material_probe_bounds_process_output_and_classifies_timeout() {
    let temp_dir = tempfile::tempdir().expect("temp dir should create");
    let input = temp_dir.path().join("input.mp4");
    fs::write(&input, b"placeholder").expect("input fixture should write");
    let runtime = fake_runtime(temp_dir.path().join("ffprobe"));
    let long_output = "x".repeat(MAX_STDERR_SUMMARY_BYTES + 512);

    let failing_executor = FakeExecutor::failed(long_output.clone(), long_output);
    let failed = probe_material_metadata(&failing_executor, &runtime, &input)
        .expect_err("failed probe should return bounded output");

    assert_eq!(failed.kind, MaterialProbeErrorKind::ProbeFailed);
    assert!(failed.stdout_summary.as_ref().unwrap().len() <= MAX_STDERR_SUMMARY_BYTES);
    assert!(failed.stderr_summary.as_ref().unwrap().len() <= MAX_STDERR_SUMMARY_BYTES);

    let timeout_executor = FakeExecutor::timeout();
    let timeout = probe_material_metadata(&timeout_executor, &runtime, &input)
        .expect_err("timeout should be classified");

    assert_eq!(timeout.kind, MaterialProbeErrorKind::Timeout);
    assert!(
        timeout
            .stderr_summary
            .as_deref()
            .unwrap_or_default()
            .contains("timed out")
    );
}

#[test]
fn material_probe_rejects_malformed_json_and_invalid_frame_rates() {
    let temp_dir = tempfile::tempdir().expect("temp dir should create");
    let input = temp_dir.path().join("input.mp4");
    fs::write(&input, b"placeholder").expect("input fixture should write");
    let runtime = fake_runtime(temp_dir.path().join("ffprobe"));

    let malformed = FakeExecutor::successful(b"not-json".to_vec());
    let malformed_error = probe_material_metadata(&malformed, &runtime, &input)
        .expect_err("malformed JSON should fail");

    assert_eq!(malformed_error.kind, MaterialProbeErrorKind::MalformedJson);

    let invalid_rate = FakeExecutor::successful(
        br#"{"streams":[{"codec_type":"video","width":1,"height":1,"r_frame_rate":"1/0"}]}"#
            .to_vec(),
    );
    let invalid_rate_error = probe_material_metadata(&invalid_rate, &runtime, &input)
        .expect_err("invalid rational fps should fail");

    assert_eq!(
        invalid_rate_error.kind,
        MaterialProbeErrorKind::InvalidFrameRate
    );
}

#[derive(Clone)]
enum FakeBehavior {
    Successful(Vec<u8>),
    Failed(String, String),
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

    fn failed(stdout: String, stderr: String) -> Self {
        Self {
            behavior: FakeBehavior::Failed(stdout, stderr),
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
        "fake-material-probe-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, _binary: &Path) -> io::Result<Output> {
        self.run(_binary, &[])
    }

    fn run(&self, _binary: &Path, _args: &[OsString]) -> io::Result<Output> {
        match &self.behavior {
            FakeBehavior::Successful(stdout) => Ok(Output {
                status: success_status(),
                stdout: stdout.clone(),
                stderr: Vec::new(),
            }),
            FakeBehavior::Failed(stdout, stderr) => Ok(Output {
                status: failure_status(),
                stdout: stdout.as_bytes().to_vec(),
                stderr: stderr.as_bytes().to_vec(),
            }),
            FakeBehavior::Timeout => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "ffprobe timed out after 100 ms",
            )),
        }
    }
}

fn fake_runtime(ffprobe_path: PathBuf) -> RuntimeConfig {
    RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: ffprobe_path.with_file_name("ffmpeg"),
            source: DiscoverySource::Path,
            version: "ffmpeg version fake".to_string(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: ffprobe_path,
            source: DiscoverySource::Path,
            version: "ffprobe version fake".to_string(),
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
