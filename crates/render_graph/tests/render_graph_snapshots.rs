#![recursion_limit = "256"]

use std::collections::BTreeMap;

use draft_model::{
    AudioEffectSlot, AudioEffectSlotKind, AudioFade, AudioPanBalance, Draft, Filter, Keyframe,
    KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue, Material, MaterialKind,
    Microseconds, RationalFrameRate, Segment, SegmentBackgroundFilling, SegmentBlendMode,
    SegmentFitMode, SegmentMask, SegmentPosition, SourceTimerange, TargetTimerange, TextAlignment,
    TextBackground, TextBox, TextBubbleRef, TextEffectRef, TextFont, TextLayoutRegion, TextSegment,
    TextSegmentSource, TextShadow, TextStroke, TextStyle, TextWrapping, Track, TrackKind,
    Transition,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use render_graph::{
    ExportMp4Preset, OutputDimensions, PreviewFrameFormat, RenderGraphErrorKind, RenderGraphPlan,
    RenderGraphSnapshot, RenderOutputProfile, build_render_graph,
};

#[test]
fn render_graph_builds_stable_visual_audio_and_text_intents_from_engine_range_state() {
    let normalized = normalize_draft(&render_graph_draft(), &EngineProfile::mvp_default())
        .expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
    )
    .expect("range state should resolve");

    let graph = build_render_graph(&normalized, &range).expect("graph should build");

    assert_eq!(
        serde_json::to_value(&graph).expect("graph should serialize"),
        serde_json::json!({
            "draftId": "draft-render-graph",
            "canvas": {
                "nodeId": canvas_node("draft-render-graph"),
                "width": 1920,
                "height": 1080
            },
            "targetTimerange": { "start": 600000, "duration": 100000 },
            "frameRate": { "numerator": 30, "denominator": 1 },
            "materials": [
                {
                    "nodeId": material_node("audio-material"),
                    "materialId": "audio-material",
                    "kind": "audio",
                    "uri": "file://audio.wav",
                    "displayName": "audio-material",
                    "duration": 2000000,
                    "frameRate": null,
                    "width": null,
                    "height": null,
                    "hasVideo": false,
                    "hasAudio": true
                },
                {
                    "nodeId": material_node("overlay-material"),
                    "materialId": "overlay-material",
                    "kind": "image",
                    "uri": "file://overlay.png",
                    "displayName": "overlay-material",
                    "duration": 2000000,
                    "frameRate": null,
                    "width": 640,
                    "height": 360,
                    "hasVideo": true,
                    "hasAudio": false
                },
                {
                    "nodeId": material_node("text-material"),
                    "materialId": "text-material",
                    "kind": "text",
                    "uri": "text://title",
                    "displayName": "text-material",
                    "duration": 2000000,
                    "frameRate": null,
                    "width": null,
                    "height": null,
                    "hasVideo": false,
                    "hasAudio": false
                },
                {
                    "nodeId": material_node("video-material"),
                    "materialId": "video-material",
                    "kind": "video",
                    "uri": "file://video.mp4",
                    "displayName": "video-material",
                    "duration": 2000000,
                    "frameRate": { "numerator": 30, "denominator": 1 },
                    "width": 1920,
                    "height": 1080,
                    "hasVideo": true,
                    "hasAudio": true
                }
            ],
            "videoLayers": [
                {
                    "nodeId": segment_node("video-track", "video-a", "video-material", "videoSegment"),
                    "trackId": "video-track",
                    "segmentId": "video-a",
                    "materialId": "video-material",
                    "materialKind": "video",
                    "stackIndex": 0,
                    "sourceTimerange": { "start": 100000, "duration": 1000000 },
                    "targetTimerange": { "start": 0, "duration": 1000000 },
                    "keyframes": [],
                    "filters": [
                        {
                            "nodeId": filter_node("video-track", "video-a", "video-material", "0"),
                            "name": "lut",
                            "parameters": { "strengthMillis": "500" },
                            "support": "degraded",
                            "reason": "filter intent is preserved for compiler/runtime capability handling"
                        }
                    ],
                    "transition": {
                        "nodeId": transition_node("video-track", "video-a", "video-material"),
                        "name": "crossfade",
                        "duration": 120000,
                        "support": "degraded",
                        "reason": "transition intent is preserved for compiler/runtime capability handling"
                    },
                    "visual": default_visual_json()
                },
                {
                    "nodeId": segment_node("overlay-track", "overlay-a", "overlay-material", "videoSegment"),
                    "trackId": "overlay-track",
                    "segmentId": "overlay-a",
                    "materialId": "overlay-material",
                    "materialKind": "image",
                    "stackIndex": 1,
                    "sourceTimerange": { "start": 0, "duration": 1000000 },
                    "targetTimerange": { "start": 0, "duration": 1000000 },
                    "keyframes": [],
                    "filters": [],
                    "transition": null,
                    "visual": default_visual_json()
                }
            ],
            "audioMixes": [
                {
                    "nodeId": segment_node("audio-track", "audio-a", "audio-material", "audioSegment"),
                    "trackId": "audio-track",
                    "segmentId": "audio-a",
                    "materialId": "audio-material",
                    "sourceTimerange": { "start": 0, "duration": 1000000 },
                    "targetTimerange": { "start": 0, "duration": 1000000 },
                    "volumeLevelMillis": 1000,
                    "filters": []
                },
                {
                    "nodeId": segment_node("video-track", "video-a", "video-material", "audioSegment"),
                    "trackId": "video-track",
                    "segmentId": "video-a",
                    "materialId": "video-material",
                    "sourceTimerange": { "start": 100000, "duration": 1000000 },
                    "targetTimerange": { "start": 0, "duration": 1000000 },
                    "volumeLevelMillis": 1000,
                    "filters": [
                        {
                            "nodeId": filter_node("video-track", "video-a", "video-material", "0"),
                            "name": "lut",
                            "parameters": { "strengthMillis": "500" },
                            "support": "degraded",
                            "reason": "filter intent is preserved for compiler/runtime capability handling"
                        }
                    ]
                }
            ],
            "textOverlays": [
                {
                    "nodeId": segment_node("text-track", "text-a", "text-material", "textOverlay"),
                    "overlay": {
                        "trackId": "text-track",
                        "segmentId": "text-a",
                        "content": "标题",
                        "stackIndex": 2,
                        "sourcePosition": 100000,
                        "targetTimerange": { "start": 500000, "duration": 500000 },
                        "source": "text",
                        "fontFamily": "Source Han Sans SC",
                        "fontRef": "source-han-local",
                        "fontCandidate": "VE_TEXT_FONT_PATH",
                        "fallbackCandidates": [
                            "VE_TEXT_FONT_PATH",
                            "/System/Library/Fonts/PingFang.ttc",
                            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
                        ],
                        "alignment": "center",
                        "textBox": {
                            "widthMillis": 600,
                            "heightMillis": 200,
                            "width": 1152,
                            "height": 216
                        },
                        "layoutRegion": {
                            "xMillis": 100,
                            "yMillis": 700,
                            "widthMillis": 800,
                            "heightMillis": 200,
                            "x": 192,
                            "y": 756,
                            "width": 1536,
                            "height": 216
                        },
                        "safeArea": { "left": 192, "right": 192, "top": 756, "bottom": 108 },
                        "wrapping": "auto",
                        "wrappingPolicy": "boundedWidth",
                        "lineHeightMillis": 1500,
                        "letterSpacingMillis": 125,
                        "fontSize": 48,
                        "style": {
                            "color": "#ffffff",
                            "stroke": {
                                "color": "#101010",
                                "width": 3
                            },
                            "shadow": {
                                "color": "#000000",
                                "offsetX": 4,
                                "offsetY": 6,
                                "blur": 8
                            },
                            "background": {
                                "color": "#202020"
                            }
                        },
                        "layoutWidth": 1152,
                        "layoutHeight": 72,
                        "diagnostics": [
                            {
                                "property": "bubble",
                                "support": "unsupported",
                                "reason": "text bubble 气泡 is unsupported"
                            },
                            {
                                "property": "effect",
                                "support": "unsupported",
                                "reason": "text effect 花字 is unsupported"
                            }
                        ]
                    },
                    "materialId": "text-material",
                    "filters": [],
                    "transition": null,
                    "visual": default_visual_json()
                }
            ],
            "sampledFrames": [
                { "nodeId": frame_node("0:at:600000"), "frameIndex": 0, "at": 600000 },
                { "nodeId": frame_node("1:at:633333"), "frameIndex": 1, "at": 633333 },
                { "nodeId": frame_node("2:at:666666"), "frameIndex": 2, "at": 666666 }
            ]
        })
    );
}

