use draft_model::{
    AudioEffectSlot, AudioEffectSlotKind, Draft, ExternalEffectReference, Filter, Keyframe,
    KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue, Material, MaterialKind,
    MaterialMetadata, Microseconds, RationalFrameRate, Segment, SegmentBlendMode, SegmentFitMode,
    SegmentMask, SourceTimerange, TargetTimerange, TextSegment, TextStyle, Track, TrackKind,
    Transition,
};
use realtime_preview_runtime::{
    RealtimePreviewCapabilityClassifier, RealtimePreviewDiagnosticDomain,
    RealtimePreviewGraphInput, RealtimePreviewGraphSupport, RealtimePreviewSupport,
    prepare_realtime_preview_graph,
};
use render_graph::OutputDimensions;

#[test]
fn capability_matrix_supports_canvas_visual_transform_opacity_fit_and_keyframe_state() {
    let mut draft = base_video_draft();
    let segment = &mut draft.tracks[0].segments[0];
    segment.visual.fit_mode = SegmentFitMode::Fit;
    segment.visual.transform.opacity.value_millis = 640;
    segment.visual.transform.position.x = 120;
    segment.visual.transform.position.y = -80;
    segment.keyframes.push(Keyframe {
        at: Microseconds::new(0),
        property: KeyframeProperty::VisualOpacity,
        value: KeyframeValue::Uint { value: 640 },
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    });

    let report = classify_draft(
        draft,
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );

    assert_eq!(report.support, RealtimePreviewGraphSupport::Supported);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Canvas
            && diagnostic.support == RealtimePreviewSupport::Supported
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::MaterialFrame
            && diagnostic.entity_id.as_deref() == Some("video-material")
            && diagnostic.support == RealtimePreviewSupport::Supported
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Keyframe
            && diagnostic.entity_id.as_deref() == Some("video-a")
            && diagnostic.support == RealtimePreviewSupport::Supported
    }));
}

#[test]
fn capability_matrix_marks_text_unsupported_when_gpu_text_parity_is_not_enabled() {
    let report = classify_draft(
        text_draft(),
        RealtimePreviewCapabilityClassifier::supported_for_tests()
            .with_gpu_text_parity(false)
            .with_bundled_text_font_registry_available(false),
    );

    assert_eq!(report.support, RealtimePreviewGraphSupport::Unsupported);
    let text = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.domain == RealtimePreviewDiagnosticDomain::Text)
        .expect("text diagnostic emitted");
    assert_eq!(
        text.support,
        RealtimePreviewSupport::Unsupported {
            reason:
                "gpu text parity has not been proven with repository fonts; realtime preview must use fallback text rasterization"
                    .to_owned()
        }
    );
    assert!(text.fallback_used);
}

#[test]
fn capability_matrix_supports_baseline_video_image_text_and_audio_when_text_parity_is_proven() {
    let report = classify_draft(
        baseline_video_image_text_audio_draft(),
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );

    assert_eq!(report.support, RealtimePreviewGraphSupport::Supported);
    for material_id in ["video-material", "image-material"] {
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.domain == RealtimePreviewDiagnosticDomain::MaterialFrame
                && diagnostic.entity_id.as_deref() == Some(material_id)
                && diagnostic.support == RealtimePreviewSupport::Supported
                && !diagnostic.fallback_used
        }));
    }
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Text
            && diagnostic.entity_id.as_deref() == Some("text-a")
            && diagnostic.support == RealtimePreviewSupport::Supported
            && !diagnostic.fallback_used
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Audio
            && diagnostic.entity_id.as_deref() == Some("audio-a")
            && diagnostic.support == RealtimePreviewSupport::Supported
            && !diagnostic.fallback_used
    }));
}

#[test]
fn capability_matrix_rejects_unsupported_audio_effects_for_baseline_playback() {
    let mut draft = audio_draft();
    draft.tracks[0].segments[0]
        .audio
        .effect_slots
        .push(AudioEffectSlot {
            slot_id: "audio-effect-1".to_owned(),
            kind: AudioEffectSlotKind::Unsupported {
                name: "robot-voice".to_owned(),
                external_ref: None,
            },
            enabled: true,
        });

    let report = classify_draft(
        draft,
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );

    assert_eq!(report.support, RealtimePreviewGraphSupport::Unsupported);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Audio
            && diagnostic.entity_id.as_deref() == Some("audio-a")
            && matches!(
                diagnostic.support,
                RealtimePreviewSupport::Unsupported { ref reason }
                    if reason.contains("robot-voice")
            )
            && diagnostic.fallback_used
    }));
}

