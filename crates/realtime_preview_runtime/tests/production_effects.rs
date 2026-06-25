use draft_model::{
    AudioRetimePolicy, Draft, Material, MaterialKind, MaterialMetadata, Microseconds,
    RationalFrameRate, RetimeMode, Segment, SegmentRetiming, SourceTimerange, SpeedRatio,
    TargetTimerange, Track, TrackKind, TrackTransition,
};
use realtime_preview_runtime::{
    RealtimePreviewCapabilityClassifier, RealtimePreviewDiagnosticDomain,
    RealtimePreviewGraphInput, RealtimePreviewGraphSupport, RealtimePreviewSupport,
    prepare_realtime_preview_graph,
};
use render_graph::OutputDimensions;

const CAPABILITIES_RS: &str = include_str!("../src/capabilities.rs");

#[test]
fn phase19_production_effects_preview_requires_registry_backed_supported_effects() {
    assert!(
        CAPABILITIES_RS.contains("ProductionEffectCapabilityRegistry")
            || CAPABILITIES_RS.contains("RealtimeProductionEffectSupport"),
        "realtime preview must classify Phase 19 effects/transitions/masks/blends through a registry-backed support matrix"
    );
    assert!(
        CAPABILITIES_RS.contains("supported_first_party_effect")
            || CAPABILITIES_RS.contains("with_supported_production_effects"),
        "supported GPU preview must be opt-in per first-party semantic effect instead of accepting generic string filters"
    );
}

#[test]
fn phase19_production_effects_preview_rejects_fallback_success_for_masks_blends_and_transitions() {
    assert!(
        CAPABILITIES_RS.contains("fallback_used: false")
            && CAPABILITIES_RS.contains("mask")
            && CAPABILITIES_RS.contains("blend")
            && CAPABILITIES_RS.contains("transition"),
        "supported Phase 19 preview diagnostics must prove real GPU support for masks, blends, and transitions with no fallback success"
    );
}

#[test]
fn phase19_production_effects_preview_classifies_unsupported_audio_retime_policy() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: preserve_pitch_retime_draft(),
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("retimed draft should prepare preview graph");
    let report = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_supported_production_effects()
        .classify(&prepared.graph);

    assert_eq!(report.support, RealtimePreviewGraphSupport::Unsupported);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Audio
            && diagnostic.entity_id.as_deref() == Some("video-a")
            && diagnostic.fallback_used
            && matches!(
                diagnostic.support,
                RealtimePreviewSupport::Unsupported { ref reason }
                    if reason.contains("preserve-pitch")
            )
    }));
}

#[test]
fn phase19_production_effects_preview_reports_first_party_dissolve_transition_support() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: transition_preview_draft(TrackTransition::dissolve(
            "left-segment",
            "right-segment",
            Microseconds::new(300_000),
        )),
        target_time: Microseconds::new(800_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("transition draft should prepare preview graph");
    let report = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_supported_production_effects()
        .classify(&prepared.graph);

    assert_eq!(report.support, RealtimePreviewGraphSupport::Supported);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Effect
            && diagnostic.entity_id.as_deref() == Some("left-segment")
            && !diagnostic.fallback_used
            && matches!(diagnostic.support, RealtimePreviewSupport::Supported)
            && diagnostic.reason.contains("transition")
    }));
}

#[test]
fn phase19_production_effects_preview_rejects_external_transition_as_product_success() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: transition_preview_draft(TrackTransition::external_reference(
            "left-segment",
            "right-segment",
            "jianying",
            "private-crossfade",
            Microseconds::new(300_000),
        )),
        target_time: Microseconds::new(800_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("external transition draft should prepare preview graph");
    let report = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_supported_production_effects()
        .classify(&prepared.graph);

    assert_eq!(report.support, RealtimePreviewGraphSupport::Unsupported);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Effect
            && diagnostic.entity_id.as_deref() == Some("left-segment")
            && diagnostic.fallback_used
            && matches!(
                diagnostic.support,
                RealtimePreviewSupport::Unsupported { ref reason }
                    if reason.contains("external") && reason.contains("private-crossfade")
            )
    }));
}

fn preserve_pitch_retime_draft() -> Draft {
    let mut draft = Draft::new("phase19-preview-retime", "Phase 19 Preview Retime");
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "file://retime.mp4",
        "video-material",
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
    draft.materials.push(material);

    let mut segment = Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(3_000_000)),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    );
    segment.retiming = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::PreservePitch,
    };
    let mut track = Track::new("video-track", TrackKind::Video, "视频");
    track.segments.push(segment);
    draft.tracks.push(track);
    draft
}

fn transition_preview_draft(transition: TrackTransition) -> Draft {
    let mut draft = Draft::new("phase19-preview-transition", "Phase 19 Preview Transition");
    draft.materials.push(preview_video_material());

    let mut track = Track::new("video-track", TrackKind::Video, "视频");
    track
        .segments
        .push(preview_segment("left-segment", 0, 0, 1_000_000));
    track.segments.push(preview_segment(
        "right-segment",
        1_000_000,
        1_000_000,
        1_000_000,
    ));
    track.transitions.push(transition);
    draft.tracks.push(track);
    draft
}

fn preview_segment(
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

fn preview_video_material() -> Material {
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "file://transition.mp4",
        "video-material",
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