#[test]
fn render_graph_preserves_complete_text_and_subtitle_intent_without_ffmpeg_syntax() {
    let mut draft = render_graph_draft();
    draft.materials.push(material(
        "subtitle-material",
        MaterialKind::Text,
        "text://subtitle",
    ));
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
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
    )
    .expect("range state should resolve");

    let graph = build_render_graph(&normalized, &range).expect("graph should build");

    assert_eq!(
        graph
            .text_overlays
            .iter()
            .map(|overlay| {
                (
                    overlay.overlay.segment_id.as_str(),
                    overlay.overlay.source,
                    overlay.overlay.layout_width,
                    overlay.overlay.layout_height,
                    overlay.overlay.line_height_millis,
                    overlay.overlay.letter_spacing_millis,
                    overlay.overlay.diagnostics.len(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("text-a", TextSegmentSource::Text, 1_152, 72, 1_500, 125, 2),
            (
                "subtitle-a",
                TextSegmentSource::Subtitle,
                960,
                100,
                1_250,
                60,
                0
            ),
        ]
    );

    let snapshot = serde_json::to_string_pretty(&graph).expect("graph should serialize");
    assert!(snapshot.contains("\"source\": \"subtitle\""));
    assert!(snapshot.contains("\"letterSpacingMillis\": 60"));
    assert!(snapshot.contains("\"property\": \"bubble\""));
    assert!(!snapshot.contains("subtitles="));
    assert!(!snapshot.contains("force_style"));
    assert!(!snapshot.contains("ffmpeg"));
}

#[test]
fn transform_render_graph_preserves_visual_intent_without_ffmpeg_syntax() {
    let mut draft = render_graph_draft();
    let video = &mut draft.tracks[0].segments[0];
    video.visual.fit_mode = SegmentFitMode::Fill;
    video.visual.transform.position = SegmentPosition { x: 180, y: -90 };
    video.visual.background_filling = SegmentBackgroundFilling::Blur;
    video.visual.blend_mode = SegmentBlendMode::Unsupported {
        name: "screen".to_owned(),
    };
    video.visual.mask = SegmentMask::Unsupported {
        name: "linear".to_owned(),
    };
    draft.tracks[1].segments[0].visual.visible = false;
    draft.tracks[3].segments[0].visual.transform.position = SegmentPosition { x: 24, y: 48 };

    let normalized =
        normalize_draft(&draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
    )
    .expect("range state should resolve");

    let graph = build_render_graph(&normalized, &range).expect("graph should build");
    assert_eq!(
        graph.video_layers.len(),
        1,
        "hidden overlay should be omitted"
    );
    assert_eq!(graph.video_layers[0].visual.fit_mode, SegmentFitMode::Fill);
    assert_eq!(graph.video_layers[0].visual.transform.position.x, 180);
    assert_eq!(graph.text_overlays[0].visual.transform.position.y, 48);
    assert_eq!(graph.visual_diagnostics.len(), 3);
    assert!(graph.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "backgroundFilling"
            && diagnostic.support == render_graph::RenderIntentSupport::Degraded
    }));
    assert!(graph.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "blendMode"
            && diagnostic.support == render_graph::RenderIntentSupport::Unsupported
    }));
    assert!(graph.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "mask"
            && diagnostic.support == render_graph::RenderIntentSupport::Unsupported
    }));

    let snapshot = serde_json::to_string_pretty(&graph).expect("graph should serialize");
    assert!(snapshot.contains("\"visual\""));
    assert!(snapshot.contains("\"visualDiagnostics\""));
    assert!(!snapshot.contains("filter_complex"));
    assert!(!snapshot.contains("overlay="));
    assert!(!snapshot.contains("ffmpeg"));
}

