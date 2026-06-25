use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bindings_node::{cancel_export, execute_command, get_export_job_status};
use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground, Draft,
    DraftCanvasConfig, ExportJobPhase, ExportPreset, Material, MaterialKind, Microseconds,
    RationalFrameRate, Segment, SourceTimerange, TargetTimerange, Track, TrackKind,
};
use media_runtime::replace_configured_bundled_runtime_directory_for_tests;
use serde_json::{Value, json};
use task_runtime::{
    JobDomain, JobEnvelope, JobFreshness, JobId, JobPriority, JobScheduler, PlaybackGeneration,
    ResourceBudget, ResourceClass, TaskCancellationToken, TaskRuntimeConfig,
    config::{QueueOverflowPolicy, QueuePolicy},
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn scheduler_export_source_guard_blocks_adapter_owned_export_authority() {
    let source = include_str!("../src/preview_export_service.rs");

    assert!(
        source.contains("editor_runtime"),
        "export binding must delegate export lifecycle authority to editor_runtime"
    );

    for forbidden in [
        "ExportJobRegistry",
        "SchedulerExportService",
        "SchedulerExportState",
        "prepare_export_job(",
        "run_scheduled_export(",
        "run_scheduled_validation(",
        "build_render_graph(",
        "compile_ffmpeg_job(",
        "media_runtime::run_export_job",
        "thread::spawn(move ||)",
    ] {
        assert!(
            !source.contains(forbidden),
            "export binding must not keep adapter-owned export semantics: {forbidden}"
        );
    }

    for required in [
        "editor_runtime::",
        "global_export_registry",
        "start_export(",
        "status(",
        "cancel(",
    ] {
        assert!(
            source.contains(required),
            "export binding must preserve explicit Node transport delegation: {required}"
        );
    }
}

#[test]
fn scheduler_export_start_status_completion_and_validation_report_scheduler_telemetry() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("complete-telemetry");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let output = sandbox.root.join("complete.mp4");

    let started = execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": export_draft("draft-scheduler-export-complete"),
            "outputPath": output,
            "preset": ExportPreset::H264AacBalanced
        },
        "requestId": "req-scheduler-export-start"
    }))
    .expect("start export should return envelope");

    assert_eq!(started["ok"], true, "{started:#}");
    assert_eq!(started["data"]["phase"], "running");
    let job_id = started["data"]["jobId"]
        .as_str()
        .expect("export job id should be present")
        .to_owned();
    assert_eq!(started["data"]["scheduler"]["jobId"], job_id);
    assert_eq!(started["data"]["scheduler"]["domain"], "export");
    assert_eq!(
        started["data"]["scheduler"]["resourceClass"],
        "ffmpegProcess"
    );
    assert_eq!(
        started["data"]["scheduler"]["validationResourceClass"],
        "validationProbe"
    );
    assert!(
        started["data"]["scheduler"]["startedCount"]
            .as_u64()
            .unwrap_or_default()
            >= 1,
        "{started:#}"
    );

    let completed = wait_for_export_phase(&job_id, ExportJobPhase::Completed);

    assert_eq!(completed["ok"], true, "{completed:#}");
    assert_eq!(completed["data"]["phase"], "completed");
    assert_eq!(completed["data"]["progressPerMille"], 1000);
    assert_eq!(completed["data"]["validation"]["width"], 1_920);
    assert_eq!(completed["data"]["validation"]["height"], 1_080);
    assert_eq!(completed["data"]["scheduler"]["jobId"], job_id);
    assert!(
        completed["data"]["scheduler"]["completedCount"]
            .as_u64()
            .unwrap_or_default()
            >= 2,
        "export and validation segments should both complete: {completed:#}"
    );
    assert!(
        completed["data"]["scheduler"]["runTimeUs"]["sampleCount"]
            .as_u64()
            .unwrap_or_default()
            >= 2,
        "scheduler run time telemetry must include export and validation: {completed:#}"
    );
    assert!(
        completed["data"]["scheduler"]["jobDurationUs"]["max"]
            .as_u64()
            .unwrap_or_default()
            > 0,
        "{completed:#}"
    );
}

