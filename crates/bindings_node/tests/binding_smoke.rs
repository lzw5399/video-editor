use bindings_node::{execute_command, ping, version};
use draft_model::{CommandErrorKind, Draft};
use media_runtime::discover_runtime_config;
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::open_project_bundle;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use testkit::generate_video_material_fixture;

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
fn execute_command_imports_and_lists_materials_through_standard_envelopes() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let runtime = discover_runtime_config().expect("ffmpeg runtime should be discoverable");
    let executor = DesktopFfmpegExecutor::default();
    let video = generate_video_material_fixture(&executor, &runtime)
        .expect("video material fixture should be generated");
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("command-import.veproj");
    let draft = Draft::new("draft-command-import", "Command import");

    let imported = execute_command(json!({
        "command": "importMaterial",
        "payload": {
            "kind": "importMaterial",
            "draft": draft,
            "bundlePath": bundle_path.display().to_string(),
            "materialPath": video.path().display().to_string(),
            "materialId": "material-command-video",
            "displayName": "command-video.mp4"
        },
        "requestId": "req-import-material"
    }))
    .expect("import material command should return a JSON envelope");

    assert_eq!(imported["ok"], true, "{imported:#}");
    assert_eq!(imported["error"], Value::Null);
    assert_eq!(imported["events"], json!([]));
    assert_eq!(
        imported["data"]["material"]["materialId"],
        "material-command-video"
    );
    assert_eq!(imported["data"]["material"]["kind"], "video");
    assert_eq!(imported["data"]["material"]["status"], "available");
    assert_eq!(
        imported["data"]["material"]["displayName"],
        "command-video.mp4"
    );
    assert_eq!(
        imported["data"]["material"]["metadata"]["duration"],
        1_000_000
    );
    assert_eq!(imported["data"]["diagnostic"], Value::Null);

    let reopened = open_project_bundle(&project_store::StdPlatformFileSystem, &bundle_path)
        .expect("import command should save the project bundle");
    assert_eq!(reopened.bundle.draft.materials.len(), 1);

    let listed = execute_command(json!({
        "command": "listMaterials",
        "payload": {
            "kind": "listMaterials",
            "draft": imported["data"]["draft"].clone()
        },
        "requestId": "req-list-materials"
    }))
    .expect("list materials command should return a JSON envelope");

    assert_eq!(listed["ok"], true);
    assert_eq!(listed["error"], Value::Null);
    assert_eq!(listed["events"], json!([]));
    assert_eq!(listed["data"]["materials"].as_array().unwrap().len(), 1);
    assert_eq!(
        listed["data"]["materials"][0]["materialId"],
        "material-command-video"
    );
}

#[test]
fn execute_command_reports_missing_material_diagnostics_without_corrupting_draft() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("command-missing.veproj");
    let missing_path = bundle_path.join("media").join("missing.mp4");
    let draft = Draft::new("draft-command-missing", "Command missing");

    let imported = execute_command(json!({
        "command": "importMaterial",
        "payload": {
            "kind": "importMaterial",
            "draft": draft,
            "bundlePath": bundle_path.display().to_string(),
            "materialPath": missing_path.display().to_string(),
            "materialId": "material-command-missing",
            "displayName": "missing.mp4",
            "materialKindHint": "video"
        },
        "requestId": "req-import-missing"
    }))
    .expect("missing import command should return a JSON envelope");

    assert_eq!(imported["ok"], true);
    assert_eq!(imported["error"], Value::Null);
    assert_eq!(imported["data"]["material"]["status"], "missing");
    assert_eq!(imported["data"]["diagnostic"]["kind"], "missingFile");
    assert_eq!(
        imported["data"]["diagnostic"]["originalUri"],
        "media/missing.mp4"
    );

    let diagnostics = execute_command(json!({
        "command": "listMissingMaterials",
        "payload": {
            "kind": "listMissingMaterials",
            "draft": imported["data"]["draft"].clone(),
            "bundlePath": bundle_path.display().to_string()
        },
        "requestId": "req-list-missing-materials"
    }))
    .expect("missing list command should return a JSON envelope");

    assert_eq!(diagnostics["ok"], true);
    assert_eq!(diagnostics["error"], Value::Null);
    assert_eq!(diagnostics["events"], json!([]));
    assert_eq!(
        diagnostics["data"]["diagnostics"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        diagnostics["data"]["diagnostics"][0]["materialId"],
        "material-command-missing"
    );
    assert_eq!(diagnostics["data"]["diagnostics"][0]["kind"], "missingFile");

    let reopened = open_project_bundle(&project_store::StdPlatformFileSystem, &bundle_path)
        .expect("missing material command should save recoverable draft");
    assert_eq!(reopened.bundle.draft.materials.len(), 1);
    assert_eq!(
        reopened.bundle.draft.materials[0].material_id.as_str(),
        "material-command-missing"
    );
}

