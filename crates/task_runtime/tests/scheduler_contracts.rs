use draft_model::Microseconds;
use serde_json::json;
use task_runtime::{
    CompletionFreshness, JobDomain, JobEnvelope, JobFreshness, JobId, JobPriority, JobResult,
    JobScheduler, ResourceBudget, ResourceClass, SchedulerRejected, TaskCancellationToken,
    TaskRuntimeConfig,
    config::{QueueOverflowPolicy, QueuePolicy},
    testing::FakeClock,
};

#[test]
fn scheduler_contracts_job_domains_cover_phase16_work() {
    let domains = [
        JobDomain::InteractivePreview,
        JobDomain::ScrubSeek,
        JobDomain::Decode,
        JobDomain::Audio,
        JobDomain::ArtifactGeneration,
        JobDomain::Export,
        JobDomain::MediaProbe,
        JobDomain::FilesystemIo,
        JobDomain::Analysis,
    ];

    let serialized = serde_json::to_value(domains).expect("domains serialize");

    assert_eq!(serialized[0], "interactivePreview");
    assert_eq!(serialized[1], "scrubSeek");
    assert_eq!(serialized[4], "artifactGeneration");
    assert_eq!(serialized[7], "filesystemIo");
}

#[test]
fn scheduler_contracts_config_serializes_portable_budget_policy() {
    let config = TaskRuntimeConfig::portable_default();
    let json = serde_json::to_value(&config).expect("config serializes");
    let encoded = serde_json::to_string(&json).expect("config encodes");

    assert!(json["resourceBudgets"].is_array());
    assert!(json["queuePolicies"].is_array());
    assert!(!encoded.contains("electron"));
    assert!(!encoded.contains("ffmpegPath"));
    assert!(!encoded.contains("renderer"));
    assert!(!encoded.contains("workerName"));
    assert!(!encoded.contains("retry"));
    assert!(!encoded.contains("fallback"));

    let bad = json!({
        "resourceBudgets": [],
        "queuePolicies": [],
        "telemetrySampleLimit": 64,
        "ffmpegPath": "/tmp/ffmpeg"
    });
    assert!(serde_json::from_value::<TaskRuntimeConfig>(bad).is_err());
}

#[test]
fn scheduler_contracts_cancellation_tokens_are_cloneable_and_observable() {
    let token = TaskCancellationToken::new(42);
    let cloned = token.clone();

    assert_eq!(token.id(), 42);
    assert!(!cloned.is_cancelled());

    token.cancel();

    assert!(token.is_cancelled());
    assert!(cloned.is_cancelled());
}

#[test]
fn scheduler_contracts_job_envelopes_carry_timeline_freshness() {
    let envelope = preview_envelope("preview-fresh", 10, 7, 0);
    let json = serde_json::to_value(&envelope).expect("job envelope serializes");

    assert_eq!(json["jobId"], "preview-fresh");
    assert_eq!(json["domain"], "interactivePreview");
    assert_eq!(json["priority"], "realtime");
    assert_eq!(json["freshness"]["kind"], "timeline");
    assert_eq!(json["freshness"]["targetTime"], 10);
    assert_eq!(json["freshness"]["playbackGeneration"], 7);
    assert!(json.get("rendererFrameToken").is_none());
}

#[test]
fn scheduler_contracts_priority_lanes_start_interactive_while_export_saturated() {
    let mut scheduler = JobScheduler::new(test_config());
    let clock = FakeClock::default();

    scheduler
        .submit(export_envelope("export-running", 0))
        .expect("first export queues");
    let running_export = scheduler
        .start_next(clock.now_us())
        .expect("export starts")
        .expect("started export");
    assert_eq!(running_export.job_id, JobId::new("export-running"));
    assert_eq!(
        scheduler.resource_in_flight(ResourceClass::FfmpegProcess),
        1
    );

    scheduler
        .submit(export_envelope("export-waiting", clock.now_us()))
        .expect("second export waits for saturated export resource");
    scheduler
        .submit(preview_envelope("preview-now", 33_333, 1, clock.now_us()))
        .expect("preview queues independently");

    let started = scheduler
        .start_next(clock.now_us())
        .expect("preview can start")
        .expect("preview started despite export saturation");

    assert_eq!(started.job_id, JobId::new("preview-now"));
    assert_eq!(started.domain, JobDomain::InteractivePreview);
    assert_eq!(scheduler.resource_in_flight(ResourceClass::GpuPresent), 1);
    assert_eq!(
        scheduler.resource_in_flight(ResourceClass::FfmpegProcess),
        1
    );
}

#[test]
fn scheduler_contracts_full_preview_queue_coalesces_obsolete_jobs() {
    let mut scheduler = JobScheduler::new(test_config());

    scheduler
        .submit(preview_envelope("preview-old", 0, 1, 0))
        .expect("first preview queues");
    scheduler
        .submit(preview_envelope("preview-new", 33_333, 1, 1_000))
        .expect("newer preview coalesces obsolete queued work");

    assert_eq!(scheduler.queued_len(), 1);
    let started = scheduler
        .start_next(2_000)
        .expect("start succeeds")
        .expect("coalesced preview remains");

    assert_eq!(started.job_id, JobId::new("preview-new"));
    assert_eq!(scheduler.telemetry_snapshot().coalesced_count, 1);
}