#[test]
fn scheduler_export_cancel_queued_job_without_running_ffmpeg() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("queued-cancel");
    let _ffmpeg = sandbox.ffmpeg_slow_with_run_log();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let first_output = sandbox.root.join("running.mp4");
    let queued_output = sandbox.root.join("queued.mp4");

    let first = start_export("draft-scheduler-export-running", &first_output);
    assert_eq!(first["data"]["phase"], "running", "{first:#}");
    let first_job_id = first["data"]["jobId"].as_str().unwrap().to_owned();

    let queued = start_export("draft-scheduler-export-queued", &queued_output);
    assert_eq!(queued["ok"], true, "{queued:#}");
    assert_eq!(queued["data"]["phase"], "queued", "{queued:#}");
    let queued_job_id = queued["data"]["jobId"].as_str().unwrap().to_owned();

    let cancelled = cancel_export(json!({ "jobId": queued_job_id }))
        .expect("explicit queued cancel should return envelope");
    assert_eq!(cancelled["ok"], true, "{cancelled:#}");
    assert_eq!(cancelled["data"]["phase"], "cancelled");
    assert_eq!(cancelled["data"]["diagnostic"]["kind"], "cancelled");
    assert!(
        cancelled["data"]["scheduler"]["canceledCount"]
            .as_u64()
            .unwrap_or_default()
            >= 1,
        "{cancelled:#}"
    );

    let _ = cancel_export(json!({ "jobId": first_job_id }))
        .expect("cleanup running export should cancel");
    thread::sleep(Duration::from_millis(150));

    let runs = sandbox.ffmpeg_runs();
    assert!(
        !runs.contains(&queued_output.display().to_string()),
        "queued export should be cancelled before FFmpeg starts; runs={runs:#?}"
    );
}

#[test]
fn scheduler_export_cancel_running_job_propagates_to_ffmpeg_and_stays_terminal() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("running-cancel");
    let _ffmpeg = sandbox.ffmpeg_slow_with_run_log();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let output = sandbox.root.join("running-cancel.mp4");

    let started = start_export("draft-scheduler-export-cancel", &output);
    assert_eq!(started["ok"], true, "{started:#}");
    let job_id = started["data"]["jobId"].as_str().unwrap().to_owned();
    sandbox.wait_for_ffmpeg_run(&output);

    let cancelled = cancel_export(json!({ "jobId": job_id }))
        .expect("explicit running cancel should return envelope");
    assert_eq!(cancelled["ok"], true, "{cancelled:#}");
    assert_eq!(cancelled["data"]["phase"], "cancelled");
    assert_eq!(cancelled["data"]["diagnostic"]["kind"], "cancelled");
    assert!(
        cancelled["data"]["scheduler"]["runTimeUs"]["sampleCount"]
            .as_u64()
            .unwrap_or_default()
            >= 1,
        "running cancel must record scheduler run duration: {cancelled:#}"
    );

    thread::sleep(Duration::from_millis(200));
    let final_status = get_export_job_status(json!({ "jobId": job_id }))
        .expect("status after running cancel should return envelope");
    assert_eq!(final_status["ok"], true, "{final_status:#}");
    assert_eq!(final_status["data"]["phase"], "cancelled");
    assert_eq!(final_status["data"]["diagnostic"]["kind"], "cancelled");
}

