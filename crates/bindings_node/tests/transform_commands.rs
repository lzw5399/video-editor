use bindings_node::execute_command;
use draft_model::CommandErrorKind;
use serde_json::{Value, json};

#[test]
fn transform_commands_update_segment_visual_through_binding_route() {
    let visual = edited_visual_json();
    let envelope = execute_command(json!({
        "command": "updateSegmentVisual",
        "payload": {
            "kind": "updateSegmentVisual",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "segmentId": "video-segment",
            "visual": visual
        },
        "requestId": "req-update-visual"
    }))
    .expect("visual update should return an envelope");

    assert_eq!(envelope["ok"], true, "{envelope:#}");
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(
        envelope["data"]["draft"]["tracks"][0]["segments"][0]["visual"],
        visual
    );
    assert_eq!(
        envelope["data"]["events"][0]["kind"],
        "segmentVisualUpdated"
    );
    assert_eq!(envelope["data"]["selection"], selected_context_json());
    assert_eq!(
        envelope["data"]["commandState"]["undoStack"][0]["label"],
        "updateSegmentVisual"
    );
}

#[test]
fn transform_commands_reject_invalid_visual_through_draft_commands() {
    let mut visual = edited_visual_json();
    visual["transform"]["scale"]["xMillis"] = json!(0);
    let envelope = execute_command(json!({
        "command": "updateSegmentVisual",
        "payload": {
            "kind": "updateSegmentVisual",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "segmentId": "video-segment",
            "visual": visual
        }
    }))
    .expect("invalid visual payload should return an envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidTimelineEdit).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "updateSegmentVisual");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .unwrap()
            .contains("visual.transform.scale.xMillis")
    );
}

fn draft_json() -> Value {
    json!({
        "schemaVersion": 1,
        "draftId": "binding-transform-draft",
        "metadata": { "name": "Binding Transform Draft" },
        "canvasConfig": default_canvas_config_json(),
        "materials": [{
            "materialId": "video-material",
            "kind": "video",
            "uri": "media/video.mp4",
            "displayName": "video.mp4",
            "metadata": {
                "duration": 2_000_000,
                "width": 160,
                "height": 90,
                "frameRate": { "numerator": 30, "denominator": 1 },
                "hasVideo": true,
                "hasAudio": false
            },
            "status": "available"
        }],
        "tracks": [{
            "trackId": "video-track",
            "kind": "video",
            "name": "Video",
            "muted": false,
            "locked": false,
            "segments": [{
                "segmentId": "video-segment",
                "materialId": "video-material",
                "sourceTimerange": { "start": 0, "duration": 1_000_000 },
                "targetTimerange": { "start": 0, "duration": 1_000_000 },
                "mainTrackMagnet": { "enabled": false },
                "keyframes": [],
                "filters": [],
                "transition": null,
                "text": null,
                "volume": { "levelMillis": 1000 },
                "visual": default_visual_json()
            }]
        }]
    })
}

fn default_canvas_config_json() -> Value {
    json!({
        "aspectRatio": { "kind": "preset", "preset": "ratio16x9" },
        "width": 1920,
        "height": 1080,
        "frameRate": { "numerator": 30, "denominator": 1 },
        "background": { "kind": "black" }
    })
}

fn default_visual_json() -> Value {
    json!({
        "visible": true,
        "transform": {
            "position": { "x": 0, "y": 0 },
            "scale": { "xMillis": 1000, "yMillis": 1000 },
            "rotation": { "degrees": 0 },
            "opacity": { "valueMillis": 1000 },
            "crop": { "leftMillis": 0, "rightMillis": 0, "topMillis": 0, "bottomMillis": 0 },
            "anchor": { "xMillis": 500, "yMillis": 500 }
        },
        "fitMode": "stretch",
        "backgroundFilling": { "kind": "none" },
        "blendMode": { "kind": "normal" },
        "mask": { "kind": "none" }
    })
}

fn edited_visual_json() -> Value {
    json!({
        "visible": true,
        "transform": {
            "position": { "x": 120, "y": -80 },
            "scale": { "xMillis": 1250, "yMillis": 900 },
            "rotation": { "degrees": 0 },
            "opacity": { "valueMillis": 760 },
            "crop": { "leftMillis": 25, "rightMillis": 0, "topMillis": 0, "bottomMillis": 0 },
            "anchor": { "xMillis": 500, "yMillis": 500 }
        },
        "fitMode": "fit",
        "backgroundFilling": { "kind": "solidColor", "color": "#101820" },
        "blendMode": { "kind": "normal" },
        "mask": { "kind": "none" }
    })
}

fn empty_command_state_json() -> Value {
    json!({
        "undoStack": [],
        "redoStack": [],
        "maxHistoryEntries": 100,
        "snapping": {
            "enabled": true,
            "threshold": 100_000
        }
    })
}

fn selected_context_json() -> Value {
    json!({
        "segmentIds": ["video-segment"],
        "trackIds": ["video-track"]
    })
}
