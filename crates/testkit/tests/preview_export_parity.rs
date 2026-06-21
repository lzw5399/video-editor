use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasBackground, CommandDelta, CommandDeltaName,
    DirtyDomain, DirtyRange, DirtyRangeSource, Draft, DraftCanvasConfig, InvalidationScope,
    Material, MaterialId, MaterialKind, Microseconds, RationalFrameRate, Segment, SegmentOpacity,
    SourceTimerange, TargetTimerange, TextAlignment, TextBackground, TextBox, TextLayoutRegion,
    TextSegment, TextSegmentSource, TextShadow, TextStroke, TextStyle, TextWrapping, Track,
    TrackKind,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{CompileContext, FfmpegCompileErrorKind, compile_ffmpeg_job};
use media_runtime::{
    CancelToken, FfmpegExecutor, FfmpegJobState, FfmpegRuntimeJob, OutputValidationExpectation,
    RationalFrameRate as RuntimeFrameRate, RuntimeConfig, discover_runtime_config, run_export_job,
    validate_rendered_output,
};
use media_runtime_desktop::DesktopFfmpegExecutor;
use preview_service::{
    ExportPrepDirtyFacts, PreviewFrameRequest, PreviewInvalidationRequest, PreviewServiceConfig,
    request_preview_frame,
};
use render_graph::{
    ExportMp4Preset, OutputDimensions, RenderGraphDiff, RenderGraphPlan, RenderGraphSnapshot,
    RenderOutputProfile, build_render_graph,
};
use testkit::render_compare::{
    ComparableFrame, PixelTolerance, RenderCompareError, RenderCompareResult, RenderSetupErrorKind,
    assert_expected_frame_metadata, compare_rgb_frames, compiler_capabilities_from_probe_outputs,
    extract_rgb_frame_at, extract_rgb_frame_index, probe_phase5_render_capabilities,
    probe_video_frame_metadata,
};

