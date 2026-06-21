use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use media_runtime::{
    BinaryKind, DiscoveryErrorKind, DiscoverySource, MAX_STDERR_SUMMARY_BYTES,
    discover_runtime_config, probe_binary_version_with_timeout,
    replace_configured_bundled_runtime_directory_for_tests,
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn discovery_runtime_config_uses_bundled_runtime_directory() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("bundled-success");
    let ffmpeg = sandbox.bin("ffmpeg", "ffmpeg version bundled-build\n", "", 0);
    let ffprobe = sandbox.bin("ffprobe", "ffprobe version bundled-build\n", "", 0);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let _legacy_ffmpeg = EnvVarGuard::remove("VE_FFMPEG_PATH");
    let _legacy_ffprobe = EnvVarGuard::remove("VE_FFPROBE_PATH");
    let _path = EnvVarGuard::set_path("PATH", sandbox.dir("poison-path"));

    let config = discover_runtime_config().expect("bundled binaries should probe");

    assert_eq!(config.ffmpeg.kind, BinaryKind::Ffmpeg);
    assert_eq!(config.ffmpeg.path, ffmpeg);
    assert_eq!(
        config.ffmpeg.source,
        DiscoverySource::Bundled {
            directory: sandbox.root.clone()
        }
    );
    assert_eq!(config.ffmpeg.version, "ffmpeg version bundled-build");

    assert_eq!(config.ffprobe.kind, BinaryKind::Ffprobe);
    assert_eq!(config.ffprobe.path, ffprobe);
    assert_eq!(
        config.ffprobe.source,
        DiscoverySource::Bundled {
            directory: sandbox.root.clone()
        }
    );
    assert_eq!(config.ffprobe.version, "ffprobe version bundled-build");
}

#[test]
fn discovery_never_falls_back_to_legacy_env_or_path() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("bundled-missing");
    let legacy = sandbox.dir("legacy");
    let _legacy_ffmpeg_path = sandbox.bin_at(legacy, "ffmpeg", "ffmpeg version legacy\n", "", 0);
    let _legacy_ffprobe_path = sandbox.bin_at(legacy, "ffprobe", "ffprobe version legacy\n", "", 0);
    let missing_bundled_dir = sandbox.root.join("missing-bundled");

    let _runtime_dir = RuntimeDirectoryGuard::set(&missing_bundled_dir);
    let _legacy_ffmpeg = EnvVarGuard::set_path("VE_FFMPEG_PATH", legacy.join("ffmpeg"));
    let _legacy_ffprobe = EnvVarGuard::set_path("VE_FFPROBE_PATH", legacy.join("ffprobe"));
    let _path = EnvVarGuard::set_path("PATH", legacy);

    let error = discover_runtime_config().expect_err("missing bundled runtime should fail");

    assert_eq!(error.kind, DiscoveryErrorKind::MissingBinary);
    assert_eq!(error.binary, BinaryKind::Ffmpeg);
    assert_eq!(
        error.checked_paths,
        vec![missing_bundled_dir.join("ffmpeg")]
    );
    assert!(
        error
            .remediation
            .contains("apps/desktop-electron/runtime/ffmpeg")
    );
    assert!(!error.remediation.contains("VE_FFMPEG_PATH"));
    assert!(!error.remediation.contains("PATH"));
}

