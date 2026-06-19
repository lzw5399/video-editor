use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground,
    CanvasBackgroundCapability, CanvasPixelPoint, Draft, DraftCanvasConfig, DraftValidationError,
    Material, MaterialKind, Microseconds, NormalizedCanvasPoint, RationalFrameRate,
    canvas_pixel_to_normalized, normalized_to_canvas_pixel, validate_draft,
};

#[test]
fn draft_new_creates_default_canvas_config() {
    let draft = Draft::new("draft-001", "Canvas draft");

    assert_eq!(
        draft.canvas_config,
        DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
            width: 1920,
            height: 1080,
            frame_rate: RationalFrameRate::new(30, 1),
            background: CanvasBackground::Black,
            adaptation_policy: CanvasAdaptationPolicy::Auto,
        }
    );

    let serialized = serde_json::to_value(&draft).expect("draft should serialize");
    assert_eq!(serialized["canvasConfig"]["width"], 1920);
    assert_eq!(serialized["canvasConfig"]["height"], 1080);
    assert_eq!(
        serialized["canvasConfig"]["frameRate"],
        serde_json::json!({ "numerator": 30, "denominator": 1 })
    );
    assert_eq!(serialized["canvasConfig"]["adaptationPolicy"], "auto");
}

#[test]
fn canvas_config_validates_dimensions_frame_rate_and_aspect_ratio() {
    let mut draft = Draft::new("draft-001", "Canvas draft");

    draft.canvas_config.width = 0;
    assert_canvas_error(&draft, "canvasConfig.width");

    draft.canvas_config = DraftCanvasConfig::mvp_default();
    draft.canvas_config.frame_rate = RationalFrameRate::new(30, 0);
    assert_eq!(
        validate_draft(&draft).expect_err("zero frame-rate denominator should fail"),
        DraftValidationError::InvalidRationalFrameRate {
            field: "canvasConfig.frameRate.denominator".to_owned(),
            reason: "denominator must be greater than zero".to_owned(),
        }
    );

    draft.canvas_config = DraftCanvasConfig::mvp_default();
    draft.canvas_config.aspect_ratio =
        CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio9x16);
    assert_canvas_error(&draft, "canvasConfig.aspectRatio");

    draft.canvas_config = DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::custom(4, 3),
        width: 1440,
        height: 1080,
        frame_rate: RationalFrameRate::new(24, 1),
        background: CanvasBackground::Black,
        adaptation_policy: CanvasAdaptationPolicy::Auto,
    };
    validate_draft(&draft).expect("matching custom aspect ratio should validate");
}

#[test]
fn canvas_background_validates_color_and_image_material_references() {
    let mut draft = Draft::new("draft-001", "Canvas background draft");

    draft.canvas_config.background = CanvasBackground::SolidColor {
        color: "#12abEF".to_owned(),
    };
    validate_draft(&draft).expect("valid hex color should pass");

    draft.canvas_config.background = CanvasBackground::SolidColor {
        color: "12abef".to_owned(),
    };
    assert_canvas_error(&draft, "canvasConfig.background.color");

    draft.canvas_config.background = CanvasBackground::Image {
        material_id: Some("missing-image".into()),
    };
    assert_eq!(
        validate_draft(&draft).expect_err("missing image material should fail"),
        DraftValidationError::MissingRequiredSemanticField {
            field: "canvasConfig.background.materialId references missing-image".to_owned(),
        }
    );

    let video_material = Material::new(
        "video-material",
        MaterialKind::Video,
        "media/video.mp4",
        "video.mp4",
    );
    draft.materials.push(video_material);
    draft.canvas_config.background = CanvasBackground::Image {
        material_id: Some("video-material".into()),
    };
    assert_canvas_error(&draft, "canvasConfig.background.materialId");

    let mut image_material = Material::new(
        "image-material",
        MaterialKind::Image,
        "media/image.png",
        "image.png",
    );
    image_material.metadata.duration = Some(Microseconds::new(1_000_000));
    image_material.metadata.has_video = true;
    draft.materials.push(image_material);
    draft.canvas_config.background = CanvasBackground::Image {
        material_id: Some("image-material".into()),
    };
    validate_draft(&draft).expect("image background material should validate");
}

#[test]
fn canvas_background_capability_classifies_deferred_modes() {
    assert_eq!(
        CanvasBackground::Black.capability(),
        CanvasBackgroundCapability::Supported
    );
    assert_eq!(
        (CanvasBackground::SolidColor {
            color: "#000000".to_owned(),
        })
        .capability(),
        CanvasBackgroundCapability::Supported
    );
    assert_eq!(
        CanvasBackground::BlurFill.capability(),
        CanvasBackgroundCapability::Degraded
    );
    assert_eq!(
        (CanvasBackground::Image { material_id: None }).capability(),
        CanvasBackgroundCapability::Unsupported
    );
}

#[test]
fn normalized_canvas_coordinates_map_center_and_edges() {
    for (width, height) in [(1920, 1080), (1080, 1920), (1080, 1080)] {
        assert_eq!(
            normalized_to_canvas_pixel(width, height, NormalizedCanvasPoint::CENTER)
                .expect("center should map"),
            CanvasPixelPoint {
                x: f64::from(width) / 2.0,
                y: f64::from(height) / 2.0,
            }
        );
        assert_eq!(
            normalized_to_canvas_pixel(width, height, NormalizedCanvasPoint { x: 1.0, y: 1.0 })
                .expect("top right should map"),
            CanvasPixelPoint {
                x: f64::from(width),
                y: 0.0,
            }
        );
        assert_eq!(
            canvas_pixel_to_normalized(
                width,
                height,
                CanvasPixelPoint {
                    x: 0.0,
                    y: f64::from(height)
                }
            )
            .expect("bottom left should map"),
            NormalizedCanvasPoint { x: -1.0, y: -1.0 }
        );
    }
}

fn assert_canvas_error(draft: &Draft, expected_field: &str) {
    let error = validate_draft(draft).expect_err("draft should fail canvas validation");
    match error {
        DraftValidationError::InvalidCanvasConfig { field, .. } => {
            assert_eq!(field, expected_field);
        }
        other => panic!("expected canvas validation error, got {other:?}"),
    }
}
