use draft_commands::{
    canvas::update_draft_canvas_config, timeline::add_segment as command_add_segment,
};
use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground,
    CommandDeltaName, CommandState, DirtyDomain, Draft, DraftCanvasConfig, Material, MaterialKind,
    Microseconds, RationalFrameRate, SegmentFitMode, SourceTimerange, TargetTimerange,
    TimelineSelection, Track, TrackKind,
};

#[test]
fn first_portrait_video_adopts_vertical_canvas_and_frame_rate() {
    let draft = draft_with_visual_track_and_material(material(
        "portrait-video",
        MaterialKind::Video,
        1080,
        1920,
        Some(RationalFrameRate::new(30000, 1001)),
    ));

    let response = add_first_segment(&draft, "portrait-video");

    assert_eq!(response.draft.canvas_config.width, 1080);
    assert_eq!(response.draft.canvas_config.height, 1920);
    assert_eq!(
        response.draft.canvas_config.aspect_ratio,
        CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio9x16)
    );
    assert_eq!(
        response.draft.canvas_config.frame_rate,
        RationalFrameRate::new(30000, 1001)
    );
    assert_eq!(
        response.draft.canvas_config.adaptation_policy,
        CanvasAdaptationPolicy::Auto
    );
    assert_eq!(
        response.draft.tracks[0].segments[0].visual.fit_mode,
        SegmentFitMode::Fit
    );
    assert_eq!(response.delta.command, CommandDeltaName::AddSegment);
    assert!(response.delta.invalidation.full_draft);
    assert!(
        response
            .delta
            .changed_domains
            .contains(&DirtyDomain::Canvas)
    );
    assert!(
        response
            .events
            .iter()
            .any(|event| event.kind == "draftCanvasAutoAdapted")
    );
}

#[test]
fn first_square_image_adopts_square_canvas_without_changing_frame_rate() {
    let draft = draft_with_visual_track_and_material(material(
        "square-image",
        MaterialKind::Image,
        1200,
        1200,
        None,
    ));

    let response = add_first_segment(&draft, "square-image");

    assert_eq!(response.draft.canvas_config.width, 1200);
    assert_eq!(response.draft.canvas_config.height, 1200);
    assert_eq!(
        response.draft.canvas_config.aspect_ratio,
        CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio1x1)
    );
    assert_eq!(
        response.draft.canvas_config.frame_rate,
        DraftCanvasConfig::mvp_default().frame_rate
    );
}

#[test]
fn first_landscape_video_adopts_landscape_canvas() {
    let draft = draft_with_visual_track_and_material(material(
        "landscape-video",
        MaterialKind::Video,
        3840,
        2160,
        Some(RationalFrameRate::new(24, 1)),
    ));

    let response = add_first_segment(&draft, "landscape-video");

    assert_eq!(response.draft.canvas_config.width, 3840);
    assert_eq!(response.draft.canvas_config.height, 2160);
    assert_eq!(
        response.draft.canvas_config.aspect_ratio,
        CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9)
    );
    assert_eq!(
        response.draft.canvas_config.frame_rate,
        RationalFrameRate::new(24, 1)
    );
}

#[test]
fn manual_canvas_update_prevents_later_auto_adaptation() {
    let draft = draft_with_visual_track_and_material(material(
        "portrait-video",
        MaterialKind::Video,
        1080,
        1920,
        Some(RationalFrameRate::new(30, 1)),
    ));
    let manual_canvas = DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
        width: 1280,
        height: 720,
        frame_rate: RationalFrameRate::new(25, 1),
        background: CanvasBackground::Black,
        adaptation_policy: CanvasAdaptationPolicy::Auto,
    };

    let manual = update_draft_canvas_config(
        &draft,
        &CommandState::empty(),
        &TimelineSelection::empty(),
        manual_canvas,
    )
    .expect("manual canvas update should commit");
    assert_eq!(
        manual.draft.canvas_config.adaptation_policy,
        CanvasAdaptationPolicy::Manual
    );

    let response = add_first_segment(&manual.draft, "portrait-video");

    assert_eq!(response.draft.canvas_config.width, 1280);
    assert_eq!(response.draft.canvas_config.height, 720);
    assert_eq!(
        response.draft.canvas_config.frame_rate,
        RationalFrameRate::new(25, 1)
    );
    assert!(
        !response
            .events
            .iter()
            .any(|event| event.kind == "draftCanvasAutoAdapted")
    );
}

fn draft_with_visual_track_and_material(material: Material) -> Draft {
    let mut draft = Draft::new("canvas-adaptation-draft", "Canvas Adaptation");
    draft
        .tracks
        .push(Track::new("video-track", TrackKind::Video, "Video"));
    draft.materials.push(material);
    draft
}

fn add_first_segment(draft: &Draft, material_id: &str) -> draft_model::TimelineCommandResponse {
    command_add_segment(
        draft,
        &CommandState::empty(),
        &TimelineSelection::empty(),
        "video-track".into(),
        "first-segment".into(),
        material_id.into(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    )
    .expect("first segment should commit")
}

fn material(
    material_id: &str,
    kind: MaterialKind,
    width: u32,
    height: u32,
    frame_rate: Option<RationalFrameRate>,
) -> Material {
    let mut material = Material::new(
        material_id,
        kind,
        format!("media/{material_id}"),
        material_id,
    );
    material.metadata.duration = Some(Microseconds::new(2_000_000));
    material.metadata.width = Some(width);
    material.metadata.height = Some(height);
    material.metadata.frame_rate = frame_rate;
    material.metadata.has_video = matches!(kind, MaterialKind::Video | MaterialKind::Image);
    material
}