const WIDTH: u32 = 160;
const HEIGHT: u32 = 90;
const FPS: u32 = 30;
const TARGET_TIME: u64 = 600_000;
const EXPORT_DURATION: u64 = 100_000;
const FRAME_DURATION: u64 = 33_334;
const SUBTITLE_EXPORT_DURATION: u64 = 3_000_000;
const SUBTITLE_CUE_ONE_TIME: u64 = 600_000;
const SUBTITLE_CUE_TWO_TIME: u64 = 1_800_000;

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
fn preview_export_parity_burns_two_cue_srt_text_into_frames() -> RenderCompareResult<()> {
    let runtime = discover_runtime_config()
        .map_err(|error| RenderCompareError::Runtime(format!("{error}: {}", error.remediation)))?;
    let executor = DesktopFfmpegExecutor::with_timeout(Duration::from_secs(20));
    let capabilities = probe_phase5_render_capabilities(&executor, &runtime)?;
    let sandbox = tempfile::tempdir()?;
    let video_path = sandbox.path().join("subtitle-video.mp4");
    let audio_path = sandbox.path().join("subtitle-audio.wav");
    generate_golden_video_duration(&executor, &runtime, &video_path, 3)?;
    generate_golden_audio_duration(&executor, &runtime, &audio_path, 3)?;

    let baseline_draft = subtitle_parity_draft(&video_path, &audio_path, false);
    let subtitled_draft = subtitle_parity_draft(&video_path, &audio_path, true);
    let preview_cache = sandbox.path().join("subtitle-preview-cache");
    let mut preview_config = PreviewServiceConfig::new(&preview_cache, &runtime.ffmpeg.path)
        .with_compiler_capabilities(capabilities.clone());
    preview_config.preview_frame_max_dimensions = OutputDimensions::new(WIDTH, HEIGHT);

    let preview = request_preview_frame(
        &executor,
        &preview_config,
        &PreviewFrameRequest {
            draft: subtitled_draft.clone(),
            target_time: Microseconds::new(SUBTITLE_CUE_ONE_TIME),
        },
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;

    let baseline_export_path = sandbox.path().join("exports/subtitle-baseline.mp4");
    let baseline_export_job = compile_export_job_for_range(
        &baseline_draft,
        &capabilities,
        &baseline_export_path,
        0,
        SUBTITLE_EXPORT_DURATION,
    )?;
    write_sidecars(&baseline_export_job)?;
    run_export_to_completion(
        &runtime,
        &baseline_export_path,
        &baseline_export_job,
        "phase5-srt-baseline-export",
    )?;

    let subtitled_export_path = sandbox.path().join("exports/subtitle-combo.mp4");
    let subtitled_export_job = compile_export_job_for_range(
        &subtitled_draft,
        &capabilities,
        &subtitled_export_path,
        0,
        SUBTITLE_EXPORT_DURATION,
    )?;
    write_sidecars(&subtitled_export_job)?;
    run_export_to_completion(
        &runtime,
        &subtitled_export_path,
        &subtitled_export_job,
        "phase5-srt-subtitled-export",
    )?;

    validate_rendered_output(
        &executor,
        &runtime,
        &subtitled_export_path,
        &OutputValidationExpectation::new()
            .with_expected_duration_microseconds(SUBTITLE_EXPORT_DURATION, FRAME_DURATION * 2)
            .with_expected_frame_rate(RuntimeFrameRate {
                numerator: FPS,
                denominator: 1,
            })
            .with_expected_dimensions(WIDTH, HEIGHT)
            .with_audio_stream(true),
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;

    let preview_frame = extract_rgb_frame_at(
        &executor,
        &runtime,
        &preview.artifact.path,
        0,
        0,
        WIDTH,
        HEIGHT,
    )?;
    let export_cue_one = extract_rgb_frame_index(
        &executor,
        &runtime,
        &subtitled_export_path,
        18,
        SUBTITLE_CUE_ONE_TIME,
        WIDTH,
        HEIGHT,
    )?;
    compare_rgb_frames(&preview_frame, &export_cue_one, PixelTolerance::phase5())?;

    let baseline_cue_one = extract_rgb_frame_index(
        &executor,
        &runtime,
        &baseline_export_path,
        18,
        SUBTITLE_CUE_ONE_TIME,
        WIDTH,
        HEIGHT,
    )?;
    let baseline_cue_two = extract_rgb_frame_index(
        &executor,
        &runtime,
        &baseline_export_path,
        54,
        SUBTITLE_CUE_TWO_TIME,
        WIDTH,
        HEIGHT,
    )?;
    let export_cue_two = extract_rgb_frame_index(
        &executor,
        &runtime,
        &subtitled_export_path,
        54,
        SUBTITLE_CUE_TWO_TIME,
        WIDTH,
        HEIGHT,
    )?;

    assert_region_changed(
        &baseline_cue_one,
        &export_cue_one,
        Region {
            x: 10,
            y: 50,
            width: 140,
            height: 36,
        },
        80,
        "first subtitle cue should alter the exported subtitle band",
    )?;
    assert_region_changed(
        &baseline_cue_two,
        &export_cue_two,
        Region {
            x: 10,
            y: 50,
            width: 140,
            height: 36,
        },
        80,
        "second subtitle cue should alter the exported subtitle band",
    )?;
    assert_region_changed(
        &export_cue_one,
        &export_cue_two,
        Region {
            x: 10,
            y: 50,
            width: 140,
            height: 36,
        },
        20,
        "two subtitle cues should burn different frame pixels",
    )?;

    assert!(
        preview.ffmpeg_job.filter_script.contains("subtitles="),
        "preview SRT parity path should include text sidecar rendering"
    );
    assert!(
        subtitled_export_job.filter_script.contains("subtitles="),
        "export SRT parity path should include text sidecar rendering"
    );

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
            adaptation_policy: CanvasAdaptationPolicy::Auto,
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

#[test]
fn preview_export_parity_dirty_facts_match_after_localized_edit_and_undo_redo()
-> RenderCompareResult<()> {
    let sandbox = tempfile::tempdir()?;
    let video_path = sandbox.path().join("video.mp4");
    let audio_path = sandbox.path().join("audio.wav");
    let before = golden_draft(&video_path, &audio_path);
    let mut edited = before.clone();
    edited.tracks[0].segments[0].target_timerange = TargetTimerange::new(100_000, 1_000_000);
    edited.tracks[0].segments[0].visual.transform.opacity = SegmentOpacity { value_millis: 700 };
    let restored = before.clone();

    let dirty_ranges = vec![
        dirty_range(0, 1_000_000, DirtyRangeSource::Previous),
        dirty_range(100_000, 1_000_000, DirtyRangeSource::Current),
    ];
    let delta = CommandDelta::targeted(
        CommandDeltaName::MoveSegment,
        Vec::new(),
        vec![DirtyDomain::Timing, DirtyDomain::Visual],
        dirty_ranges.clone(),
        InvalidationScope {
            full_draft: false,
            material_ids: vec![MaterialId::new("video-material")],
            graph_node_ids: vec![
                "draft:draft-phase5-parity:track:video-track:segment:video-a:video".to_owned(),
            ],
            consumer_domains: vec![DirtyDomain::PreviewCache, DirtyDomain::ExportPrep],
        },
        "localized parity edit",
    );
    let preview_request = PreviewInvalidationRequest::from_command_delta(&delta)
        .with_runtime_capability_fingerprint("runtime:software")
        .with_output_profile_fingerprint("output:preview-export");
    let export_facts = ExportPrepDirtyFacts::from_invalidation_request(&preview_request);

    assert_eq!(
        preview_request.dirty_ranges,
        vec![dirty_range(
            0,
            1_100_000,
            DirtyRangeSource::PreviousAndCurrent
        )]
    );
    assert_eq!(export_facts.dirty_ranges, preview_request.dirty_ranges);
    assert_eq!(
        export_facts.changed_material_ids,
        preview_request.changed_material_ids
    );
    assert_eq!(
        export_facts.changed_graph_node_keys,
        preview_request.changed_graph_node_keys
    );
    assert_eq!(
        export_facts.changed_domains,
        preview_request.changed_domains
    );
    assert!(
        export_facts
            .changed_domains
            .contains(&DirtyDomain::ExportPrep)
    );

    let before_snapshot = snapshot_for(&before, output_profile(160, 90), "runtime:software");
    let edited_snapshot = snapshot_for(&edited, output_profile(160, 90), "runtime:software");
    let restored_snapshot = snapshot_for(&restored, output_profile(160, 90), "runtime:software");
    let edit_diff = RenderGraphDiff::between(
        &before_snapshot,
        &edited_snapshot,
        &preview_request.dirty_ranges,
        &preview_request.changed_domains,
    );
    let undo_diff = RenderGraphDiff::between(
        &before_snapshot,
        &restored_snapshot,
        &preview_request.dirty_ranges,
        &preview_request.changed_domains,
    );

    assert!(edit_diff.added.is_empty());
    assert!(edit_diff.removed.is_empty());
    assert!(
        edit_diff
            .changed
            .iter()
            .any(|change| change.node_id.stable_key()
                == "draft:draft-phase5-parity:track:video-track:segment:video-a:video")
    );
    assert!(
        undo_diff.added.is_empty() && undo_diff.removed.is_empty() && undo_diff.changed.is_empty(),
        "restored undo snapshot should match the original graph exactly"
    );

    let capabilities = ffmpeg_compiler::CompilerCapabilities::all_available_for_tests();
    let preview_job =
        compile_preview_frame_job(&edited, &capabilities, &sandbox.path().join("preview.png"))?;
    let export_job =
        compile_export_job(&edited, &capabilities, &sandbox.path().join("export.mp4"))?;

    assert_eq!(
        preview_job.validation.expected_width,
        export_job.validation.expected_width
    );
    assert_eq!(
        preview_job.validation.expected_height,
        export_job.validation.expected_height
    );
    assert_eq!(
        preview_job.validation.expected_frame_rate,
        export_job.validation.expected_frame_rate
    );
    assert!(
        preview_job.filter_script.contains("subtitles=")
            && export_job.filter_script.contains("subtitles="),
        "preview and export should keep using the shared text render path"
    );

    Ok(())
}

fn compile_export_job(
    draft: &Draft,
    capabilities: &ffmpeg_compiler::CompilerCapabilities,
    output_path: &Path,
) -> RenderCompareResult<ffmpeg_compiler::FfmpegJob> {
    compile_export_job_for_range(
        draft,
        capabilities,
        output_path,
        TARGET_TIME,
        EXPORT_DURATION,
    )
}

fn compile_export_job_for_range(
    draft: &Draft,
    capabilities: &ffmpeg_compiler::CompilerCapabilities,
    output_path: &Path,
    target_start: u64,
    target_duration: u64,
) -> RenderCompareResult<ffmpeg_compiler::FfmpegJob> {
    let profile = EngineProfile::from_draft_canvas(draft)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let normalized = normalize_draft(draft, &profile)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(
            Microseconds::new(target_start),
            Microseconds::new(target_duration),
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
                Microseconds::new(target_start),
                Microseconds::new(target_duration),
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

fn snapshot_for(
    draft: &Draft,
    output_profile: RenderOutputProfile,
    runtime_capability_fingerprint: &str,
) -> RenderGraphSnapshot {
    let profile = EngineProfile::from_draft_canvas(draft).expect("canvas profile should resolve");
    let normalized = normalize_draft(draft, &profile).expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(
            Microseconds::new(TARGET_TIME),
            Microseconds::new(EXPORT_DURATION),
        ),
    )
    .expect("range should resolve");
    let graph = build_render_graph(&normalized, &range).expect("graph should build");
    RenderGraphSnapshot::from_graph(&graph, &output_profile, runtime_capability_fingerprint)
}

fn output_profile(width: u32, height: u32) -> RenderOutputProfile {
    RenderOutputProfile::preview_frame_png(
        OutputDimensions::new(width, height),
        RationalFrameRate::new(FPS, 1),
        TargetTimerange::new(
            Microseconds::new(TARGET_TIME),
            Microseconds::new(EXPORT_DURATION),
        ),
    )
}

fn dirty_range(start: u64, duration: u64, source: DirtyRangeSource) -> DirtyRange {
    DirtyRange {
        target_timerange: TargetTimerange::new(start, duration),
        source,
    }
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

fn run_export_to_completion(
    runtime: &RuntimeConfig,
    export_path: &Path,
    export_job: &ffmpeg_compiler::FfmpegJob,
    job_id: &str,
) -> RenderCompareResult<()> {
    let runtime_job = FfmpegRuntimeJob::new(
        job_id,
        runtime.ffmpeg.path.clone(),
        export_job.args.clone(),
        export_path,
    )
    .with_expected_duration_microseconds(SUBTITLE_EXPORT_DURATION)
    .with_timeout(Duration::from_secs(20));
    let export_result = run_export_job(&runtime_job, &CancelToken::new(), |_| {})
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    if export_result.state != FfmpegJobState::Completed {
        return Err(RenderCompareError::Assertion(format!(
            "expected export job to complete, got {:?}",
            export_result.state
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Region {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn assert_region_changed(
    before: &ComparableFrame,
    after: &ComparableFrame,
    region: Region,
    min_changed_pixels: usize,
    message: &str,
) -> RenderCompareResult<()> {
    if before.metadata.width != after.metadata.width
        || before.metadata.height != after.metadata.height
        || before.rgb24.len() != after.rgb24.len()
    {
        return Err(RenderCompareError::Assertion(
            "cannot compare subtitle region for mismatched frames".to_owned(),
        ));
    }
    let frame_width = before.metadata.width;
    let frame_height = before.metadata.height;
    if region.x.saturating_add(region.width) > frame_width
        || region.y.saturating_add(region.height) > frame_height
    {
        return Err(RenderCompareError::Assertion(format!(
            "subtitle evidence region {:?} exceeds frame {}x{}",
            region, frame_width, frame_height
        )));
    }

    let mut changed_pixels = 0_usize;
    for row in region.y..region.y + region.height {
        for column in region.x..region.x + region.width {
            let index = ((row as usize * frame_width as usize + column as usize) * 3)
                .min(before.rgb24.len().saturating_sub(3));
            let delta = before.rgb24[index].abs_diff(after.rgb24[index]) as u16
                + before.rgb24[index + 1].abs_diff(after.rgb24[index + 1]) as u16
                + before.rgb24[index + 2].abs_diff(after.rgb24[index + 2]) as u16;
            if delta > 32 {
                changed_pixels += 1;
            }
        }
    }

    if changed_pixels < min_changed_pixels {
        return Err(RenderCompareError::Assertion(format!(
            "{message}: expected at least {min_changed_pixels} changed pixels, got {changed_pixels}"
        )));
    }
    Ok(())
}

fn generate_golden_video(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
) -> RenderCompareResult<()> {
    generate_golden_video_duration(executor, runtime, output_path, 1)
}

fn generate_golden_video_duration(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
    duration_seconds: u32,
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
            &format!("color=c=0x1f6feb:size=160x90:rate=30:duration={duration_seconds}"),
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
    generate_golden_audio_duration(executor, runtime, output_path, 1)
}

fn generate_golden_audio_duration(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
    duration_seconds: u32,
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
            &format!("sine=frequency=660:sample_rate=44100:duration={duration_seconds}"),
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
            adaptation_policy: CanvasAdaptationPolicy::Auto,
        },
    )
}

fn subtitle_parity_draft(video_path: &Path, audio_path: &Path, include_subtitles: bool) -> Draft {
    let mut draft = Draft::new("draft-phase5-srt-parity", "Phase 5 SRT Parity");
    draft.canvas_config = DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::custom(WIDTH, HEIGHT),
        width: WIDTH,
        height: HEIGHT,
        frame_rate: RationalFrameRate::new(FPS, 1),
        background: CanvasBackground::Black,
        adaptation_policy: CanvasAdaptationPolicy::Auto,
    };
    draft.materials = vec![
        video_material_with_duration(video_path, SUBTITLE_EXPORT_DURATION),
        audio_material_with_duration(audio_path, SUBTITLE_EXPORT_DURATION),
        text_material_with_id("text-material", "组合文字"),
    ];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track.segments.push(segment(
        "video-a",
        "video-material",
        0,
        0,
        SUBTITLE_EXPORT_DURATION,
    ));

    let mut audio_track = Track::new("audio-track", TrackKind::Audio, "音频");
    audio_track.segments.push(segment(
        "audio-a",
        "audio-material",
        0,
        0,
        SUBTITLE_EXPORT_DURATION,
    ));

    let mut text_track = Track::new("text-track", TrackKind::Text, "文字");
    text_track.segments.push(text_segment(
        "text-a",
        "text-material",
        "产品级组合文字",
        TextSegmentSource::Text,
        TargetTimerange::new(0, SUBTITLE_EXPORT_DURATION),
        TextLayoutRegion {
            x_millis: 100,
            y_millis: 100,
            width_millis: 800,
            height_millis: 250,
        },
    ));

    if include_subtitles {
        draft.materials.push(text_material_with_id(
            "subtitle-material-1",
            "第一条组合字幕",
        ));
        draft.materials.push(text_material_with_id(
            "subtitle-material-2",
            "第二条完全不同字幕",
        ));
        text_track.segments.push(text_segment(
            "subtitle-a",
            "subtitle-material-1",
            "第一条组合字幕",
            TextSegmentSource::Subtitle,
            TargetTimerange::new(0, 1_400_000),
            subtitle_layout_region(),
        ));
        text_track.segments.push(text_segment(
            "subtitle-b",
            "subtitle-material-2",
            "第二条完全不同字幕",
            TextSegmentSource::Subtitle,
            TargetTimerange::new(1_400_000, 1_600_000),
            subtitle_layout_region(),
        ));
    }

    draft.tracks = vec![video_track, audio_track, text_track];
    draft
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
        source: TextSegmentSource::Text,
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
            ..TextStyle::default()
        },
        text_box: TextBox::default(),
        layout_region: TextLayoutRegion::default(),
        wrapping: TextWrapping::default(),
        bubble: None,
        effect: None,
    });
    text_track.segments.push(text);

    draft.tracks = vec![video_track, audio_track, text_track];
    draft
}

fn video_material(path: &Path) -> Material {
    video_material_with_duration(path, 1_000_000)
}

fn video_material_with_duration(path: &Path, duration: u64) -> Material {
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        file_uri(path),
        "视频素材",
    );
    material.metadata.duration = Some(Microseconds::new(duration));
    material.metadata.width = Some(WIDTH);
    material.metadata.height = Some(HEIGHT);
    material.metadata.frame_rate = Some(RationalFrameRate::new(FPS, 1));
    material.metadata.has_video = true;
    material
}

fn audio_material(path: &Path) -> Material {
    audio_material_with_duration(path, 1_000_000)
}

fn audio_material_with_duration(path: &Path, duration: u64) -> Material {
    let mut material = Material::new(
        "audio-material",
        MaterialKind::Audio,
        file_uri(path),
        "音频素材",
    );
    material.metadata.duration = Some(Microseconds::new(duration));
    material.metadata.has_audio = true;
    material.metadata.audio_sample_rate = Some(44_100);
    material.metadata.audio_channels = Some(1);
    material
}

fn text_material() -> Material {
    text_material_with_id("text-material", "标题文字")
}

fn text_material_with_id(material_id: &str, display_name: &str) -> Material {
    Material::new(
        material_id,
        MaterialKind::Text,
        format!("text://{material_id}"),
        display_name,
    )
}

fn text_segment(
    segment_id: &str,
    material_id: &str,
    content: &str,
    source: TextSegmentSource,
    target_timerange: TargetTimerange,
    layout_region: TextLayoutRegion,
) -> Segment {
    let mut segment = segment(
        segment_id,
        material_id,
        0,
        target_timerange.start.get(),
        target_timerange.duration.get(),
    );
    segment.text = Some(TextSegment {
        content: content.to_owned(),
        source,
        style: TextStyle {
            font_size: 20,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            stroke: Some(TextStroke {
                color: "#000000".to_owned(),
                width: 2,
            }),
            shadow: Some(TextShadow {
                color: "#000000".to_owned(),
                offset_x: 1,
                offset_y: 1,
                blur: 2,
            }),
            background: None,
            ..TextStyle::default()
        },
        text_box: TextBox {
            width_millis: 850,
            height_millis: 250,
        },
        layout_region,
        wrapping: TextWrapping::default(),
        bubble: None,
        effect: None,
    });
    segment
}

fn subtitle_layout_region() -> TextLayoutRegion {
    TextLayoutRegion {
        x_millis: 75,
        y_millis: 620,
        width_millis: 850,
        height_millis: 300,
    }
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