#[test]
fn discovery_bad_bundled_binary_error_uses_bounded_output_summary() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("bad-binary");
    let long_stderr = "x".repeat(MAX_STDERR_SUMMARY_BYTES + 128);
    let bad_ffmpeg = sandbox.bin("ffmpeg", "not really ffmpeg\n", &long_stderr, 23);
    let _good_ffprobe = sandbox.bin("ffprobe", "ffprobe version bundled-build\n", "", 0);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);

    let error = discover_runtime_config().expect_err("bad ffmpeg should fail version probe");

    assert_eq!(error.kind, DiscoveryErrorKind::VersionProbeFailed);
    assert_eq!(error.binary, BinaryKind::Ffmpeg);
    assert_eq!(error.checked_paths, vec![bad_ffmpeg]);
    assert_eq!(error.stdout_summary.as_deref(), Some("not really ffmpeg"));
    assert!(
        error.stderr_summary.as_ref().unwrap().len() <= MAX_STDERR_SUMMARY_BYTES,
        "stderr summary should be bounded"
    );
    assert!(
        error.remediation.contains("bundled runtime directory"),
        "remediation should point at the bundled runtime contract"
    );
}

#[test]
fn discovery_unsupported_bundled_binary_does_not_suggest_system_install() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("unsupported-binary");
    let bad_ffmpeg = sandbox.bin("ffmpeg", "custom tool 1.0\n", "", 0);
    let _good_ffprobe = sandbox.bin("ffprobe", "ffprobe version bundled-build\n", "", 0);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let _path = EnvVarGuard::set_path("PATH", sandbox.dir("poison-path"));

    let error = discover_runtime_config().expect_err("unsupported bundled ffmpeg should fail");

    assert_eq!(error.kind, DiscoveryErrorKind::UnsupportedVersion);
    assert_eq!(error.binary, BinaryKind::Ffmpeg);
    assert_eq!(error.checked_paths, vec![bad_ffmpeg]);
    assert!(error.remediation.contains("bundled ffmpeg"));
    assert!(!error.remediation.contains("Install"));
    assert!(!error.remediation.contains("PATH"));
}

#[test]
fn discovery_version_probe_times_out_for_hung_binary() {
    let sandbox = Sandbox::new("hung-binary");
    let hung_ffmpeg = sandbox.bin_sleep("ffmpeg", 2);

    let error = probe_binary_version_with_timeout(
        BinaryKind::Ffmpeg,
        hung_ffmpeg.clone(),
        DiscoverySource::Bundled {
            directory: sandbox.root.clone(),
        },
        Duration::from_millis(100),
    )
    .expect_err("hung version probe should time out");

    assert_eq!(error.kind, DiscoveryErrorKind::VersionProbeFailed);
    assert_eq!(error.binary, BinaryKind::Ffmpeg);
    assert_eq!(error.checked_paths, vec![hung_ffmpeg]);
    assert!(
        error
            .stderr_summary
            .as_deref()
            .unwrap_or_default()
            .contains("timed out"),
        "timeout errors should be actionable"
    );
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
            "video-editor-media-runtime-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn dir(&self, name: &str) -> &Path {
        let dir = self.root.join(name);
        fs::create_dir_all(&dir).unwrap();
        Box::leak(dir.into_boxed_path())
    }

    fn bin(&self, name: &str, stdout: &str, stderr: &str, exit_code: i32) -> PathBuf {
        let root = self.root.clone();
        self.bin_at(&root, name, stdout, stderr, exit_code)
    }

    fn bin_at(
        &self,
        dir: impl AsRef<Path>,
        name: &str,
        stdout: &str,
        stderr: &str,
        exit_code: i32,
    ) -> PathBuf {
        let path = dir.as_ref().join(name);
        let script = format!(
            "#!/bin/sh\nprintf '{}'\nprintf '{}' >&2\nexit {exit_code}\n",
            shell_escape_single_quotes(stdout),
            shell_escape_single_quotes(stderr)
        );
        fs::write(&path, script).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        path
    }

    fn bin_sleep(&self, name: &str, seconds: u64) -> PathBuf {
        let path = self.root.join(name);
        let script = format!("#!/bin/sh\nsleep {seconds}\nprintf 'ffmpeg version late\\n'\n");
        fs::write(&path, script).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        path
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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
    previous: Option<std::ffi::OsString>,
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

fn shell_escape_single_quotes(value: &str) -> String {
    value.replace('\'', r#"'\''"#)
}