#[test]
fn keyframe_render_graph_preserves_typed_intent_and_sampled_animation_states() {
    let mut draft = render_graph_draft();
    let video = &mut draft.tracks[0].segments[0];
    video.keyframes.extend([
        int_keyframe(KeyframeProperty::VisualPositionX, 600_000, 0),
        int_keyframe(KeyframeProperty::VisualPositionX, 666_666, 600),
    ]);
    let audio = &mut draft.tracks[2].segments[0];
    audio.keyframes.extend([
        uint_keyframe(KeyframeProperty::Volume, 600_000, 1_000),
        uint_keyframe(KeyframeProperty::Volume, 666_666, 2_000),
    ]);
    let text = &mut draft.tracks[3].segments[0];
    text.keyframes.extend([
        uint_keyframe(KeyframeProperty::TextFontSize, 100_000, 40),
        uint_keyframe(KeyframeProperty::TextFontSize, 166_666, 70),
    ]);

    let normalized =
        normalize_draft(&draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
    )
    .expect("range state should resolve");

    let graph = build_render_graph(&normalized, &range).expect("graph should build");

    assert_eq!(
        graph.video_layers[0]
            .keyframes
            .iter()
            .map(|keyframe| (&keyframe.property, keyframe.at.get()))
            .collect::<Vec<_>>(),
        vec![
            (&KeyframeProperty::VisualPositionX, 600_000),
            (&KeyframeProperty::VisualPositionX, 666_666),
        ]
    );
    assert_eq!(
        graph.audio_mixes[0]
            .keyframes
            .iter()
            .map(|keyframe| (&keyframe.property, keyframe.at.get()))
            .collect::<Vec<_>>(),
        vec![
            (&KeyframeProperty::Volume, 600_000),
            (&KeyframeProperty::Volume, 666_666),
        ]
    );
    assert_eq!(
        graph.text_overlays[0]
            .keyframes
            .iter()
            .map(|keyframe| (&keyframe.property, keyframe.at.get()))
            .collect::<Vec<_>>(),
        vec![
            (&KeyframeProperty::TextFontSize, 100_000),
            (&KeyframeProperty::TextFontSize, 166_666),
        ]
    );

    assert_eq!(
        graph
            .sampled_animation_states
            .iter()
            .map(|state| {
                (
                    state.at.get(),
                    state.visual_layers[0].visual.transform.position.x,
                    state.audio_segments[0].volume_level_millis,
                    state.text_overlays[0].font_size,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (600_000, 0, 1_000, 40),
            (633_333, 300, 1_500, 55),
            (666_666, 600, 2_000, 70),
        ]
    );
    assert!(graph.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "keyframe.visualPositionX"
            && diagnostic.support == render_graph::RenderIntentSupport::Degraded
    }));
    assert!(graph.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "keyframe.volume"
            && diagnostic.support == render_graph::RenderIntentSupport::Degraded
    }));
    assert!(graph.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "keyframe.textFontSize"
            && diagnostic.support == render_graph::RenderIntentSupport::Degraded
    }));

    let snapshot = serde_json::to_string_pretty(&graph).expect("graph should serialize");
    assert!(snapshot.contains("\"sampledAnimationStates\""));
    assert!(snapshot.contains("\"keyframes\""));
    assert!(!snapshot.contains("filter_complex"));
    assert!(!snapshot.contains("ffmpeg"));
}

