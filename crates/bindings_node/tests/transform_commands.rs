use bindings_node::execute_command;
use draft_model::CommandErrorKind;
use serde_json::{Value, json};

#[test]
fn transform_commands_are_not_public_execute_command_timeline_routes() {
    let envelope = execute_command(json!({
        "command": "updateSegmentVisual",
        "payload": { "kind": "updateSegmentVisual" },
        "requestId": "req-public-reject-update-visual"
    }))
    .expect("public transform timeline command should return a structured error envelope");

    assert_eq!(envelope["ok"], false, "{envelope:#}");
    assert_eq!(envelope["data"], Value::Null, "{envelope:#}");
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::UnsupportedCommand).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "updateSegmentVisual");
}
