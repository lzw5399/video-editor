use bindings_node::execute_command;
use draft_model::CommandErrorKind;
use serde_json::{Value, json};

#[test]
fn keyframe_commands_are_not_public_execute_command_timeline_routes() {
    for command in ["setSegmentKeyframe", "removeSegmentKeyframe"] {
        let envelope = execute_command(json!({
            "command": command,
            "payload": { "kind": command },
            "requestId": format!("req-public-reject-{command}")
        }))
        .expect("public keyframe timeline command should return a structured error envelope");

        assert_eq!(envelope["ok"], false, "{command}: {envelope:#}");
        assert_eq!(envelope["data"], Value::Null, "{command}: {envelope:#}");
        assert_eq!(
            envelope["error"]["kind"],
            serde_json::to_value(CommandErrorKind::UnsupportedCommand).unwrap(),
            "{command}: {envelope:#}"
        );
        assert_eq!(
            envelope["error"]["command"], command,
            "{command}: {envelope:#}"
        );
    }
}
