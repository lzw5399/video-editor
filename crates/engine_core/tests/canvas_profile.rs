use draft_model::{
    CanvasAspectRatio, CanvasAspectRatioPreset, CanvasPixelPoint, Draft, DraftCanvasConfig,
    Material, MaterialKind, Microseconds, NormalizedCanvasPoint, RationalFrameRate, Segment,
    SourceTimerange, TargetTimerange, TextAlignment, TextSegment, TextStyle, Track, TrackKind,
    normalized_to_canvas_pixel,
};
use engine_core::{EngineProfile, normalize_draft, resolve_frame_state, resolve_render_range};

#[test]
fn vertical_draft_canvas_resolves_engine_profile_and_text_layout() {
    let draft = draft_with_canvas(
        "draft-vertical-canvas",
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio9x16),
            width: 1_080,
            height: 1_920,
            frame_rate: RationalFrameRate::new(25, 1),
            background: draft_model::CanvasBackground::Black,
        },
    );

    let profile =
        EngineProfile::from_draft_canvas(&draft).expect("draft canvas profile should resolve");
    assert_eq!(profile.canvas_width, 1_080);
    assert_eq!(profile.canvas_height, 1_920);
    assert_eq!(profile.frame_rate, RationalFrameRate::new(25, 1));
    profile
        .validate()
        .expect("text layout derived from vertical canvas should be valid");

    let normalized = normalize_draft(&draft, &profile).expect("vertical draft should normalize");
    let frame = resolve_frame_state(&normalized, Microseconds::ZERO)
        .expect("text frame state should resolve");
    let overlay = frame
        .text_overlays
        .first()
        .expect("active text overlay should be resolved");
    assert_eq!(overlay.layout_width, 972);
    assert_eq!(overlay.safe_area.left, 54);
    assert_eq!(overlay.safe_area.top, 96);
}

#[test]
fn square_draft_canvas_drives_render_range_without_mvp_defaults() {
    let draft = draft_with_canvas(
        "draft-square-canvas",
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio1x1),
            width: 1_024,
            height: 1_024,
            frame_rate: RationalFrameRate::new(24, 1),
            background: draft_model::CanvasBackground::Black,
        },
    );

    let profile =
        EngineProfile::from_draft_canvas(&draft).expect("square canvas profile should resolve");
    let normalized = normalize_draft(&draft, &profile).expect("square draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    )
    .expect("square render range should resolve");

    assert_eq!(normalized.profile.canvas_width, 1_024);
    assert_eq!(normalized.profile.canvas_height, 1_024);
    assert_eq!(range.frame_rate, RationalFrameRate::new(24, 1));
    assert_eq!(range.frames.len(), 24);
    assert_ne!(normalized.profile.canvas_width, 1_920);
    assert_ne!(range.frame_rate, RationalFrameRate::new(30, 1));
}

#[test]
fn coordinate_conversion_uses_documented_center_origin_canvas_contract() {
    // docs/canvas-coordinate-system.md: origin at canvas center, +X right, +Y up,
    // and 1.0 maps to half canvas width/height from the center.
    let draft = draft_with_canvas(
        "draft-custom-canvas",
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::custom(5, 4),
            width: 1_500,
            height: 1_200,
            frame_rate: RationalFrameRate::new(48, 1),
            background: draft_model::CanvasBackground::Black,
        },
    );
    let profile =
        EngineProfile::from_draft_canvas(&draft).expect("custom canvas profile should resolve");

    assert_eq!(
        normalized_to_canvas_pixel(
            profile.canvas_width,
            profile.canvas_height,
            NormalizedCanvasPoint::CENTER,
        ),
        Some(CanvasPixelPoint { x: 750.0, y: 600.0 })
    );
    assert_eq!(
        normalized_to_canvas_pixel(
            profile.canvas_width,
            profile.canvas_height,
            NormalizedCanvasPoint { x: 1.0, y: 1.0 },
        ),
        Some(CanvasPixelPoint { x: 1_500.0, y: 0.0 })
    );
    assert_eq!(
        normalized_to_canvas_pixel(
            profile.canvas_width,
            profile.canvas_height,
            NormalizedCanvasPoint { x: -1.0, y: -1.0 },
        ),
        Some(CanvasPixelPoint { x: 0.0, y: 1_200.0 })
    );
}

fn draft_with_canvas(draft_id: &str, canvas_config: DraftCanvasConfig) -> Draft {
    let mut draft = Draft::new(draft_id, "Canvas Profile");
    draft.canvas_config = canvas_config;
    draft.materials = vec![material(
        "text-material",
        MaterialKind::Text,
        "text://caption",
    )];

    let mut text_track = Track::new("text-track", TrackKind::Text, "文字");
    let mut text = segment("text-a", "text-material", 0, 0, 1_000_000);
    text.text = Some(TextSegment {
        content: "画布文字".to_owned(),
        source: Default::default(),
        style: TextStyle {
            font_size: 40,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            ..TextStyle::default()
        },
        text_box: Default::default(),
        layout_region: Default::default(),
        wrapping: Default::default(),
        bubble: None,
        effect: None,
    });
    text_track.segments.push(text);
    draft.tracks = vec![text_track];
    draft
}

fn material(material_id: &str, kind: MaterialKind, uri: &str) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(1_000_000));
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
