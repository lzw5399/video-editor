use bindings_node::execute_command;
use serde_json::{Value, json};

#[test]
fn artifact_store_commands_return_status_quota_and_gc_summaries() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle_path = temp.path().join("draft.veproj");

    let status = execute_command(json!({
        "command": "getArtifactStatus",
        "payload": {
            "kind": "getArtifactStatus",
            "sessionId": "session-artifacts",
            "bundlePath": bundle_path
        },
        "requestId": "req-artifact-status"
    }))
    .expect("artifact status command should return envelope");
    assert_eq!(status["ok"], true, "{status:#}");
    assert_eq!(status["data"]["sessionId"], "session-artifacts");
    assert!(status["data"]["materials"].as_array().is_some());
    assert!(status["data"]["tasks"].as_array().is_some());
    assert_eq!(status["data"]["quota"]["statusLabel"], "缓存空间正常");

    let quota = execute_command(json!({
        "command": "getArtifactQuotaStatus",
        "payload": {
            "kind": "getArtifactQuotaStatus",
            "sessionId": "session-artifacts",
            "bundlePath": bundle_path
        },
        "requestId": "req-artifact-quota"
    }))
    .expect("artifact quota command should return envelope");
    assert_eq!(quota["ok"], true, "{quota:#}");
    assert_eq!(quota["data"]["statusLabel"], "缓存空间正常");
    assert_eq!(quota["data"]["cleanupAvailable"], false);

    let gc = execute_command(json!({
        "command": "runArtifactGarbageCollection",
        "payload": {
            "kind": "runArtifactGarbageCollection",
            "sessionId": "session-artifacts",
            "bundlePath": bundle_path,
            "dryRun": true
        },
        "requestId": "req-artifact-gc"
    }))
    .expect("artifact GC command should return envelope");
    assert_eq!(gc["ok"], true, "{gc:#}");
    assert_eq!(gc["data"]["mode"], "dryRun");
    assert_eq!(gc["data"]["completed"], true);

    assert_ui_safe(&status);
    assert_ui_safe(&quota);
    assert_ui_safe(&gc);
}

#[test]
fn artifact_store_commands_generation_actions_classify_unknown_jobs_without_panics() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle_path = temp.path().join("draft.veproj");

    for command in [
        "retryArtifactGeneration",
        "resumeArtifactGeneration",
        "cancelArtifactGeneration",
    ] {
        let envelope = execute_command(json!({
            "command": command,
            "payload": {
                "kind": command,
                "sessionId": "session-artifacts",
                "bundlePath": bundle_path,
                "jobId": "missing-job"
            },
            "requestId": format!("req-{command}")
        }))
        .expect("artifact action command should return envelope");

        assert_eq!(
            envelope["ok"], false,
            "{command} should classify missing jobs"
        );
        assert_eq!(envelope["error"]["kind"], "artifactStoreFailed");
        assert_ui_safe(&envelope);
    }
}

#[test]
fn artifact_store_commands_reject_mismatched_payload_pairs() {
    let envelope = execute_command(json!({
        "command": "getArtifactQuotaStatus",
        "payload": {
            "kind": "runArtifactGarbageCollection",
            "sessionId": "session-artifacts",
            "bundlePath": "/tmp/project.veproj",
            "dryRun": true
        },
        "requestId": "req-artifact-mismatch"
    }))
    .expect("mismatched artifact command should return envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["kind"], "invalidPayload");
}

fn assert_ui_safe(value: &Value) {
    let serialized = serde_json::to_string(value).expect("response serializes");
    for forbidden in [
        "artifact-store.sqlite",
        "rusqlite",
        "CREATE TABLE",
        "cacheKey",
        "fingerprint",
        "graphNode",
        "dirtyRange",
        "ffmpegArgs",
        "/Users/",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "artifact binding response must not expose {forbidden}: {serialized}"
        );
    }
}