#[test]
fn execute_command_rejects_mismatched_command_payload_kind() {
    let envelope = execute_command(json!({
        "command": "version",
        "payload": { "kind": "ping" },
        "requestId": "req-mismatch"
    }))
    .expect("mismatched command returns an error envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidPayload).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "version");
    assert_eq!(envelope["events"], json!([]));
}

#[test]
fn execute_command_routes_timeline_add_move_and_selection() {
    let draft = timeline_draft_json();

    let added = execute_command(json!({
        "command": "addSegment",
        "payload": {
            "kind": "addSegment",
            "draft": draft,
            "commandState": empty_command_state_json(),
            "selection": empty_selection_json(),
            "trackId": "video-track",
            "segmentId": "segment-a",
            "materialId": "video-material",
            "sourceTimerange": { "start": 100_000, "duration": 400_000 },
            "targetTimerange": { "start": 0, "duration": 400_000 }
        },
        "requestId": "req-add-segment"
    }))
    .expect("add segment command should return a JSON envelope");

    assert_eq!(added["ok"], true, "{added:#}");
    assert_eq!(added["error"], Value::Null);
    assert_eq!(added["data"]["events"][0]["kind"], "segmentAdded");
    assert_eq!(
        added["data"]["draft"]["tracks"][0]["segments"][0]["segmentId"],
        "segment-a"
    );

    let moved = execute_command(json!({
        "command": "moveSegment",
        "payload": {
            "kind": "moveSegment",
            "draft": added["data"]["draft"].clone(),
            "commandState": added["data"]["commandState"].clone(),
            "selection": added["data"]["selection"].clone(),
            "segmentId": "segment-a",
            "targetTrackId": "video-track",
            "targetStart": 500_000
        },
        "requestId": "req-move-segment"
    }))
    .expect("move segment command should return a JSON envelope");

    assert_eq!(moved["ok"], true, "{moved:#}");
    assert_eq!(moved["data"]["events"][0]["kind"], "segmentMoved");
    assert_eq!(
        moved["data"]["draft"]["tracks"][0]["segments"][0]["targetTimerange"]["start"],
        500_000
    );
    assert_eq!(
        moved["data"]["draft"]["tracks"][0]["segments"][0]["sourceTimerange"],
        json!({ "start": 100_000, "duration": 400_000 })
    );

    let selected = execute_command(json!({
        "command": "selectTimelineSegments",
        "payload": {
            "kind": "selectTimelineSegments",
            "draft": moved["data"]["draft"].clone(),
            "commandState": moved["data"]["commandState"].clone(),
            "selection": moved["data"]["selection"].clone(),
            "segmentIds": ["segment-a"],
            "trackIds": ["video-track"]
        },
        "requestId": "req-select-segment"
    }))
    .expect("select timeline segments command should return a JSON envelope");

    assert_eq!(selected["ok"], true, "{selected:#}");
    assert_eq!(
        selected["data"]["selection"],
        json!({ "segmentIds": ["segment-a"], "trackIds": ["video-track"] })
    );
    assert_eq!(
        selected["data"]["draft"],
        moved["data"]["draft"],
        "selection command must not mutate draft"
    );
}

#[test]
fn execute_command_rejects_invalid_timeline_edit_with_standard_error() {
    let mut draft = timeline_draft_json();
    draft["tracks"][0]["segments"] = json!([{
        "segmentId": "segment-a",
        "materialId": "video-material",
        "sourceTimerange": { "start": 0, "duration": 400_000 },
        "targetTimerange": { "start": 0, "duration": 400_000 },
        "mainTrackMagnet": { "enabled": false },
        "keyframes": [],
        "filters": []
    }]);

    let envelope = execute_command(json!({
        "command": "addSegment",
        "payload": {
            "kind": "addSegment",
            "draft": draft,
            "commandState": empty_command_state_json(),
            "selection": empty_selection_json(),
            "trackId": "video-track",
            "segmentId": "overlap",
            "materialId": "video-material",
            "sourceTimerange": { "start": 400_000, "duration": 200_000 },
            "targetTimerange": { "start": 100_000, "duration": 200_000 }
        },
        "requestId": "req-invalid-add-segment"
    }))
    .expect("invalid timeline command should return a JSON envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidTimelineEdit).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "addSegment");
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

fn empty_command_state_json() -> Value {
    json!({
        "undoStack": [],
        "redoStack": [],
        "maxHistoryEntries": 100,
        "snapping": {
            "enabled": true,
            "threshold": 50_000
        }
    })
}

fn empty_selection_json() -> Value {
    json!({
        "segmentIds": [],
        "trackIds": []
    })
}

fn timeline_draft_json() -> Value {
    json!({
        "schemaVersion": 1,
        "draftId": "binding-timeline-draft",
        "metadata": { "name": "Binding Timeline Draft" },
        "materials": [{
            "materialId": "video-material",
            "kind": "video",
            "uri": "media/video.mp4",
            "displayName": "video.mp4",
            "metadata": {
                "duration": 1_000_000,
                "width": 160,
                "height": 90,
                "frameRate": { "numerator": 24, "denominator": 1 },
                "hasVideo": true,
                "hasAudio": false
            },
            "status": "available"
        }],
        "tracks": [{
            "trackId": "video-track",
            "kind": "video",
            "name": "Video",
            "muted": false,
            "locked": false,
            "segments": []
        }]
    })
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
