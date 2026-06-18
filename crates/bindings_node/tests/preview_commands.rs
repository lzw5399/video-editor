use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::Mutex;

use bindings_node::execute_command;
use bindings_node::preview_export_service::{
    invalidate_preview_cache_command, request_preview_frame_with_executor,
    request_preview_segment_with_executor,
};
use draft_model::{
    CommandErrorKind, Draft, InvalidatePreviewCacheCommandPayload, Material, MaterialId,
    MaterialKind, Microseconds, PreviewCacheEntryRef, PreviewOutputProfile,
    RequestPreviewFrameCommandPayload, RequestPreviewSegmentCommandPayload, Segment,
    SourceTimerange, TargetTimerange, TextAlignment, TextBox, TextLayoutRegion, TextSegment,
    TextSegmentSource, TextStyle, TextWrapping, Track, TrackKind,
};
use media_runtime::FfmpegExecutor;
use preview_service::PreviewServiceConfig;
use serde_json::{Value, json};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[test]
fn preview_commands_request_frame_and_segment_through_preview_service_adapter() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executor = FakePreviewExecutor::successful();
    let config = PreviewServiceConfig::new(temp.path(), "/bin/ffmpeg");
    let draft = preview_draft();

    let frame = request_preview_frame_with_executor(
        &executor,
        &config,
        RequestPreviewFrameCommandPayload {
            draft: draft.clone(),
            cache_root: temp.path().display().to_string(),
            target_time: Microseconds::new(500_000),
        },
    )
    .expect("frame preview should route through preview_service");

    let segment = request_preview_segment_with_executor(
        &executor,
        &config,
        RequestPreviewSegmentCommandPayload {
            draft,
            cache_root: temp.path().display().to_string(),
            target_timerange: TargetTimerange::new(
                Microseconds::new(500_000),
                Microseconds::new(100_000),
            ),
        },
    )
    .expect("segment preview should route through preview_service");

    assert_eq!(frame.profile, PreviewOutputProfile::FramePng);
    assert_eq!(frame.mime_type, "image/png");
    assert_eq!(frame.status, draft_model::PreviewStatus::Generated);
    assert_eq!(segment.profile, PreviewOutputProfile::SegmentMp4);
    assert_eq!(segment.mime_type, "video/mp4");
    assert_eq!(executor.calls(), 2);
}

#[test]
fn preview_commands_invalidate_cache_without_mutating_draft() {
    let payload = InvalidatePreviewCacheCommandPayload {
        entries: vec![
            cache_entry_ref(
                "video.png",
                PreviewOutputProfile::FramePng,
                0,
                200_000,
                &["video"],
            ),
            cache_entry_ref(
                "text.png",
                PreviewOutputProfile::FramePng,
                400_000,
                100_000,
                &["text"],
            ),
            cache_entry_ref(
                "audio.mp4",
                PreviewOutputProfile::SegmentMp4,
                800_000,
                200_000,
                &["audio"],
            ),
        ],
        changed_ranges: vec![TargetTimerange::new(
            Microseconds::new(450_000),
            Microseconds::new(50_000),
        )],
        changed_material_ids: vec![MaterialId::new("audio")],
        reason: "accepted edit".to_owned(),
    };

    let response = invalidate_preview_cache_command(payload);

    assert_eq!(response.invalidated_count, 2);
    assert_eq!(response.retained_count, 1);

    let envelope = execute_command(json!({
        "command": "invalidatePreviewCache",
        "payload": {
            "kind": "invalidatePreviewCache",
            "entries": [
                {
                    "profile": "framePng",
                    "targetTimerange": { "start": 0, "duration": 200000 },
                    "materialDependencies": ["video"],
                    "artifactPath": "/cache/video.png"
                },
                {
                    "profile": "segmentMp4",
                    "targetTimerange": { "start": 800000, "duration": 200000 },
                    "materialDependencies": ["audio"],
                    "artifactPath": "/cache/audio.mp4"
                }
            ],
            "changedRanges": [],
            "changedMaterialIds": ["audio"],
            "reason": "material changed"
        },
        "requestId": "req-invalidate-preview"
    }))
    .expect("invalidate preview command should return envelope");

    assert_eq!(envelope["ok"], true, "{envelope:#}");
    assert_eq!(envelope["data"]["invalidatedCount"], 1);
    assert_eq!(envelope["data"]["retainedCount"], 1);
    assert_eq!(envelope["error"], Value::Null);
}

