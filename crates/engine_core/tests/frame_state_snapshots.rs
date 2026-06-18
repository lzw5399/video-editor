use draft_model::{
    Draft, Material, MaterialKind, Microseconds, RationalFrameRate, Segment,
    SegmentBackgroundFilling, SegmentBlendMode, SegmentFitMode, SegmentMask, SegmentPosition,
    SourceTimerange, TargetTimerange, TextAlignment, TextBackground, TextBox, TextBubbleRef,
    TextEffectRef, TextFont, TextLayoutRegion, TextSegment, TextSegmentSource, TextShadow,
    TextStroke, TextStyle, TextWrapping, Track, TrackKind,
};
use engine_core::{
    EngineErrorKind, EngineProfile, TextLayoutProfile, frame_index_to_microseconds,
    normalize_draft, resolve_frame_state, resolve_render_range,
};

#[test]
fn frame_state_resolves_active_visual_audio_and_text_segments_at_microsecond_position() {
    let normalized = normalize_draft(&frame_state_draft(), &EngineProfile::mvp_default())
        .expect("draft should normalize");

    let frame = resolve_frame_state(&normalized, Microseconds::new(600_000))
        .expect("frame state should resolve");

    assert_eq!(frame.at, Microseconds::new(600_000));
    assert_eq!(
        frame
            .visual_layers
            .iter()
            .map(|layer| {
                (
                    layer.track_id.as_str(),
                    layer.segment_id.as_str(),
                    layer.material_id.as_str(),
                    layer.stack_index,
                    layer.source_position.get(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("video-track", "video-a", "video-material", 0, 700_000),
            ("overlay-track", "overlay-a", "overlay-material", 1, 600_000),
        ]
    );
    assert_eq!(
        frame
            .audio_segments
            .iter()
            .map(|audio| {
                (
                    audio.track_id.as_str(),
                    audio.segment_id.as_str(),
                    audio.material_id.as_str(),
                    audio.source_position.get(),
                    audio.volume_level_millis,
                )
            })
            .collect::<Vec<_>>(),
        vec![("audio-track", "audio-a", "audio-material", 600_000, 1_000)]
    );
    assert_eq!(
        frame
            .text_overlays
            .iter()
            .map(|overlay| {
                (
                    overlay.track_id.as_str(),
                    overlay.segment_id.as_str(),
                    overlay.content.as_str(),
                    overlay.stack_index,
                    overlay.source_position.get(),
                )
            })
            .collect::<Vec<_>>(),
        vec![("text-track", "text-a", "标题", 2, 100_000)]
    );
}

#[test]
fn transform_frame_state_carries_visual_semantics_and_omits_hidden_segments() {
    let mut draft = frame_state_draft();
    let video = &mut draft.tracks[0].segments[0];
    video.visual.fit_mode = SegmentFitMode::Fill;
    video.visual.transform.position = SegmentPosition { x: 250, y: -125 };
    video.visual.background_filling = SegmentBackgroundFilling::Blur;
    video.visual.blend_mode = SegmentBlendMode::Unsupported {
        name: "screen".to_owned(),
    };
    video.visual.mask = SegmentMask::Unsupported {
        name: "linear".to_owned(),
    };
    draft.tracks[1].segments[0].visual.visible = false;

    let normalized =
        normalize_draft(&draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    assert_eq!(normalized.diagnostics.len(), 3);
    assert!(normalized.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == EngineErrorKind::DegradedVisualIntent
            && diagnostic.message.contains("backgroundFilling")
    }));
    assert!(normalized.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == EngineErrorKind::UnsupportedVisualIntent
            && diagnostic.message.contains("blendMode")
    }));
    assert!(normalized.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == EngineErrorKind::UnsupportedVisualIntent
            && diagnostic.message.contains("mask")
    }));

    let frame = resolve_frame_state(&normalized, Microseconds::new(600_000))
        .expect("frame state should resolve");

    assert_eq!(
        frame.visual_layers.len(),
        1,
        "hidden overlay should be omitted"
    );
    let layer = &frame.visual_layers[0];
    assert_eq!(layer.segment_id.as_str(), "video-a");
    assert_eq!(layer.visual.fit_mode, SegmentFitMode::Fill);
    assert_eq!(layer.visual.transform.position.x, 250);
    assert_eq!(layer.visual.transform.position.y, -125);
}

#[test]
fn frame_index_sampling_uses_rational_frame_rate_without_floating_point_fields() {
    let ntsc = RationalFrameRate::new(30_000, 1_001);

    assert_eq!(
        frame_index_to_microseconds(0, &ntsc).expect("frame zero"),
        Microseconds::new(0)
    );
    assert_eq!(
        frame_index_to_microseconds(30, &ntsc).expect("frame thirty"),
        Microseconds::new(1_001_000)
    );
}

