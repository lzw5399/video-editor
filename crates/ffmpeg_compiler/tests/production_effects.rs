mod common;

use draft_model::{
    AudioRetimePolicy, Draft, Filter, Material, MaterialKind, Microseconds, RationalFrameRate,
    RetimeMode, Segment, SegmentRetiming, SourceTimerange, SpeedRatio, TargetTimerange, Track,
    TrackKind, TrackTransition,
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

#[test]
fn phase19_production_effects_compiler_emits_dissolve_transition_from_graph_intent()
-> Result<(), FfmpegCompileError> {
    let plan = transition_export_plan(TrackTransition::dissolve(
        "left-segment",
        "right-segment",
        Microseconds::new(300_000),
    ));
    let job = compile_ffmpeg_job(&plan, &common::compile_context())?;

    assert!(
        job.filter_script
            .contains("xfade=transition=fade:duration=0.300000:offset=0.700000"),
        "first-party dissolve must compile to deterministic compiler-owned xfade timing"
    );
    assert!(
        job.filter_script
            .contains("[vtransition_left-segment_to_right-segment]"),
        "transition output label should be deterministic and endpoint-based"
    );
    assert!(
        !job.visual_diagnostics.iter().any(|diagnostic| {
            diagnostic.property == "transition"
                && diagnostic.support != RenderIntentSupport::Supported
        }),
        "supported dissolve transitions should not emit export unsupported diagnostics"
    );

    Ok(())
}

#[test]
fn phase19_production_effects_compiler_emits_chained_dissolve_transitions_without_label_reuse()
-> Result<(), FfmpegCompileError> {
    let plan = transition_chain_export_plan(vec![
        TrackTransition::dissolve("left-segment", "middle-segment", Microseconds::new(300_000)),
        TrackTransition::dissolve(
            "middle-segment",
            "right-segment",
            Microseconds::new(300_000),
        ),
    ]);
    let job = compile_ffmpeg_job(&plan, &common::compile_context())?;

    assert_eq!(
        job.filter_script
            .matches("xfade=transition=fade:duration=0.300000")
            .count(),
        2,
        "each canonical transition relationship should compile once"
    );
    assert!(
        job.filter_script
            .contains("[v1]split=3[v1main][v1transition0][v1transition1]"),
        "middle segment must be split once for main output plus both transition taps"
    );
    assert!(
        job.filter_script
            .contains("[vtransition_left-segment_to_middle-segment]")
            && job
                .filter_script
                .contains("[vtransition_middle-segment_to_right-segment]"),
        "transition labels should remain deterministic and endpoint-based"
    );

    Ok(())
}

#[test]
fn phase19_production_effects_compiler_reports_external_transition_without_success()
-> Result<(), FfmpegCompileError> {
    let plan = transition_export_plan(TrackTransition::external_reference(
        "left-segment",
        "right-segment",
        "jianying",
        "private-crossfade",
        Microseconds::new(300_000),
    ));
    let job = compile_ffmpeg_job(&plan, &common::compile_context())?;

    assert!(
        !job.filter_script.contains("private-crossfade"),
        "external transition identifiers must not become FFmpeg filter semantics"
    );
    assert!(job.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "transition"
            && diagnostic.support == RenderIntentSupport::Unsupported
            && diagnostic.reason.contains("external")
            && diagnostic.reason.contains("private-crossfade")
    }));

    Ok(())
}

#[test]
fn phase19_production_effects_compiler_emits_first_party_filter_stack_from_graph_intent()
-> Result<(), FfmpegCompileError> {
    let plan = effect_stack_export_plan(vec![
        Filter::gaussian_blur(250),
        Filter::basic_color_adjustment(120, 1_250, 800),
        Filter::opacity_adjustment(640),
        disabled_filter(Filter::gaussian_blur(900)),
    ]);
    let job = compile_ffmpeg_job(&plan, &common::compile_context())?;

    assert!(
        job.filter_script.contains("gblur=sigma=2.000000"),
        "Gaussian blur must compile from typed radius_millis into compiler-owned gblur output"
    );
    assert!(
        job.filter_script
            .contains("eq=brightness=0.120000:contrast=1.250000:saturation=0.800000"),
        "basic color adjustment must compile from typed millis into compiler-owned color output"
    );
    assert!(
        job.filter_script
            .contains("format=rgba,colorchannelmixer=aa=0.640000"),
        "opacity adjustment must compile from typed millis into compiler-owned alpha output"
    );
    assert!(
        !job.filter_script.contains("7.200000"),
        "disabled filters must stay in the render intent but must not emit active export filters"
    );

    let blur_index = job
        .filter_script
        .find("gblur=sigma=2.000000")
        .expect("blur filter should be present");
    let color_index = job
        .filter_script
        .find("eq=brightness=0.120000")
        .expect("color filter should be present");
    let opacity_index = job
        .filter_script
        .find("colorchannelmixer=aa=0.640000")
        .expect("opacity filter should be present");
    assert!(
        blur_index < color_index && color_index < opacity_index,
        "effect export order must follow render graph filter order"
    );
    assert!(
        !job.visual_diagnostics.iter().any(|diagnostic| {
            diagnostic.property == "filter"
                && diagnostic.support != RenderIntentSupport::Supported
        }),
        "supported first-party filters should not emit unsupported export diagnostics"
    );

    Ok(())
}

