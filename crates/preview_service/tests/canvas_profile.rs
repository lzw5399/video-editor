use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground, Draft,
    DraftCanvasConfig, Material, MaterialKind, Microseconds, RationalFrameRate, Segment,
    SourceTimerange, TargetTimerange, Track, TrackKind,
};
use media_runtime::FfmpegExecutor;
use preview_service::{
    PreviewFrameRequest, PreviewSegmentRequest, PreviewServiceConfig, request_preview_frame,
    request_preview_segment,
};
use render_graph::{RenderCanvasBackgroundMode, RenderIntentSupport};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[test]
fn preview_frame_uses_draft_canvas_profile_and_preserves_vertical_aspect_ratio() {
    let temp = tempfile::tempdir().expect("cache temp dir");
    let executor = FakePreviewExecutor;
    let config = PreviewServiceConfig::new(temp.path(), "/bin/ffmpeg");
    let draft = draft_with_canvas(
        "draft-preview-vertical",
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio9x16),
            width: 1080,
            height: 1920,
            frame_rate: RationalFrameRate::new(24, 1),
            background: CanvasBackground::Black,
            adaptation_policy: CanvasAdaptationPolicy::Auto,
        },
    );

    let response = request_preview_frame(
        &executor,
        &config,
        &PreviewFrameRequest {
            draft,
            target_time: Microseconds::new(100_000),
        },
    )
    .expect("vertical preview frame should prepare");

    assert_eq!(response.ffmpeg_job.validation.expected_width, 304);
    assert_eq!(response.ffmpeg_job.validation.expected_height, 540);
    assert_eq!(
        response.ffmpeg_job.validation.expected_frame_rate,
        RationalFrameRate::new(24, 1)
    );
    assert_eq!(response.ffmpeg_job.encode_settings.dimensions.width, 304);
    assert_eq!(response.ffmpeg_job.encode_settings.dimensions.height, 540);
}

#[test]
fn preview_segment_uses_square_and_custom_draft_canvas_profiles() {
    let square = preview_segment_for(
        "draft-preview-square",
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio1x1),
            width: 1080,
            height: 1080,
            frame_rate: RationalFrameRate::new(25, 1),
            background: CanvasBackground::Black,
            adaptation_policy: CanvasAdaptationPolicy::Auto,
        },
    );
    assert_eq!(square.ffmpeg_job.validation.expected_width, 540);
    assert_eq!(square.ffmpeg_job.validation.expected_height, 540);
    assert_eq!(
        square.ffmpeg_job.validation.expected_frame_rate,
        RationalFrameRate::new(25, 1)
    );

    let custom = preview_segment_for(
        "draft-preview-custom",
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::custom(3, 2),
            width: 1200,
            height: 800,
            frame_rate: RationalFrameRate::new(48, 1),
            background: CanvasBackground::Black,
            adaptation_policy: CanvasAdaptationPolicy::Auto,
        },
    );
    assert_eq!(custom.ffmpeg_job.validation.expected_width, 810);
    assert_eq!(custom.ffmpeg_job.validation.expected_height, 540);
    assert_eq!(
        custom.ffmpeg_job.validation.expected_frame_rate,
        RationalFrameRate::new(48, 1)
    );
}

#[test]
fn preview_keeps_degraded_and_unsupported_canvas_background_diagnostics() {
    let degraded = preview_segment_for(
        "draft-preview-blur-background",
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
            width: 1920,
            height: 1080,
            frame_rate: RationalFrameRate::new(30, 1),
            background: CanvasBackground::BlurFill,
            adaptation_policy: CanvasAdaptationPolicy::Auto,
        },
    );
    assert_eq!(degraded.ffmpeg_job.canvas_diagnostics.len(), 1);
    assert_eq!(
        degraded.ffmpeg_job.canvas_diagnostics[0].mode,
        RenderCanvasBackgroundMode::BlurFill
    );
    assert_eq!(
        degraded.ffmpeg_job.canvas_diagnostics[0].support,
        RenderIntentSupport::Degraded
    );

    let unsupported = preview_segment_for(
        "draft-preview-image-background",
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
            width: 1920,
            height: 1080,
            frame_rate: RationalFrameRate::new(30, 1),
            background: CanvasBackground::Image { material_id: None },
            adaptation_policy: CanvasAdaptationPolicy::Auto,
        },
    );
    assert_eq!(unsupported.ffmpeg_job.canvas_diagnostics.len(), 1);
    assert_eq!(
        unsupported.ffmpeg_job.canvas_diagnostics[0].mode,
        RenderCanvasBackgroundMode::Image
    );
    assert_eq!(
        unsupported.ffmpeg_job.canvas_diagnostics[0].support,
        RenderIntentSupport::Unsupported
    );
}

fn preview_segment_for(
    draft_id: &str,
    canvas_config: DraftCanvasConfig,
) -> preview_service::PreviewSegmentResponse {
    let temp = tempfile::tempdir().expect("cache temp dir");
    let executor = FakePreviewExecutor;
    let config = PreviewServiceConfig::new(temp.path(), "/bin/ffmpeg");
    request_preview_segment(
        &executor,
        &config,
        &PreviewSegmentRequest {
            draft: draft_with_canvas(draft_id, canvas_config),
            target_timerange: TargetTimerange::new(
                Microseconds::new(100_000),
                Microseconds::new(100_000),
            ),
        },
    )
    .expect("preview segment should prepare")
}

struct FakePreviewExecutor;

impl FfmpegExecutor for FakePreviewExecutor {
    fn executor_name(&self) -> &'static str {
        "fake-canvas-preview-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, binary: &Path) -> io::Result<Output> {
        self.run(binary, &[])
    }

    fn run(&self, _binary: &Path, args: &[OsString]) -> io::Result<Output> {
        let output_path = args
            .last()
            .map(PathBuf::from)
            .expect("preview args should end with output path");
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output_path, b"preview artifact")?;
        Ok(Output {
            status: success_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

fn draft_with_canvas(draft_id: &str, canvas_config: DraftCanvasConfig) -> Draft {
    let mut draft = Draft::new(draft_id, "Canvas Preview");
    draft.canvas_config = canvas_config;
    draft.materials = vec![material("video-material")];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(segment("video-a", "video-material", 0, 0, 1_000_000));
    draft.tracks = vec![video_track];
    draft
}

fn material(material_id: &str) -> Material {
    let mut material = Material::new(
        material_id,
        MaterialKind::Video,
        "file:///media/video.mp4",
        material_id,
    );
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

#[cfg(unix)]
fn success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}
