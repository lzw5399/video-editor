use bindings_node::execute_command;
use draft_model::CommandErrorKind;
use serde_json::{Value, json};

#[test]
fn canvas_commands_update_draft_through_binding_route() {
    let envelope = execute_command(json!({
        "command": "updateDraftCanvasConfig",
        "payload": {
            "kind": "updateDraftCanvasConfig",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "canvasConfig": vertical_canvas_config_json()
        },
        "requestId": "req-update-canvas"
    }))
    .expect("canvas update should return an envelope");

    assert_eq!(envelope["ok"], true, "{envelope:#}");
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(
        envelope["data"]["draft"]["canvasConfig"],
        vertical_canvas_config_json()
    );
    assert_eq!(
        envelope["data"]["events"][0]["kind"],
        "draftCanvasConfigUpdated"
    );
    assert_eq!(envelope["data"]["selection"], selected_context_json());
    assert_eq!(
        envelope["data"]["commandState"]["undoStack"][0]["draft"]["canvasConfig"],
        default_canvas_config_json()
    );
}

#[test]
fn canvas_commands_reject_invalid_canvas_config_through_draft_commands() {
    let envelope = execute_command(json!({
        "command": "updateDraftCanvasConfig",
        "payload": {
            "kind": "updateDraftCanvasConfig",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "canvasConfig": {
                "aspectRatio": { "kind": "preset", "preset": "ratio9x16" },
                "width": 0,
                "height": 1920,
                "frameRate": { "numerator": 25, "denominator": 1 },
                "background": { "kind": "black" }
            }
        }
    }))
    .expect("invalid canvas config should return an envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidTimelineEdit).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "updateDraftCanvasConfig");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .unwrap()
            .contains("canvasConfig.width")
    );
}

#[test]
fn canvas_commands_reject_bad_image_background_reference_through_draft_commands() {
    let envelope = execute_command(json!({
        "command": "updateDraftCanvasConfig",
        "payload": {
            "kind": "updateDraftCanvasConfig",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "canvasConfig": {
                "aspectRatio": { "kind": "preset", "preset": "ratio16x9" },
                "width": 1920,
                "height": 1080,
                "frameRate": { "numerator": 30, "denominator": 1 },
                "background": { "kind": "image", "materialId": "video-material" }
            }
        }
    }))
    .expect("bad image background reference should return an envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidTimelineEdit).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "updateDraftCanvasConfig");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .unwrap()
            .contains("canvasConfig.background.materialId")
    );
}

#[test]
fn canvas_commands_reject_malformed_payload_and_mismatched_envelope() {
    let malformed = execute_command(json!({
        "command": "updateDraftCanvasConfig",
        "payload": {
            "kind": "updateDraftCanvasConfig",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "canvasConfig": {
                "aspectRatio": { "kind": "preset", "preset": "ratio9x16" },
                "width": 1080,
                "frameRate": { "numerator": 25, "denominator": 1 },
                "background": { "kind": "black" }
            }
        }
    }))
    .expect("malformed canvas payload should return an envelope");

    assert_eq!(malformed["ok"], false);
    assert_eq!(
        malformed["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidPayload).unwrap()
    );
    assert_eq!(malformed["error"]["command"], "updateDraftCanvasConfig");

    let mismatched = execute_command(json!({
        "command": "updateDraftCanvasConfig",
        "payload": {
            "kind": "setTrackMute",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": selected_context_json(),
            "trackId": "video-track",
            "muted": true
        }
    }))
    .expect("mismatched canvas payload should return an envelope");

    assert_eq!(mismatched["ok"], false);
    assert_eq!(
        mismatched["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidPayload).unwrap()
    );
    assert_eq!(mismatched["error"]["command"], "updateDraftCanvasConfig");
}

#[test]
fn canvas_commands_keep_unsupported_command_error_structured() {
    let envelope = execute_command(json!({
        "command": "updateCanvasLocally",
        "payload": { "kind": "updateCanvasLocally" }
    }))
    .expect("unsupported command should return an envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::UnsupportedCommand).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "updateCanvasLocally");
}

fn draft_json() -> Value {
    json!({
        "schemaVersion": 1,
        "draftId": "binding-canvas-draft",
        "metadata": { "name": "Binding Canvas Draft" },
        "canvasConfig": default_canvas_config_json(),
        "materials": [
            {
                "materialId": "image-material",
                "kind": "image",
                "uri": "media/background.png",
                "displayName": "background.png",
                "metadata": {
                    "hasVideo": false,
                    "hasAudio": false
                },
                "status": "available"
            },
            {
                "materialId": "video-material",
                "kind": "video",
                "uri": "media/video.mp4",
                "displayName": "video.mp4",
                "metadata": {
                    "duration": 1_000_000,
                    "width": 160,
                    "height": 90,
                    "frameRate": { "numerator": 30, "denominator": 1 },
                    "hasVideo": true,
                    "hasAudio": false
                },
                "status": "available"
            }
        ],
        "tracks": [{
            "trackId": "video-track",
            "kind": "video",
            "name": "Video",
            "muted": false,
            "locked": false,
            "segments": []
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

fn vertical_canvas_config_json() -> Value {
    json!({
        "aspectRatio": { "kind": "preset", "preset": "ratio9x16" },
        "width": 1080,
        "height": 1920,
        "frameRate": { "numerator": 25, "denominator": 1 },
        "background": { "kind": "solidColor", "color": "#101820" }
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
        "segmentIds": ["selected-segment"],
        "trackIds": ["video-track"]
    })
}
