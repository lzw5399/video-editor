use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use draft_model::{
    CanvasAspectRatio, CanvasBackground, Draft, DraftCanvasConfig, Material, MaterialKind,
    Microseconds, RationalFrameRate, Segment, SourceTimerange, TargetTimerange, TextAlignment,
    TextBackground, TextSegment, TextShadow, TextStroke, TextStyle, Track, TrackKind,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{CompileContext, FfmpegCompileErrorKind, compile_ffmpeg_job};
use media_runtime::{
    CancelToken, FfmpegExecutor, FfmpegJobState, FfmpegRuntimeJob, OutputValidationExpectation,
    RationalFrameRate as RuntimeFrameRate, RuntimeConfig, discover_runtime_config, run_export_job,
    validate_rendered_output,
};
use media_runtime_desktop::DesktopFfmpegExecutor;
use preview_service::{PreviewFrameRequest, PreviewServiceConfig, request_preview_frame};
use render_graph::{
    ExportMp4Preset, OutputDimensions, RenderGraphPlan, RenderOutputProfile, build_render_graph,
};
use testkit::render_compare::{
    PixelTolerance, RenderCompareError, RenderCompareResult, RenderSetupErrorKind,
    assert_expected_frame_metadata, compare_rgb_frames, compiler_capabilities_from_probe_outputs,
    extract_rgb_frame_at, probe_phase5_render_capabilities, probe_video_frame_metadata,
};

const WIDTH: u32 = 160;
const HEIGHT: u32 = 90;
const FPS: u32 = 30;
const TARGET_TIME: u64 = 600_000;
const EXPORT_DURATION: u64 = 100_000;
const FRAME_DURATION: u64 = 33_334;

#[test]
fn preview_export_parity_matches_shared_render_path_for_video_audio_text() -> RenderCompareResult<()>
{
    let runtime = discover_runtime_config()
        .map_err(|error| RenderCompareError::Runtime(format!("{error}: {}", error.remediation)))?;
    let executor = DesktopFfmpegExecutor::with_timeout(Duration::from_secs(20));
    let capabilities = probe_phase5_render_capabilities(&executor, &runtime)?;
    let sandbox = tempfile::tempdir()?;
    let video_path = sandbox.path().join("golden-video.mp4");
    let audio_path = sandbox.path().join("golden-audio.wav");
    generate_golden_video(&executor, &runtime, &video_path)?;
    generate_golden_audio(&executor, &runtime, &audio_path)?;

    let draft = golden_draft(&video_path, &audio_path);
    let preview_cache = sandbox.path().join("preview-cache");
    let mut preview_config = PreviewServiceConfig::new(&preview_cache, &runtime.ffmpeg.path)
        .with_compiler_capabilities(capabilities.clone());
    preview_config.preview_frame_max_dimensions = OutputDimensions::new(WIDTH, HEIGHT);
    preview_config.preview_segment_max_dimensions = OutputDimensions::new(WIDTH, HEIGHT);
    let preview = request_preview_frame(
        &executor,
        &preview_config,
        &PreviewFrameRequest {
            draft: draft.clone(),
            target_time: Microseconds::new(TARGET_TIME),
        },
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;

    let export_path = sandbox.path().join("exports/golden-export.mp4");
    let export_job = compile_export_job(&draft, &capabilities, &export_path)?;
    write_sidecars(&export_job)?;
    let mut export_events = Vec::new();
    let runtime_job = FfmpegRuntimeJob::new(
        "phase5-preview-export-parity",
        runtime.ffmpeg.path.clone(),
        export_job.args.clone(),
        &export_path,
    )
    .with_expected_duration_microseconds(EXPORT_DURATION)
    .with_timeout(Duration::from_secs(20));
    let export_result = run_export_job(&runtime_job, &CancelToken::new(), |event| {
        export_events.push(event);
    })
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    if export_result.state != FfmpegJobState::Completed {
        return Err(RenderCompareError::Assertion(format!(
            "expected export job to complete, got {:?}",
            export_result.state
        )));
    }

    validate_rendered_output(
        &executor,
        &runtime,
        &export_path,
        &OutputValidationExpectation::new()
            .with_expected_duration_microseconds(EXPORT_DURATION, FRAME_DURATION * 2)
            .with_expected_frame_rate(RuntimeFrameRate {
                numerator: FPS,
                denominator: 1,
            })
            .with_expected_dimensions(WIDTH, HEIGHT)
            .with_audio_stream(true),
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;

    let export_frames = probe_video_frame_metadata(&executor, &runtime, &export_path)?;
    let first_export_frame = export_frames.first().ok_or_else(|| {
        RenderCompareError::Assertion("export did not contain a video frame".to_owned())
    })?;
    assert_expected_frame_metadata(first_export_frame, 0, 0, FRAME_DURATION)?;

    let preview_frame = extract_rgb_frame_at(
        &executor,
        &runtime,
        &preview.artifact.path,
        0,
        0,
        WIDTH,
        HEIGHT,
    )?;
    let export_frame =
        extract_rgb_frame_at(&executor, &runtime, &export_path, 0, 0, WIDTH, HEIGHT)?;
    let comparison = compare_rgb_frames(&preview_frame, &export_frame, PixelTolerance::phase5())?;

    assert!(
        preview.ffmpeg_job.filter_script.contains("subtitles="),
        "preview path should include text sidecar rendering"
    );
    assert!(
        export_job.filter_script.contains("subtitles="),
        "export path should include text sidecar rendering"
    );
    assert!(
        export_events
            .iter()
            .any(|event| { matches!(event, media_runtime::FfmpegJobEvent::Started { .. }) }),
        "export runtime should emit job events"
    );
    assert_eq!(comparison.width, WIDTH);
    assert_eq!(comparison.height, HEIGHT);

    Ok(())
}

#[test]
fn preview_export_parity_compiles_custom_canvas_metadata_from_draft() -> RenderCompareResult<()> {
    let draft = draft_with_canvas(
        Path::new("/tmp/video.mp4"),
        Path::new("/tmp/audio.wav"),
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::custom(4, 5),
            width: 144,
            height: 180,
            frame_rate: RationalFrameRate::new(24, 1),
            background: CanvasBackground::Black,
        },
    );
    let sandbox = tempfile::tempdir()?;
    let capabilities = ffmpeg_compiler::CompilerCapabilities::all_available_for_tests();
    let preview_job =
        compile_preview_frame_job(&draft, &capabilities, &sandbox.path().join("preview.png"))?;
    let export_job = compile_export_job(&draft, &capabilities, &sandbox.path().join("export.mp4"))?;

    assert_eq!(preview_job.validation.expected_width, 144);
    assert_eq!(preview_job.validation.expected_height, 180);
    assert_eq!(
        preview_job.validation.expected_frame_rate,
        RationalFrameRate::new(24, 1)
    );
    assert_eq!(export_job.validation.expected_width, 144);
    assert_eq!(export_job.validation.expected_height, 180);
    assert_eq!(
        export_job.validation.expected_frame_rate,
        RationalFrameRate::new(24, 1)
    );

    Ok(())
}

#[test]
fn preview_export_parity_setup_failures_are_classified() {
    let missing_encoder = compiler_capabilities_from_probe_outputs(
        "Encoders:\n A..... aac",
        "Filters:\n ..C ass V->V\n ..C subtitles V->V",
        vec![PathBuf::from("/System/Library/Fonts/PingFang.ttc")],
        None,
    )
    .expect_err("missing libx264 should be a classified setup error");
    assert!(matches!(
        missing_encoder,
        RenderCompareError::Setup(error)
            if error.kind == RenderSetupErrorKind::MissingEncoder
                && error.remediation.contains("libx264")
    ));

    let missing_filter = compiler_capabilities_from_probe_outputs(
        "Encoders:\n V..... libx264\n A..... aac",
        "Filters:\n ..C subtitles V->V",
        vec![PathBuf::from("/System/Library/Fonts/PingFang.ttc")],
        None,
    )
    .expect_err("missing ASS filter should be a classified setup error");
    assert!(matches!(
        missing_filter,
        RenderCompareError::Setup(error)
            if error.kind == RenderSetupErrorKind::MissingFilter
                && error.remediation.contains("ASS")
    ));

    let missing_font = compiler_capabilities_from_probe_outputs(
        "Encoders:\n V..... libx264\n A..... aac",
        "Filters:\n ..C ass V->V\n ..C subtitles V->V",
        Vec::new(),
        None,
    )
    .expect_err("missing deterministic font should be a classified setup error");
    assert!(matches!(
        missing_font,
        RenderCompareError::Setup(error)
            if error.kind == RenderSetupErrorKind::MissingFont
                && error.remediation.contains("VE_TEXT_FONT_PATH")
    ));

    let compile_error = compile_ffmpeg_job(
        &export_plan_for_compile_error(),
        &CompileContext::new("/tmp/out.mp4", "/tmp").with_capabilities(
            ffmpeg_compiler::CompilerCapabilities::all_available_for_tests()
                .with_h264_encoder(false),
        ),
    )
    .expect_err("compiler should classify missing export encoder");
    assert_eq!(
        compile_error.kind,
        FfmpegCompileErrorKind::UnsupportedEncoder
    );
}

fn compile_export_job(
    draft: &Draft,
    capabilities: &ffmpeg_compiler::CompilerCapabilities,
    output_path: &Path,
) -> RenderCompareResult<ffmpeg_compiler::FfmpegJob> {
    let profile = EngineProfile::from_draft_canvas(draft)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let normalized = normalize_draft(draft, &profile)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(
            Microseconds::new(TARGET_TIME),
            Microseconds::new(EXPORT_DURATION),
        ),
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let graph = build_render_graph(&normalized, &range)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let plan = RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(profile.canvas_width, profile.canvas_height),
            range.frame_rate.clone(),
            TargetTimerange::new(
                Microseconds::new(TARGET_TIME),
                Microseconds::new(EXPORT_DURATION),
            ),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let artifact_dir = output_path
        .parent()
        .ok_or_else(|| RenderCompareError::Runtime("export output path has no parent".to_owned()))?
        .join("sidecars");
    let context =
        CompileContext::new(output_path, &artifact_dir).with_capabilities(capabilities.clone());
    compile_ffmpeg_job(&plan, &context)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))
}

