use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bindings_node::{cancel_export, execute_command, get_export_job_status};
use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground,
    CommandErrorKind, Draft, DraftCanvasConfig, ExportDiagnosticKind, ExportJobPhase, ExportPreset,
    Material, MaterialKind, Microseconds, RationalFrameRate, Segment, SourceTimerange,
    TargetTimerange, Track, TrackKind,
};
use media_runtime::replace_configured_bundled_runtime_directory_for_tests;
use serde_json::{Value, json};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn export_commands_start_status_and_complete_through_binding_registry() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("export-complete");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let output = sandbox.root.join("导出.mp4");

    let started = execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": export_draft("draft-export-complete"),
            "outputPath": output,
            "preset": ExportPreset::H264AacBalanced
        },
        "requestId": "req-export-start"
    }))
    .expect("start export should return envelope");

    assert_eq!(started["ok"], true, "{started:#}");
    assert_eq!(started["data"]["phase"], "running");
    assert_eq!(started["data"]["progressPerMille"], 0);
    assert_eq!(started["error"], Value::Null);

    let job_id = started["data"]["jobId"]
        .as_str()
        .expect("export job id should be present")
        .to_owned();
    let completed = wait_for_export_phase(&job_id, ExportJobPhase::Completed);

    assert_eq!(completed["ok"], true, "{completed:#}");
    assert_eq!(completed["data"]["phase"], "completed");
    assert_eq!(completed["data"]["progressPerMille"], 1000);
    assert_eq!(completed["data"]["validation"]["width"], 1_920);
    assert_eq!(completed["data"]["validation"]["height"], 1_080);
    assert_eq!(completed["data"]["validation"]["hasAudio"], true);
    assert_eq!(completed["data"]["logSummary"], "导出完成，输出校验通过");
}

#[test]
fn export_commands_transport_export_prep_dirty_facts() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("export-dirty-facts");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let output = sandbox.root.join("dirty-facts.mp4");

    let started = execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": export_draft("draft-export-dirty-facts"),
            "outputPath": output,
            "preset": ExportPreset::H264AacBalanced,
            "dirtyFacts": {
                "dirtyRanges": [
                    { "targetTimerange": { "start": 250000, "duration": 250000 }, "source": "current" }
                ],
                "changedMaterialIds": ["video-material"],
                "changedGraphNodeIds": ["draft:draft-export-dirty-facts:track:video-track:segment:video-a:video"],
                "changedDomains": ["visual", "exportPrep", "previewCache"],
                "runtimeCapabilityFingerprint": "runtime-export-v1",
                "outputProfileFingerprint": "profile-export-v1",
                "fullDraft": false,
                "reason": "accepted visual edit",
                "artifactSchemaVersion": 2,
                "generatorVersion": "preview-cache-generator-v2"
            }
        },
        "requestId": "req-export-dirty-facts"
    }))
    .expect("start export should return envelope");

    assert_eq!(started["ok"], true, "{started:#}");
    assert_eq!(started["data"]["phase"], "running");
    assert_eq!(started["data"]["dirtyFacts"]["fullDraft"], false);
    assert_eq!(
        started["data"]["dirtyFacts"]["changedGraphNodeIds"],
        json!(["draft:draft-export-dirty-facts:track:video-track:segment:video-a:video"])
    );
    assert_eq!(
        started["data"]["dirtyFacts"]["runtimeCapabilityFingerprint"],
        "runtime-export-v1"
    );
    assert_eq!(
        started["data"]["dirtyFacts"]["generatorVersion"],
        "preview-cache-generator-v2"
    );
}

