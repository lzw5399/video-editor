use std::time::{Duration, Instant};

use draft_import::AdaptationStatus;
use draft_model::{
    AudioRetimePolicy, Draft, Filter, Material, MaterialKind, MaterialMetadata, Microseconds,
    RationalFrameRate, RetimeMode, Segment, SegmentBlendMode, SegmentMask, SegmentRetiming,
    SourceTimerange, SpeedCurvePoint, SpeedRatio, TargetTimerange, TextSegment, TextStyle, Track,
    TrackKind, TrackTransition,
};
use realtime_preview_runtime::{
    RealtimePreviewCapabilityClassifier, RealtimePreviewGraphInput, RealtimePreviewGraphSupport,
    RealtimePreviewSupport, prepare_realtime_preview_graph,
};
use render_graph::{OutputDimensions, RenderIntentSupport};

const PRODUCTION_EFFECTS_PREVIEW_RS: &str = include_str!("production_effects_preview.rs");
const PHASE19_SOURCE_GUARD_SH: &str = include_str!("../../../scripts/phase19-source-guards.sh");

#[test]
fn phase19_production_effects_preview_fixtures_cover_retime_effect_transition_parity() {
    assert!(
        PRODUCTION_EFFECTS_PREVIEW_RS.contains(
            "phase19_production_effects_preview_retime_constant_speed_uses_graph_mapping_without_fallback"
        ) && PRODUCTION_EFFECTS_PREVIEW_RS.contains(
            "phase19_production_effects_preview_retime_speed_curve_reports_degraded_typed_evidence"
        ) && occurrences(
            PRODUCTION_EFFECTS_PREVIEW_RS,
            "phase19_production_effects_preview_complex_template_fixture_uses_gpu_evidence_without_fallback",
        ) >= 2
            && PHASE19_SOURCE_GUARD_SH
                .contains("crates/testkit/tests/production_effects_preview.rs"),
        "production preview fixtures must cover concrete retime parity tests and source guard evidence"
    );
}

fn occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

#[test]
fn phase19_production_effects_preview_fixtures_reject_fallback_evidence() {
    assert!(
        PRODUCTION_EFFECTS_PREVIEW_RS.contains("fallback_used")
            && PRODUCTION_EFFECTS_PREVIEW_RS.contains("RealtimePreviewGraphSupport::Degraded"),
        "Phase 19 preview fixtures must keep fallback/degraded retime evidence explicit"
    );
}

#[test]
fn phase19_production_effects_preview_retime_constant_speed_uses_graph_mapping_without_fallback() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: retimed_preview_draft(SegmentRetiming {
            mode: RetimeMode::Constant {
                speed: SpeedRatio::new(2, 1),
            },
            audio_policy: AudioRetimePolicy::FollowVideoSpeed,
        }),
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("constant-speed retime preview graph should prepare");
    let layer = prepared
        .graph
        .video_layers
        .first()
        .expect("retime preview graph should contain a video layer");
    let mix = prepared
        .graph
        .audio_mixes
        .first()
        .expect("retime preview graph should contain a matching audio mix");

    assert_eq!(
        layer.retime.source_mapping.source_timerange,
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(3_000_000)),
        "preview graph must keep original source range as semantic evidence"
    );
    assert_eq!(
        layer.retime.source_mapping.retimed_source_timerange,
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(2_000_000)),
        "preview graph must use engine-owned source mapping for 2x retime"
    );
    assert_eq!(
        mix.retime.source_mapping, layer.retime.source_mapping,
        "preview video and audio must share the same typed retime mapping facts"
    );

    let report = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_supported_production_effects()
        .classify(&prepared.graph);
    assert_eq!(report.support, RealtimePreviewGraphSupport::Supported);
    assert!(
        report
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.fallback_used
                && matches!(diagnostic.support, RealtimePreviewSupport::Supported)),
        "constant-speed preview parity must not count fallback/degraded diagnostics as product evidence: {report:#?}"
    );
}