fn compile_preview_frame_job(
    draft: &Draft,
    capabilities: &ffmpeg_compiler::CompilerCapabilities,
    output_path: &Path,
) -> RenderCompareResult<ffmpeg_compiler::FfmpegJob> {
    let profile = EngineProfile::from_draft_canvas(draft)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let normalized = normalize_draft(draft, &profile)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(
            Microseconds::new(TARGET_TIME),
            Microseconds::new(EXPORT_DURATION),
        ),
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let graph = build_render_graph(&normalized, &range)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let plan = RenderGraphPlan::new(
        graph,
        RenderOutputProfile::preview_frame_png(
            OutputDimensions::new(profile.canvas_width, profile.canvas_height),
            range.frame_rate.clone(),
            TargetTimerange::new(
                Microseconds::new(TARGET_TIME),
                Microseconds::new(EXPORT_DURATION),
            ),
        ),
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let artifact_dir = output_path
        .parent()
        .ok_or_else(|| RenderCompareError::Runtime("preview output path has no parent".to_owned()))?
        .join("sidecars");
    let context =
        CompileContext::new(output_path, &artifact_dir).with_capabilities(capabilities.clone());
    compile_ffmpeg_job(&plan, &context)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))
}

fn export_plan_for_compile_error() -> RenderGraphPlan {
    let draft = golden_draft(Path::new("/tmp/video.mp4"), Path::new("/tmp/audio.wav"));
    let profile = EngineProfile::from_draft_canvas(&draft).expect("golden draft profile");
    let normalized = normalize_draft(&draft, &profile).expect("golden draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(
            Microseconds::new(TARGET_TIME),
            Microseconds::new(EXPORT_DURATION),
        ),
    )
    .expect("range should resolve");
    let graph = build_render_graph(&normalized, &range).expect("graph should build");
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(profile.canvas_width, profile.canvas_height),
            range.frame_rate.clone(),
            TargetTimerange::new(
                Microseconds::new(TARGET_TIME),
                Microseconds::new(EXPORT_DURATION),
            ),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("plan should build")
}

