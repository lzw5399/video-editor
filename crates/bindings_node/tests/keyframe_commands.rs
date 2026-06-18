use bindings_node::execute_command;
use draft_model::CommandErrorKind;
use serde_json::{Value, json};

#[test]
fn keyframe_commands_route_set_and_remove_through_binding() {
    let keyframe = visual_opacity_keyframe_json(500_000, 500);
    let added = execute_command(json!({
        "command": "setSegmentKeyframe",
        "payload": {
            "kind": "setSegmentKeyframe",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "segmentId": "video-segment",
            "keyframe": keyframe
        },
        "requestId": "req-set-keyframe"
    }))
    .expect("setSegmentKeyframe should return a JSON envelope");

    assert_eq!(added["ok"], true, "{added:#}");
    assert_eq!(added["error"], Value::Null);
    assert_eq!(added["data"]["events"][0]["kind"], "segmentKeyframeSet");
    assert_eq!(
        added["data"]["draft"]["tracks"][0]["segments"][0]["keyframes"][0],
        keyframe
    );
    assert_eq!(added["data"]["selection"], selected_context_json());
    assert_eq!(
        added["data"]["commandState"]["undoStack"][0]["label"],
        "setSegmentKeyframe"
    );

    let removed = execute_command(json!({
        "command": "removeSegmentKeyframe",
        "payload": {
            "kind": "removeSegmentKeyframe",
            "draft": added["data"]["draft"].clone(),
            "commandState": added["data"]["commandState"].clone(),
            "selection": added["data"]["selection"].clone(),
            "segmentId": "video-segment",
            "property": "visualOpacity",
            "at": 500_000
        },
        "requestId": "req-remove-keyframe"
    }))
    .expect("removeSegmentKeyframe should return a JSON envelope");

    assert_eq!(removed["ok"], true, "{removed:#}");
    assert_eq!(removed["error"], Value::Null);
    assert_eq!(
        removed["data"]["events"][0]["kind"],
        "segmentKeyframeRemoved"
    );
    assert!(
        removed["data"]["draft"]["tracks"][0]["segments"][0]["keyframes"]
            .as_array()
            .expect("keyframes should be an array")
            .is_empty()
    );
    assert_eq!(
        removed["data"]["commandState"]["undoStack"][1]["label"],
        "removeSegmentKeyframe"
    );
}

#[test]
fn keyframe_commands_reject_invalid_and_mismatched_envelopes() {
    let malformed = execute_command(json!({
        "command": "setSegmentKeyframe",
        "payload": {
            "kind": "setSegmentKeyframe",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "segmentId": "video-segment"
        }
    }))
    .expect("malformed keyframe payload should return an envelope");

    assert_eq!(malformed["ok"], false);
    assert_eq!(
        malformed["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidPayload).unwrap()
    );
    assert_eq!(malformed["error"]["command"], "setSegmentKeyframe");

    let mismatched = execute_command(json!({
        "command": "setSegmentKeyframe",
        "payload": {
            "kind": "removeSegmentKeyframe",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "segmentId": "video-segment",
            "property": "visualOpacity",
            "at": 500_000
        }
    }))
    .expect("mismatched keyframe payload should return an envelope");

    assert_eq!(mismatched["ok"], false);
    assert_eq!(
        mismatched["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidPayload).unwrap()
    );
    assert_eq!(mismatched["error"]["command"], "setSegmentKeyframe");

    let invalid_value = execute_command(json!({
        "command": "setSegmentKeyframe",
        "payload": {
            "kind": "setSegmentKeyframe",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "segmentId": "video-segment",
            "keyframe": {
                "at": 500_000,
                "property": "visualOpacity",
                "value": { "kind": "color", "value": "#ffffff" },
                "interpolation": "linear",
                "easing": "none"
            }
        }
    }))
    .expect("invalid keyframe value should return an envelope");

    assert_eq!(invalid_value["ok"], false);
    assert_eq!(
        invalid_value["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidTimelineEdit).unwrap()
    );
    assert_eq!(invalid_value["error"]["command"], "setSegmentKeyframe");
    assert!(
        invalid_value["error"]["message"]
            .as_str()
            .expect("error should include a message")
            .contains("keyframe")
    );
}

fn draft_json() -> Value {
    json!({
        "schemaVersion": 1,
        "draftId": "binding-keyframe-draft",
        "metadata": { "name": "Binding Keyframe Draft" },
        "canvasConfig": {
            "aspectRatio": { "kind": "preset", "preset": "ratio16x9" },
            "width": 1920,
            "height": 1080,
            "frameRate": { "numerator": 30, "denominator": 1 },
            "background": { "kind": "black" }
        },
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

fn visual_opacity_keyframe_json(at: u64, value: u32) -> Value {
    json!({
        "at": at,
        "property": "visualOpacity",
        "value": { "kind": "uint", "value": value },
        "interpolation": "linear",
        "easing": "none"
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
