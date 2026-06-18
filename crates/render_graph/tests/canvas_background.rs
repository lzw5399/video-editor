use draft_model::{
    CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground, Draft, DraftCanvasConfig,
    Microseconds, RationalFrameRate, TargetTimerange,
};
use engine_core::{normalize_draft, resolve_render_range, EngineProfile};
use render_graph::{
    build_render_graph, RenderCanvasBackgroundMode, RenderGraph, RenderIntentSupport,
};

#[test]
fn black_and_solid_canvas_backgrounds_are_supported_graph_data() {
    let black = canvas_graph(CanvasBackground::Black);
    assert_eq!(
        black.canvas.background.mode,
        RenderCanvasBackgroundMode::Black
    );
    assert_eq!(
        black.canvas.background.support,
        RenderIntentSupport::Supported
    );
    assert!(black.canvas.diagnostics.is_empty());
    assert_eq!(
        serde_json::to_value(&black.canvas).expect("black canvas should serialize compatibly"),
        serde_json::json!({
            "nodeId": canvas_node(),
            "width": 720,
            "height": 720
        })
    );

    let solid = canvas_snapshot(CanvasBackground::SolidColor {
        color: "#112233".to_owned(),
    });
    assert_eq!(
        solid,
        serde_json::json!({
            "nodeId": canvas_node(),
            "width": 720,
            "height": 720,
            "background": {
                "mode": "solidColor",
                "color": "#112233",
                "support": "supported",
                "reason": "solid color canvas background is directly supported"
            }
        })
    );
}

#[test]
fn blur_and_image_canvas_backgrounds_surface_explicit_diagnostics() {
    let blur = canvas_snapshot(CanvasBackground::BlurFill);
    assert_eq!(
        blur,
        serde_json::json!({
            "nodeId": canvas_node(),
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
            "nodeId": canvas_node(),
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
    serde_json::to_value(canvas_graph(background).canvas).expect("canvas should serialize")
}

fn canvas_node() -> serde_json::Value {
    serde_json::json!({
        "role": "canvas",
        "draftId": "draft-canvas-background"
    })
}

fn canvas_graph(background: CanvasBackground) -> RenderGraph {
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
    build_render_graph(&normalized, &range).expect("render graph should build")
}
