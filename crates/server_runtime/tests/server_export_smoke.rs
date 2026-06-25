use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground, Draft,
    DraftCanvasConfig, ExportJobPhase, ExportPreset, Material, MaterialKind, Microseconds,
    RationalFrameRate, Segment, SourceTimerange, TargetTimerange, TextAlignment, TextSegment,
    TextSegmentSource, TextStyle, Track, TrackKind,
};
use media_runtime::discover_runtime_config;
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::{StdPlatformFileSystem, save_project_bundle};
use serde_json::Value;
use server_runtime::{
    ServerExportRequest, ServerRuntime, cancel_export, get_export_status, open_project,
    start_export, wait_for_export,
};
use testkit::{
    generate_audio_material_fixture, generate_image_material_fixture,
    generate_video_material_fixture,
};

const WIDTH: u32 = 160;
const HEIGHT: u32 = 90;
const FPS: u32 = 10;

#[test]
fn server_export_smoke_exports_bundle_relative_multimedia_project_and_validates_output() {
    let fixture = ServerSmokeFixture::new("complete", 1_000_000);
    let runtime = ServerRuntime::new().expect("server runtime should start");
    let opened = open_project(&runtime, &fixture.bundle_path).expect("project should open");
    assert!(opened.warnings.is_empty(), "{:#?}", opened.warnings);

    let output_path = fixture.output_path("server-complete.mp4");
    let started = start_export(
        &runtime,
        ServerExportRequest::new(
            opened.handle.clone(),
            output_path.clone(),
            ExportPreset::H264AacBalanced,
        ),
    )
    .expect("server export should start");
    assert_eq!(
        started.status.output_path,
        output_path.display().to_string()
    );
    assert!(
        started.scheduler.started_count >= 1,
        "scheduler should start export work: {started:#?}"
    );

    let completed = wait_for_export(&runtime, &started.status.job_id, Duration::from_secs(30))
        .expect("server export should reach a terminal phase");

    assert_eq!(completed.status.phase, ExportJobPhase::Completed);
    assert_eq!(completed.status.progress_per_mille, Some(1000));
    assert!(output_path.is_file(), "export output should exist");
    let validation = completed
        .status
        .validation
        .as_ref()
        .expect("completed export should include validation");
    assert_eq!(validation.width, Some(WIDTH));
    assert_eq!(validation.height, Some(HEIGHT));
    assert_eq!(validation.frame_rate, Some(RationalFrameRate::new(FPS, 1)));
    assert_eq!(validation.has_audio, true);
    assert!(validation.file_size_bytes > 0);
    assert!(
        completed.scheduler.completed_count >= 2,
        "export and validation scheduler jobs should complete: {completed:#?}"
    );
}

#[test]
fn server_export_progress_and_cancel_reports_scheduler_diagnostics() {
    let fixture = ServerSmokeFixture::new("cancel", 8_000_000);
    let runtime = ServerRuntime::new().expect("server runtime should start");
    let opened = open_project(&runtime, &fixture.bundle_path).expect("project should open");
    let output_path = fixture.output_path("server-cancel.mp4");
    let started = start_export(
        &runtime,
        ServerExportRequest::new(
            opened.handle.clone(),
            output_path,
            ExportPreset::H264AacBalanced,
        ),
    )
    .expect("server export should start");
    assert!(started.status.progress_per_mille.unwrap_or_default() <= 1000);

    let cancelled =
        cancel_export(&runtime, &started.status.job_id).expect("server export should cancel");
    assert_eq!(cancelled.status.phase, ExportJobPhase::Cancelled);
    assert_eq!(
        cancelled
            .status
            .diagnostic
            .as_ref()
            .map(|diagnostic| diagnostic.kind),
        Some(draft_model::ExportDiagnosticKind::Cancelled)
    );
    assert!(
        cancelled.scheduler.canceled_count >= 1 || cancelled.scheduler.started_count >= 1,
        "cancel should report scheduler telemetry: {cancelled:#?}"
    );

    std::thread::sleep(Duration::from_millis(200));
    let final_status =
        get_export_status(&runtime, &started.status.job_id).expect("status should remain readable");
    assert_eq!(final_status.status.phase, ExportJobPhase::Cancelled);
}

#[test]
fn server_runtime_cli_exports_bundle_and_prints_json_progress() {
    let fixture = ServerSmokeFixture::new("cli", 1_000_000);
    let output_path = fixture.output_path("server-cli.mp4");
    let output = Command::new(env!("CARGO_BIN_EXE_server_runtime"))
        .arg("export")
        .arg(&fixture.bundle_path)
        .arg(&output_path)
        .arg("h264-aac-draft")
        .output()
        .expect("server runtime CLI should launch");

    assert!(
        output.status.success(),
        "CLI failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let lines = String::from_utf8(output.stdout).expect("CLI stdout should be UTF-8");
    let events = lines
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("CLI line should be JSON"))
        .collect::<Vec<_>>();

    assert!(
        events.iter().any(|event| event["type"] == "opened"),
        "{events:#?}"
    );
    assert!(
        events.iter().any(|event| event["type"] == "started"),
        "{events:#?}"
    );
    let final_status = events
        .iter()
        .rev()
        .find(|event| event["type"] == "status")
        .expect("CLI should print status events");
    assert_eq!(final_status["status"]["phase"], "completed");
    assert_eq!(final_status["status"]["progressPerMille"], 1000);
    assert_eq!(final_status["status"]["validation"]["width"], WIDTH);
    assert_eq!(final_status["status"]["validation"]["height"], HEIGHT);
    assert!(output_path.is_file(), "CLI export output should exist");
}