#[test]
fn export_commands_validate_against_draft_canvas_instead_of_preset_dimensions() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

    for (preset, name) in [
        (ExportPreset::H264AacDraft, "draft-preset"),
        (ExportPreset::H264AacBalanced, "balanced-preset"),
    ] {
        let sandbox = Sandbox::new(name);
        let _ffmpeg = sandbox.ffmpeg_complete();
        let _ffprobe = sandbox.ffprobe_success_with_frame_rate(1080, 1920, 24, 1, true);
        let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
        let output = sandbox.root.join(format!("{name}.mp4"));

        let started = execute_command(json!({
            "command": "startExport",
            "payload": {
                "kind": "startExport",
                "draft": export_draft_with_canvas(
                    &format!("draft-export-canvas-{name}"),
                    DraftCanvasConfig {
                        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio9x16),
                        width: 1080,
                        height: 1920,
                        frame_rate: RationalFrameRate::new(24, 1),
                        background: CanvasBackground::Black,
                        adaptation_policy: CanvasAdaptationPolicy::Auto,
                    }
                ),
                "outputPath": output,
                "preset": preset
            },
            "requestId": format!("req-export-canvas-{name}")
        }))
        .expect("start export should return envelope");

        assert_eq!(started["ok"], true, "{started:#}");
        let job_id = started["data"]["jobId"]
            .as_str()
            .expect("export job id should be present")
            .to_owned();
        let completed = wait_for_export_phase(&job_id, ExportJobPhase::Completed);

        assert_eq!(completed["ok"], true, "{completed:#}");
        assert_eq!(completed["data"]["phase"], "completed");
        assert_eq!(completed["data"]["validation"]["width"], 1080);
        assert_eq!(completed["data"]["validation"]["height"], 1920);
        assert_eq!(
            completed["data"]["validation"]["frameRate"]["numerator"],
            24
        );
        assert_eq!(
            completed["data"]["validation"]["frameRate"]["denominator"],
            1
        );
    }
}

#[test]
fn export_commands_cancel_running_job_and_report_classified_status() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("export-cancel");
    let _ffmpeg = sandbox.ffmpeg_slow();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let output = sandbox.root.join("cancel.mp4");

    let started = execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": export_draft("draft-export-cancel"),
            "outputPath": output,
            "preset": ExportPreset::H264AacBalanced
        },
        "requestId": "req-export-cancel-start"
    }))
    .expect("start export should return envelope");
    let job_id = started["data"]["jobId"].as_str().unwrap().to_owned();

    thread::sleep(Duration::from_millis(50));
    let cancelled = execute_command(json!({
        "command": "cancelExport",
        "payload": {
            "kind": "cancelExport",
            "jobId": job_id
        },
        "requestId": "req-export-cancel"
    }))
    .expect("cancel export should return envelope");

    assert_eq!(cancelled["ok"], true, "{cancelled:#}");
    assert_eq!(cancelled["data"]["phase"], "cancelled");
    assert_eq!(cancelled["data"]["diagnostic"]["kind"], "cancelled");
    assert!(
        cancelled["data"]["logSummary"]
            .as_str()
            .unwrap()
            .contains("取消")
    );
}

#[test]
fn explicit_export_control_apis_query_and_cancel_jobs_without_command_envelopes() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("export-explicit-control");
    let _ffmpeg = sandbox.ffmpeg_slow();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let output = sandbox.root.join("explicit-control.mp4");

    let started = execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": export_draft("draft-export-explicit-control"),
            "outputPath": output,
            "preset": ExportPreset::H264AacBalanced
        },
        "requestId": "req-export-explicit-control-start"
    }))
    .expect("start export should return envelope");
    assert_eq!(started["ok"], true, "{started:#}");
    let job_id = started["data"]["jobId"].as_str().unwrap().to_owned();

    let status = get_export_job_status(json!({ "jobId": job_id }))
        .expect("explicit export status API should return envelope");
    assert_eq!(status["ok"], true, "{status:#}");
    assert_eq!(status["data"]["jobId"], job_id);

    let cancelled = cancel_export(json!({ "jobId": job_id }))
        .expect("explicit export cancel API should return envelope");
    assert_eq!(cancelled["ok"], true, "{cancelled:#}");
    assert_eq!(cancelled["data"]["phase"], "cancelled");
    assert_eq!(cancelled["data"]["diagnostic"]["kind"], "cancelled");
}