#[test]
fn phase19_production_effects_preview_retime_speed_curve_reports_degraded_typed_evidence() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: retimed_preview_draft(SegmentRetiming {
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
        }),
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("speed-curve retime preview graph should prepare");
    let layer = prepared
        .graph
        .video_layers
        .first()
        .expect("speed-curve preview graph should contain a video layer");

    assert_eq!(
        layer.retime.support,
        RenderIntentSupport::Degraded,
        "speed curves must remain typed degraded render intent, not disappear into fallback evidence"
    );
    assert_eq!(
        layer.retime.source_mapping.target_timerange,
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
        "speed-curve preview parity must be bound to the same target range used for export"
    );

    let report = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_supported_production_effects()
        .classify(&prepared.graph);
    assert_eq!(report.support, RealtimePreviewGraphSupport::Degraded);
    assert!(
        report.diagnostics.iter().any(|diagnostic| {
            diagnostic.reason.contains("speed curve")
                && diagnostic.reason.contains("typed")
                && matches!(diagnostic.support, RealtimePreviewSupport::Degraded { .. })
        }),
        "speed-curve preview must report degraded typed retime support: {report:#?}"
    );
}

#[test]
fn phase19_production_effects_preview_complex_template_fixture_uses_gpu_evidence_without_fallback()
{
    let started = Instant::now();
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: complex_production_effects_draft(),
        target_time: Microseconds::new(1_800_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("complex production effects preview graph should prepare");
    assert!(
        started.elapsed() <= Duration::from_secs(2),
        "complex production effects preview prep exceeded fixture budget: {:?}",
        started.elapsed()
    );

    let report = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_supported_production_effects()
        .classify(&prepared.graph);
    assert_eq!(report.support, RealtimePreviewGraphSupport::Supported);
    assert!(
        report.diagnostics.iter().all(|diagnostic| {
            !diagnostic.fallback_used
                && matches!(diagnostic.support, RealtimePreviewSupport::Supported)
        }),
        "complex production effects preview must be real GPU-supported evidence without fallback: {report:#?}"
    );

    let layer = prepared
        .graph
        .video_layers
        .iter()
        .find(|layer| layer.segment_id.as_str() == "video-a")
        .expect("complex fixture should include the retimed effect video segment");
    assert_eq!(
        layer.retime.source_mapping.retimed_source_timerange,
        SourceTimerange::new(Microseconds::ZERO, Microseconds::new(3_000_000)),
        "complex fixture preview must preserve engine-owned retime mapping"
    );
    assert_eq!(layer.filters.len(), 3);
    assert!(layer.transition.is_some());
    assert!(matches!(layer.mask.mask, SegmentMask::Rectangle { .. }));
    assert_eq!(layer.blend.blend_mode, SegmentBlendMode::Multiply);
    assert!(
        !prepared.graph.text_overlays.is_empty(),
        "complex fixture should include text overlay preview work"
    );
    assert!(
        !prepared.graph.audio_mixes.is_empty(),
        "complex fixture should include audio preview work"
    );
    assert_eq!(
        complex_template_report_statuses(),
        vec![
            AdaptationStatus::Supported,
            AdaptationStatus::NeedsNativeEffect,
            AdaptationStatus::Dropped
        ],
        "complex fixture should keep template adaptation report states explicit"
    );
    assert_no_provider_or_fallback_tokens(&prepared.graph);
}

#[test]
fn phase19_production_effects_retime_source_guard_requires_preview_coverage() {
    assert!(
        PHASE19_SOURCE_GUARD_SH.contains("require_retime_render_graph_compiler_testkit_coverage"),
        "retiming source guard must require graph/compiler/testkit retime coverage"
    );
    assert!(
        PHASE19_SOURCE_GUARD_SH.contains("scan_retime_filter_ownership"),
        "retiming source guard must scan for FFmpeg retime filter ownership violations"
    );
}

fn retimed_preview_draft(retiming: SegmentRetiming) -> Draft {
    let mut draft = Draft::new("phase19-testkit-retime-preview", "Phase 19 Retime Preview");
    draft.materials.push(retimed_video_material());
    let mut segment = Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(3_000_000)),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    );
    segment.retiming = retiming;
    let mut track = Track::new("video-track", TrackKind::Video, "Video");
    track.segments.push(segment);
    draft.tracks.push(track);
    draft
}

fn retimed_video_material() -> Material {
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "file://retime-preview.mp4",
        "Retime Preview Video",
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

fn complex_production_effects_draft() -> Draft {
    let mut draft = Draft::new(
        "phase19-complex-template-preview",
        "Phase 19 Complex Template Preview",
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

fn assert_no_provider_or_fallback_tokens(graph: &render_graph::RenderGraph) {
    let serialized = serde_json::to_string(graph).expect("render graph should serialize");
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
            "complex preview graph leaked provider/fallback token {forbidden}: {serialized}"
        );
    }
}
