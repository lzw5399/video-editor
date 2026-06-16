use draft_model::{
    CommandEnvelope, CommandError, CommandErrorKind, CommandEvent, CommandName, CommandPayload,
    CommandResultEnvelope, PingResponse, VersionResponse,
};
use serde_json::json;

#[test]
fn contract_deserializes_ping_and_version_envelopes() {
    let ping: CommandEnvelope = serde_json::from_value(json!({
        "command": "ping",
        "payload": { "kind": "ping" },
        "requestId": "req-ping-1"
    }))
    .expect("ping command envelope should deserialize");

    assert_eq!(ping.command, CommandName::Ping);
    assert!(matches!(ping.payload, CommandPayload::Ping(_)));
    assert_eq!(ping.request_id.as_deref(), Some("req-ping-1"));

    let version: CommandEnvelope = serde_json::from_value(json!({
        "command": "version",
        "payload": { "kind": "version" }
    }))
    .expect("version command envelope should deserialize");

    assert_eq!(version.command, CommandName::Version);
    assert!(matches!(version.payload, CommandPayload::Version(_)));
    assert_eq!(version.request_id, None);
}

#[test]
fn contract_serializes_ok_error_and_events_envelope_fields() {
    let ok = CommandResultEnvelope {
        ok: true,
        data: Some(PingResponse { pong: true }),
        error: None,
        events: vec![CommandEvent {
            kind: "commandAccepted".to_owned(),
            message: Some("ping accepted".to_owned()),
        }],
    };

    assert_eq!(
        serde_json::to_value(&ok).expect("ok envelope serializes"),
        json!({
            "ok": true,
            "data": { "pong": true },
            "error": null,
            "events": [{ "kind": "commandAccepted", "message": "ping accepted" }]
        })
    );

    let error: CommandResultEnvelope<VersionResponse> = CommandResultEnvelope {
        ok: false,
        data: None,
        error: Some(CommandError {
            kind: CommandErrorKind::UnsupportedCommand,
            message: "unsupported command".to_owned(),
            command: Some("splitSegment".to_owned()),
        }),
        events: vec![],
    };

    assert_eq!(
        serde_json::to_value(&error).expect("error envelope serializes"),
        json!({
            "ok": false,
            "data": null,
            "error": {
                "kind": "unsupportedCommand",
                "message": "unsupported command",
                "command": "splitSegment"
            },
            "events": []
        })
    );
}

#[test]
fn contract_rejects_unknown_top_level_fields() {
    let result = serde_json::from_value::<CommandEnvelope>(json!({
        "command": "ping",
        "payload": { "kind": "ping" },
        "unexpected": true
    }));

    assert!(result.is_err(), "unknown envelope fields must fail");
}