#[test]
fn render_range_state_samples_frame_positions_and_resolves_stable_json_snapshot() {
    let normalized = normalize_draft(&frame_state_draft(), &EngineProfile::mvp_default())
        .expect("draft should normalize");

    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(100_000)),
    )
    .expect("render range should resolve");

    let snapshot = serde_json::to_value(&range).expect("range should serialize");
    assert_eq!(
        snapshot,
        serde_json::json!({
            "targetTimerange": { "start": 0, "duration": 100000 },
            "frameRate": { "numerator": 30, "denominator": 1 },
            "frames": [
                {
                    "at": 0,
                    "visualLayers": [
                        {
                            "trackId": "video-track",
                            "segmentId": "video-a",
                            "materialId": "video-material",
                            "materialKind": "video",
                            "stackIndex": 0,
                            "sourcePosition": 100000,
                            "targetTimerange": { "start": 0, "duration": 1000000 },
                            "visual": default_visual_json()
                        },
                        {
                            "trackId": "overlay-track",
                            "segmentId": "overlay-a",
                            "materialId": "overlay-material",
                            "materialKind": "image",
                            "stackIndex": 1,
                            "sourcePosition": 0,
                            "targetTimerange": { "start": 0, "duration": 1000000 },
                            "visual": default_visual_json()
                        }
                    ],
                    "audioSegments": [
                        {
                            "trackId": "audio-track",
                            "segmentId": "audio-a",
                            "materialId": "audio-material",
                            "sourcePosition": 0,
                            "targetTimerange": { "start": 0, "duration": 1000000 },
                            "volumeLevelMillis": 1000
                        }
                    ],
                    "textOverlays": []
                },
                {
                    "at": 33333,
                    "visualLayers": [
                        {
                            "trackId": "video-track",
                            "segmentId": "video-a",
                            "materialId": "video-material",
                            "materialKind": "video",
                            "stackIndex": 0,
                            "sourcePosition": 133333,
                            "targetTimerange": { "start": 0, "duration": 1000000 },
                            "visual": default_visual_json()
                        },
                        {
                            "trackId": "overlay-track",
                            "segmentId": "overlay-a",
                            "materialId": "overlay-material",
                            "materialKind": "image",
                            "stackIndex": 1,
                            "sourcePosition": 33333,
                            "targetTimerange": { "start": 0, "duration": 1000000 },
                            "visual": default_visual_json()
                        }
                    ],
                    "audioSegments": [
                        {
                            "trackId": "audio-track",
                            "segmentId": "audio-a",
                            "materialId": "audio-material",
                            "sourcePosition": 33333,
                            "targetTimerange": { "start": 0, "duration": 1000000 },
                            "volumeLevelMillis": 1000
                        }
                    ],
                    "textOverlays": []
                },
                {
                    "at": 66666,
                    "visualLayers": [
                        {
                            "trackId": "video-track",
                            "segmentId": "video-a",
                            "materialId": "video-material",
                            "materialKind": "video",
                            "stackIndex": 0,
                            "sourcePosition": 166666,
                            "targetTimerange": { "start": 0, "duration": 1000000 },
                            "visual": default_visual_json()
                        },
                        {
                            "trackId": "overlay-track",
                            "segmentId": "overlay-a",
                            "materialId": "overlay-material",
                            "materialKind": "image",
                            "stackIndex": 1,
                            "sourcePosition": 66666,
                            "targetTimerange": { "start": 0, "duration": 1000000 },
                            "visual": default_visual_json()
                        }
                    ],
                    "audioSegments": [
                        {
                            "trackId": "audio-track",
                            "segmentId": "audio-a",
                            "materialId": "audio-material",
                            "sourcePosition": 66666,
                            "targetTimerange": { "start": 0, "duration": 1000000 },
                            "volumeLevelMillis": 1000
                        }
                    ],
                    "textOverlays": []
                }
            ]
        })
    );
}

