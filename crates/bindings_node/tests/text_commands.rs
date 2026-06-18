use bindings_node::execute_command;
use serde_json::{Value, json};

#[test]
fn text_commands_execute_command_routes_import_subtitle_srt_success() {
    let envelope = execute_command(json!({
        "command": "importSubtitleSrt",
        "payload": {
            "kind": "importSubtitleSrt",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": empty_selection_json(),
            "trackId": "subtitle-track",
            "trackName": "字幕",
            "srtContent": "1\n00:00:00,000 --> 00:00:01,000\n第一行\n\n2\n00:00:01,500 --> 00:00:02,000\n第二行\n",
            "timeOffset": 250_000,
            "segmentIdPrefix": "subtitle-segment",
            "materialIdPrefix": "subtitle-material",
            "style": {
                "font": { "family": "PingFang SC" },
                "fontSize": 30,
                "color": "#ffee00",
                "alignment": "center",
                "lineHeightMillis": 1_150,
                "letterSpacingMillis": 20
            },
            "textBox": { "widthMillis": 720, "heightMillis": 180 },
            "layoutRegion": {
                "xMillis": 140,
                "yMillis": 680,
                "widthMillis": 720,
                "heightMillis": 220
            },
            "wrapping": "auto"
        },
        "requestId": "req-import-subtitle-srt"
    }))
    .expect("importSubtitleSrt should return a standard JSON envelope");

    assert_eq!(envelope["ok"], true, "{envelope:#}");
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(envelope["data"]["events"][0]["kind"], "subtitleSrtImported");
    assert_eq!(
        envelope["data"]["draft"]["tracks"][0]["segments"][0]["targetTimerange"]["start"],
        250_000
    );
    assert_eq!(
        envelope["data"]["draft"]["tracks"][0]["segments"][0]["text"]["source"],
        "subtitle"
    );
    assert_eq!(
        envelope["data"]["draft"]["tracks"][0]["segments"][1]["text"]["content"],
        "第二行"
    );
    assert_eq!(
        envelope["data"]["commandState"]["undoStack"][0]["label"],
        "importSubtitleSrt"
    );
}

#[test]
fn text_commands_execute_command_routes_import_subtitle_srt_malformed_failure() {
    let envelope = execute_command(json!({
        "command": "importSubtitleSrt",
        "payload": {
            "kind": "importSubtitleSrt",
            "draft": draft_json(),
            "commandState": empty_command_state_json(),
            "selection": empty_selection_json(),
            "trackId": "subtitle-track",
            "trackName": "字幕",
            "srtContent": "1\n00:00:02,000 --> 00:00:01,000\n反向时间\n",
            "timeOffset": 0,
            "segmentIdPrefix": "subtitle-segment",
            "materialIdPrefix": "subtitle-material",
            "style": {
                "font": { "family": "PingFang SC" },
                "fontSize": 30,
                "color": "#ffee00",
                "alignment": "center",
                "lineHeightMillis": 1_150,
                "letterSpacingMillis": 20
            },
            "textBox": { "widthMillis": 720, "heightMillis": 180 },
            "layoutRegion": {
                "xMillis": 140,
                "yMillis": 680,
                "widthMillis": 720,
                "heightMillis": 220
            },
            "wrapping": "auto"
        },
        "requestId": "req-import-bad-subtitle-srt"
    }))
    .expect("malformed importSubtitleSrt should return an error envelope");

    assert_eq!(envelope["ok"], false, "{envelope:#}");
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(envelope["error"]["kind"], "invalidTimelineEdit");
    assert_eq!(envelope["error"]["command"], "importSubtitleSrt");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .expect("error should include a message")
            .contains("SRT")
    );
}

fn draft_json() -> Value {
    json!({
        "schemaVersion": 1,
        "draftId": "binding-subtitle-draft",
        "metadata": {
            "name": "Binding Subtitle Draft"
        },
        "canvasConfig": {
            "width": 1920,
            "height": 1080,
            "frameRate": { "numerator": 30, "denominator": 1 },
            "aspectRatio": { "kind": "preset", "preset": "ratio16x9" },
            "background": { "kind": "black" }
        },
        "materials": [],
        "tracks": []
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

fn empty_selection_json() -> Value {
    json!({
        "segmentIds": [],
        "trackIds": []
    })
}
