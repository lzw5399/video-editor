use draft_model::{
    CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground, Draft, DraftCanvasConfig,
    Microseconds, RationalFrameRate, TargetTimerange,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{CompileContext, CompilerCapabilities, compile_ffmpeg_job};
use render_graph::{
    ExportMp4Preset, OutputDimensions, RenderGraphPlan, RenderOutputProfile, build_render_graph,
};

#[test]
fn export_encode_settings_and_validation_use_vertical_draft_canvas_profile() {
    let plan = export_plan_from_draft_canvas(DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio9x16),
        width: 1_080,
        height: 1_920,
        frame_rate: RationalFrameRate::new(25, 1),
        background: CanvasBackground::Black,
    });
    let job = compile_ffmpeg_job(&plan, &compile_context())
        .expect("vertical canvas export job should compile");

    assert_eq!(job.encode_settings.dimensions.width, 1_080);
    assert_eq!(job.encode_settings.dimensions.height, 1_920);
    assert_eq!(
        job.encode_settings.frame_rate,
        RationalFrameRate::new(25, 1)
    );
    assert_eq!(job.validation.expected_width, 1_080);
    assert_eq!(job.validation.expected_height, 1_920);
    assert_eq!(
        job.validation.expected_frame_rate,
        RationalFrameRate::new(25, 1)
    );
}

#[test]
fn export_encode_settings_and_validation_use_square_custom_draft_canvas_profile() {
    let plan = export_plan_from_draft_canvas(DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::custom(1, 1),
        width: 1_024,
        height: 1_024,
        frame_rate: RationalFrameRate::new(48, 1),
        background: CanvasBackground::SolidColor {
            color: "#222222".to_owned(),
        },
    });
    let job = compile_ffmpeg_job(&plan, &compile_context())
        .expect("square canvas export job should compile");

    assert_eq!(job.encode_settings.dimensions.width, 1_024);
    assert_eq!(job.encode_settings.dimensions.height, 1_024);
    assert_eq!(
        job.encode_settings.frame_rate,
        RationalFrameRate::new(48, 1)
    );
    assert_eq!(job.validation.expected_width, 1_024);
    assert_eq!(job.validation.expected_height, 1_024);
    assert_eq!(
        job.validation.expected_frame_rate,
        RationalFrameRate::new(48, 1)
    );
}

#[test]
fn unsupported_image_canvas_background_is_visible_in_compiled_job_diagnostics() {
    let plan = export_plan_from_draft_canvas(DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
        width: 1_280,
        height: 720,
        frame_rate: RationalFrameRate::new(30, 1),
        background: CanvasBackground::Image { material_id: None },
    });
    let job = compile_ffmpeg_job(&plan, &compile_context())
        .expect("diagnostic-only image background job should still compile");
    let snapshot = serde_json::to_value(&job).expect("job should serialize");

    assert_eq!(
        snapshot["canvasDiagnostics"],
        serde_json::json!([
            {
                "mode": "image",
                "support": "unsupported",
                "reason": "image canvas background is unsupported until background material rendering is implemented"
            }
        ])
    );
    assert!(!job.filter_script.contains("background-image"));
    assert!(!job.filter_script.contains("gblur"));
}

fn export_plan_from_draft_canvas(canvas_config: DraftCanvasConfig) -> RenderGraphPlan {
    let mut draft = Draft::new("draft-compiler-canvas", "Compiler Canvas");
    draft.canvas_config = canvas_config;

    let profile =
        EngineProfile::from_draft_canvas(&draft).expect("draft canvas profile should resolve");
    let normalized = normalize_draft(&draft, &profile).expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(100_000)),
    )
    .expect("range state should resolve");
    let graph = build_render_graph(&normalized, &range).expect("graph should build");
    let dimensions = OutputDimensions::new(graph.canvas.width, graph.canvas.height);
    let frame_rate = graph.frame_rate.clone();

    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            dimensions,
            frame_rate,
            TargetTimerange::new(Microseconds::ZERO, Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export profile should validate")
}

fn compile_context() -> CompileContext {
    CompileContext::new("/derived/canvas-output.mp4", "/derived")
        .with_capabilities(CompilerCapabilities::all_available_for_tests())
}