struct ServerSmokeFixture {
    _temp_dir: tempfile::TempDir,
    bundle_path: PathBuf,
}

impl ServerSmokeFixture {
    fn new(name: &str, duration: u64) -> Self {
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("server-runtime-{name}-"))
            .tempdir()
            .expect("tempdir should be created");
        let bundle_path = temp_dir.path().join(format!("{name}.veproj"));
        let media_dir = bundle_path.join("media");
        fs::create_dir_all(&media_dir).expect("media dir should be created");

        let runtime = discover_runtime_config().expect("bundled runtime should be available");
        let executor = DesktopFfmpegExecutor::default();
        let video = generate_video_material_fixture(&executor, &runtime)
            .expect("video fixture should generate");
        let image = generate_image_material_fixture(&executor, &runtime)
            .expect("image fixture should generate");
        let audio = generate_audio_material_fixture(&executor, &runtime)
            .expect("audio fixture should generate");

        fs::copy(video.path(), media_dir.join("server-video.mp4"))
            .expect("video should be copied into bundle");
        fs::copy(image.path(), media_dir.join("server-image.png"))
            .expect("image should be copied into bundle");
        fs::copy(audio.path(), media_dir.join("server-audio.wav"))
            .expect("audio should be copied into bundle");

        let draft = multimedia_draft(duration);
        save_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
            .expect("project bundle should save");

        Self {
            _temp_dir: temp_dir,
            bundle_path,
        }
    }

    fn output_path(&self, name: &str) -> PathBuf {
        self.bundle_path.join("exports").join(name)
    }
}

fn multimedia_draft(duration: u64) -> Draft {
    let mut draft = Draft::new("server-runtime-draft", "Server Runtime Draft");
    draft.canvas_config = DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
        width: WIDTH,
        height: HEIGHT,
        frame_rate: RationalFrameRate::new(FPS, 1),
        background: CanvasBackground::Black,
        adaptation_policy: CanvasAdaptationPolicy::Auto,
    };
    draft.materials = vec![
        media_material(
            "video-material",
            MaterialKind::Video,
            "media/server-video.mp4",
            duration,
        ),
        media_material(
            "image-material",
            MaterialKind::Image,
            "media/server-image.png",
            duration,
        ),
        media_material(
            "audio-material",
            MaterialKind::Audio,
            "media/server-audio.wav",
            duration,
        ),
        Material::new(
            "text-material",
            MaterialKind::Text,
            "text://server-title",
            "服务器文字",
        ),
    ];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track.segments.push(segment(
        "video-a",
        "video-material",
        0,
        0,
        1_000_000.min(duration),
    ));

    let mut image_track = Track::new("image-track", TrackKind::Video, "图片");
    image_track
        .segments
        .push(segment("image-a", "image-material", 0, 0, duration));

    let mut audio_track = Track::new("audio-track", TrackKind::Audio, "音频");
    audio_track
        .segments
        .push(segment("audio-a", "audio-material", 0, 0, duration));

    let mut text_track = Track::new("text-track", TrackKind::Text, "文字");
    let mut text = segment("text-a", "text-material", 0, 100_000, duration / 2);
    text.text = Some(TextSegment {
        content: "服务器导出".to_owned(),
        source: TextSegmentSource::Text,
        style: TextStyle {
            font_size: 20,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            ..TextStyle::default()
        },
        text_box: Default::default(),
        layout_region: Default::default(),
        wrapping: Default::default(),
        bubble: None,
        effect: None,
    });
    text_track.segments.push(text);

    draft.tracks = vec![video_track, image_track, audio_track, text_track];
    draft
}

fn media_material(material_id: &str, kind: MaterialKind, uri: &str, duration: u64) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(duration));
    material.metadata.frame_rate = Some(RationalFrameRate::new(FPS, 1));
    material.metadata.has_video = matches!(kind, MaterialKind::Video | MaterialKind::Image);
    material.metadata.has_audio = matches!(kind, MaterialKind::Video | MaterialKind::Audio);
    match kind {
        MaterialKind::Video => {
            material.metadata.width = Some(WIDTH);
            material.metadata.height = Some(HEIGHT);
            material.metadata.audio_sample_rate = Some(44_100);
            material.metadata.audio_channels = Some(1);
        }
        MaterialKind::Image => {
            material.metadata.width = Some(80);
            material.metadata.height = Some(60);
        }
        MaterialKind::Audio => {
            material.metadata.audio_sample_rate = Some(44_100);
            material.metadata.audio_channels = Some(1);
        }
        _ => {}
    }
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

#[allow(dead_code)]
fn assert_inside_bundle(path: &Path, bundle: &Path) {
    assert!(
        path.starts_with(bundle),
        "{} should stay inside {}",
        path.display(),
        bundle.display()
    );
}
