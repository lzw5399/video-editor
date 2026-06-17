use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use media_runtime::{
    CancelToken, FfmpegJobEvent, FfmpegJobState, FfmpegRuntimeErrorKind, FfmpegRuntimeJob,
    MAX_STDERR_SUMMARY_BYTES, parse_progress_lines, run_export_job,
};

#[test]
fn export_job_parses_progress_events() {
    let progress = parse_progress_lines(
        "frame=1\nout_time_us=250000\nprogress=continue\nout_time=00:00:00.750000\nprogress=end\n",
        Some(1_000_000),
    )
    .expect("progress should parse");

    assert_eq!(progress.len(), 2);
    assert_eq!(progress[0].out_time_microseconds, 250_000);
    assert_eq!(progress[0].progress_per_mille, Some(250));
    assert_eq!(progress[1].out_time_microseconds, 750_000);

    let malformed = parse_progress_lines("out_time_us=not-a-number\n", Some(1_000_000))
        .expect_err("malformed progress should fail");
    assert!(malformed.contains("malformed FFmpeg progress"));
}

#[test]
fn export_job_cancel_returns_cancelled_state_and_bounded_logs() {
    let sandbox = Sandbox::new("cancel");
    let progress_written = sandbox.root.join("progress-written");
    let script = sandbox.bin(
        "ffmpeg",
        &format!(
            "#!/bin/sh\nprintf 'out_time_us=100000\\n'\nprintf 'starting export\\n' >&2\ntouch '{}'\nsleep 2\nprintf 'out_time_us=900000\\n'\n",
            progress_written.display()
        ),
    );
    let job = FfmpegRuntimeJob::new("cancel-job", script, vec![], sandbox.root.join("out.mp4"))
        .with_expected_duration_microseconds(1_000_000)
        .with_timeout(Duration::from_secs(5));
    let cancel = CancelToken::new();
    let cancel_clone = cancel.clone();

    thread::spawn(move || {
        for _ in 0..200 {
            if progress_written.exists() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        cancel_clone.cancel();
    });

    let mut events = Vec::new();
    let result = run_export_job(&job, &cancel, |event| events.push(event))
        .expect("cancel should produce a classified job result");

    assert_eq!(result.state, FfmpegJobState::Cancelled);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, FfmpegJobEvent::Progress { .. }))
    );
    assert!(result.stderr_summary.as_ref().unwrap().len() <= MAX_STDERR_SUMMARY_BYTES);
}

#[test]
fn export_job_classifies_timeout_launch_nonzero_malformed_progress_missing_encoder_missing_filter()
{
    let sandbox = Sandbox::new("classify");

    let missing = FfmpegRuntimeJob::new(
        "missing-runtime",
        sandbox.root.join("missing-ffmpeg"),
        vec![],
        sandbox.root.join("out.mp4"),
    );
    let missing_error = run_export_job(&missing, &CancelToken::new(), |_| {})
        .expect_err("missing runtime should classify");
    assert_eq!(
        missing_error.kind,
        FfmpegRuntimeErrorKind::RuntimeUnavailable
    );

    let timeout_script = sandbox.bin("ffmpeg-timeout", "#!/bin/sh\nsleep 2\n");
    let timeout = FfmpegRuntimeJob::new(
        "timeout",
        timeout_script,
        vec![],
        sandbox.root.join("out.mp4"),
    )
    .with_timeout(Duration::from_millis(50));
    let timeout_error =
        run_export_job(&timeout, &CancelToken::new(), |_| {}).expect_err("timeout should classify");
    assert_eq!(timeout_error.kind, FfmpegRuntimeErrorKind::Timeout);

    let malformed_script = sandbox.bin(
        "ffmpeg-malformed",
        "#!/bin/sh\nprintf 'out_time_us=oops\\n'\nsleep 1\n",
    );
    let malformed = FfmpegRuntimeJob::new(
        "malformed",
        malformed_script,
        vec![],
        sandbox.root.join("out.mp4"),
    );
    let malformed_error = run_export_job(&malformed, &CancelToken::new(), |_| {})
        .expect_err("malformed progress should classify");
    assert_eq!(
        malformed_error.kind,
        FfmpegRuntimeErrorKind::MalformedProgress
    );

    let encoder_script = sandbox.bin(
        "ffmpeg-encoder",
        "#!/bin/sh\nprintf 'Unknown encoder h264_nvenc\\n' >&2\nexit 1\n",
    );
    let encoder = FfmpegRuntimeJob::new(
        "encoder",
        encoder_script,
        vec![],
        sandbox.root.join("out.mp4"),
    );
    let encoder_error = run_export_job(&encoder, &CancelToken::new(), |_| {})
        .expect_err("missing encoder should classify");
    assert_eq!(encoder_error.kind, FfmpegRuntimeErrorKind::MissingEncoder);

    let filter_script = sandbox.bin(
        "ffmpeg-filter",
        "#!/bin/sh\nprintf 'No such filter: badfilter\\n' >&2\nexit 1\n",
    );
    let filter = FfmpegRuntimeJob::new(
        "filter",
        filter_script,
        vec![],
        sandbox.root.join("out.mp4"),
    );
    let filter_error =
        run_export_job(&filter, &CancelToken::new(), |_| {}).expect_err("filter should classify");
    assert_eq!(filter_error.kind, FfmpegRuntimeErrorKind::MissingFilter);
}

#[test]
fn export_job_bounds_stdout_stderr_summaries() {
    let sandbox = Sandbox::new("bounded");
    let long = "x".repeat(MAX_STDERR_SUMMARY_BYTES + 512);
    let script = sandbox.bin(
        "ffmpeg-long",
        &format!(
            "#!/bin/sh\nprintf '{}'\nprintf '{}' >&2\nexit 1\n",
            long, long
        ),
    );
    let job = FfmpegRuntimeJob::new("bounded", script, vec![], sandbox.root.join("out.mp4"));

    let error =
        run_export_job(&job, &CancelToken::new(), |_| {}).expect_err("non-zero should fail");

    assert_eq!(error.kind, FfmpegRuntimeErrorKind::NonZeroExit);
    assert!(error.stdout_summary.as_ref().unwrap().len() <= MAX_STDERR_SUMMARY_BYTES);
    assert!(error.stderr_summary.as_ref().unwrap().len() <= MAX_STDERR_SUMMARY_BYTES);
}

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "video-editor-export-job-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn bin(&self, name: &str, script: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, script).unwrap();
        make_executable(&path);
        path
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

fn _args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
