use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::{Mutex, OnceLock};

use draft_model::{BUNDLED_TEXT_FONT_FAMILY, BUNDLED_TEXT_FONT_REF};
use media_runtime::{
    BinaryKind, DiscoveredBinary, DiscoveryErrorKind, DiscoverySource, FfmpegExecutor,
    RuntimeCapabilityStatus, RuntimeConfig, discover_runtime_config, probe_runtime_capabilities,
    replace_configured_bundled_runtime_directory_for_tests,
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn runtime_capability_report_contains_binaries_features_fonts_and_license_posture() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = tempfile::tempdir().unwrap();
    let font_path = sandbox.path().join("PingFang.ttc");
    std::fs::write(&font_path, b"font").unwrap();
    let _font_env = EnvVarGuard::set_path("VE_TEXT_FONT_PATH", &font_path);
    let bundled_dir = PathBuf::from("/runtime/bin");

    let runtime = RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: bundled_dir.join("ffmpeg"),
            source: DiscoverySource::Bundled {
                directory: bundled_dir.clone(),
            },
            version: "ffmpeg version test-build".to_owned(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: bundled_dir.join("ffprobe"),
            source: DiscoverySource::Bundled {
                directory: bundled_dir.clone(),
            },
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
    assert_eq!(report.ffmpeg.source, "bundled");
    assert_eq!(report.ffmpeg.version, "ffmpeg version test-build");
    assert!(
        report
            .ffmpeg
            .configure_summary
            .as_deref()
            .unwrap()
            .contains("--enable-libx264")
    );
    assert_eq!(report.ffprobe.source, "bundled");
    assert!(report.h264_encoder.available);
    assert!(report.aac_encoder.available);
    assert!(report.ass_filter.available);
    assert!(report.subtitles_filter.available);
    assert_eq!(
        report.font_readiness.env_text_font_path,
        Some(font_path.clone())
    );
    assert_eq!(
        report.font_readiness.bundled_font_ref.as_deref(),
        Some(BUNDLED_TEXT_FONT_REF)
    );
    assert_eq!(
        report.font_readiness.bundled_font_family.as_deref(),
        Some(BUNDLED_TEXT_FONT_FAMILY)
    );
    assert!(
        report
            .font_readiness
            .bundled_font_path
            .as_ref()
            .is_some_and(|path| path.is_file())
    );
    assert_eq!(
        report.font_readiness.bundled_font_license.as_deref(),
        Some("OFL-1.1")
    );
    assert!(
        report
            .font_readiness
            .available_font_paths
            .contains(&font_path)
    );
    assert!(!report.license_posture.external_runtime);
    assert!(!report.license_posture.redistributable_build);
    assert_eq!(report.license_posture.source, "bundledRuntime");
}

#[test]
fn runtime_capability_report_marks_bundled_runtime_without_redistribution_approval() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _font_env = EnvVarGuard::set_path("VE_TEXT_FONT_PATH", "/missing/font.ttf");
    let bundled_dir = PathBuf::from("/app/resources/ffmpeg/darwin-arm64");
    let runtime = RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: bundled_dir.join("ffmpeg"),
            source: DiscoverySource::Bundled {
                directory: bundled_dir.clone(),
            },
            version: "ffmpeg version bundled-build".to_owned(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: bundled_dir.join("ffprobe"),
            source: DiscoverySource::Bundled {
                directory: bundled_dir.clone(),
            },
            version: "ffprobe version bundled-build".to_owned(),
        },
    };
    let executor = FakeExecutor::new()
        .with_version("ffmpeg version bundled-build\n")
        .with_probe(
            &["-hide_banner", "-encoders"],
            " V..... libx264 H.264 encoder\n A..... aac AAC encoder\n",
        )
        .with_probe(
            &["-hide_banner", "-filters"],
            " ... ass Render ASS subtitles\n ... subtitles Render text subtitles\n",
        );

    let report = probe_runtime_capabilities(&executor, &runtime);

    assert_eq!(report.ffmpeg.source, "bundled");
    assert_eq!(report.ffprobe.source, "bundled");
    assert!(!report.license_posture.external_runtime);
    assert!(!report.license_posture.redistributable_build);
    assert_eq!(report.license_posture.source, "bundledRuntime");
    assert!(report.license_posture.message.contains("法律审查"));
}

#[test]
fn runtime_capability_report_classifies_missing_optional_capabilities_as_warning() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _font_env = EnvVarGuard::set_path("VE_TEXT_FONT_PATH", "/missing/font.ttf");
    let bundled_dir = PathBuf::from("/runtime/bin");
    let runtime = RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: bundled_dir.join("ffmpeg"),
            source: DiscoverySource::Bundled {
                directory: bundled_dir.clone(),
            },
            version: "ffmpeg version test-build".to_owned(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: bundled_dir.join("ffprobe"),
            source: DiscoverySource::Bundled {
                directory: bundled_dir.clone(),
            },
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
fn runtime_capability_missing_bundled_dir_returns_classified_discovery_error() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let missing_dir = PathBuf::from("/definitely-missing/video-editor/bundled-ffmpeg");
    let missing_ffmpeg = missing_dir.join("ffmpeg");
    let _runtime_dir = RuntimeDirectoryGuard::set(&missing_dir);
    let _legacy_ffmpeg = EnvVarGuard::set_path("VE_FFMPEG_PATH", "/tmp/ignored-ffmpeg");
    let _legacy_ffprobe = EnvVarGuard::set_path("VE_FFPROBE_PATH", "/tmp/ignored-ffprobe");
    let _path = EnvVarGuard::set_path("PATH", "/definitely-missing/video-editor/bin");

    let error = discover_runtime_config().expect_err("missing runtime should be classified");

    assert_eq!(error.kind, DiscoveryErrorKind::MissingBinary);
    assert_eq!(error.binary, BinaryKind::Ffmpeg);
    assert!(error.checked_paths.contains(&missing_ffmpeg));
    assert!(
        error
            .remediation
            .contains("apps/desktop-electron/runtime/ffmpeg")
    );
    assert!(!error.remediation.contains("VE_FFMPEG_PATH"));
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

struct RuntimeDirectoryGuard {
    previous: Option<PathBuf>,
}

impl RuntimeDirectoryGuard {
    fn set(directory: impl AsRef<Path>) -> Self {
        Self {
            previous: replace_configured_bundled_runtime_directory_for_tests(Some(
                directory.as_ref().to_path_buf(),
            )),
        }
    }
}

impl Drop for RuntimeDirectoryGuard {
    fn drop(&mut self) {
        replace_configured_bundled_runtime_directory_for_tests(self.previous.take());
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