#[test]
fn scheduler_contracts_non_coalescing_full_queue_rejects_background() {
    let mut scheduler = JobScheduler::new(test_config());

    scheduler
        .submit(export_envelope("export-a", 0))
        .expect("first export queues");
    let rejected = scheduler
        .submit(export_envelope("export-b", 1))
        .expect_err("export queue rejects instead of growing unbounded");

    assert!(matches!(
        rejected,
        SchedulerRejected::QueueFull {
            domain: JobDomain::Export,
            ..
        }
    ));
    assert_eq!(scheduler.telemetry_snapshot().rejected_count, 1);
}

#[test]
fn scheduler_contracts_cancellation_releases_accounting_once() {
    let mut scheduler = JobScheduler::new(test_config());

    scheduler
        .submit(preview_envelope("preview-cancel", 0, 1, 0))
        .expect("preview queues");
    scheduler
        .start_next(0)
        .expect("start succeeds")
        .expect("preview starts");
    assert_eq!(scheduler.resource_in_flight(ResourceClass::GpuPresent), 1);

    scheduler
        .cancel(&JobId::new("preview-cancel"))
        .expect("running preview cancels");
    assert_eq!(scheduler.resource_in_flight(ResourceClass::GpuPresent), 0);
    assert_eq!(scheduler.telemetry_snapshot().canceled_count, 1);

    let completion = scheduler.complete_with_commit(
        &JobId::new("preview-cancel"),
        JobResult::completed(JobId::new("preview-cancel")),
        10,
        CompletionFreshness::playback_generation(task_runtime::PlaybackGeneration::new(1)),
        |_| panic!("canceled completion must not commit visible state"),
    );

    assert!(matches!(
        completion,
        Ok(task_runtime::JobCompletion::Cancelled { .. })
    ));
    assert_eq!(scheduler.telemetry_snapshot().canceled_count, 1);
}

#[test]
fn scheduler_contracts_stale_completion_does_not_commit_visible_state() {
    let mut scheduler = JobScheduler::new(test_config());
    let mut committed = false;

    scheduler
        .submit(preview_envelope("preview-stale", 0, 1, 0))
        .expect("preview queues");
    scheduler
        .start_next(0)
        .expect("start succeeds")
        .expect("preview starts");

    let completion = scheduler
        .complete_with_commit(
            &JobId::new("preview-stale"),
            JobResult::completed(JobId::new("preview-stale")),
            10,
            CompletionFreshness::playback_generation(task_runtime::PlaybackGeneration::new(2)),
            |_| committed = true,
        )
        .expect("stale completion is classified");

    assert!(matches!(
        completion,
        task_runtime::JobCompletion::StaleRejected { .. }
    ));
    assert!(!committed);
    assert_eq!(scheduler.telemetry_snapshot().stale_rejected_count, 1);
    assert_eq!(scheduler.resource_in_flight(ResourceClass::GpuPresent), 0);
}

fn test_config() -> TaskRuntimeConfig {
    TaskRuntimeConfig {
        resource_budgets: vec![
            ResourceBudget {
                resource_class: ResourceClass::GpuPresent,
                max_in_flight: 1,
            },
            ResourceBudget {
                resource_class: ResourceClass::FfmpegProcess,
                max_in_flight: 1,
            },
        ],
        queue_policies: vec![
            QueuePolicy {
                domain: JobDomain::InteractivePreview,
                max_queued: 1,
                overflow: QueueOverflowPolicy::CoalesceObsolete,
            },
            QueuePolicy {
                domain: JobDomain::Export,
                max_queued: 1,
                overflow: QueueOverflowPolicy::Reject,
            },
        ],
        telemetry_sample_limit: 64,
    }
}

fn preview_envelope(
    id: &str,
    target_time_us: u64,
    generation: u64,
    submitted_at_us: u64,
) -> JobEnvelope {
    JobEnvelope::new(
        JobId::new(id),
        JobDomain::InteractivePreview,
        JobPriority::Realtime,
        ResourceClass::GpuPresent,
        TaskCancellationToken::new(submitted_at_us.saturating_add(1)),
        submitted_at_us,
    )
    .with_freshness(JobFreshness::timeline(
        Microseconds::new(target_time_us),
        task_runtime::PlaybackGeneration::new(generation),
    ))
}

fn export_envelope(id: &str, submitted_at_us: u64) -> JobEnvelope {
    JobEnvelope::new(
        JobId::new(id),
        JobDomain::Export,
        JobPriority::Background,
        ResourceClass::FfmpegProcess,
        TaskCancellationToken::new(submitted_at_us.saturating_add(100)),
        submitted_at_us,
    )
}
