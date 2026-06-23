use bindings_node::{
    apply_task_runtime_dev_config, get_task_runtime_status, get_task_runtime_telemetry,
};
use draft_model::CommandErrorKind;
use serde_json::json;
use task_runtime::TaskRuntimeConfig;

#[test]
fn scheduler_runtime_binding_exports_product_safe_status_and_telemetry() {
    let status = get_task_runtime_status(json!({})).expect("status call returns an envelope");

    assert_eq!(status["ok"], true, "{status:#}");
    assert_eq!(status["data"]["status"], "ready", "{status:#}");
    assert_eq!(status["data"]["statusLabel"], "调度服务就绪", "{status:#}");

    let telemetry =
        get_task_runtime_telemetry(json!({})).expect("telemetry call returns an envelope");
    assert_eq!(telemetry["ok"], true, "{telemetry:#}");

    for required in [
        "submittedCount",
        "admittedCount",
        "startedCount",
        "completedCount",
        "rejectedCount",
        "coalescedCount",
        "canceledCount",
        "staleRejectedCount",
        "fallbackCount",
        "unavailableCount",
        "cacheHitCount",
        "firstFrameTimeUs",
        "droppedFrameCount",
        "repeatedFrameCount",
        "resourceSaturationCount",
        "queueLatencyUs",
        "waitTimeUs",
        "runTimeUs",
        "jobDurationUs",
    ] {
        assert!(
            telemetry["data"].get(required).is_some(),
            "product-safe telemetry must include {required}: {telemetry:#}"
        );
    }

    let product_safe =
        serde_json::to_string(&telemetry["data"]).expect("telemetry data should serialize");
    for forbidden in [
        "resourceBudgets",
        "resourceUsage",
        "queueDepthByDomain",
        "currentQueueDepth",
        "maxQueueDepth",
        "maxInFlight",
        "worker",
        "workerName",
        "ffmpegPath",
        "processArgs",
        "jobPriority",
        "playbackGeneration",
        "retryPolicy",
        "fallbackPolicy",
        "ffmpegPolicy",
    ] {
        assert!(
            !product_safe.contains(forbidden),
            "product-safe telemetry must not expose raw scheduler internals: {forbidden} in {product_safe}"
        );
    }
}

#[test]
fn scheduler_runtime_diagnostics_flag_is_native_only_and_includes_raw_details() {
    let telemetry = get_task_runtime_telemetry(json!({ "diagnostics": true }))
        .expect("diagnostic telemetry returns an envelope");

    assert_eq!(telemetry["ok"], true, "{telemetry:#}");
    assert!(
        telemetry["data"]["details"]["currentQueueDepth"].is_number(),
        "native diagnostic telemetry should carry raw details: {telemetry:#}"
    );
    assert!(
        telemetry["data"]["details"]["resourceUsage"].is_array(),
        "native diagnostic telemetry should carry resource detail arrays: {telemetry:#}"
    );
}

#[test]
fn scheduler_runtime_dev_config_rejects_invalid_payload_envelope() {
    let invalid = apply_task_runtime_dev_config(json!({
        "developerDiagnostics": true,
        "config": {
            "resourceBudgets": [
                { "resourceClass": "gpuPresent", "maxInFlight": 0 }
            ],
            "queuePolicies": [
                { "domain": "interactivePreview", "maxQueued": 4, "overflow": "coalesceObsolete" }
            ],
            "telemetrySampleLimit": 64
        }
    }))
    .expect("invalid config returns an envelope");

    assert_eq!(invalid["ok"], false, "{invalid:#}");
    assert_eq!(
        invalid["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidPayload).unwrap()
    );
    assert_eq!(invalid["error"]["command"], "applyTaskRuntimeDevConfig");
}

#[test]
fn scheduler_runtime_dev_config_accepts_portable_scheduler_fields_only() {
    let config = TaskRuntimeConfig::portable_default();
    let encoded = serde_json::to_string(&config).expect("config serializes");
    for forbidden in [
        "electron",
        "renderer",
        "ffmpegPath",
        "ffprobePath",
        "processArgs",
        "retryPolicy",
        "fallbackPolicy",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "portable scheduler config must not contain {forbidden}: {encoded}"
        );
    }

    let applied = apply_task_runtime_dev_config(json!({
        "developerDiagnostics": true,
        "config": config
    }))
    .expect("valid config returns an envelope");

    assert_eq!(applied["ok"], true, "{applied:#}");
    assert_eq!(applied["data"]["applied"], true, "{applied:#}");
    assert_eq!(
        applied["data"]["resourceClassCount"],
        TaskRuntimeConfig::portable_default().resource_budgets.len()
    );
    assert_eq!(
        applied["data"]["domainPolicyCount"],
        TaskRuntimeConfig::portable_default().queue_policies.len()
    );
}
