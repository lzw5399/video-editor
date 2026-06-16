use bindings_node::{execute_command, ping, version};
use draft_model::CommandErrorKind;
use serde_json::{Value, json};

#[test]
fn ping_returns_standard_ok_envelope() {
    let envelope = ping().expect("ping returns a JSON envelope");

    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"], json!({ "pong": true }));
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(envelope["events"], json!([]));
}

#[test]
fn version_returns_standard_ok_envelope() {
    let envelope = version().expect("version returns a JSON envelope");

    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"]["coreVersion"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        envelope["data"]["contractVersion"],
        draft_model::DRAFT_MODEL_VERSION
    );
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(envelope["events"], json!([]));
}

#[test]
fn execute_command_matches_direct_phase_one_envelopes() {
    let ping_from_command = execute_command(json!({
        "command": "ping",
        "payload": { "kind": "ping" },
        "requestId": "req-ping"
    }))
    .expect("command ping returns a JSON envelope");

    let version_from_command = execute_command(json!({
        "command": "version",
        "payload": { "kind": "version" },
        "requestId": "req-version"
    }))
    .expect("command version returns a JSON envelope");

    assert_eq!(ping_from_command, ping().expect("direct ping returns"));
    assert_eq!(
        version_from_command,
        version().expect("direct version returns")
    );
}

#[test]
fn execute_command_rejects_non_phase_one_command_with_structured_error() {
    let envelope = execute_command(json!({
        "command": "addSegment",
        "payload": { "kind": "addSegment" },
        "requestId": "req-add-segment"
    }))
    .expect("unsupported command returns an error envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::UnsupportedCommand).unwrap()
    );
    assert_eq!(envelope["error"]["command"], "addSegment");
    assert_eq!(envelope["events"], json!([]));
}
