use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::Mutex;

use draft_model::{
    Draft, Material, MaterialKind, Microseconds, RationalFrameRate, Segment, SourceTimerange,
    TargetTimerange, TextAlignment, TextBox, TextLayoutRegion, TextSegment, TextSegmentSource,
    TextStyle, TextWrapping, Track, TrackKind,
};
use media_runtime::FfmpegExecutor;
use preview_service::{
    PreviewCacheProfile, PreviewFrameRequest, PreviewSegmentRequest, PreviewServiceConfig,
    PreviewServiceErrorKind, request_preview_frame, request_preview_segment,
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[test]
fn preview_generation_frame_uses_shared_engine_graph_compiler_path_and_returns_artifact() {
    let temp = tempfile::tempdir().expect("cache temp dir");
    let executor = FakePreviewExecutor::successful();
    let config = PreviewServiceConfig::new(temp.path(), "/bin/ffmpeg");
    let request = PreviewFrameRequest {
        draft: preview_draft(),
        target_time: Microseconds::new(600_000),
    };

    let response =
        request_preview_frame(&executor, &config, &request).expect("preview frame should generate");

    assert_eq!(response.artifact.profile, PreviewCacheProfile::FramePng);
    assert_eq!(response.artifact.mime_type, "image/png");
    assert!(response.artifact.path.ends_with(".png"));
    assert_eq!(response.ffmpeg_job.output_path, response.artifact.path);
    assert!(response.ffmpeg_job.filter_script.contains("subtitles="));
    assert_eq!(executor.calls(), 1);
    assert!(Path::new(&response.artifact.path).is_file());
    assert!(
        response
            .cache_entry
            .key
            .material_dependencies
            .iter()
            .any(|id| id.as_str() == "video-material")
    );
}

#[test]
fn preview_generation_segment_uses_same_compiler_path_and_reuses_existing_cache_artifact() {
    let temp = tempfile::tempdir().expect("cache temp dir");
    let executor = FakePreviewExecutor::successful();
    let config = PreviewServiceConfig::new(temp.path(), "/bin/ffmpeg");
    let request = PreviewSegmentRequest {
        draft: preview_draft(),
        target_timerange: TargetTimerange::new(
            Microseconds::new(600_000),
            Microseconds::new(100_000),
        ),
    };

    let first = request_preview_segment(&executor, &config, &request)
        .expect("preview segment should generate");
    let second = request_preview_segment(&executor, &config, &request)
        .expect("preview segment should come from cache");

    assert_eq!(first.artifact.profile, PreviewCacheProfile::SegmentMp4);
    assert_eq!(first.artifact.mime_type, "video/mp4");
    assert!(
        first
            .ffmpeg_job
            .args_as_strings()
            .contains(&"libx264".to_owned())
    );
    assert!(!first.from_cache);
    assert!(second.from_cache);
    assert_eq!(first.artifact.path, second.artifact.path);
    assert_eq!(executor.calls(), 1);
}

#[test]
fn preview_generation_classifies_runtime_failure_and_preserves_input_draft() {
    let temp = tempfile::tempdir().expect("cache temp dir");
    let executor = FakePreviewExecutor::failed();
    let config = PreviewServiceConfig::new(temp.path(), "/bin/ffmpeg");
    let draft = preview_draft();
    let before = serde_json::to_value(&draft).expect("draft should serialize");
    let request = PreviewFrameRequest {
        draft: draft.clone(),
        target_time: Microseconds::new(600_000),
    };

    let error = request_preview_frame(&executor, &config, &request)
        .expect_err("runtime failure should be classified");
    let after = serde_json::to_value(&draft).expect("draft should serialize");

    assert_eq!(error.kind, PreviewServiceErrorKind::RuntimeFailed);
    assert_eq!(before, after);
}

struct FakePreviewExecutor {
    behavior: FakePreviewBehavior,
    calls: Mutex<usize>,
}

enum FakePreviewBehavior {
    Successful,
    Failed,
}

impl FakePreviewExecutor {
    fn successful() -> Self {
        Self {
            behavior: FakePreviewBehavior::Successful,
            calls: Mutex::new(0),
        }
    }

    fn failed() -> Self {
        Self {
            behavior: FakePreviewBehavior::Failed,
            calls: Mutex::new(0),
        }
    }

    fn calls(&self) -> usize {
        *self.calls.lock().expect("call count lock")
    }
}

impl FfmpegExecutor for FakePreviewExecutor {
    fn executor_name(&self) -> &'static str {
        "fake-preview-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, binary: &Path) -> io::Result<Output> {
        self.run(binary, &[])
    }

    fn run(&self, _binary: &Path, args: &[OsString]) -> io::Result<Output> {
        *self.calls.lock().expect("call count lock") += 1;
        let output_path = args
            .last()
            .map(PathBuf::from)
            .expect("preview args should end with output path");

        match self.behavior {
            FakePreviewBehavior::Successful => {
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&output_path, b"preview artifact")?;
                Ok(success_output())
            }
            FakePreviewBehavior::Failed => Ok(failed_output()),
        }
    }
}

fn preview_draft() -> Draft {
    let mut draft = Draft::new("draft-preview", "Preview");
    draft.materials = vec![
        material(
            "video-material",
            MaterialKind::Video,
            "file:///media/video.mp4",
        ),
        material("text-material", MaterialKind::Text, "text://title"),
    ];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(segment("video-a", "video-material", 100_000, 0, 1_000_000));

    let mut text_track = Track::new("text-track", TrackKind::Text, "文字");
    let mut text = segment("text-a", "text-material", 0, 500_000, 500_000);
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
            material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
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

fn success_output() -> Output {
    Output {
        status: success_status(),
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

fn failed_output() -> Output {
    Output {
        status: failure_status(),
        stdout: b"preview stdout".to_vec(),
        stderr: b"preview stderr".to_vec(),
    }
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}

#[cfg(unix)]
fn failure_status() -> ExitStatus {
    ExitStatus::from_raw(1)
}