#[test]
fn scheduler_export_interactive_lanes_start_under_export_saturation() {
    let mut scheduler = JobScheduler::new(export_starvation_config());

    scheduler
        .submit(export_envelope("export-running", 0))
        .expect("running export queues");
    scheduler
        .start_next(0)
        .expect("running export starts")
        .expect("export started");
    scheduler
        .submit(export_envelope("export-waiting", 1))
        .expect("waiting export queues behind saturated FFmpeg");
    scheduler
        .submit(interactive_envelope(
            "preview-frame",
            JobDomain::InteractivePreview,
            JobPriority::Realtime,
            ResourceClass::GpuPresent,
            2,
        ))
        .expect("preview queues");
    scheduler
        .submit(interactive_envelope(
            "scrub-seek",
            JobDomain::ScrubSeek,
            JobPriority::Interactive,
            ResourceClass::GpuPresent,
            2,
        ))
        .expect("scrub queues");
    scheduler
        .submit(interactive_envelope(
            "audio-refill",
            JobDomain::Audio,
            JobPriority::Realtime,
            ResourceClass::AudioRealtime,
            2,
        ))
        .expect("audio queues");

    let mut started = BTreeSet::new();
    for _ in 0..3 {
        let envelope = scheduler
            .start_next(3)
            .expect("interactive start scan succeeds")
            .expect("interactive lane starts while export resource is saturated");
        started.insert(envelope.job_id.as_str().to_owned());
    }

    assert_eq!(
        started,
        BTreeSet::from([
            "audio-refill".to_owned(),
            "preview-frame".to_owned(),
            "scrub-seek".to_owned(),
        ])
    );
    assert!(
        scheduler
            .start_next(3)
            .expect("saturated export scan succeeds")
            .is_none(),
        "waiting export should remain blocked by FFmpeg resource saturation"
    );
    let snapshot = scheduler.telemetry_snapshot();
    assert_eq!(snapshot.current_queue_depth, 1);
    assert!(
        snapshot
            .resource_saturation
            .iter()
            .any(|item| item.resource_class == ResourceClass::FfmpegProcess && item.count > 0),
        "{snapshot:#?}"
    );
    assert!(
        snapshot.queue_latency_us.p95.is_some_and(|p95| p95 <= 3),
        "{snapshot:#?}"
    );
}

#[test]
fn scheduler_export_queue_rejection_is_classified_when_export_capacity_is_full() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("queue-reject");
    let _ffmpeg = sandbox.ffmpeg_slow_with_run_log();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);

    let mut job_ids = Vec::new();
    for index in 0..5 {
        let output = sandbox.root.join(format!("queued-{index}.mp4"));
        let started = start_export(&format!("draft-scheduler-export-{index}"), &output);
        assert_eq!(started["ok"], true, "{started:#}");
        job_ids.push(started["data"]["jobId"].as_str().unwrap().to_owned());
    }

    let rejected_output = sandbox.root.join("rejected.mp4");
    let rejected = start_export("draft-scheduler-export-rejected", &rejected_output);
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert!(
        rejected["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("scheduler export queue rejected"),
        "{rejected:#}"
    );
    assert_eq!(
        rejected["data"]["diagnostic"]["kind"],
        json!("runtimeFailed"),
        "{rejected:#}"
    );

    for job_id in job_ids {
        let _ = cancel_export(json!({ "jobId": job_id }));
    }
}

fn start_export(draft_id: &str, output_path: &Path) -> Value {
    execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": export_draft(draft_id),
            "outputPath": output_path,
            "preset": ExportPreset::H264AacBalanced
        },
        "requestId": format!("req-{draft_id}")
    }))
    .expect("start export should return envelope")
}

