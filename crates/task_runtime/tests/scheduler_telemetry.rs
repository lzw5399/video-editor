use draft_model::Microseconds;
use task_runtime::{
    CompletionFreshness, JobDiagnosticClassification, JobDomain, JobEnvelope, JobFreshness, JobId,
    JobPriority, JobResult, JobResultKind, JobScheduler, PlaybackGeneration, ResourceBudget,
    ResourceClass, TaskCancellationToken, TaskRuntimeConfig,
    config::{QueueOverflowPolicy, QueuePolicy},
    testing::FakeClock,
};

#[test]
fn scheduler_telemetry_records_latency_and_duration_summaries() {
    let mut scheduler = JobScheduler::new(test_config());
    let mut clock = FakeClock::default();
    let job_id = JobId::new("preview-latency");

    scheduler
        .submit(preview_envelope(job_id.as_str(), 0, 1, clock.now_us()))
        .expect("preview queues");
    clock.advance_us(10_000);
    scheduler
        .start_next(clock.now_us())
        .expect("start succeeds")
        .expect("preview starts");
    clock.advance_us(40_000);
    scheduler
        .complete_with_commit(
            &job_id,
            JobResult::completed(job_id.clone())
                .with_cache_hit(true)
                .with_first_frame_time_us(12_000)
                .with_dropped_frame_count(2)
                .with_repeated_frame_count(1),
            clock.now_us(),
            CompletionFreshness::playback_generation(PlaybackGeneration::new(1)),
            |_| {},
        )
        .expect("completion succeeds");

    let snapshot = scheduler.telemetry_snapshot();

    assert_eq!(snapshot.completed_count, 1);
    assert_eq!(snapshot.cache_hit_count, 1);
    assert_eq!(snapshot.dropped_frame_count, 2);
    assert_eq!(snapshot.repeated_frame_count, 1);
    assert_eq!(snapshot.first_frame_time_us, Some(12_000));
    assert_eq!(snapshot.queue_latency_us.p50, Some(10_000));
    assert_eq!(snapshot.queue_latency_us.p95, Some(10_000));
    assert_eq!(snapshot.wait_time_us.max, Some(10_000));
    assert_eq!(snapshot.run_time_us.p50, Some(40_000));
    assert_eq!(snapshot.job_duration_us.max, Some(50_000));
}

#[test]
fn scheduler_telemetry_classifies_cancel_stale_reject_fallback_unavailable_depth_and_saturation() {
    let mut scheduler = JobScheduler::new(test_config());
    let mut clock = FakeClock::default();

    scheduler
        .submit(export_envelope("export-running", clock.now_us()))
        .expect("first export queues");
    scheduler
        .start_next(clock.now_us())
        .expect("start succeeds")
        .expect("export starts");
    scheduler
        .submit(export_envelope("export-waiting", clock.now_us()))
        .expect("second export waits");
    assert!(scheduler.start_next(clock.now_us()).expect("scan succeeds").is_none());

    scheduler
        .submit(export_envelope("export-rejected", clock.now_us()))
        .expect_err("full export queue rejects");

    scheduler
        .submit(preview_envelope("preview-cancel", 0, 1, clock.now_us()))
        .expect("preview queues");
    scheduler
        .cancel(&JobId::new("preview-cancel"))
        .expect("queued preview cancels");

    scheduler
        .submit(preview_envelope("preview-stale", 0, 1, clock.now_us()))
        .expect("stale candidate queues");
    scheduler
        .start_next(clock.now_us())
        .expect("preview starts")
        .expect("started");
    clock.advance_us(1_000);
    scheduler
        .complete_with_commit(
            &JobId::new("preview-stale"),
            JobResult::completed(JobId::new("preview-stale")),
            clock.now_us(),
            CompletionFreshness::playback_generation(PlaybackGeneration::new(2)),
            |_| panic!("stale job must not commit"),
        )
        .expect("stale completion is classified");

    let fallback_id = JobId::new("preview-fallback");
    scheduler
        .submit(preview_envelope(fallback_id.as_str(), 33_333, 2, clock.now_us()))
        .expect("fallback job queues");
    scheduler
        .start_next(clock.now_us())
        .expect("fallback job starts")
        .expect("started");
    scheduler
        .complete_with_commit(
            &fallback_id,
            JobResult::new(
                fallback_id.clone(),
                JobResultKind::Fallback {
                    classification: JobDiagnosticClassification::RuntimeFallback,
                },
            ),
            clock.now_us(),
            CompletionFreshness::playback_generation(PlaybackGeneration::new(2)),
            |_| {},
        )
        .expect("fallback completion records diagnostics");

    scheduler
        .cancel(&JobId::new("export-running"))
        .expect("release export resource");
    let unavailable_id = JobId::new("export-unavailable");
    scheduler
        .submit(export_envelope(unavailable_id.as_str(), clock.now_us()))
        .expect("unavailable export queues after release");
    scheduler
        .start_next(clock.now_us())
        .expect("unavailable export starts")
        .expect("started");
    scheduler
        .complete_with_commit(
            &unavailable_id,
            JobResult::new(
                unavailable_id.clone(),
                JobResultKind::Unavailable {
                    classification: JobDiagnosticClassification::RuntimeUnavailable,
                },
            ),
            clock.now_us(),
            CompletionFreshness::none(),
            |_| {},
        )
        .expect("unavailable completion records diagnostics");

    let snapshot = scheduler.telemetry_snapshot();

    assert_eq!(snapshot.rejected_count, 1);
    assert_eq!(snapshot.canceled_count, 2);
    assert_eq!(snapshot.stale_rejected_count, 1);
    assert_eq!(snapshot.fallback_count, 1);
    assert_eq!(snapshot.unavailable_count, 1);
    assert_eq!(snapshot.max_queue_depth, 1);
    assert_eq!(snapshot.resource_saturation_count, 1);
    assert!(snapshot
        .resource_saturation
        .iter()
        .any(|item| item.resource_class == ResourceClass::FfmpegProcess && item.count == 1));
    assert!(snapshot.fallback_classifications.iter().any(|item| {
        item.classification == JobDiagnosticClassification::RuntimeFallback && item.count == 1
    }));
    assert!(snapshot.unavailable_classifications.iter().any(|item| {
        item.classification == JobDiagnosticClassification::RuntimeUnavailable && item.count == 1
    }));
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

fn preview_envelope(id: &str, target_time_us: u64, generation: u64, submitted_at_us: u64) -> JobEnvelope {
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
        PlaybackGeneration::new(generation),
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
