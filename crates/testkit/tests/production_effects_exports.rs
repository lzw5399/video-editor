use std::time::{Duration, Instant};

use draft_import::AdaptationStatus;
use draft_model::{
    AudioRetimePolicy, Draft, Filter, Material, MaterialKind, MaterialMetadata, Microseconds,
    RationalFrameRate, RetimeMode, Segment, SegmentBlendMode, SegmentMask, SegmentRetiming,
    SourceTimerange, SpeedCurvePoint, SpeedRatio, TargetTimerange, TextSegment, TextStyle, Track,
    TrackKind, TrackTransition,
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
fn phase19_production_effects_export_complex_template_fixture_preserves_supported_subset_and_diagnostics()
 {
    let started = Instant::now();
    let plan = complex_export_plan();
    let job = compile_ffmpeg_job(&plan, &retime_compile_context())
        .expect("complex production effects export should compile supported subset");
    assert!(
        started.elapsed() <= Duration::from_secs(2),
        "complex production effects export compile exceeded fixture budget: {:?}",
        started.elapsed()
    );

    let layer = plan
        .graph
        .video_layers
        .iter()
        .find(|layer| layer.segment_id.as_str() == "video-a")
        .expect("complex export graph should contain the retimed effect layer");
    assert_eq!(
        layer.retime.source_mapping.retimed_source_timerange,
        SourceTimerange::new(Microseconds::ZERO, Microseconds::new(3_000_000)),
        "export graph must preserve the same retime mapping asserted by preview"
    );
    assert_eq!(layer.filters.len(), 3);
    assert!(
        layer
            .transition
            .as_ref()
            .is_some_and(|transition| transition.support == RenderIntentSupport::Supported),
        "complex export graph must carry supported dissolve transition intent"
    );
    assert_eq!(layer.mask.support, RenderIntentSupport::Supported);
    assert_eq!(layer.blend.blend_mode, SegmentBlendMode::Multiply);
    assert!(!plan.graph.text_overlays.is_empty());
    assert!(!plan.graph.audio_mixes.is_empty());

    for required_filter in [
        "setpts=",
        "atempo=1.500000",
        "gblur=sigma",
        "eq=brightness",
        "colorchannelmixer=aa",
        "geq=",
        "alpha(X,Y)",
        "xfade=transition=fade",
    ] {
        assert!(
            job.filter_script.contains(required_filter),
            "complex export filter script missing {required_filter}: {}",
            job.filter_script
        );
    }
    assert!(
        job.filter_script_diagnostics.is_empty(),
        "constant-speed supported subset should not create filter script diagnostics: {:?}",
        job.filter_script_diagnostics
    );
    assert!(
        job.visual_diagnostics.iter().any(|diagnostic| {
            diagnostic.property == "blendMode"
                && diagnostic.support == RenderIntentSupport::Unsupported
                && diagnostic
                    .reason
                    .contains("alpha-correct FFmpeg blend compositing")
        }),
        "multiply blend must remain an explicit export diagnostic, not fallback success: {:?}",
        job.visual_diagnostics
    );
    assert_eq!(
        complex_template_report_statuses(),
        vec![
            AdaptationStatus::Supported,
            AdaptationStatus::NeedsNativeEffect,
            AdaptationStatus::Dropped
        ],
        "complex fixture should carry supported/report-only template states"
    );
    assert_no_provider_or_fallback_tokens(&plan, &job);
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

fn complex_export_plan() -> RenderGraphPlan {
    let draft = complex_production_effects_draft();
    let normalized =
        normalize_draft(&draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    let target_timerange =
        TargetTimerange::new(Microseconds::new(1_800_000), Microseconds::new(700_000));
    let range = resolve_render_range(&normalized, target_timerange.clone())
        .expect("complex render range should resolve");
    let graph = build_render_graph(&normalized, &range).expect("complex render graph should build");
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            target_timerange,
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("complex export plan should validate")
}

fn complex_production_effects_draft() -> Draft {
    let mut draft = Draft::new(
        "phase19-complex-template-export",
        "Phase 19 Complex Template Export",
    );
    draft
        .materials
        .push(complex_video_material("video-material"));
    draft
        .materials
        .push(complex_video_material("video-material-b"));
    draft.materials.push(complex_audio_material());
    draft.materials.push(Material::new(
        "text-material",
        MaterialKind::Text,
        "text://phase19-title",
        "Phase 19 Title",
    ));

    let mut video_a = Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::ZERO, Microseconds::new(3_000_000)),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(2_000_000)),
    );
    video_a.retiming = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(3, 2),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    };
    video_a.filters = vec![
        Filter::gaussian_blur(1_400),
        Filter::basic_color_adjustment(-80, 1_100, 900),
        Filter::opacity_adjustment(760),
    ];
    video_a.visual.mask = SegmentMask::Rectangle {
        x_millis: 120,
        y_millis: 140,
        width_millis: 760,
        height_millis: 700,
        feather_millis: 80,
        opacity_millis: 900,
        inverted: false,
    };
    video_a.visual.blend_mode = SegmentBlendMode::Multiply;

    let video_b = Segment::new(
        "video-b",
        "video-material-b",
        SourceTimerange::new(Microseconds::ZERO, Microseconds::new(2_000_000)),
        TargetTimerange::new(Microseconds::new(1_700_000), Microseconds::new(1_000_000)),
    );
    let mut video_track = Track::new("video-track", TrackKind::Video, "Video");
    video_track.segments.push(video_a);
    video_track.segments.push(video_b);
    video_track.transitions.push(TrackTransition::dissolve(
        "video-a",
        "video-b",
        Microseconds::new(300_000),
    ));
    draft.tracks.push(video_track);

    let mut text = Segment::new(
        "text-a",
        "text-material",
        SourceTimerange::new(Microseconds::ZERO, Microseconds::new(2_000_000)),
        TargetTimerange::new(Microseconds::new(300_000), Microseconds::new(2_000_000)),
    );
    text.text = Some(TextSegment {
        content: "模板效果".to_owned(),
        source: Default::default(),
        style: TextStyle::default_title(),
        text_box: Default::default(),
        layout_region: Default::default(),
        wrapping: Default::default(),
        bubble: None,
        effect: None,
    });
    let mut text_track = Track::new("text-track", TrackKind::Text, "Text");
    text_track.segments.push(text);
    draft.tracks.push(text_track);

    let mut audio_track = Track::new("audio-track", TrackKind::Audio, "Audio");
    audio_track.segments.push(Segment::new(
        "audio-a",
        "audio-material",
        SourceTimerange::new(Microseconds::ZERO, Microseconds::new(3_000_000)),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(3_000_000)),
    ));
    draft.tracks.push(audio_track);

    draft
}