#[test]
fn audio_render_graph_maps_dsp_mix_intent_for_export_without_ffmpeg_syntax() {
    let mut draft = render_graph_draft();
    let audio = &mut draft.tracks[2].segments[0];
    audio.audio.gain_millis = 750;
    audio.audio.pan_balance_millis = AudioPanBalance {
        balance_millis: -400,
    };
    audio.audio.fade_in_duration = AudioFade {
        duration: Microseconds::new(125_000),
    };
    audio.audio.fade_out_duration = AudioFade {
        duration: Microseconds::new(250_000),
    };
    audio.audio.effect_slots.push(AudioEffectSlot {
        slot_id: "slot-vendor-space".to_owned(),
        enabled: true,
        kind: AudioEffectSlotKind::Unsupported {
            name: "vendor-space".to_owned(),
            external_ref: Some("jianying://audio/effects/space".to_owned()),
        },
    });
    audio
        .keyframes
        .push(uint_keyframe(KeyframeProperty::Volume, 625_000, 1_250));

    let normalized =
        normalize_draft(&draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
    )
    .expect("range state should resolve");

    let graph = build_render_graph(&normalized, &range).expect("graph should build");
    let mix = graph
        .audio_mixes
        .iter()
        .find(|mix| mix.segment_id.as_str() == "audio-a")
        .expect("audio segment should map into render graph");

    assert_eq!(mix.gain_millis, 750);
    assert_eq!(mix.pan_balance_millis, -400);
    assert_eq!(mix.fade_in_duration, Microseconds::new(125_000));
    assert_eq!(mix.fade_out_duration, Microseconds::new(250_000));
    assert_eq!(mix.volume_keyframes.len(), 1);
    assert_eq!(
        mix.volume_keyframes[0].target_time,
        Microseconds::new(625_000)
    );
    assert_eq!(mix.volume_keyframes[0].gain_millis, 1_250);
    assert_eq!(mix.effect_slots.len(), 1);
    assert_eq!(mix.effect_slots[0].slot_id, "slot-vendor-space");
    assert_eq!(
        mix.effect_slots[0].support,
        render_graph::RenderAudioEffectSlotSupport::Unsupported
    );

    let snapshot = serde_json::to_string_pretty(&graph).expect("graph should serialize");
    assert!(snapshot.contains("\"gainMillis\": 750"));
    assert!(snapshot.contains("\"panBalanceMillis\": -400"));
    assert!(snapshot.contains("\"fadeInDuration\": 125000"));
    assert!(snapshot.contains("\"fadeOutDuration\": 250000"));
    assert!(snapshot.contains("\"support\": \"unsupported\""));
    assert!(!snapshot.contains("filter_complex"));
    assert!(!snapshot.contains("atrim"));
    assert!(!snapshot.contains("volume="));
    assert!(!snapshot.contains("pan="));
}