#[test]
fn preview_commands_reject_mismatched_preview_command_payload_pair() {
    let envelope = execute_command(json!({
        "command": "requestPreviewFrame",
        "payload": {
            "kind": "requestPreviewSegment",
            "draft": preview_draft(),
            "cacheRoot": "/cache",
            "targetTimerange": { "start": 0, "duration": 100000 }
        },
        "requestId": "req-preview-mismatch"
    }))
    .expect("mismatched preview command should return envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidPayload).unwrap()
    );
}

struct FakePreviewExecutor {
    calls: Mutex<usize>,
}

impl FakePreviewExecutor {
    fn successful() -> Self {
        Self {
            calls: Mutex::new(0),
        }
    }

    fn calls(&self) -> usize {
        *self.calls.lock().expect("calls lock")
    }
}

impl FfmpegExecutor for FakePreviewExecutor {
    fn executor_name(&self) -> &'static str {
        "fake-preview-binding-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, binary: &Path) -> io::Result<Output> {
        self.run(binary, &[])
    }

    fn run(&self, _binary: &Path, args: &[OsString]) -> io::Result<Output> {
        *self.calls.lock().expect("calls lock") += 1;
        let output_path = args
            .last()
            .map(PathBuf::from)
            .expect("preview args should end with output path");
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, b"preview")?;
        Ok(Output {
            status: success_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

fn preview_draft() -> Draft {
    let mut draft = Draft::new("draft-preview", "Preview");
    draft.materials = vec![
        material("video", MaterialKind::Video, "file:///media/video.mp4"),
        material("text", MaterialKind::Text, "text://title"),
    ];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(segment("video-a", "video", 0, 0, 1_000_000));

    let mut text_track = Track::new("text-track", TrackKind::Text, "文字");
    let mut text = segment("text-a", "text", 0, 500_000, 500_000);
    text.text = Some(TextSegment {
        content: "标题".to_owned(),
        source: TextSegmentSource::Text,
        style: TextStyle {
            font_size: 48,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            stroke: None,
            shadow: None,
            background: None,
            ..TextStyle::default()
        },
        text_box: TextBox::default(),
        layout_region: TextLayoutRegion::default(),
        wrapping: TextWrapping::default(),
        bubble: None,
        effect: None,
    });
    text_track.segments.push(text);
    draft.tracks = vec![video_track, text_track];
    draft
}

fn material(material_id: &str, kind: MaterialKind, uri: &str) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(2_000_000));
    match kind {
        MaterialKind::Video => {
            material.metadata.width = Some(1_920);
            material.metadata.height = Some(1_080);
            material.metadata.frame_rate = Some(draft_model::RationalFrameRate::new(30, 1));
            material.metadata.has_video = true;
            material.metadata.has_audio = true;
        }
        MaterialKind::Text => {}
        MaterialKind::Image | MaterialKind::Audio | MaterialKind::Sticker => {}
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

fn cache_entry_ref(
    artifact: &str,
    profile: PreviewOutputProfile,
    start: u64,
    duration: u64,
    material_ids: &[&str],
) -> PreviewCacheEntryRef {
    PreviewCacheEntryRef {
        profile,
        target_timerange: TargetTimerange::new(
            Microseconds::new(start),
            Microseconds::new(duration),
        ),
        material_dependencies: material_ids
            .iter()
            .map(|material_id| MaterialId::new(*material_id))
            .collect(),
        artifact_path: format!("/cache/{artifact}"),
    }
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}
