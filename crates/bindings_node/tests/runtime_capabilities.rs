use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use bindings_node::{execute_command, probe_runtime_capabilities};
use draft_model::CommandErrorKind;
use serde_json::{Value, json};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
#[cfg(unix)]
fn runtime_capabilities_explicit_api_returns_report() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("runtime-capabilities-ok");
    let ffmpeg = sandbox.bin(
        "ffmpeg",
        r#"
case "$1" in
  -version)
    printf 'ffmpeg version test-build\nconfiguration: --enable-libx264 --enable-libass\n'
    ;;
  -hide_banner)
    case "$2" in
      -encoders)
        printf ' V..... libx264 H.264 encoder\n A..... aac AAC encoder\n'
        ;;
      -filters)
        printf ' ... ass Render ASS subtitles\n ... subtitles Render text subtitles\n'
        ;;
      *)
        printf 'unexpected probe args: %s %s\n' "$1" "$2" >&2
        exit 2
        ;;
    esac
    ;;
  *)
    printf 'unexpected ffmpeg args: %s\n' "$*" >&2
    exit 2
    ;;
esac
"#,
    );
    let ffprobe = sandbox.bin(
        "ffprobe",
        r#"
if [ "$1" = "-version" ]; then
  printf 'ffprobe version test-build\n'
else
  printf 'unexpected ffprobe args: %s\n' "$*" >&2
  exit 2
fi
"#,
    );
    let font_path = sandbox.root.join("PingFang.ttc");
    fs::write(&font_path, b"font").unwrap();

    let _runtime_dir = EnvVarGuard::set_path("VE_BUNDLED_FFMPEG_DIR", &sandbox.root);
    let _font_env = EnvVarGuard::set_path("VE_TEXT_FONT_PATH", &font_path);
    let _path = EnvVarGuard::set_path("PATH", sandbox.root.join("empty-path"));

    let envelope =
        probe_runtime_capabilities().expect("runtime capabilities should return a JSON envelope");

    assert_eq!(envelope["ok"], true, "{envelope:#}");
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(envelope["events"], json!([]));
    assert_eq!(envelope["data"]["status"], "ready");
    assert_eq!(envelope["data"]["executorName"], "desktop-ffmpeg-executor");
    assert_eq!(envelope["data"]["ffmpeg"]["kind"], "ffmpeg");
    assert_eq!(
        envelope["data"]["ffmpeg"]["path"],
        ffmpeg.display().to_string()
    );
    assert_eq!(envelope["data"]["ffmpeg"]["source"], "bundled");
    assert_eq!(
        envelope["data"]["ffmpeg"]["version"],
        "ffmpeg version test-build"
    );
    assert!(
        envelope["data"]["ffmpeg"]["configureSummary"]
            .as_str()
            .unwrap()
            .contains("--enable-libx264")
    );
    assert_eq!(
        envelope["data"]["ffprobe"]["path"],
        ffprobe.display().to_string()
    );
    assert_eq!(envelope["data"]["ffprobe"]["source"], "bundled");
    assert_eq!(envelope["data"]["h264Encoder"]["available"], true);
    assert_eq!(envelope["data"]["aacEncoder"]["available"], true);
    assert_eq!(envelope["data"]["assFilter"]["available"], true);
    assert_eq!(envelope["data"]["subtitlesFilter"]["available"], true);
    assert_eq!(
        envelope["data"]["fontReadiness"]["envTextFontPath"],
        font_path.display().to_string()
    );
    assert_eq!(envelope["data"]["licensePosture"]["externalRuntime"], false);
    assert_eq!(
        envelope["data"]["licensePosture"]["source"],
        "bundledRuntime"
    );
    assert_eq!(
        envelope["data"]["licensePosture"]["redistributableBuild"],
        false
    );
    assert!(envelope["data"]["mediaIo"].is_object(), "{envelope:#}");
    assert!(envelope["data"]["mediaIo"]["windows"].is_object());
    assert!(envelope["data"]["mediaIo"]["macos"].is_object());
    assert_eq!(
        envelope["data"]["mediaIo"]["textureInterop"]["compatibleWithPreviewDevice"],
        false
    );
    assert_eq!(
        envelope["data"]["mediaIo"]["fallbackLadder"]["paths"][0]["path"],
        "nativeHardwareTexture"
    );
    assert!(
        envelope["data"]["mediaIo"]["codecs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|codec| codec["codec"] == "h264"
                && codec["containers"] == json!(["mp4", "mov"])
                && codec["firstNativeHardwareDecodeTarget"] == true),
        "{envelope:#}"
    );
}

#[test]
fn runtime_capabilities_reports_missing_ffmpeg_with_chinese_action() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("runtime-capabilities-missing-ffmpeg");
    let _runtime_dir = EnvVarGuard::set_path("VE_BUNDLED_FFMPEG_DIR", &sandbox.root);
    let _path = EnvVarGuard::set_path("PATH", sandbox.root.join("empty-path"));

    let envelope =
        probe_runtime_capabilities().expect("missing runtime should return a JSON envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::RuntimeDiscoveryFailed).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "probeRuntimeCapabilities");
    let message = envelope["error"]["message"].as_str().unwrap();
    assert!(message.contains("未找到内置 FFmpeg"), "{message}");
    assert!(message.contains("runtime/ffmpeg"), "{message}");
    assert!(!message.contains("VE_BUNDLED_FFMPEG_DIR"), "{message}");
    assert!(!message.contains("VE_FFMPEG_PATH"), "{message}");
}

#[test]
#[cfg(unix)]
fn runtime_capabilities_reports_missing_ffprobe_with_chinese_action() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("runtime-capabilities-missing-ffprobe");
    let _ffmpeg = sandbox.bin(
        "ffmpeg",
        r#"
if [ "$1" = "-version" ]; then
  printf 'ffmpeg version test-build\n'
else
  printf 'unexpected ffmpeg args: %s\n' "$*" >&2
  exit 2
fi
"#,
    );
    let _runtime_dir = EnvVarGuard::set_path("VE_BUNDLED_FFMPEG_DIR", &sandbox.root);
    let _path = EnvVarGuard::set_path("PATH", sandbox.root.join("empty-path"));

    let envelope =
        probe_runtime_capabilities().expect("missing runtime should return a JSON envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::RuntimeDiscoveryFailed).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "probeRuntimeCapabilities");
    let message = envelope["error"]["message"].as_str().unwrap();
    assert!(message.contains("未找到内置 ffprobe"), "{message}");
    assert!(message.contains("runtime/ffmpeg"), "{message}");
    assert!(!message.contains("VE_BUNDLED_FFMPEG_DIR"), "{message}");
    assert!(!message.contains("VE_FFPROBE_PATH"), "{message}");
}

#[test]
fn runtime_capabilities_legacy_execute_command_is_unsupported() {
    let envelope = execute_command(json!({
        "command": "probeRuntimeCapabilities",
        "payload": { "kind": "probeRuntimeCapabilities" },
        "requestId": "req-runtime-capabilities-legacy"
    }))
    .expect("legacy runtime capabilities command should return a JSON envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::UnsupportedCommand).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "probeRuntimeCapabilities");
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
            "video-editor-bindings-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("empty-path")).unwrap();
        Self { root }
    }

    #[cfg(unix)]
    fn bin(&self, name: &str, body: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
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
