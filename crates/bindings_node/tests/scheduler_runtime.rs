use bindings_node::{
    apply_task_runtime_dev_config, get_task_runtime_status, get_task_runtime_telemetry,
    task_runtime_service::{
        TaskRuntimeTelemetrySource, clear_task_runtime_scheduler_snapshots,
        record_task_runtime_scheduler_snapshot,
    },
};
use draft_model::CommandErrorKind;
use serde_json::json;
use task_runtime::{
    CompletionFreshness, JobDomain, JobEnvelope, JobId, JobPriority, JobResult, JobScheduler,
    ResourceClass, SchedulerTelemetrySnapshot, TaskCancellationToken, TaskRuntimeConfig,
};

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
fn scheduler_runtime_telemetry_aggregates_recorded_domain_snapshots() {
    clear_task_runtime_scheduler_snapshots();

    let baseline =
        get_task_runtime_telemetry(json!({})).expect("baseline telemetry returns an envelope");
    assert_eq!(baseline["ok"], true, "{baseline:#}");
    let baseline_submitted = baseline["data"]["submittedCount"].as_u64().unwrap_or(0);

    let preview_snapshot = completed_scheduler_snapshot(
        "aggregate-preview",
        JobDomain::InteractivePreview,
        JobPriority::Realtime,
        ResourceClass::GpuPresent,
        10,
        25,
        90,
        Some(33),
    );
    let export_snapshot = completed_scheduler_snapshot(
        "aggregate-export",
        JobDomain::Export,
        JobPriority::UserVisible,
        ResourceClass::FfmpegProcess,
        20,
        60,
        180,
        Some(55),
    );
    record_task_runtime_scheduler_snapshot(
        TaskRuntimeTelemetrySource::InteractivePreview,
        &preview_snapshot,
    );
    record_task_runtime_scheduler_snapshot(TaskRuntimeTelemetrySource::Export, &export_snapshot);

    let telemetry = get_task_runtime_telemetry(json!({ "diagnostics": true }))
        .expect("aggregated telemetry returns an envelope");

    assert_eq!(telemetry["ok"], true, "{telemetry:#}");
    assert!(
        telemetry["data"]["submittedCount"]
            .as_u64()
            .is_some_and(|count| count >= baseline_submitted + 2),
        "domain scheduler submissions should be included: {telemetry:#}"
    );
    assert!(
        telemetry["data"]["startedCount"]
            .as_u64()
            .is_some_and(|count| count >= 2),
        "domain scheduler starts should be included: {telemetry:#}"
    );
    assert!(
        telemetry["data"]["completedCount"]
            .as_u64()
            .is_some_and(|count| count >= 2),
        "domain scheduler completions should be included: {telemetry:#}"
    );
    assert_eq!(
        telemetry["data"]["firstFrameTimeUs"].as_u64(),
        Some(33),
        "aggregate should keep the earliest first-frame time: {telemetry:#}"
    );
    assert!(
        telemetry["data"]["queueLatencyUs"]["sampleCount"]
            .as_u64()
            .is_some_and(|count| count >= 2),
        "queue latency samples should be aggregated: {telemetry:#}"
    );

    clear_task_runtime_scheduler_snapshots();
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

fn completed_scheduler_snapshot(
    id: &str,
    domain: JobDomain,
    priority: JobPriority,
    resource_class: ResourceClass,
    submitted_at_us: u64,
    started_at_us: u64,
    completed_at_us: u64,
    first_frame_time_us: Option<u64>,
) -> SchedulerTelemetrySnapshot {
    let mut scheduler = JobScheduler::new(TaskRuntimeConfig::portable_default());
    let job_id = JobId::new(id);
    scheduler
        .submit(JobEnvelope::new(
            job_id.clone(),
            domain,
            priority,
            resource_class,
            TaskCancellationToken::new(submitted_at_us),
            submitted_at_us,
        ))
        .expect("job queues");
    scheduler
        .start_next(started_at_us)
        .expect("scheduler starts")
        .expect("job starts");
    let mut result = JobResult::completed(job_id.clone());
    if let Some(first_frame_time_us) = first_frame_time_us {
        result = result.with_first_frame_time_us(first_frame_time_us);
    }
    scheduler
        .complete_with_commit(
            &job_id,
            result,
            completed_at_us,
            CompletionFreshness::none(),
            |_| {},
        )
        .expect("job completes");
    scheduler.telemetry_snapshot()
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