#[test]
fn render_graph_preserves_filter_and_transition_intents_without_ffmpeg_syntax() {
    let normalized = normalize_draft(&render_graph_draft(), &EngineProfile::mvp_default())
        .expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
    )
    .expect("range state should resolve");

    let graph = build_render_graph(&normalized, &range).expect("graph should build");
    let snapshot = serde_json::to_string_pretty(&graph).expect("graph should serialize");

    assert!(snapshot.contains("\"support\": \"degraded\""));
    assert!(snapshot.contains("\"name\": \"crossfade\""));
    assert!(!snapshot.contains("ffmpeg"));
    assert!(!snapshot.contains("-filter_complex"));
    assert!(!snapshot.contains("overlay="));
}

#[test]
fn render_graph_rejects_range_state_from_a_different_normalized_draft() {
    let normalized = normalize_draft(&render_graph_draft(), &EngineProfile::mvp_default())
        .expect("draft should normalize");
    let other = normalize_draft(&unrelated_draft(), &EngineProfile::mvp_default())
        .expect("other draft should normalize");
    let range = resolve_render_range(
        &other,
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(100_000)),
    )
    .expect("other range state should resolve");

    let error = build_render_graph(&normalized, &range)
        .expect_err("range state must come from the same normalized draft semantics");

    assert_eq!(error.kind, RenderGraphErrorKind::UnknownSegmentInRangeState);
}

