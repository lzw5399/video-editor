mod common;

use draft_model::{
    AudioRetimePolicy, Draft, Microseconds, RationalFrameRate, RetimeMode, SegmentRetiming,
    SourceTimerange, SpeedRatio, TargetTimerange,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{FfmpegCompileError, compile_ffmpeg_job};
use render_graph::{
    ExportMp4Preset, OutputDimensions, RenderGraphPlan, RenderIntentSupport, RenderOutputProfile,
    build_render_graph,
};

const LIB_RS: &str = include_str!("../src/lib.rs");
const FILTERS_RS: &str = include_str!("../src/filters.rs");
const JOB_RS: &str = include_str!("../src/job.rs");

#[test]
fn phase19_production_effects_compiler_owns_filtergraph_output_from_render_intent() {
    assert!(
        LIB_RS.contains("production_effects") || FILTERS_RS.contains("compile_production_effect"),
        "ffmpeg_compiler must expose compiler-owned production effect filtergraph compilation"
    );
    assert!(
        FILTERS_RS.contains("RenderRetimeIntent")
            && FILTERS_RS.contains("RenderTransitionWindow")
            && FILTERS_RS.contains("ProductionEffectCapabilityDecision"),
        "compiler output must be derived from typed render graph retime, transition, and effect intents"
    );
}

#[test]
fn phase19_production_effects_compiler_classifies_unsupported_export_paths() {
    assert!(
        JOB_RS.contains("UnsupportedProductionEffect")
            || FILTERS_RS.contains("UnsupportedProductionEffect"),
        "unsupported Phase 19 export semantics must be classified instead of silently compiling fallback filters"
    );
    assert!(
        FILTERS_RS.contains("setpts") && FILTERS_RS.contains("xfade"),
        "retime and transition compiler support must be explicit FFmpeg compiler output, never renderer strings"
    );
}

#[test]
fn phase19_production_effects_compiler_emits_retime_filters_from_typed_graph_intent()
-> Result<(), FfmpegCompileError> {
    let plan = retimed_export_plan(AudioRetimePolicy::FollowVideoSpeed);
    let job = compile_ffmpeg_job(&plan, &common::compile_context())?;

    assert!(
        job.filter_script
            .contains("trim=start=0.100000:duration=2.000000"),
        "retimed video trim must use the engine-derived source mapping range"
    );
    assert!(
        job.filter_script.contains("setpts=(PTS-STARTPTS)*1/2"),
        "constant-speed video retime must be emitted as compiler-owned setpts"
    );
    assert!(
        job.filter_script.contains("atempo=2.000000"),
        "follow-speed audio retime must use an atempo-safe compiler-owned chain"
    );
    assert!(
        !job.filter_script.contains("filter_complex"),
        "filter scripts must not embed renderer-style FFmpeg command construction"
    );

    Ok(())
}

#[test]
fn phase19_production_effects_compiler_reports_unsupported_preserve_pitch_retime()
-> Result<(), FfmpegCompileError> {
    let plan = retimed_export_plan(AudioRetimePolicy::PreservePitch);
    let job = compile_ffmpeg_job(&plan, &common::compile_context())?;

    assert!(job.filter_script_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "retime.audio"
            && diagnostic.support == RenderIntentSupport::Unsupported
            && diagnostic.reason.contains("preserve-pitch")
    }));

    Ok(())
}

fn retimed_export_plan(audio_policy: AudioRetimePolicy) -> RenderGraphPlan {
    let mut draft = common::compiler_draft();
    let material = draft
        .materials
        .iter_mut()
        .find(|material| material.material_id.as_str() == "video-material")
        .expect("video material fixture exists");
    material.metadata.duration = Some(Microseconds::new(4_000_000));
    let segment = &mut draft.tracks[0].segments[0];
    segment.source_timerange =
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(3_000_000));
    segment.target_timerange =
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000));
    segment.retiming = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy,
    };
    graph_plan_from_draft(&draft)
}

fn graph_plan_from_draft(draft: &Draft) -> RenderGraphPlan {
    let normalized =
        normalize_draft(draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    let target_timerange = TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000));
    let range = resolve_render_range(&normalized, target_timerange.clone())
        .expect("range state should resolve");
    let graph = build_render_graph(&normalized, &range).expect("render graph should build");
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            target_timerange,
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("retimed export plan should validate")
}