fn write_sidecars(job: &ffmpeg_compiler::FfmpegJob) -> RenderCompareResult<()> {
    for sidecar in &job.sidecars {
        let path = Path::new(&sidecar.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, sidecar.contents.as_bytes())?;
    }
    Ok(())
}

fn generate_golden_video(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
) -> RenderCompareResult<()> {
    run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x1f6feb:size=160x90:rate=30:duration=1",
            "-an",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ],
        output_path,
    )
}

fn generate_golden_audio(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
) -> RenderCompareResult<()> {
    run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=660:sample_rate=44100:duration=1",
            "-ac",
            "1",
        ],
        output_path,
    )
}

fn run_ffmpeg(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    args: &[&str],
    output_path: &Path,
) -> RenderCompareResult<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut args = args.iter().map(OsString::from).collect::<Vec<_>>();
    args.push(output_path.as_os_str().to_owned());
    let output = executor.run(&runtime.ffmpeg.path, &args).map_err(|error| {
        RenderCompareError::Runtime(format!(
            "failed to run FFmpeg fixture generation at {}: {error}",
            runtime.ffmpeg.path.display()
        ))
    })?;
    if !output.status.success() {
        return Err(RenderCompareError::Runtime(format!(
            "FFmpeg fixture generation failed: stdout=`{}` stderr=`{}`",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

fn golden_draft(video_path: &Path, audio_path: &Path) -> Draft {
    draft_with_canvas(
        video_path,
        audio_path,
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::custom(WIDTH, HEIGHT),
            width: WIDTH,
            height: HEIGHT,
            frame_rate: RationalFrameRate::new(FPS, 1),
            background: CanvasBackground::Black,
        },
    )
}

fn draft_with_canvas(
    video_path: &Path,
    audio_path: &Path,
    canvas_config: DraftCanvasConfig,
) -> Draft {
    let mut draft = Draft::new("draft-phase5-parity", "Phase 5 Parity");
    draft.canvas_config = canvas_config;
    draft.materials = vec![
        video_material(video_path),
        audio_material(audio_path),
        text_material(),
    ];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(segment("video-a", "video-material", 0, 0, 1_000_000));

    let mut audio_track = Track::new("audio-track", TrackKind::Audio, "音频");
    audio_track
        .segments
        .push(segment("audio-a", "audio-material", 0, 0, 1_000_000));

    let mut text_track = Track::new("text-track", TrackKind::Text, "文字");
    let mut text = segment("text-a", "text-material", 0, 300_000, 600_000);
    text.text = Some(TextSegment {
        content: "标题".to_owned(),
        style: TextStyle {
            font_size: 48,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            stroke: Some(TextStroke {
                color: "#101010".to_owned(),
                width: 2,
            }),
            shadow: Some(TextShadow {
                color: "#000000".to_owned(),
                offset_x: 1,
                offset_y: 1,
                blur: 2,
            }),
            background: Some(TextBackground {
                color: "#202020".to_owned(),
            }),
        },
    });
    text_track.segments.push(text);

    draft.tracks = vec![video_track, audio_track, text_track];
    draft
}

fn video_material(path: &Path) -> Material {
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        file_uri(path),
        "视频素材",
    );
    material.metadata.duration = Some(Microseconds::new(1_000_000));
    material.metadata.width = Some(WIDTH);
    material.metadata.height = Some(HEIGHT);
    material.metadata.frame_rate = Some(RationalFrameRate::new(FPS, 1));
    material.metadata.has_video = true;
    material
}

fn audio_material(path: &Path) -> Material {
    let mut material = Material::new(
        "audio-material",
        MaterialKind::Audio,
        file_uri(path),
        "音频素材",
    );
    material.metadata.duration = Some(Microseconds::new(1_000_000));
    material.metadata.has_audio = true;
    material.metadata.audio_sample_rate = Some(44_100);
    material.metadata.audio_channels = Some(1);
    material
}

fn text_material() -> Material {
    Material::new(
        "text-material",
        MaterialKind::Text,
        "text://phase5-title",
        "标题文字",
    )
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

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}
