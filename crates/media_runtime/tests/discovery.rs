use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use media_runtime::{
    BinaryKind, DiscoveryErrorKind, DiscoverySource, MAX_STDERR_SUMMARY_BYTES,
    discover_runtime_config,
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn discovery_runtime_config_prefers_explicit_env_paths_before_path() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("env-before-path");
    let env_ffmpeg = sandbox.bin("env", "ffmpeg", "ffmpeg version env-build\n", "", 0);
    let env_ffprobe = sandbox.bin("env", "ffprobe", "ffprobe version env-build\n", "", 0);
    let _path_ffmpeg = sandbox.bin("path", "ffmpeg", "ffmpeg version path-build\n", "", 0);
    let _path_ffprobe = sandbox.bin("path", "ffprobe", "ffprobe version path-build\n", "", 0);

    let _env_ffmpeg = EnvVarGuard::set_path("VE_FFMPEG_PATH", &env_ffmpeg);
    let _env_ffprobe = EnvVarGuard::set_path("VE_FFPROBE_PATH", &env_ffprobe);
    let _path = EnvVarGuard::set_path("PATH", sandbox.dir("path"));

    let config = discover_runtime_config().expect("explicit env binaries should probe");

    assert_eq!(config.ffmpeg.kind, BinaryKind::Ffmpeg);
    assert_eq!(config.ffmpeg.path, env_ffmpeg);
    assert_eq!(
        config.ffmpeg.source,
        DiscoverySource::Env {
            variable: "VE_FFMPEG_PATH".to_string()
        }
    );
    assert_eq!(config.ffmpeg.version, "ffmpeg version env-build");

    assert_eq!(config.ffprobe.kind, BinaryKind::Ffprobe);
    assert_eq!(config.ffprobe.path, env_ffprobe);
    assert_eq!(
        config.ffprobe.source,
        DiscoverySource::Env {
            variable: "VE_FFPROBE_PATH".to_string()
        }
    );
    assert_eq!(config.ffprobe.version, "ffprobe version env-build");
}

#[test]
fn discovery_runtime_config_falls_back_to_path_when_env_vars_are_absent() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("path-success");
    let path_ffmpeg = sandbox.bin("path", "ffmpeg", "ffmpeg version path-build\n", "", 0);
    let path_ffprobe = sandbox.bin("path", "ffprobe", "ffprobe version path-build\n", "", 0);

    let _env_ffmpeg = EnvVarGuard::remove("VE_FFMPEG_PATH");
    let _env_ffprobe = EnvVarGuard::remove("VE_FFPROBE_PATH");
    let _path = EnvVarGuard::set_path("PATH", sandbox.dir("path"));

    let config = discover_runtime_config().expect("PATH binaries should probe");

    assert_eq!(config.ffmpeg.path, path_ffmpeg);
    assert_eq!(config.ffmpeg.source, DiscoverySource::Path);
    assert_eq!(config.ffprobe.path, path_ffprobe);
    assert_eq!(config.ffprobe.source, DiscoverySource::Path);
}

#[test]
fn discovery_missing_binary_error_includes_kind_checked_paths_and_remediation() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("missing");
    let missing_ffmpeg = sandbox.root.join("does-not-exist-ffmpeg");
    let _env_ffmpeg = EnvVarGuard::set_path("VE_FFMPEG_PATH", &missing_ffmpeg);
    let _env_ffprobe = EnvVarGuard::remove("VE_FFPROBE_PATH");
    let _path = EnvVarGuard::set_path("PATH", sandbox.dir("empty-path"));

    let error = discover_runtime_config().expect_err("missing ffmpeg should fail");

    assert_eq!(error.kind, DiscoveryErrorKind::MissingBinary);
    assert_eq!(error.binary, BinaryKind::Ffmpeg);
    assert!(error.checked_paths.contains(&missing_ffmpeg));
    assert!(
        error
            .remediation
            .contains("Set VE_FFMPEG_PATH to a valid ffmpeg binary")
    );
    assert_eq!(error.stdout_summary, None);
    assert_eq!(error.stderr_summary, None);
}

#[test]
fn discovery_bad_binary_error_uses_bounded_output_summary() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("bad-binary");
    let long_stderr = "x".repeat(MAX_STDERR_SUMMARY_BYTES + 128);
    let bad_ffmpeg = sandbox.bin("env", "ffmpeg", "not really ffmpeg\n", &long_stderr, 23);
    let good_ffprobe = sandbox.bin("env", "ffprobe", "ffprobe version env-build\n", "", 0);
    let _env_ffmpeg = EnvVarGuard::set_path("VE_FFMPEG_PATH", &bad_ffmpeg);
    let _env_ffprobe = EnvVarGuard::set_path("VE_FFPROBE_PATH", &good_ffprobe);
    let _path = EnvVarGuard::set_path("PATH", sandbox.dir("empty-path"));

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
        error
            .remediation
            .contains("Verify VE_FFMPEG_PATH points to a working ffmpeg binary")
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

    fn bin(&self, dir: &str, name: &str, stdout: &str, stderr: &str, exit_code: i32) -> PathBuf {
        let path = self.dir(dir).join(name);
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
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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