#[test]
fn text_layout_resolves_pinned_profile_values_for_active_text_overlay() {
    let normalized = normalize_draft(&frame_state_draft(), &EngineProfile::mvp_default())
        .expect("draft should normalize");

    let frame = resolve_frame_state(&normalized, Microseconds::new(600_000))
        .expect("frame state should resolve");
    let overlay = frame
        .text_overlays
        .first()
        .expect("active text overlay should be resolved");

    assert_eq!(overlay.source, TextSegmentSource::Text);
    assert_eq!(overlay.font_family, "Source Han Sans SC");
    assert_eq!(overlay.font_ref.as_deref(), Some("source-han-local"));
    assert_eq!(overlay.font_candidate, "VE_TEXT_FONT_PATH");
    assert_eq!(
        overlay.fallback_candidates,
        vec![
            "VE_TEXT_FONT_PATH",
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
    );
    assert_eq!(overlay.alignment, TextAlignment::Center);
    assert_eq!(overlay.text_box.width_millis, 600);
    assert_eq!(overlay.text_box.width, 1_152);
    assert_eq!(overlay.text_box.height, 216);
    assert_eq!(overlay.layout_region.x, 192);
    assert_eq!(overlay.layout_region.y, 756);
    assert_eq!(overlay.layout_region.width, 1_536);
    assert_eq!(overlay.layout_region.height, 216);
    assert_eq!(overlay.wrapping, TextWrapping::Auto);
    assert_eq!(overlay.wrapping_policy.as_str(), "boundedWidth");
    assert_eq!(overlay.line_height_millis, 1_500);
    assert_eq!(overlay.letter_spacing_millis, 125);
    assert_eq!(overlay.style.color, "#ffffff");
    assert_eq!(overlay.style.stroke.as_ref().expect("stroke").width, 3);
    assert_eq!(overlay.style.shadow.as_ref().expect("shadow").offset_x, 4);
    assert_eq!(
        overlay.style.background.as_ref().expect("background").color,
        "#202020"
    );
    assert_eq!(overlay.layout_width, 1_152);
    assert_eq!(overlay.layout_height, 144);
    assert_eq!(overlay.diagnostics.len(), 2);
    assert!(overlay.diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "bubble"
            && diagnostic.support == "unsupported"
            && diagnostic.reason.contains("气泡")
    }));
    assert!(overlay.diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "effect"
            && diagnostic.support == "unsupported"
            && diagnostic.reason.contains("花字")
    }));
}

