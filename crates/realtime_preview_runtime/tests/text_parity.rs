use draft_model::{
    Draft, Material, MaterialKind, Microseconds, Segment, SourceTimerange, TargetTimerange,
    TextSegment, TextStyle, Track, TrackKind,
};
use realtime_preview_runtime::{
    RealtimePreviewCapabilityClassifier, RealtimePreviewDiagnosticDomain,
    RealtimePreviewFallbackReason, RealtimePreviewGraphInput, RealtimePreviewGraphSupport,
    RealtimePreviewSupport, prepare_realtime_preview_graph,
};
use render_graph::OutputDimensions;

#[test]
fn text_parity_classifies_gpu_text_as_unsupported_without_repository_font_proof() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: text_draft(),
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("text draft prepares graph");

    let outcome = realtime_preview_runtime::gpu::text::classify_text_preview_outcome(
        &prepared.graph,
        &RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );

    assert_eq!(outcome.support, RealtimePreviewGraphSupport::Unsupported);
    assert_eq!(
        outcome.fallback_reason,
        Some(RealtimePreviewFallbackReason::TextParityUnsupported)
    );
    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Text
            && diagnostic.entity_id.as_deref() == Some("text-a")
            && matches!(
                diagnostic.support,
                RealtimePreviewSupport::Unsupported { ref reason }
                    if reason == "gpu text parity has not been proven with repository fonts; realtime preview must use fallback text rasterization"
            )
            && diagnostic.fallback_used
    }));
}

#[test]
fn text_parity_default_classifier_never_marks_export_supported_text_as_realtime_supported() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: text_draft(),
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("text draft prepares graph");

    let report = RealtimePreviewCapabilityClassifier::supported_for_tests().classify(&prepared.graph);
    let text = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.domain == RealtimePreviewDiagnosticDomain::Text)
        .expect("text diagnostic emitted");

    assert_ne!(report.support, RealtimePreviewGraphSupport::Supported);
    assert!(matches!(text.support, RealtimePreviewSupport::Unsupported { .. }));
    assert!(text.fallback_used);
}

fn text_draft() -> Draft {
    let mut draft = Draft::new("text-parity", "Text parity");
    draft.materials.push(Material::new(
        "text-material",
        MaterialKind::Text,
        "text://title",
        "text-material",
    ));

    let mut segment = Segment::new(
        "text-a",
        "text-material",
        SourceTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
    );
    segment.text = Some(TextSegment {
        content: "标题".to_owned(),
        source: Default::default(),
        style: TextStyle::default_title(),
        text_box: Default::default(),
        layout_region: Default::default(),
        wrapping: Default::default(),
        bubble: None,
        effect: None,
    });

    let mut track = Track::new("text-track", TrackKind::Text, "Text");
    track.segments.push(segment);
    draft.tracks.push(track);
    draft
}
