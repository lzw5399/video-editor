use draft_model::{
    Draft, Filter, Material, MaterialKind, MaterialMetadata, Microseconds, RationalFrameRate,
    Segment, SourceTimerange, TargetTimerange, TextSegment, TextStyle, Track, TrackKind,
};
use realtime_preview_runtime::{
    RealtimePreviewCapabilityClassifier, RealtimePreviewGraphInput, RealtimePreviewGraphSupport,
    prepare_realtime_preview_graph, realtime_preview_parity_diagnostics,
};
use render_graph::OutputDimensions;

#[test]
fn realtime_preview_parity_supported_graph_has_no_export_divergence() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: supported_video_draft(),
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("supported draft prepares graph");

    let report = RealtimePreviewCapabilityClassifier::supported_for_tests().classify(&prepared.graph);
    let diagnostics = realtime_preview_parity_diagnostics(&prepared.graph, &report);

    assert_eq!(report.support, RealtimePreviewGraphSupport::Supported);
    assert_eq!(
        serde_json::to_value(&diagnostics).expect("diagnostics serialize"),
        serde_json::json!([])
    );
}

#[test]
fn realtime_preview_parity_golden_records_text_and_effect_divergence() {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: divergent_text_and_effect_draft(),
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("divergent draft prepares graph");

    let report = RealtimePreviewCapabilityClassifier::supported_for_tests().classify(&prepared.graph);
    let diagnostics = realtime_preview_parity_diagnostics(&prepared.graph, &report);

    assert_eq!(report.support, RealtimePreviewGraphSupport::Unsupported);
    assert_eq!(
        serde_json::to_value(&diagnostics).expect("diagnostics serialize"),
        serde_json::json!([
            {
                "entityId": "video-a",
                "domain": "effect",
                "previewSupport": {
                    "unsupported": {
                        "reason": "filter cinematic-lut is unsupported in realtime preview"
                    }
                },
                "exportSupport": "degraded",
                "reason": "realtime preview effect support diverges from export graph intent",
                "fallbackUsed": true
            },
            {
                "entityId": "text-a",
                "domain": "text",
                "previewSupport": {
                    "unsupported": {
                        "reason": "gpu text parity has not been proven with repository fonts; realtime preview must use fallback text rasterization"
                    }
                },
                "exportSupport": "supported",
                "reason": "realtime preview text support diverges from export graph intent",
                "fallbackUsed": true
            }
        ])
    );
}

fn supported_video_draft() -> Draft {
    let mut draft = Draft::new("realtime-preview-parity-supported", "Realtime parity supported");
    draft.materials.push(video_material());

    let mut track = Track::new("video-track", TrackKind::Video, "Video");
    track.segments.push(video_segment());
    draft.tracks.push(track);
    draft
}

fn divergent_text_and_effect_draft() -> Draft {
    let mut draft = supported_video_draft();
    draft.materials.push(Material::new(
        "text-material",
        MaterialKind::Text,
        "text://title",
        "text-material",
    ));

    draft.tracks[0].segments[0].filters.push(Filter {
        name: "cinematic-lut".to_owned(),
        parameters: Default::default(),
    });

    let mut text = Segment::new(
        "text-a",
        "text-material",
        SourceTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
    );
    text.text = Some(TextSegment {
        content: "标题".to_owned(),
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
    draft
}

fn video_material() -> Material {
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
    material
}

fn video_segment() -> Segment {
    Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
    )
}
