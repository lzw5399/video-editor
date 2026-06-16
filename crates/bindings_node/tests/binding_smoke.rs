use bindings_node::{execute_command, ping, version};
use draft_model::CommandErrorKind;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn ping_returns_standard_ok_envelope() {
    let envelope = ping().expect("ping returns a JSON envelope");

    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"], json!({ "pong": true }));
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(envelope["events"], json!([]));
}

#[test]
fn version_returns_standard_ok_envelope() {
    let envelope = version().expect("version returns a JSON envelope");

    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"]["coreVersion"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        envelope["data"]["contractVersion"],
        draft_model::DRAFT_MODEL_VERSION
    );
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(envelope["events"], json!([]));
}

#[test]
fn execute_command_matches_direct_phase_one_envelopes() {
    let ping_from_command = execute_command(json!({
        "command": "ping",
        "payload": { "kind": "ping" },
        "requestId": "req-ping"
    }))
    .expect("command ping returns a JSON envelope");

    let version_from_command = execute_command(json!({
        "command": "version",
        "payload": { "kind": "version" },
        "requestId": "req-version"
    }))
    .expect("command version returns a JSON envelope");

    assert_eq!(ping_from_command, ping().expect("direct ping returns"));
    assert_eq!(
        version_from_command,
        version().expect("direct version returns")
    );
}

#[test]
fn execute_command_rejects_non_phase_one_command_with_structured_error() {
    let envelope = execute_command(json!({
        "command": "addSegment",
        "payload": { "kind": "addSegment" },
        "requestId": "req-add-segment"
    }))
    .expect("unsupported command returns an error envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::UnsupportedCommand).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "addSegment");
    assert_eq!(envelope["events"], json!([]));
}

#[test]
fn execute_command_probe_media_runtime_returns_standard_ok_envelope() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("binding-probe-ok");
    let ffmpeg = sandbox.bin("ffmpeg", "ffmpeg version binding-test\n", "", 0);
    let ffprobe = sandbox.bin("ffprobe", "ffprobe version binding-test\n", "", 0);
    let _env_ffmpeg = EnvVarGuard::set_path("VE_FFMPEG_PATH", &ffmpeg);
    let _env_ffprobe = EnvVarGuard::set_path("VE_FFPROBE_PATH", &ffprobe);

    let envelope = execute_command(json!({
        "command": "probeMediaRuntime",
        "payload": { "kind": "probeMediaRuntime" },
        "requestId": "req-runtime-probe"
    }))
    .expect("runtime probe returns a JSON envelope");

    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(envelope["events"], json!([]));
    assert_eq!(envelope["data"]["ffmpeg"]["kind"], "ffmpeg");
    assert_eq!(envelope["data"]["ffmpeg"]["path"], json!(ffmpeg));
    assert_eq!(envelope["data"]["ffmpeg"]["source"]["kind"], "env");
    assert_eq!(
        envelope["data"]["ffmpeg"]["source"]["variable"],
        "VE_FFMPEG_PATH"
    );
    assert_eq!(
        envelope["data"]["ffmpeg"]["version"],
        "ffmpeg version binding-test"
    );
    assert_eq!(envelope["data"]["ffprobe"]["kind"], "ffprobe");
    assert_eq!(envelope["data"]["ffprobe"]["path"], json!(ffprobe));
    assert_eq!(envelope["data"]["ffprobe"]["source"]["kind"], "env");
    assert_eq!(
        envelope["data"]["ffprobe"]["source"]["variable"],
        "VE_FFPROBE_PATH"
    );
    assert_eq!(
        envelope["data"]["ffprobe"]["version"],
        "ffprobe version binding-test"
    );
}

#[test]
fn execute_command_probe_media_runtime_maps_discovery_failure_to_stable_error() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("binding-probe-error");
    let bad_stderr = "runtime probe failure ".repeat(300);
    let ffmpeg = sandbox.bin("ffmpeg", "not ffmpeg\n", &bad_stderr, 42);
    let ffprobe = sandbox.bin("ffprobe", "ffprobe version binding-test\n", "", 0);
    let _env_ffmpeg = EnvVarGuard::set_path("VE_FFMPEG_PATH", &ffmpeg);
    let _env_ffprobe = EnvVarGuard::set_path("VE_FFPROBE_PATH", &ffprobe);

    let envelope = execute_command(json!({
        "command": "probeMediaRuntime",
        "payload": { "kind": "probeMediaRuntime" },
        "requestId": "req-runtime-probe"
    }))
    .expect("runtime discovery failures return an error envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::RuntimeDiscoveryFailed).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "probeMediaRuntime");
    assert_eq!(envelope["events"], json!([]));

    let message = envelope["error"]["message"].as_str().unwrap();
    assert!(message.contains("versionProbeFailed"));
    assert!(message.contains("Verify VE_FFMPEG_PATH"));
    assert!(message.contains("runtime probe failure"));
    assert!(
        message.len() < 4_800,
        "runtime error message should not expose unbounded process output"
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
            "video-editor-binding-runtime-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn bin(&self, name: &str, stdout: &str, stderr: &str, exit_code: i32) -> PathBuf {
        let path = self.root.join(name);
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
