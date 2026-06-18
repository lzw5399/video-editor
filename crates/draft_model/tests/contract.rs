use draft_model::{
    CommandEnvelope, CommandError, CommandErrorKind, CommandEvent, CommandName, CommandPayload,
    CommandResultEnvelope, CommandState, Draft, Microseconds, PingResponse, TargetTimerange,
    TimelineCommandResponse, TimelineSelection, VersionResponse,
};
use serde_json::json;

#[test]
fn contract_deserializes_phase_one_command_envelopes() {
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

    let runtime_probe: CommandEnvelope = serde_json::from_value(json!({
        "command": "probeMediaRuntime",
        "payload": { "kind": "probeMediaRuntime" },
        "requestId": "req-runtime-probe"
    }))
    .expect("runtime probe command envelope should deserialize");

    assert_eq!(runtime_probe.command, CommandName::ProbeMediaRuntime);
    assert!(matches!(
        runtime_probe.payload,
        CommandPayload::ProbeMediaRuntime(_)
    ));
    assert_eq!(
        runtime_probe.request_id.as_deref(),
        Some("req-runtime-probe")
    );
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

#[test]
fn contract_rejects_mismatched_command_and_payload_kind() {
    let result = serde_json::from_value::<CommandEnvelope>(json!({
        "command": "version",
        "payload": { "kind": "ping" }
    }));

    assert!(
        result.is_err(),
        "command name and payload kind must describe the same operation"
    );
}

#[test]
fn contract_serializes_timeline_command_response_as_rust_owned_transport() {
    let response = TimelineCommandResponse {
        draft: Draft::new("phase13-contract-draft", "Phase 13 Contract"),
        command_state: CommandState::empty(),
        selection: TimelineSelection::empty(),
        events: vec![CommandEvent {
            kind: "phase13HarnessReady".to_owned(),
            message: Some("delta assertions attach here in downstream plans".to_owned()),
        }],
    };

    let serialized = serde_json::to_value(&response).expect("timeline response serializes");
    assert_eq!(
        serialized,
        json!({
            "draft": {
                "schemaVersion": 1,
                "draftId": "phase13-contract-draft",
                "metadata": { "name": "Phase 13 Contract" },
                "canvasConfig": {
                    "aspectRatio": { "kind": "preset", "preset": "ratio16x9" },
                    "width": 1920,
                    "height": 1080,
                    "frameRate": { "numerator": 30, "denominator": 1 },
                    "background": { "kind": "black" }
                },
                "materials": [],
                "tracks": []
            },
            "commandState": {
                "undoStack": [],
                "redoStack": [],
                "maxHistoryEntries": 100,
                "snapping": { "enabled": true, "threshold": 100000 }
            },
            "selection": { "segmentIds": [], "trackIds": [] },
            "events": [{
                "kind": "phase13HarnessReady",
                "message": "delta assertions attach here in downstream plans"
            }]
        })
    );

    let serialized_text = serialized.to_string();
    for forbidden in [
        "ffmpegArgs",
        "renderGraph",
        "previewCacheKey",
        "dirtyRanges",
        "artifactStore",
    ] {
        assert!(
            !serialized_text.contains(forbidden),
            "timeline command response should not leak derived ownership field {forbidden}"
        );
    }
}

#[test]
fn contract_documents_integer_half_open_dirty_range_anchor() {
    let range = TargetTimerange::new(Microseconds::new(100_000), Microseconds::new(50_000));
    let adjacent = TargetTimerange::new(Microseconds::new(150_000), Microseconds::new(25_000));
    let overlapping = TargetTimerange::new(Microseconds::new(149_999), Microseconds::new(1));

    assert_eq!(range.start.get(), 100_000);
    assert_eq!(range.duration.get(), 50_000);
    assert!(!half_open_overlap(&range, &adjacent));
    assert!(half_open_overlap(&range, &overlapping));
}

fn half_open_overlap(first: &TargetTimerange, second: &TargetTimerange) -> bool {
    let first_end = first
        .start
        .get()
        .checked_add(first.duration.get())
        .expect("test ranges should not overflow");
    let second_end = second
        .start
        .get()
        .checked_add(second.duration.get())
        .expect("test ranges should not overflow");
    first.start.get() < second_end && second.start.get() < first_end
}