#[test]
fn export_commands_cancelled_validation_job_stays_cancelled() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("export-cancel-validation");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_slow_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let output = sandbox.root.join("cancel-validation.mp4");

    let started = execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": export_draft("draft-export-cancel-validation"),
            "outputPath": output,
            "preset": ExportPreset::H264AacBalanced
        },
        "requestId": "req-export-cancel-validation-start"
    }))
    .expect("start export should return envelope");
    let job_id = started["data"]["jobId"].as_str().unwrap().to_owned();

    let validating = wait_for_export_phase(&job_id, ExportJobPhase::Validating);
    assert_eq!(validating["ok"], true, "{validating:#}");

    let cancelled = execute_command(json!({
        "command": "cancelExport",
        "payload": {
            "kind": "cancelExport",
            "jobId": job_id
        },
        "requestId": "req-export-cancel-validation"
    }))
    .expect("cancel export should return envelope");
    assert_eq!(cancelled["data"]["phase"], "cancelled");

    thread::sleep(Duration::from_millis(500));
    let final_status = execute_command(json!({
        "command": "getExportJobStatus",
        "payload": {
            "kind": "getExportJobStatus",
            "jobId": job_id
        },
        "requestId": "req-export-cancel-validation-status"
    }))
    .expect("status command should return envelope");

    assert_eq!(final_status["ok"], true, "{final_status:#}");
    assert_eq!(final_status["data"]["phase"], "cancelled");
    assert_eq!(final_status["data"]["diagnostic"]["kind"], "cancelled");
}

#[test]
fn export_commands_classify_invalid_output_path_as_export_service_failure() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("export-invalid");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);

    let envelope = execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": export_draft("draft-export-invalid"),
            "outputPath": sandbox.root.join("bad.mov"),
            "preset": ExportPreset::H264AacBalanced
        },
        "requestId": "req-export-invalid"
    }))
    .expect("invalid export command should return envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::ExportServiceFailed).unwrap()
    );
    assert_eq!(
        envelope["data"]["diagnostic"]["kind"],
        serde_json::to_value(ExportDiagnosticKind::InvalidOutputPath).unwrap()
    );
}

#[test]
fn export_commands_reject_mismatched_export_command_payload_pair() {
    let envelope = execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "cancelExport",
            "jobId": "export-job"
        },
        "requestId": "req-export-mismatch"
    }))
    .expect("mismatched export command should return envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidPayload).unwrap()
    );
}

fn wait_for_export_phase(job_id: &str, expected: ExportJobPhase) -> Value {
    let expected_value = serde_json::to_value(expected).unwrap();
    let mut last = Value::Null;
    for _ in 0..100 {
        last = execute_command(json!({
            "command": "getExportJobStatus",
            "payload": {
                "kind": "getExportJobStatus",
                "jobId": job_id
            },
            "requestId": "req-export-status"
        }))
        .expect("status command should return envelope");

        if last["data"]["phase"] == expected_value {
            return last;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("export job did not reach {expected:?}; last={last:#}");
}

fn export_draft(draft_id: &str) -> Draft {
    let mut draft = Draft::new(draft_id, "Export");
    draft.materials = vec![material(
        "video-material",
        MaterialKind::Video,
        "file:///media/video.mp4",
    )];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(segment("video-a", "video-material", 0, 0, 1_000_000));
    draft.tracks = vec![video_track];
    draft
}

fn export_draft_with_canvas(draft_id: &str, canvas_config: DraftCanvasConfig) -> Draft {
    let mut draft = export_draft(draft_id);
    draft.canvas_config = canvas_config;
    draft
}

fn material(material_id: &str, kind: MaterialKind, uri: &str) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(1_000_000));
    material.metadata.width = Some(1_920);
    material.metadata.height = Some(1_080);
    material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    material.metadata.has_video = true;
    material.metadata.has_audio = true;
    material
}