fn complex_video_material(material_id: &str) -> Material {
    let mut material = Material::new(
        material_id,
        MaterialKind::Video,
        format!("file://{material_id}.mp4"),
        material_id,
    );
    material.metadata = MaterialMetadata {
        duration: Some(Microseconds::new(4_000_000)),
        width: Some(1_920),
        height: Some(1_080),
        frame_rate: Some(RationalFrameRate::new(30, 1)),
        has_video: true,
        has_audio: true,
        audio_sample_rate: Some(48_000),
        audio_channels: Some(2),
        probe_error: None,
    };
    material
}

fn complex_audio_material() -> Material {
    let mut material = Material::new(
        "audio-material",
        MaterialKind::Audio,
        "file://complex-template-audio.m4a",
        "Complex Template Audio",
    );
    material.metadata = MaterialMetadata {
        duration: Some(Microseconds::new(3_000_000)),
        width: None,
        height: None,
        frame_rate: None,
        has_video: false,
        has_audio: true,
        audio_sample_rate: Some(48_000),
        audio_channels: Some(2),
        probe_error: None,
    };
    material
}

fn complex_template_report_statuses() -> Vec<AdaptationStatus> {
    vec![
        AdaptationStatus::Supported,
        AdaptationStatus::NeedsNativeEffect,
        AdaptationStatus::Dropped,
    ]
}

fn assert_no_provider_or_fallback_tokens(plan: &RenderGraphPlan, job: &ffmpeg_compiler::FfmpegJob) {
    let serialized = serde_json::to_string(&(plan, job)).expect("plan and job should serialize");
    for forbidden in [
        "provider-private",
        "native-effect",
        "beautyRetouch",
        "remoteRenderUrl",
        "renderUrl",
        "http://",
        "https://",
        "mock",
        "artifact",
        "cpuReadback",
        "domOverlay",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "complex export evidence leaked provider/fallback token {forbidden}: {serialized}"
        );
    }
}