#[test]
fn capability_matrix_marks_filters_transitions_masks_and_blends_unsupported() {
    let mut draft = base_video_draft();
    let segment = &mut draft.tracks[0].segments[0];
    segment
        .filters
        .push(Filter::external_reference("fixture", "cinematic-lut"));
    segment.transition = Some(Transition::external_reference(
        "fixture",
        "crossfade",
        Microseconds::new(120_000),
    ));
    segment.visual.mask = SegmentMask::ExternalReference {
        reference: ExternalEffectReference::new("fixture", "linear-gradient-mask"),
    };
    segment.visual.blend_mode = SegmentBlendMode::ExternalReference {
        reference: ExternalEffectReference::new("fixture", "screen"),
    };

    let report = classify_draft(
        draft,
        RealtimePreviewCapabilityClassifier::supported_for_tests(),
    );

    assert_eq!(report.support, RealtimePreviewGraphSupport::Unsupported);
    for domain in [
        RealtimePreviewDiagnosticDomain::Effect,
        RealtimePreviewDiagnosticDomain::VisualLayer,
    ] {
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.domain == domain
                && matches!(
                    diagnostic.support,
                    RealtimePreviewSupport::Unsupported { .. }
                )
        }));
    }
}

#[test]
fn capability_matrix_classifies_surface_and_backend_availability() {
    let graph = prepare_graph(base_video_draft());

    let no_surface = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_surface_available(false)
        .classify(&graph);
    assert_eq!(no_surface.support, RealtimePreviewGraphSupport::Degraded);
    assert!(no_surface.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Surface
            && matches!(diagnostic.support, RealtimePreviewSupport::Degraded { .. })
            && diagnostic.fallback_used
    }));

    let no_backend = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_runtime_backend_available(false)
        .classify(&graph);
    assert_eq!(no_backend.support, RealtimePreviewGraphSupport::Unsupported);
    assert!(no_backend.diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::Runtime
            && matches!(
                diagnostic.support,
                RealtimePreviewSupport::Unsupported { .. }
            )
    }));
}

fn classify_draft(
    draft: Draft,
    classifier: RealtimePreviewCapabilityClassifier,
) -> realtime_preview_runtime::RealtimePreviewCapabilityReport {
    let graph = prepare_graph(draft);
    classifier.classify(&graph)
}

fn prepare_graph(draft: Draft) -> render_graph::RenderGraph {
    prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft,
        target_time: Microseconds::new(500_000),
        preview_dimensions: OutputDimensions::new(960, 540),
    })
    .expect("draft prepares graph")
    .graph
}

fn base_video_draft() -> Draft {
    let mut draft = Draft::new("capability-matrix", "Capability matrix");
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
    let mut draft = Draft::new("capability-text", "Capability text");
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

fn baseline_video_image_text_audio_draft() -> Draft {
    let mut draft = base_video_draft();

    let mut image = image_draft();
    draft.materials.append(&mut image.materials);
    draft.tracks.append(&mut image.tracks);

    let mut text = text_draft();
    draft.materials.append(&mut text.materials);
    draft.tracks.append(&mut text.tracks);

    let mut audio = audio_draft();
    draft.materials.append(&mut audio.materials);
    draft.tracks.append(&mut audio.tracks);

    draft
}

fn image_draft() -> Draft {
    let mut draft = Draft::new("capability-image", "Capability image");
    let mut material = Material::new(
        "image-material",
        MaterialKind::Image,
        "file://poster.png",
        "image-material",
    );
    material.metadata = MaterialMetadata {
        duration: Some(Microseconds::new(1_000_000)),
        width: Some(1080),
        height: Some(1080),
        frame_rate: None,
        has_video: true,
        has_audio: false,
        audio_sample_rate: None,
        audio_channels: None,
        probe_error: None,
    };
    draft.materials.push(material);

    let mut track = Track::new("image-track", TrackKind::Video, "Image");
    track.segments.push(Segment::new(
        "image-a",
        "image-material",
        SourceTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
    ));
    draft.tracks.push(track);
    draft
}

fn audio_draft() -> Draft {
    let mut draft = Draft::new("capability-audio", "Capability audio");
    let mut material = Material::new(
        "audio-material",
        MaterialKind::Audio,
        "file://music.m4a",
        "audio-material",
    );
    material.metadata = MaterialMetadata {
        duration: Some(Microseconds::new(1_000_000)),
        width: None,
        height: None,
        frame_rate: None,
        has_video: false,
        has_audio: true,
        audio_sample_rate: Some(48_000),
        audio_channels: Some(2),
        probe_error: None,
    };
    draft.materials.push(material);

    let mut track = Track::new("audio-track", TrackKind::Audio, "Audio");
    track.segments.push(Segment::new(
        "audio-a",
        "audio-material",
        SourceTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
    ));
    draft.tracks.push(track);
    draft
}
