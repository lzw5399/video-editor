#![allow(dead_code)]

use std::collections::BTreeMap;

use draft_model::{
    AudioEffectSlot, AudioEffectSlotKind, AudioFade, AudioPanBalance, Draft, Filter, Keyframe,
    KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue, Material, MaterialKind,
    Microseconds, RationalFrameRate, Segment, SourceTimerange, TargetTimerange, TextAlignment,
    TextBackground, TextBox, TextBubbleRef, TextEffectRef, TextFont, TextLayoutRegion, TextSegment,
    TextSegmentSource, TextShadow, TextStroke, TextStyle, TextWrapping, Track, TrackKind,
    Transition, BUNDLED_TEXT_FONT_REF,
};
use engine_core::{normalize_draft, resolve_render_range, EngineProfile};
use ffmpeg_compiler::{CompileContext, CompilerCapabilities, TextRenderCapability};
use render_graph::{
    build_render_graph, ExportMp4Preset, OutputDimensions, RenderGraphPlan, RenderOutputProfile,
};

pub fn compile_context() -> CompileContext {
    CompileContext::new("/derived/output.mp4", "/derived")
        .with_capabilities(CompilerCapabilities::all_available_for_tests())
}

pub fn preview_frame_context() -> CompileContext {
    CompileContext::new("/derived/preview.png", "/derived")
        .with_capabilities(CompilerCapabilities::all_available_for_tests())
}

pub fn no_font_context() -> CompileContext {
    CompileContext::new("/derived/output.mp4", "/derived").with_capabilities(
        CompilerCapabilities::all_available_for_tests().with_text(TextRenderCapability {
            supports_ass_filter: true,
            supports_subtitles_filter: true,
            env_text_font_path: None,
            available_font_paths: Vec::new(),
            bundled_font_ref: None,
            bundled_font_family: None,
            bundled_font_path: None,
            bundled_font_license: None,
        }),
    )
}

pub fn no_h264_context() -> CompileContext {
    CompileContext::new("/derived/output.mp4", "/derived")
        .with_capabilities(CompilerCapabilities::all_available_for_tests().with_h264_encoder(false))
}

pub fn no_subtitle_filter_context() -> CompileContext {
    CompileContext::new("/derived/output.mp4", "/derived").with_capabilities(
        CompilerCapabilities::all_available_for_tests().with_text(TextRenderCapability {
            supports_ass_filter: false,
            supports_subtitles_filter: true,
            env_text_font_path: Some("/fonts/PingFang.ttc".to_owned()),
            available_font_paths: vec!["/fonts/PingFang.ttc".to_owned()],
            bundled_font_ref: None,
            bundled_font_family: None,
            bundled_font_path: None,
            bundled_font_license: None,
        }),
    )
}

pub fn preview_frame_plan() -> RenderGraphPlan {
    let graph = sample_graph();
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::preview_frame_png(
            OutputDimensions::new(960, 540),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(33_333)),
        ),
    )
    .expect("preview frame plan should validate")
}

pub fn preview_segment_plan() -> RenderGraphPlan {
    let graph = sample_graph();
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::preview_segment_mp4(
            OutputDimensions::new(960, 540),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
        ),
    )
    .expect("preview segment plan should validate")
}

pub fn export_plan() -> RenderGraphPlan {
    let graph = sample_graph();
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export plan should validate")
}

pub fn export_plan_with_unsupported_text_resources() -> RenderGraphPlan {
    let mut draft = compiler_draft();
    let text = draft.tracks[3].segments[0]
        .text
        .as_mut()
        .expect("compiler draft should include text");
    text.style.font.font_ref = Some("vendor-font-ref".to_owned());
    text.bubble = Some(TextBubbleRef::Unsupported {
        name: "气泡".to_owned(),
        external_ref: Some("bubble-vendor-01".to_owned()),
    });
    text.effect = Some(TextEffectRef::Unsupported {
        name: "花字".to_owned(),
        external_ref: Some("effect-vendor-01".to_owned()),
    });

    let graph = sample_graph_from_draft(&draft);
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export plan should validate")
}

pub fn export_plan_with_bundled_font_ref() -> RenderGraphPlan {
    let mut draft = compiler_draft();
    let text = draft.tracks[3].segments[0]
        .text
        .as_mut()
        .expect("compiler draft should include text");
    text.style.font = TextFont::default();
    assert_eq!(
        text.style.font.font_ref.as_deref(),
        Some(BUNDLED_TEXT_FONT_REF)
    );

    let graph = sample_graph_from_draft(&draft);
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export plan should validate")
}

pub fn export_plan_with_wrapped_text() -> RenderGraphPlan {
    let mut draft = compiler_draft();
    let text = draft.tracks[3].segments[0]
        .text
        .as_mut()
        .expect("compiler draft should include text");
    text.content = "abcdefghij".to_owned();
    text.text_box.width_millis = 100;

    let graph = sample_graph_from_draft(&draft);
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export plan should validate")
}

