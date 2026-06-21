use bindings_node::execute_command;
use draft_model::CommandErrorKind;
use serde_json::{Value, json};

#[test]
fn canvas_edit_command_is_not_a_public_execute_command_timeline_route() {
    let envelope = execute_command(json!({
        "command": "updateDraftCanvasConfig",
        "payload": { "kind": "updateDraftCanvasConfig" },
        "requestId": "req-public-reject-update-canvas"
    }))
    .expect("public canvas edit command should return a structured error envelope");

    assert_eq!(envelope["ok"], false, "{envelope:#}");
    assert_eq!(envelope["data"], Value::Null, "{envelope:#}");
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::UnsupportedCommand).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "updateDraftCanvasConfig");
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
