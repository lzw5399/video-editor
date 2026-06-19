use artifact_store::jobs::{
    ArtifactGenerationRequest, ArtifactKind, GenerationProgress, acknowledge_generation_cancelled,
    cancel_generation_job, create_generation_job, fail_generation_chunk, start_generation_chunk,
};
use artifact_store::schema::open_artifact_store;
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
fn artifact_store_commands_retry_and_resume_restart_terminal_jobs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle_path = temp.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    create_generation_job(
        &mut store,
        job_request("job-retry", "artifact-retry", ArtifactKind::Thumbnail),
    )
    .expect("retry job should be created");
    start_generation_chunk(&mut store, "job-retry", 0).expect("retry chunk should start");
    fail_generation_chunk(&mut store, "job-retry", 0, "decode failed")
        .expect("retry chunk should fail");
    create_generation_job(
        &mut store,
        job_request("job-resume", "artifact-resume", ArtifactKind::Waveform),
    )
    .expect("resume job should be created");
    start_generation_chunk(&mut store, "job-resume", 0).expect("resume chunk should start");
    cancel_generation_job(&mut store, "job-resume").expect("resume job cancel should request");
    acknowledge_generation_cancelled(&mut store, "job-resume")
        .expect("resume job cancel should acknowledge");
    drop(store);

    let retry = execute_command(json!({
        "command": "retryArtifactGeneration",
        "payload": {
            "kind": "retryArtifactGeneration",
            "sessionId": "session-artifacts",
            "bundlePath": bundle_path,
            "jobId": "job-retry"
        },
        "requestId": "req-artifact-retry"
    }))
    .expect("retry command should return envelope");
    assert_eq!(retry["ok"], true, "{retry:#}");
    assert_eq!(retry["data"]["status"], "resumable");
    assert_eq!(retry["data"]["canCancel"], true);
    assert_persisted_job_state(&bundle_path, "job-retry", "resumable", 0);

    let resume = execute_command(json!({
        "command": "resumeArtifactGeneration",
        "payload": {
            "kind": "resumeArtifactGeneration",
            "sessionId": "session-artifacts",
            "bundlePath": bundle_path,
            "jobId": "job-resume"
        },
        "requestId": "req-artifact-resume"
    }))
    .expect("resume command should return envelope");
    assert_eq!(resume["ok"], true, "{resume:#}");
    assert_eq!(resume["data"]["status"], "resumable");
    assert_eq!(resume["data"]["canCancel"], true);
    assert_persisted_job_state(&bundle_path, "job-resume", "resumable", 0);

    assert_ui_safe(&retry);
    assert_ui_safe(&resume);
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

fn job_request(job_id: &str, artifact_id: &str, kind: ArtifactKind) -> ArtifactGenerationRequest {
    ArtifactGenerationRequest {
        job_id: job_id.to_owned(),
        artifact_id: Some(artifact_id.to_owned()),
        kind,
        stable_key: format!("material:material-001:{}", kind.as_str()),
        resource_id: None,
        material_id: None,
        source_ref: None,
        generation_parameters_json: json!({
            "materialId": "material-001",
            "sourceRef": "media/source.mp4"
        }),
        source_fingerprint: Some("source:v1".to_owned()),
        runtime_capability_fingerprint: Some("runtime:v1".to_owned()),
        output_profile_fingerprint: Some("output:v1".to_owned()),
        graph_fingerprint: None,
        chunks: vec![GenerationProgress::new(Some(0), Some(1_000_000), Some(0))],
    }
}

fn assert_persisted_job_state(
    bundle_path: &std::path::Path,
    job_id: &str,
    expected_status: &str,
    expected_cancel_requested: i64,
) {
    let store = open_artifact_store(bundle_path).expect("store should reopen");
    let (status, cancel_requested): (String, i64) = store
        .connection()
        .query_row(
            "SELECT status, cancel_requested FROM generation_job WHERE job_id = ?1",
            [job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("job state should persist");
    assert_eq!(status, expected_status);
    assert_eq!(cancel_requested, expected_cancel_requested);
}
