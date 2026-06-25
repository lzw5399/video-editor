use draft_model::{
    AudioRetimePolicy, Draft, Material, MaterialKind, MaterialMetadata, Microseconds,
    RationalFrameRate, RetimeMode, Segment, SegmentRetiming, SourceTimerange, SpeedCurvePoint,
    SpeedRatio, TargetTimerange, Track, TrackKind,
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
        ) && PHASE19_SOURCE_GUARD_SH.contains("crates/testkit/tests/production_effects_preview.rs"),
        "production preview fixtures must cover concrete retime parity tests and source guard evidence"
    );
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