pub fn export_plan_with_audio_mix_intent() -> RenderGraphPlan {
    let mut draft = compiler_draft();
    let audio = &mut draft.tracks[2].segments[0];
    audio.audio.gain_millis = 750;
    audio.audio.pan_balance_millis = AudioPanBalance {
        balance_millis: -500,
    };
    audio.audio.fade_in_duration = AudioFade {
        duration: Microseconds::new(100_000),
    };
    audio.audio.fade_out_duration = AudioFade {
        duration: Microseconds::new(200_000),
    };
    audio.audio.effect_slots.push(AudioEffectSlot {
        slot_id: "slot-external-space".to_owned(),
        enabled: true,
        kind: AudioEffectSlotKind::Unsupported {
            name: "external-space".to_owned(),
            external_ref: Some("jianying://effect/external-space".to_owned()),
        },
    });
    let graph = sample_graph_from_draft(&draft);
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export plan should validate")
}

pub fn export_plan_with_audio_volume_keyframes() -> RenderGraphPlan {
    let mut draft = compiler_draft();
    let audio = &mut draft.tracks[2].segments[0];
    audio.keyframes.push(Keyframe {
        at: Microseconds::new(650_000),
        property: KeyframeProperty::Volume,
        value: KeyframeValue::Uint { value: 1_250 },
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    });

    let graph = sample_graph_from_draft(&draft);
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export plan should validate")
}

pub fn export_plan_with_delayed_audio_mix() -> RenderGraphPlan {
    let mut draft = compiler_draft();
    let audio = &mut draft.tracks[2].segments[0];
    audio.target_timerange =
        TargetTimerange::new(Microseconds::new(500_000), Microseconds::new(500_000));

    let graph = sample_graph_from_draft(&draft);
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export plan should validate")
}

pub fn export_plan_with_audio_outside_output_range() -> RenderGraphPlan {
    let mut draft = compiler_draft();
    draft.materials[0].metadata.has_audio = false;
    let audio = &mut draft.tracks[2].segments[0];
    audio.target_timerange =
        TargetTimerange::new(Microseconds::new(900_000), Microseconds::new(100_000));

    let graph = sample_graph_from_draft_for_range(
        &draft,
        TargetTimerange::new(Microseconds::new(900_000), Microseconds::new(100_000)),
    );
    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::ZERO, Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export plan should validate")
}

fn sample_graph() -> render_graph::RenderGraph {
    sample_graph_from_draft(&compiler_draft())
}

fn sample_graph_from_draft(draft: &Draft) -> render_graph::RenderGraph {
    sample_graph_from_draft_for_range(
        draft,
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
    )
}

fn sample_graph_from_draft_for_range(
    draft: &Draft,
    target_timerange: TargetTimerange,
) -> render_graph::RenderGraph {
    let normalized =
        normalize_draft(draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    let range =
        resolve_render_range(&normalized, target_timerange).expect("range state should resolve");
    build_render_graph(&normalized, &range).expect("graph should build")
}

pub fn compiler_draft() -> Draft {
    let mut draft = Draft::new("draft-compiler", "Compiler");
    draft.materials = vec![
        material(
            "video-material",
            MaterialKind::Video,
            "file:///media/video.mp4",
        ),
        material(
            "overlay-material",
            MaterialKind::Image,
            "file:///media/overlay.png",
        ),
        material(
            "audio-material",
            MaterialKind::Audio,
            "file:///media/audio.wav",
        ),
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
        content: "标题 {一}\n第二行".to_owned(),
        source: TextSegmentSource::Text,
        style: TextStyle {
            font: TextFont {
                family: "PingFang SC".to_owned(),
                font_ref: None,
            },
            font_size: 48,
            color: "#33ccff".to_owned(),
            alignment: TextAlignment::Center,
            line_height_millis: 1_500,
            letter_spacing_millis: 125,
            stroke: Some(TextStroke {
                color: "#101010".to_owned(),
                width: 2,
            }),
            shadow: Some(TextShadow {
                color: "#000000".to_owned(),
                offset_x: 2,
                offset_y: 2,
                blur: 4,
            }),
            background: Some(TextBackground {
                color: "#202020".to_owned(),
            }),
        },
        text_box: TextBox {
            width_millis: 600,
            height_millis: 260,
        },
        layout_region: TextLayoutRegion {
            x_millis: 100,
            y_millis: 700,
            width_millis: 800,
            height_millis: 200,
        },
        wrapping: TextWrapping::Auto,
        bubble: None,
        effect: None,
    });
    text_track.segments.push(text);

    draft.tracks = vec![video_track, overlay_track, audio_track, text_track];
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
            material.metadata.audio_sample_rate = Some(48_000);
            material.metadata.audio_channels = Some(2);
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