fn wait_for_export_phase(job_id: &str, expected: ExportJobPhase) -> Value {
    let expected_value = serde_json::to_value(expected).unwrap();
    let mut last = Value::Null;
    for _ in 0..100 {
        last = get_export_job_status(json!({ "jobId": job_id }))
            .expect("explicit export status should return envelope");
        if last["data"]["phase"] == expected_value {
            return last;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("export job did not reach {expected:?}; last={last:#}");
}

fn export_draft(draft_id: &str) -> Draft {
    let mut draft = Draft::new(draft_id, "Scheduler Export");
    draft.canvas_config = DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
        width: 1_920,
        height: 1_080,
        frame_rate: RationalFrameRate::new(30, 1),
        background: CanvasBackground::Black,
        adaptation_policy: CanvasAdaptationPolicy::Auto,
    };
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

fn export_starvation_config() -> TaskRuntimeConfig {
    TaskRuntimeConfig {
        resource_budgets: vec![
            ResourceBudget {
                resource_class: ResourceClass::FfmpegProcess,
                max_in_flight: 1,
            },
            ResourceBudget {
                resource_class: ResourceClass::GpuPresent,
                max_in_flight: 2,
            },
            ResourceBudget {
                resource_class: ResourceClass::AudioRealtime,
                max_in_flight: 1,
            },
        ],
        queue_policies: vec![
            queue_policy(JobDomain::Export, 4, QueueOverflowPolicy::Reject),
            queue_policy(
                JobDomain::InteractivePreview,
                4,
                QueueOverflowPolicy::CoalesceObsolete,
            ),
            queue_policy(
                JobDomain::ScrubSeek,
                4,
                QueueOverflowPolicy::CoalesceObsolete,
            ),
            queue_policy(JobDomain::Audio, 4, QueueOverflowPolicy::CoalesceObsolete),
        ],
        telemetry_sample_limit: 64,
    }
}

fn queue_policy(
    domain: JobDomain,
    max_queued: usize,
    overflow: QueueOverflowPolicy,
) -> QueuePolicy {
    QueuePolicy {
        domain,
        max_queued,
        overflow,
    }
}

fn export_envelope(id: &str, submitted_at_us: u64) -> JobEnvelope {
    JobEnvelope::new(
        JobId::new(id),
        JobDomain::Export,
        JobPriority::UserVisible,
        ResourceClass::FfmpegProcess,
        TaskCancellationToken::new(submitted_at_us.saturating_add(10)),
        submitted_at_us,
    )
}

fn interactive_envelope(
    id: &str,
    domain: JobDomain,
    priority: JobPriority,
    resource_class: ResourceClass,
    submitted_at_us: u64,
) -> JobEnvelope {
    JobEnvelope::new(
        JobId::new(id),
        domain,
        priority,
        resource_class,
        TaskCancellationToken::new(submitted_at_us.saturating_add(20)),
        submitted_at_us,
    )
    .with_freshness(JobFreshness::timeline(
        Microseconds::new(33_333),
        PlaybackGeneration::new(1),
    ))
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
            "video-editor-binding-scheduler-export-{name}-{}-{nonce}",
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
  printf 'ffmpeg version scheduler-export-test\n'
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

    fn ffmpeg_slow_with_run_log(&self) -> PathBuf {
        let run_log = self.run_log_path();
        self.script(
            "ffmpeg",
            &format!(
                r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  printf 'ffmpeg version scheduler-export-test\n'
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
printf '%s\n' "$last" >> "{}"
printf 'out_time_us=100000\n'
printf 'export running\n' >&2
sleep 5
"#,
                run_log.display()
            ),
        )
    }

    fn ffprobe_success(&self, width: u32, height: u32, has_audio: bool) -> PathBuf {
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
  printf 'ffprobe version scheduler-export-test\n'
  exit 0
fi
cat <<'JSON'
{{"streams":[{{"codec_type":"video","codec_name":"h264","width":{width},"height":{height},"r_frame_rate":"30/1","duration":"1.000000"}}{audio_stream}],"format":{{"duration":"1.000000"}}}}
JSON
"#
            ),
        )
    }

    fn wait_for_ffmpeg_run(&self, output: &Path) {
        let needle = output.display().to_string();
        for _ in 0..100 {
            if self.ffmpeg_runs().contains(&needle) {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("FFmpeg did not start for {}", output.display());
    }

    fn ffmpeg_runs(&self) -> Vec<String> {
        fs::read_to_string(self.run_log_path())
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn run_log_path(&self) -> PathBuf {
        self.root.join("ffmpeg-runs.txt")
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
