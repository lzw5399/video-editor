use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::{Mutex, OnceLock};

use media_runtime::{
    BinaryKind, DiscoveredBinary, DiscoveryErrorKind, DiscoverySource, FfmpegExecutor,
    RuntimeCapabilityStatus, RuntimeConfig, discover_runtime_config, probe_runtime_capabilities,
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn runtime_capability_report_contains_binaries_features_fonts_and_license_posture() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = tempfile::tempdir().unwrap();
    let font_path = sandbox.path().join("PingFang.ttc");
    std::fs::write(&font_path, b"font").unwrap();
    let _font_env = EnvVarGuard::set_path("VE_TEXT_FONT_PATH", &font_path);

    let runtime = RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: PathBuf::from("/runtime/bin/ffmpeg"),
            source: DiscoverySource::Env {
                variable: "VE_FFMPEG_PATH".to_owned(),
            },
            version: "ffmpeg version test-build".to_owned(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: PathBuf::from("/runtime/bin/ffprobe"),
            source: DiscoverySource::Path,
            version: "ffprobe version test-build".to_owned(),
        },
    };
    let executor = FakeExecutor::new()
        .with_version(
            "ffmpeg version test-build\nconfiguration: --enable-libx264 --enable-libass\n",
        )
        .with_probe(
            &["-hide_banner", "-encoders"],
            " V..... libx264 H.264 encoder\n A..... aac AAC encoder\n",
        )
        .with_probe(
            &["-hide_banner", "-filters"],
            " ... ass Render ASS subtitles\n ... subtitles Render text subtitles\n",
        );

    let report = probe_runtime_capabilities(&executor, &runtime);

    assert_eq!(report.status, RuntimeCapabilityStatus::Ready);
    assert_eq!(report.executor_name, "fake-executor");
    assert_eq!(report.ffmpeg.path, PathBuf::from("/runtime/bin/ffmpeg"));
    assert_eq!(report.ffmpeg.source, "VE_FFMPEG_PATH");
    assert_eq!(report.ffmpeg.version, "ffmpeg version test-build");
    assert!(
        report
            .ffmpeg
            .configure_summary
            .as_deref()
            .unwrap()
            .contains("--enable-libx264")
    );
    assert_eq!(report.ffprobe.source, "PATH");
    assert!(report.h264_encoder.available);
    assert!(report.aac_encoder.available);
    assert!(report.ass_filter.available);
    assert!(report.subtitles_filter.available);
    assert_eq!(
        report.font_readiness.env_text_font_path,
        Some(font_path.clone())
    );
    assert!(
        report
            .font_readiness
            .available_font_paths
            .contains(&font_path)
    );
    assert!(report.license_posture.external_runtime);
    assert!(!report.license_posture.redistributable_build);
    assert_eq!(report.license_posture.source, "externalRuntime");
}

#[test]
fn runtime_capability_report_classifies_missing_optional_capabilities_as_warning() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _font_env = EnvVarGuard::set_path("VE_TEXT_FONT_PATH", "/missing/font.ttf");
    let runtime = RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: PathBuf::from("/runtime/bin/ffmpeg"),
            source: DiscoverySource::Path,
            version: "ffmpeg version test-build".to_owned(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: PathBuf::from("/runtime/bin/ffprobe"),
            source: DiscoverySource::Path,
            version: "ffprobe version test-build".to_owned(),
        },
    };
    let executor = FakeExecutor::new()
        .with_version("ffmpeg version test-build\n")
        .with_probe(
            &["-hide_banner", "-encoders"],
            " V..... rawvideo raw video\n",
        )
        .with_probe(&["-hide_banner", "-filters"], " ... scale Scale video\n");

    let report = probe_runtime_capabilities(&executor, &runtime);

    assert_eq!(report.status, RuntimeCapabilityStatus::Warning);
    assert!(!report.h264_encoder.available);
    assert!(!report.aac_encoder.available);
    assert!(!report.ass_filter.available);
    assert!(!report.subtitles_filter.available);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|message| message.contains("H.264"))
    );
}

#[test]
fn runtime_capability_missing_env_path_returns_classified_discovery_error() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let missing_ffmpeg = PathBuf::from("/definitely-missing/video-editor/ffmpeg");
    let _env_ffmpeg = EnvVarGuard::set_path("VE_FFMPEG_PATH", &missing_ffmpeg);
    let _env_ffprobe = EnvVarGuard::remove("VE_FFPROBE_PATH");
    let _path = EnvVarGuard::set_path("PATH", "/definitely-missing/video-editor/bin");

    let error = discover_runtime_config().expect_err("missing runtime should be classified");

    assert_eq!(error.kind, DiscoveryErrorKind::MissingBinary);
    assert_eq!(error.binary, BinaryKind::Ffmpeg);
    assert!(error.checked_paths.contains(&missing_ffmpeg));
    assert!(error.remediation.contains("VE_FFMPEG_PATH"));
}

#[derive(Default)]
struct FakeExecutor {
    version_stdout: Vec<u8>,
    probes: BTreeMap<Vec<String>, Output>,
}

impl FakeExecutor {
    fn new() -> Self {
        Self::default()
    }

    fn with_version(mut self, stdout: &str) -> Self {
        self.version_stdout = stdout.as_bytes().to_vec();
        self
    }

    fn with_probe(mut self, args: &[&str], stdout: &str) -> Self {
        self.probes.insert(
            args.iter().map(|value| (*value).to_owned()).collect(),
            Output {
                status: success_status(),
                stdout: stdout.as_bytes().to_vec(),
                stderr: Vec::new(),
            },
        );
        self
    }
}

impl FfmpegExecutor for FakeExecutor {
    fn executor_name(&self) -> &'static str {
        "fake-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, _binary: &Path) -> std::io::Result<Output> {
        Ok(Output {
            status: success_status(),
            stdout: self.version_stdout.clone(),
            stderr: Vec::new(),
        })
    }

    fn run(&self, _binary: &Path, args: &[OsString]) -> std::io::Result<Output> {
        let key = args
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        Ok(self.probes.get(&key).cloned().unwrap_or_else(|| Output {
            status: failure_status(),
            stdout: Vec::new(),
            stderr: format!("unexpected probe args: {key:?}").into_bytes(),
        }))
    }
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set_path(key: &'static str, value: impl AsRef<Path>) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value.as_ref());
        }
        Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(0)
}

#[cfg(unix)]
fn failure_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
fn success_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(0)
}

#[cfg(windows)]
fn failure_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(1)
}
