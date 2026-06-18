use draft_model::{
    CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground, Draft, DraftCanvasConfig,
    Microseconds, RationalFrameRate, TargetTimerange,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use render_graph::build_render_graph;

#[test]
fn black_and_solid_canvas_backgrounds_are_supported_graph_data() {
    let black = canvas_snapshot(CanvasBackground::Black);
    assert_eq!(
        black,
        serde_json::json!({
            "width": 720,
            "height": 720,
            "background": {
                "mode": "black",
                "support": "supported",
                "reason": "black canvas background is directly supported"
            },
            "diagnostics": []
        })
    );

    let solid = canvas_snapshot(CanvasBackground::SolidColor {
        color: "#112233".to_owned(),
    });
    assert_eq!(
        solid,
        serde_json::json!({
            "width": 720,
            "height": 720,
            "background": {
                "mode": "solidColor",
                "color": "#112233",
                "support": "supported",
                "reason": "solid color canvas background is directly supported"
            },
            "diagnostics": []
        })
    );
}

#[test]
fn blur_and_image_canvas_backgrounds_surface_explicit_diagnostics() {
    let blur = canvas_snapshot(CanvasBackground::BlurFill);
    assert_eq!(
        blur,
        serde_json::json!({
            "width": 720,
            "height": 720,
            "background": {
                "mode": "blurFill",
                "support": "degraded",
                "reason": "blur fill canvas background is preserved as degraded until render support is implemented"
            },
            "diagnostics": [
                {
                    "mode": "blurFill",
                    "support": "degraded",
                    "reason": "blur fill canvas background is preserved as degraded until render support is implemented"
                }
            ]
        })
    );

    let image = canvas_snapshot(CanvasBackground::Image { material_id: None });
    assert_eq!(
        image,
        serde_json::json!({
            "width": 720,
            "height": 720,
            "background": {
                "mode": "image",
                "support": "unsupported",
                "reason": "image canvas background is unsupported until background material rendering is implemented"
            },
            "diagnostics": [
                {
                    "mode": "image",
                    "support": "unsupported",
                    "reason": "image canvas background is unsupported until background material rendering is implemented"
                }
            ]
        })
    );
}

fn canvas_snapshot(background: CanvasBackground) -> serde_json::Value {
    let mut draft = Draft::new("draft-canvas-background", "Canvas Background");
    draft.canvas_config = DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio1x1),
        width: 720,
        height: 720,
        frame_rate: RationalFrameRate::new(24, 1),
        background,
    };
    let profile = EngineProfile::from_draft_canvas(&draft).expect("canvas profile should resolve");
    let normalized = normalize_draft(&draft, &profile).expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(100_000)),
    )
    .expect("render range should resolve");
    let graph = build_render_graph(&normalized, &range).expect("render graph should build");
    serde_json::to_value(&graph.canvas).expect("canvas should serialize")
}