#[test]
fn output_profiles_share_the_same_graph_shape_with_distinct_profile_metadata() {
    let graph = sample_graph();
    let preview_frame = RenderGraphPlan::new(
        graph.clone(),
        RenderOutputProfile::preview_frame_png(
            OutputDimensions::new(960, 540),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(33_333)),
        ),
    )
    .expect("preview frame profile should validate");
    let preview_segment = RenderGraphPlan::new(
        graph.clone(),
        RenderOutputProfile::preview_segment_mp4(
            OutputDimensions::new(960, 540),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
        ),
    )
    .expect("preview segment profile should validate");
    let export = RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export profile should validate");

    let frame_snapshot = serde_json::to_value(&preview_frame).expect("frame plan serializes");
    let segment_snapshot = serde_json::to_value(&preview_segment).expect("segment plan serializes");
    let export_snapshot = serde_json::to_value(&export).expect("export plan serializes");

    assert_eq!(frame_snapshot["graph"], segment_snapshot["graph"]);
    assert_eq!(segment_snapshot["graph"], export_snapshot["graph"]);
    assert_eq!(
        frame_snapshot["outputProfile"],
        serde_json::json!({
            "kind": "previewFrame",
            "profileId": "preview-frame-png",
            "dimensions": { "width": 960, "height": 540 },
            "frameRate": { "numerator": 30, "denominator": 1 },
            "targetTimerange": { "start": 600000, "duration": 33333 },
            "format": "png",
            "validationHints": [
                "single-frame still output",
                "preserve alpha only if compiler/runtime supports it"
            ]
        })
    );
    assert_eq!(
        segment_snapshot["outputProfile"],
        serde_json::json!({
            "kind": "previewSegment",
            "profileId": "preview-segment-mp4-h264",
            "dimensions": { "width": 960, "height": 540 },
            "frameRate": { "numerator": 30, "denominator": 1 },
            "targetTimerange": { "start": 600000, "duration": 100000 },
            "container": "mp4",
            "videoCodec": "h264",
            "audioCodec": "aac",
            "presetId": "preview-segment-balanced",
            "validationHints": [
                "short derived preview cache artifact",
                "compiled through the same render graph as export"
            ]
        })
    );
    assert_eq!(
        export_snapshot["outputProfile"],
        serde_json::json!({
            "kind": "exportMp4",
            "profileId": "export-mp4-h264-balanced",
            "dimensions": { "width": 1920, "height": 1080 },
            "frameRate": { "numerator": 30, "denominator": 1 },
            "targetTimerange": { "start": 600000, "duration": 100000 },
            "preset": {
                "presetId": "h264-aac-balanced",
                "container": "mp4",
                "videoCodec": "h264",
                "audioCodec": "aac",
                "crf": 20,
                "audioBitrateKbps": 192
            },
            "validationHints": [
                "validate file exists and is non-empty",
                "validate duration, frame rate, resolution, and audio stream with runtime metadata"
            ]
        })
    );
}

#[test]
fn render_graph_snapshot_collects_in_memory_node_fingerprints() {
    let graph = sample_graph();
    let output_profile = RenderOutputProfile::preview_frame_png(
        OutputDimensions::new(960, 540),
        RationalFrameRate::new(30, 1),
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(33_333)),
    );

    let snapshot = RenderGraphSnapshot::from_graph(&graph, &output_profile, "runtime:software:v1");

    assert_eq!(snapshot.draft_id.as_str(), "draft-render-graph");
    assert_eq!(
        snapshot.graph_schema_version,
        render_graph::GRAPH_SCHEMA_VERSION
    );
    assert_eq!(
        snapshot.generator_version,
        render_graph::GRAPH_GENERATOR_VERSION
    );
    assert!(
        snapshot
            .node_fingerprint_by_key("draft:draft-render-graph:canvas")
            .is_some()
    );
    assert!(
        snapshot
            .node_fingerprint_by_key(
                "draft:draft-render-graph:track:video-track:segment:video-a:video"
            )
            .is_some()
    );
    assert!(
        snapshot
            .node_fingerprint_by_key(
                "draft:draft-render-graph:track:video-track:segment:video-a:filter:0"
            )
            .is_some()
    );
    assert!(
        snapshot
            .node_fingerprint_by_key(
                "draft:draft-render-graph:track:video-track:segment:video-a:transition"
            )
            .is_some()
    );
    assert_eq!(
        snapshot
            .node_fingerprints
            .iter()
            .map(|fingerprint| fingerprint.node_id.stable_key())
            .collect::<Vec<_>>(),
        {
            let mut keys = snapshot
                .node_fingerprints
                .iter()
                .map(|fingerprint| fingerprint.node_id.stable_key())
                .collect::<Vec<_>>();
            keys.sort();
            keys
        },
        "snapshot fingerprints should be sorted for deterministic comparisons"
    );
}

#[test]
fn output_profiles_reject_unsupported_dimensions_frame_rates_and_ranges() {
    let graph = sample_graph();

    let error = RenderGraphPlan::new(
        graph.clone(),
        RenderOutputProfile::preview_frame(
            "custom-preview-frame",
            OutputDimensions::new(0, 540),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(33_333)),
            PreviewFrameFormat::Png,
        ),
    )
    .expect_err("zero width should be classified");
    assert_eq!(error.kind, RenderGraphErrorKind::UnsupportedProfileSetting);

    let error = RenderGraphPlan::new(
        graph,
        RenderOutputProfile::preview_segment_mp4(
            OutputDimensions::new(960, 540),
            RationalFrameRate::new(0, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
        ),
    )
    .expect_err("zero frame-rate numerator should be classified");
    assert_eq!(error.kind, RenderGraphErrorKind::UnsupportedProfileSetting);
}

