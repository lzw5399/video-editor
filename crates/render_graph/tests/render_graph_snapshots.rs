use std::collections::BTreeMap;

use draft_model::{
    Draft, Filter, Material, MaterialKind, Microseconds, RationalFrameRate, Segment,
    SourceTimerange, TargetTimerange, TextAlignment, TextSegment, TextStyle, Track, TrackKind,
    Transition,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use render_graph::{RenderGraphErrorKind, build_render_graph};

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
            "canvas": { "width": 1920, "height": 1080 },
            "targetTimerange": { "start": 600000, "duration": 100000 },
            "frameRate": { "numerator": 30, "denominator": 1 },
            "materials": [
                {
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
                            "name": "lut",
                            "parameters": { "strengthMillis": "500" },
                            "support": "degraded",
                            "reason": "filter intent is preserved for compiler/runtime capability handling"
                        }
                    ],
                    "transition": {
                        "name": "crossfade",
                        "duration": 120000,
                        "support": "degraded",
                        "reason": "transition intent is preserved for compiler/runtime capability handling"
                    }
                },
                {
                    "trackId": "overlay-track",
                    "segmentId": "overlay-a",
                    "materialId": "overlay-material",
                    "materialKind": "image",
                    "stackIndex": 1,
                    "sourceTimerange": { "start": 0, "duration": 1000000 },
                    "targetTimerange": { "start": 0, "duration": 1000000 },
                    "keyframes": [],
                    "filters": [],
                    "transition": null
                }
            ],
            "audioMixes": [
                {
                    "trackId": "audio-track",
                    "segmentId": "audio-a",
                    "materialId": "audio-material",
                    "sourceTimerange": { "start": 0, "duration": 1000000 },
                    "targetTimerange": { "start": 0, "duration": 1000000 },
                    "volumeLevelMillis": 1000,
                    "filters": []
                },
                {
                    "trackId": "video-track",
                    "segmentId": "video-a",
                    "materialId": "video-material",
                    "sourceTimerange": { "start": 100000, "duration": 1000000 },
                    "targetTimerange": { "start": 0, "duration": 1000000 },
                    "volumeLevelMillis": 1000,
                    "filters": [
                        {
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
                    "overlay": {
                        "trackId": "text-track",
                        "segmentId": "text-a",
                        "content": "标题",
                        "stackIndex": 2,
                        "sourcePosition": 100000,
                        "targetTimerange": { "start": 500000, "duration": 500000 },
                        "fontFamily": "PingFang SC",
                        "fontCandidate": "VE_TEXT_FONT_PATH",
                        "fallbackCandidates": [
                            "VE_TEXT_FONT_PATH",
                            "/System/Library/Fonts/PingFang.ttc",
                            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
                        ],
                        "alignment": "center",
                        "safeArea": { "left": 96, "right": 96, "top": 54, "bottom": 54 },
                        "wrappingPolicy": "boundedWidth",
                        "fontSize": 48,
                        "layoutWidth": 1728,
                        "layoutHeight": 58
                    },
                    "materialId": "text-material",
                    "filters": [],
                    "transition": null
                }
            ],
            "sampledFrames": [
                { "frameIndex": 0, "at": 600000 },
                { "frameIndex": 1, "at": 633333 },
                { "frameIndex": 2, "at": 666666 }
            ]
        })
    );
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
        style: TextStyle {
            font_size: 48,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            stroke: None,
            shadow: None,
            background: None,
        },
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