#[test]
fn text_layout_resolves_multiple_text_and_subtitle_overlays_in_stack_order() {
    let mut draft = frame_state_draft();
    draft.materials
        .push(material("subtitle-material", MaterialKind::Text, "text://subtitle"));
    let mut subtitle = segment("subtitle-a", "subtitle-material", 0, 400_000, 600_000);
    subtitle.text = Some(TextSegment {
        content: "字幕第一行\n字幕第二行".to_owned(),
        source: TextSegmentSource::Subtitle,
        style: TextStyle {
            font: TextFont {
                family: "PingFang SC".to_owned(),
                font_ref: None,
            },
            font_size: 40,
            color: "#ffcc33".to_owned(),
            alignment: TextAlignment::Left,
            line_height_millis: 1_250,
            letter_spacing_millis: 60,
            stroke: None,
            shadow: None,
            background: None,
        },
        text_box: TextBox {
            width_millis: 500,
            height_millis: 240,
        },
        layout_region: TextLayoutRegion {
            x_millis: 200,
            y_millis: 700,
            width_millis: 600,
            height_millis: 220,
        },
        wrapping: TextWrapping::Auto,
        bubble: None,
        effect: None,
    });
    let mut subtitle_track = Track::new("subtitle-track", TrackKind::Text, "字幕");
    subtitle_track.segments.push(subtitle);
    draft.tracks.push(subtitle_track);

    let normalized =
        normalize_draft(&draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    let frame = resolve_frame_state(&normalized, Microseconds::new(600_000))
        .expect("frame state should resolve");

    assert_eq!(
        frame
            .text_overlays
            .iter()
            .map(|overlay| {
                (
                    overlay.track_id.as_str(),
                    overlay.segment_id.as_str(),
                    overlay.source,
                    overlay.stack_index,
                    overlay.layout_width,
                    overlay.layout_height,
                    overlay.line_height_millis,
                    overlay.letter_spacing_millis,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "text-track",
                "text-a",
                TextSegmentSource::Text,
                2,
                1_152,
                144,
                1_500,
                125
            ),
            (
                "subtitle-track",
                "subtitle-a",
                TextSegmentSource::Subtitle,
                3,
                960,
                100,
                1_250,
                60
            ),
        ]
    );
}

#[test]
fn text_layout_missing_profile_returns_classified_engine_error() {
    let profile = EngineProfile {
        text_layout: None,
        ..EngineProfile::mvp_default()
    };
    let normalized = normalize_draft(&frame_state_draft(), &profile)
        .expect("missing text layout is classified when text overlay is resolved");

    let error = resolve_frame_state(&normalized, Microseconds::new(600_000))
        .expect_err("active text segment without profile should fail");

    assert_eq!(error.kind, EngineErrorKind::MissingTextLayoutProfile);
}

#[test]
fn text_layout_invalid_profile_returns_classified_engine_error() {
    let profile = EngineProfile {
        text_layout: Some(TextLayoutProfile::invalid_for_tests()),
        ..EngineProfile::mvp_default()
    };

    let error = normalize_draft(&frame_state_draft(), &profile)
        .expect_err("invalid text profile should not silently change layout");

    assert_eq!(error.kind, EngineErrorKind::InvalidTextLayoutProfile);
}

fn frame_state_draft() -> Draft {
    let mut draft = Draft::new("draft-frame-state", "Frame State");
    draft.materials = vec![
        material("video-material", MaterialKind::Video, "file://video.mp4"),
        material(
            "overlay-material",
            MaterialKind::Image,
            "file://overlay.png",
        ),
        material("audio-material", MaterialKind::Audio, "file://audio.wav"),
        material("text-material", MaterialKind::Text, "text://title"),
    ];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(segment("video-a", "video-material", 100_000, 0, 1_000_000));

    let mut overlay_track = Track::new("overlay-track", TrackKind::Video, "叠加");
    overlay_track
        .segments
        .push(segment("overlay-a", "overlay-material", 0, 0, 1_000_000));

    let mut audio_track = Track::new("audio-track", TrackKind::Audio, "音频");
    audio_track
        .segments
        .push(segment("audio-a", "audio-material", 0, 0, 1_000_000));

    let mut text_track = Track::new("text-track", TrackKind::Text, "文字");
    let mut text = segment("text-a", "text-material", 0, 500_000, 500_000);
    text.text = Some(TextSegment {
        content: "标题".to_owned(),
        source: TextSegmentSource::Text,
        style: TextStyle {
            font: TextFont {
                family: "Source Han Sans SC".to_owned(),
                font_ref: Some("source-han-local".to_owned()),
            },
            font_size: 48,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            line_height_millis: 1_500,
            letter_spacing_millis: 125,
            stroke: Some(TextStroke {
                color: "#101010".to_owned(),
                width: 3,
            }),
            shadow: Some(TextShadow {
                color: "#000000".to_owned(),
                offset_x: 4,
                offset_y: 6,
                blur: 8,
            }),
            background: Some(TextBackground {
                color: "#202020".to_owned(),
            }),
        },
        text_box: TextBox {
            width_millis: 600,
            height_millis: 200,
        },
        layout_region: TextLayoutRegion {
            x_millis: 100,
            y_millis: 700,
            width_millis: 800,
            height_millis: 200,
        },
        wrapping: TextWrapping::Auto,
        bubble: Some(TextBubbleRef::Unsupported {
            name: "气泡".to_owned(),
            external_ref: Some("bubble-vendor-01".to_owned()),
        }),
        effect: Some(TextEffectRef::Unsupported {
            name: "花字".to_owned(),
            external_ref: Some("effect-vendor-01".to_owned()),
        }),
    });
    text_track.segments.push(text);

    draft.tracks = vec![video_track, overlay_track, audio_track, text_track];
    draft
}

fn material(material_id: &str, kind: MaterialKind, uri: &str) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(2_000_000));
    material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    material.metadata.width = Some(1920);
    material.metadata.height = Some(1080);
    material.metadata.has_video = matches!(kind, MaterialKind::Video | MaterialKind::Image);
    material.metadata.has_audio = matches!(kind, MaterialKind::Audio | MaterialKind::Video);
    material
}

fn segment(
    segment_id: &str,
    material_id: &str,
    source_start: u64,
    target_start: u64,
    duration: u64,
) -> Segment {
    Segment::new(
        segment_id,
        material_id,
        SourceTimerange::new(source_start, duration),
        TargetTimerange::new(target_start, duration),
    )
}

fn default_visual_json() -> serde_json::Value {
    serde_json::json!({
        "visible": true,
        "transform": {
            "position": { "x": 0, "y": 0 },
            "scale": { "xMillis": 1000, "yMillis": 1000 },
            "rotation": { "degrees": 0 },
            "opacity": { "valueMillis": 1000 },
            "crop": {
                "leftMillis": 0,
                "rightMillis": 0,
                "topMillis": 0,
                "bottomMillis": 0
            },
            "anchor": { "xMillis": 500, "yMillis": 500 }
        },
        "fitMode": "stretch",
        "backgroundFilling": { "kind": "none" },
        "blendMode": { "kind": "normal" },
        "mask": { "kind": "none" }
    })
}