fn segment(
    segment_id: &str,
    material_id: &str,
    source_start: u64,
    target_start: u64,
    duration: u64,
) -> Segment {
    Segment::new(
        segment_id,
        material_id,
        SourceTimerange::new(Microseconds::new(source_start), Microseconds::new(duration)),
        TargetTimerange::new(Microseconds::new(target_start), Microseconds::new(duration)),
    )
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
            "video-editor-binding-export-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn ffmpeg_complete(&self) -> PathBuf {
        self.script(
            "ffmpeg",
            r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  printf 'ffmpeg version export-test\n'
  exit 0
fi
for arg in "$@"; do
  if [ "$arg" = "-encoders" ]; then
    printf ' V..... libx264 H.264 encoder\n'
    printf ' A..... aac AAC encoder\n'
    exit 0
  fi
  if [ "$arg" = "-filters" ]; then
    printf ' ... ass Render ASS subtitles\n'
    printf ' ... subtitles Render text subtitles\n'
    exit 0
  fi
done
last=""
for arg in "$@"; do
  last="$arg"
done
mkdir -p "$(dirname "$last")"
printf 'out_time_us=500000\n'
printf 'progress=continue\n'
printf 'fake mp4\n' > "$last"
printf 'out_time_us=1000000\n'
printf 'progress=end\n'
printf 'export complete\n' >&2
"#,
        )
    }

    fn ffmpeg_slow(&self) -> PathBuf {
        self.script(
            "ffmpeg",
            r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  printf 'ffmpeg version export-test\n'
  exit 0
fi
for arg in "$@"; do
  if [ "$arg" = "-encoders" ]; then
    printf ' V..... libx264 H.264 encoder\n'
    printf ' A..... aac AAC encoder\n'
    exit 0
  fi
  if [ "$arg" = "-filters" ]; then
    printf ' ... ass Render ASS subtitles\n'
    printf ' ... subtitles Render text subtitles\n'
    exit 0
  fi
done
printf 'out_time_us=100000\n'
printf 'export running\n' >&2
sleep 5
"#,
        )
    }

    fn ffprobe_success(&self, width: u32, height: u32, has_audio: bool) -> PathBuf {
        self.ffprobe_success_with_frame_rate(width, height, 30, 1, has_audio)
    }

    fn ffprobe_slow_success(&self, width: u32, height: u32, has_audio: bool) -> PathBuf {
        self.ffprobe_success_script(width, height, 30, 1, has_audio, "sleep 0.25\n")
    }

    fn ffprobe_success_with_frame_rate(
        &self,
        width: u32,
        height: u32,
        frame_rate_numerator: u32,
        frame_rate_denominator: u32,
        has_audio: bool,
    ) -> PathBuf {
        self.ffprobe_success_script(
            width,
            height,
            frame_rate_numerator,
            frame_rate_denominator,
            has_audio,
            "",
        )
    }

    fn ffprobe_success_script(
        &self,
        width: u32,
        height: u32,
        frame_rate_numerator: u32,
        frame_rate_denominator: u32,
        has_audio: bool,
        delay: &str,
    ) -> PathBuf {
        let audio_stream = if has_audio {
            r#",{"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2,"duration":"1.000000"}"#
        } else {
            ""
        };
        self.script(
            "ffprobe",
            &format!(
                r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  printf 'ffprobe version export-test\n'
  exit 0
fi
{delay}
cat <<'JSON'
{{"streams":[{{"codec_type":"video","codec_name":"h264","width":{width},"height":{height},"r_frame_rate":"{frame_rate_numerator}/{frame_rate_denominator}","duration":"1.000000"}}{audio_stream}],"format":{{"duration":"1.000000"}}}}
JSON
"#
            ),
        )
    }

    fn script(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, contents).unwrap();
        make_executable(&path);
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

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}