#[test]
fn phase19_production_effects_compiler_reports_external_filter_without_export_fallback()
-> Result<(), FfmpegCompileError> {
    let plan = effect_stack_export_plan(vec![
        Filter::external_reference("jianying", "private-glow"),
        Filter::gaussian_blur(250),
    ]);
    let job = compile_ffmpeg_job(&plan, &common::compile_context())?;

    assert!(
        !job.filter_script.contains("private-glow"),
        "external filter identifiers must not become FFmpeg filter semantics"
    );
    assert!(
        job.filter_script.contains("gblur=sigma=2.000000"),
        "supported first-party filters should still compile when adjacent external filters are diagnostics"
    );
    assert!(job.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "filter"
            && diagnostic.support == RenderIntentSupport::Unsupported
            && diagnostic.reason.contains("jianying:private-glow")
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

fn transition_export_plan(transition: TrackTransition) -> RenderGraphPlan {
    two_segment_transition_export_plan(transition)
}

fn transition_chain_export_plan(transitions: Vec<TrackTransition>) -> RenderGraphPlan {
    let mut draft = Draft::new(
        "phase19-transition-compiler",
        "Phase 19 Transition Compiler",
    );
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "file:///media/transition-source.mp4",
        "Transition Source",
    );
    material.metadata.duration = Some(Microseconds::new(4_000_000));
    material.metadata.width = Some(1_920);
    material.metadata.height = Some(1_080);
    material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    material.metadata.has_video = true;
    material.metadata.has_audio = false;
    draft.materials.push(material);

    let mut track = Track::new("video-track", TrackKind::Video, "视频");
    track
        .segments
        .push(transition_segment("left-segment", 0, 0, 1_000_000));
    track.segments.push(transition_segment(
        "middle-segment",
        1_000_000,
        1_000_000,
        1_000_000,
    ));
    track.segments.push(transition_segment(
        "right-segment",
        2_000_000,
        2_000_000,
        1_000_000,
    ));
    track.transitions.extend(transitions);
    draft.tracks.push(track);

    graph_plan_from_draft_with_range(
        &draft,
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(3_000_000)),
    )
}

fn two_segment_transition_export_plan(transition: TrackTransition) -> RenderGraphPlan {
    let mut draft = Draft::new(
        "phase19-transition-compiler",
        "Phase 19 Transition Compiler",
    );
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "file:///media/transition-source.mp4",
        "Transition Source",
    );
    material.metadata.duration = Some(Microseconds::new(4_000_000));
    material.metadata.width = Some(1_920);
    material.metadata.height = Some(1_080);
    material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    material.metadata.has_video = true;
    material.metadata.has_audio = false;
    draft.materials.push(material);

    let mut track = Track::new("video-track", TrackKind::Video, "视频");
    track
        .segments
        .push(transition_segment("left-segment", 0, 0, 1_000_000));
    track.segments.push(transition_segment(
        "right-segment",
        1_000_000,
        1_000_000,
        1_000_000,
    ));
    track.transitions.push(transition);
    draft.tracks.push(track);

    graph_plan_from_draft_with_range(
        &draft,
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(2_000_000)),
    )
}

fn transition_segment(
    segment_id: &str,
    source_start: u64,
    target_start: u64,
    duration: u64,
) -> Segment {
    Segment::new(
        segment_id,
        "video-material",
        SourceTimerange::new(Microseconds::new(source_start), Microseconds::new(duration)),
        TargetTimerange::new(Microseconds::new(target_start), Microseconds::new(duration)),
    )
}

fn effect_stack_export_plan(filters: Vec<Filter>) -> RenderGraphPlan {
    let mut draft = Draft::new(
        "phase19-effect-compiler",
        "Phase 19 Effect Compiler",
    );
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "file:///media/effect-source.mp4",
        "Effect Source",
    );
    material.metadata.duration = Some(Microseconds::new(4_000_000));
    material.metadata.width = Some(1_920);
    material.metadata.height = Some(1_080);
    material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    material.metadata.has_video = true;
    material.metadata.has_audio = false;
    draft.materials.push(material);

    let mut track = Track::new("video-track", TrackKind::Video, "视频");
    let mut segment = Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    );
    segment.filters = filters;
    track.segments.push(segment);
    draft.tracks.push(track);

    graph_plan_from_draft(&draft)
}

fn disabled_filter(mut filter: Filter) -> Filter {
    filter.enabled = false;
    filter
}

fn graph_plan_from_draft(draft: &Draft) -> RenderGraphPlan {
    graph_plan_from_draft_with_range(
        draft,
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    )
}

fn graph_plan_from_draft_with_range(
    draft: &Draft,
    target_timerange: TargetTimerange,
) -> RenderGraphPlan {
    let normalized =
        normalize_draft(draft, &EngineProfile::mvp_default()).expect("draft should normalize");
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
