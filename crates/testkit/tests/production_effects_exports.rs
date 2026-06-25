use draft_model::{
    AudioRetimePolicy, Draft, Microseconds, RationalFrameRate, RetimeMode, SegmentRetiming,
    SourceTimerange, SpeedCurvePoint, SpeedRatio, TargetTimerange,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{CompileContext, CompilerCapabilities, compile_ffmpeg_job};
use render_graph::{
    ExportMp4Preset, OutputDimensions, RenderGraphPlan, RenderIntentSupport, RenderOutputProfile,
    build_render_graph,
};

const PREVIEW_EXPORT_PARITY_RS: &str = include_str!("preview_export_parity.rs");
const PRODUCTION_EFFECTS_EXPORTS_RS: &str = include_str!("production_effects_exports.rs");
const PHASE19_SOURCE_GUARD_SH: &str = include_str!("../../../scripts/phase19-source-guards.sh");

#[test]
fn phase19_production_effects_export_fixtures_cover_preview_export_parity() {
    assert!(
        PRODUCTION_EFFECTS_EXPORTS_RS.contains(
            "phase19_production_effects_export_retime_constant_speed_matches_preview_source_mapping"
        ) && PRODUCTION_EFFECTS_EXPORTS_RS.contains(
            "phase19_production_effects_export_retime_speed_curve_keeps_degraded_audio_evidence"
        ) && occurrences(
            PRODUCTION_EFFECTS_EXPORTS_RS,
            "phase19_production_effects_export_complex_template_fixture_preserves_supported_subset_and_diagnostics",
        ) >= 2
            && PHASE19_SOURCE_GUARD_SH
                .contains("crates/testkit/tests/production_effects_exports.rs"),
        "production exports must verify retime preview/export semantic parity, not just output existence"
    );
}

fn occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

#[test]
fn phase19_production_effects_export_fixtures_reject_fallback_reports_as_success() {
    assert!(
        PRODUCTION_EFFECTS_EXPORTS_RS.contains("filter_script_diagnostics")
            && PRODUCTION_EFFECTS_EXPORTS_RS.contains("RenderIntentSupport::Degraded"),
        "Phase 19 export fixtures must keep degraded retime reports explicit instead of treating them as product success"
    );
}

#[test]
fn phase19_production_effects_export_retime_constant_speed_matches_preview_source_mapping() {
    let plan = retimed_export_plan(SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    });
    let layer = plan
        .graph
        .video_layers
        .first()
        .expect("constant-speed export graph should contain a video layer");
    let mix = plan
        .graph
        .audio_mixes
        .first()
        .expect("constant-speed export graph should contain an audio mix");
    let job = compile_ffmpeg_job(&plan, &retime_compile_context())
        .expect("constant-speed retime export should compile");

    assert_eq!(
        layer.retime.source_mapping.retimed_source_timerange,
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(2_000_000)),
        "export graph must use the same engine-owned source mapping asserted by preview parity"
    );
    assert_eq!(
        mix.retime.source_mapping, layer.retime.source_mapping,
        "export audio/video retime facts must stay synchronized before compiler output"
    );
    assert!(
        job.filter_script_diagnostics.is_empty(),
        "constant follow-speed retime should not need degraded export diagnostics: {:?}",
        job.filter_script_diagnostics
    );
}

#[test]
fn phase19_production_effects_export_retime_speed_curve_keeps_degraded_audio_evidence() {
    let plan = retimed_export_plan(SegmentRetiming {
        mode: RetimeMode::SpeedCurve {
            points: vec![
                SpeedCurvePoint {
                    target_time: Microseconds::ZERO,
                    speed: SpeedRatio::new(1, 1),
                },
                SpeedCurvePoint {
                    target_time: Microseconds::new(500_000),
                    speed: SpeedRatio::new(3, 2),
                },
            ],
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    });
    let layer = plan
        .graph
        .video_layers
        .first()
        .expect("speed-curve export graph should contain a video layer");
    let job =
        compile_ffmpeg_job(&plan, &retime_compile_context()).expect("speed-curve should compile");

    assert_eq!(
        layer.retime.support,
        RenderIntentSupport::Degraded,
        "speed-curve export must preserve degraded typed render intent"
    );
    assert!(
        job.filter_script_diagnostics.iter().any(|diagnostic| {
            diagnostic.property == "retime.audio"
                && diagnostic.support == RenderIntentSupport::Degraded
                && diagnostic.reason.contains("speed-curve audio retime")
        }),
        "speed-curve export must return typed audio retime diagnostics: {:?}",
        job.filter_script_diagnostics
    );
}

#[test]
fn phase19_production_effects_retime_source_guard_requires_export_coverage() {
    assert!(
        PREVIEW_EXPORT_PARITY_RS.contains("retime")
            || PRODUCTION_EFFECTS_EXPORTS_RS.contains("retime"),
        "testkit must keep retime preview/export parity evidence in product-path tests"
    );
    assert!(
        PHASE19_SOURCE_GUARD_SH.contains("FFMPEG_RETIME_FILTER_PATTERN")
            && PHASE19_SOURCE_GUARD_SH
                .contains("crates/testkit/tests/production_effects_exports.rs"),
        "retiming source guard must require export parity coverage and filter-string ownership scans"
    );
}

fn retimed_export_plan(retiming: SegmentRetiming) -> RenderGraphPlan {
    let mut draft = Draft::new("phase19-testkit-retime-export", "Phase 19 Retime Export");
    draft.materials.push(testkit_export_material());
    let mut segment = draft_model::Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(3_000_000)),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    );
    segment.retiming = retiming;
    let mut track = draft_model::Track::new("video-track", draft_model::TrackKind::Video, "Video");
    track.segments.push(segment);
    draft.tracks.push(track);

    let normalized =
        normalize_draft(&draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    let target_timerange = TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000));
    let range = resolve_render_range(&normalized, target_timerange.clone())
        .expect("render range should resolve");
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

fn testkit_export_material() -> draft_model::Material {
    let mut material = draft_model::Material::new(
        "video-material",
        draft_model::MaterialKind::Video,
        "file://retime-export.mp4",
        "Retime Export Video",
    );
    material.metadata.duration = Some(Microseconds::new(4_000_000));
    material.metadata.width = Some(1_920);
    material.metadata.height = Some(1_080);
    material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    material.metadata.has_video = true;
    material.metadata.has_audio = true;
    material
}

fn retime_compile_context() -> CompileContext {
    CompileContext::new("/derived/retime-export.mp4", "/derived")
        .with_capabilities(CompilerCapabilities::all_available_for_tests())
}
