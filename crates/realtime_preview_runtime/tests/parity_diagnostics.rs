use draft_model::{
    Draft, Material, MaterialKind, MaterialMetadata, Microseconds, RationalFrameRate, Segment,
    SourceTimerange, TargetTimerange, TextSegment, TextStyle, Track, TrackKind,
};
use realtime_preview_runtime::{
    RealtimePreviewCapabilityClassifier, RealtimePreviewGraphInput,
    RealtimePreviewGraphPrepareErrorKind, prepare_realtime_preview_graph,
    realtime_preview_parity_diagnostics,
};
use render_graph::OutputDimensions;

#[test]
fn parity_diagnostics_graph_prepare_builds_engine_owned_single_frame_render_graph_without_ffmpeg() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: supported_draft(),
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("supported draft prepares graph");

    assert_eq!(prepared.target_time, Microseconds::new(500_000));
    assert_eq!(prepared.preview_dimensions, OutputDimensions::new(960, 540));
    assert_eq!(prepared.profile.canvas_width, 1920);
    assert_eq!(prepared.profile.canvas_height, 1080);
    assert_eq!(prepared.frame_rate, RationalFrameRate::new(30, 1));
    assert_eq!(
        prepared.render_range.target_timerange,
        TargetTimerange::new(Microseconds::new(500_000), Microseconds::new(33_333))
    );
    assert_eq!(prepared.render_range.frames.len(), 1);
    assert_eq!(prepared.graph.video_layers.len(), 1);
    assert_eq!(
        prepared.graph.materials[0].material_id.as_str(),
        "video-material"
    );
    assert_eq!(prepared.diagnostics.len(), 0);
}

#[test]
fn parity_diagnostics_graph_prepare_returns_classified_errors_for_invalid_profile_and_range_inputs()
{
    let invalid_dimensions = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: supported_draft(),
        target_time: Microseconds::new(0),
        preview_dimensions: OutputDimensions::new(0, 540),
    })
    .expect_err("zero preview dimensions should be classified");

    assert_eq!(
        invalid_dimensions.kind,
        RealtimePreviewGraphPrepareErrorKind::InvalidPreviewProfile
    );
    assert!(
        invalid_dimensions
            .message
            .contains("preview dimensions width and height")
    );

    let empty_range = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: supported_draft(),
        target_time: Microseconds::new(u64::MAX),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect_err("overflowing target range should be classified");

    assert_eq!(
        empty_range.kind,
        RealtimePreviewGraphPrepareErrorKind::EngineFailed
    );
    assert!(empty_range.message.contains("render range"));
}

#[test]
fn parity_diagnostics_snapshot_serializes_realtime_export_divergence() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: text_draft(),
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("text draft prepares graph");
    let report = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_gpu_text_parity(false)
        .classify(&prepared.graph);
    let diagnostics = realtime_preview_parity_diagnostics(&prepared.graph, &report);

    assert_eq!(
        serde_json::to_value(&diagnostics).expect("parity diagnostics serialize"),
        serde_json::json!([
            {
                "entityId": "text-a",
                "domain": "text",
                "previewSupport": {
                    "degraded": {
                        "reason": "gpu text parity disabled; realtime preview must use fallback text rasterization"
                    }
                },
                "exportSupport": "supported",
                "reason": "realtime preview text support diverges from export graph intent",
                "fallbackUsed": true
            }
        ])
    );
}

fn supported_draft() -> Draft {
    let mut draft = Draft::new("realtime-graph-prepare", "Realtime graph prepare");
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "file://video.mp4",
        "video-material",
    );
    material.metadata = MaterialMetadata {
        duration: Some(Microseconds::new(1_000_000)),
        width: Some(1920),
        height: Some(1080),
        frame_rate: Some(RationalFrameRate::new(30, 1)),
        has_video: true,
        has_audio: true,
        audio_sample_rate: Some(48_000),
        audio_channels: Some(2),
        probe_error: None,
    };
    draft.materials.push(material);

    let mut track = Track::new("video-track", TrackKind::Video, "Video");
    track.segments.push(Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
    ));
    draft.tracks.push(track);
    draft
}

fn text_draft() -> Draft {
    let mut draft = Draft::new("realtime-parity-text", "Realtime parity text");
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