fn sample_graph() -> render_graph::RenderGraph {
    let normalized = normalize_draft(&render_graph_draft(), &EngineProfile::mvp_default())
        .expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
    )
    .expect("range state should resolve");
    build_render_graph(&normalized, &range).expect("graph should build")
}

fn render_graph_draft() -> Draft {
    let mut draft = Draft::new("draft-render-graph", "Render Graph");
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
    let mut video = segment("video-a", "video-material", 100_000, 0, 1_000_000);
    video.filters.push(Filter {
        name: "lut".to_owned(),
        parameters: BTreeMap::from([("strengthMillis".to_owned(), "500".to_owned())]),
    });
    video.transition = Some(Transition {
        name: "crossfade".to_owned(),
        duration: Microseconds::new(120_000),
    });
    video_track.segments.push(video);

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

fn unrelated_draft() -> Draft {
    let mut draft = Draft::new("draft-unrelated", "Unrelated");
    draft.materials = vec![material(
        "other-video-material",
        MaterialKind::Video,
        "file://other.mp4",
    )];
    let mut track = Track::new("other-video-track", TrackKind::Video, "视频");
    track.segments.push(segment(
        "other-video-a",
        "other-video-material",
        0,
        0,
        1_000_000,
    ));
    draft.tracks = vec![track];
    draft
}

fn material(material_id: &str, kind: MaterialKind, uri: &str) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(2_000_000));
    match kind {
        MaterialKind::Video => {
            material.metadata.width = Some(1_920);
            material.metadata.height = Some(1_080);
            material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
            material.metadata.has_video = true;
            material.metadata.has_audio = true;
        }
        MaterialKind::Image => {
            material.metadata.width = Some(640);
            material.metadata.height = Some(360);
            material.metadata.has_video = true;
        }
        MaterialKind::Audio => {
            material.metadata.has_audio = true;
        }
        MaterialKind::Text | MaterialKind::Sticker => {}
    }
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
        SourceTimerange::new(Microseconds::new(source_start), Microseconds::new(duration)),
        TargetTimerange::new(Microseconds::new(target_start), Microseconds::new(duration)),
    )
}

fn int_keyframe(property: KeyframeProperty, at: u64, value: i32) -> Keyframe {
    Keyframe {
        at: Microseconds::new(at),
        property,
        value: KeyframeValue::Int { value },
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    }
}

fn uint_keyframe(property: KeyframeProperty, at: u64, value: u32) -> Keyframe {
    Keyframe {
        at: Microseconds::new(at),
        property,
        value: KeyframeValue::Uint { value },
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    }
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

fn canvas_node(draft_id: &str) -> serde_json::Value {
    serde_json::json!({
        "role": "canvas",
        "draftId": draft_id
    })
}

fn material_node(material_id: &str) -> serde_json::Value {
    serde_json::json!({
        "role": "material",
        "draftId": "draft-render-graph",
        "materialId": material_id
    })
}

fn segment_node(
    track_id: &str,
    segment_id: &str,
    material_id: &str,
    role: &str,
) -> serde_json::Value {
    serde_json::json!({
        "role": role,
        "draftId": "draft-render-graph",
        "trackId": track_id,
        "segmentId": segment_id,
        "materialId": material_id
    })
}

fn filter_node(
    track_id: &str,
    segment_id: &str,
    material_id: &str,
    local_id: &str,
) -> serde_json::Value {
    serde_json::json!({
        "role": "segmentFilter",
        "draftId": "draft-render-graph",
        "trackId": track_id,
        "segmentId": segment_id,
        "materialId": material_id,
        "localId": local_id
    })
}

fn transition_node(track_id: &str, segment_id: &str, material_id: &str) -> serde_json::Value {
    serde_json::json!({
        "role": "segmentTransition",
        "draftId": "draft-render-graph",
        "trackId": track_id,
        "segmentId": segment_id,
        "materialId": material_id
    })
}

fn frame_node(local_id: &str) -> serde_json::Value {
    serde_json::json!({
        "role": "sampledFrame",
        "draftId": "draft-render-graph",
        "localId": local_id
    })
}
