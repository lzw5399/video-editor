use std::collections::BTreeSet;

use draft_model::Microseconds;
use task_runtime::{
    CompletionFreshness, JobCompletion, JobDomain, JobEnvelope, JobFreshness, JobId, JobPriority,
    JobResult, JobScheduler, PlaybackGeneration, QueueOverflowPolicy, QueuePolicy, ResourceBudget,
    ResourceClass, TaskCancellationToken, TaskRuntimeConfig, testing::FakeClock,
};

#[test]
fn starvation_interactive_preview_audio_and_analysis_start_under_background_pressure() {
    let mut scheduler = JobScheduler::new(starvation_config());
    let mut clock = FakeClock::default();

    for envelope in [
        background_envelope(
            "export-running",
            JobDomain::Export,
            ResourceClass::FfmpegProcess,
            0,
        ),
        background_envelope(
            "artifact-running",
            JobDomain::ArtifactGeneration,
            ResourceClass::BackgroundCpu,
            0,
        ),
        background_envelope(
            "probe-running",
            JobDomain::MediaProbe,
            ResourceClass::ValidationProbe,
            0,
        ),
        background_envelope(
            "filesystem-running",
            JobDomain::FilesystemIo,
            ResourceClass::DiskIo,
            0,
        ),
    ] {
        scheduler.submit(envelope).expect("background job queues");
        scheduler
            .start_next(clock.now_us())
            .expect("background job starts")
            .expect("background job started");
    }

    for envelope in [
        background_envelope(
            "export-waiting",
            JobDomain::Export,
            ResourceClass::FfmpegProcess,
            1,
        ),
        background_envelope(
            "artifact-waiting",
            JobDomain::ArtifactGeneration,
            ResourceClass::BackgroundCpu,
            1,
        ),
        background_envelope(
            "probe-waiting",
            JobDomain::MediaProbe,
            ResourceClass::ValidationProbe,
            1,
        ),
        background_envelope(
            "filesystem-waiting",
            JobDomain::FilesystemIo,
            ResourceClass::DiskIo,
            1,
        ),
        timeline_envelope(
            "preview-playback",
            JobDomain::InteractivePreview,
            JobPriority::Realtime,
            ResourceClass::GpuPresent,
            33_333,
            7,
            1,
        ),
        timeline_envelope(
            "preview-first-frame",
            JobDomain::InteractivePreview,
            JobPriority::Interactive,
            ResourceClass::GpuPresent,
            0,
            7,
            1,
        ),
        timeline_envelope(
            "scrub-seek",
            JobDomain::ScrubSeek,
            JobPriority::Interactive,
            ResourceClass::GpuPresent,
            66_666,
            7,
            1,
        ),
        timeline_envelope(
            "audio-refill",
            JobDomain::Audio,
            JobPriority::Realtime,
            ResourceClass::AudioRealtime,
            33_333,
            7,
            1,
        ),
        timeline_envelope(
            "decode-window",
            JobDomain::Decode,
            JobPriority::Interactive,
            ResourceClass::CpuDecode,
            33_333,
            7,
            1,
        ),
        timeline_envelope(
            "inspector-analysis",
            JobDomain::Analysis,
            JobPriority::Interactive,
            ResourceClass::CpuDecode,
            33_333,
            7,
            1,
        ),
    ] {
        scheduler.submit(envelope).expect("job queues");
    }

    clock.advance_us(2_000);
    let mut started = BTreeSet::new();
    while started.len() < 6 {
        let envelope = scheduler
            .start_next(clock.now_us())
            .expect("interactive scan succeeds")
            .expect("interactive job starts despite background saturation");
        started.insert(envelope.job_id.as_str().to_owned());
    }

    assert_eq!(
        started,
        BTreeSet::from([
            "audio-refill".to_owned(),
            "decode-window".to_owned(),
            "inspector-analysis".to_owned(),
            "preview-first-frame".to_owned(),
            "preview-playback".to_owned(),
            "scrub-seek".to_owned(),
        ])
    );
    assert!(
        !started.iter().any(|id| id.ends_with("-waiting")),
        "background waiting work must not consume interactive starts"
    );

    assert!(
        scheduler
            .start_next(clock.now_us())
            .expect("saturated scan succeeds")
            .is_none(),
        "only saturated background jobs should remain"
    );
    let snapshot = scheduler.telemetry_snapshot();
    assert_eq!(snapshot.current_queue_depth, 4);
    assert!(snapshot.max_queue_depth <= 10);
    assert!(
        snapshot
            .queue_latency_us
            .p95
            .is_some_and(|p95| p95 <= 2_000),
        "interactive queue latency p95 exceeded budget: {:?}",
        snapshot.queue_latency_us.p95
    );
    assert!(snapshot.resource_saturation_count >= 4);
    for resource_class in [
        ResourceClass::FfmpegProcess,
        ResourceClass::BackgroundCpu,
        ResourceClass::ValidationProbe,
        ResourceClass::DiskIo,
    ] {
        assert!(
            snapshot
                .resource_saturation
                .iter()
                .any(|item| item.resource_class == resource_class && item.count > 0),
            "expected saturation telemetry for {resource_class:?}"
        );
    }
}

#[test]
fn starvation_stale_preview_and_audio_completions_do_not_mutate_visible_state() {
    let mut scheduler = JobScheduler::new(starvation_config());
    let mut committed = Vec::new();

    for envelope in [
        timeline_envelope(
            "preview-stale",
            JobDomain::InteractivePreview,
            JobPriority::Realtime,
            ResourceClass::GpuPresent,
            33_333,
            1,
            0,
        ),
        timeline_envelope(
            "audio-stale",
            JobDomain::Audio,
            JobPriority::Realtime,
            ResourceClass::AudioRealtime,
            33_333,
            1,
            0,
        ),
    ] {
        let job_id = envelope.job_id.clone();
        scheduler.submit(envelope).expect("stale candidate queues");
        scheduler
            .start_next(0)
            .expect("candidate starts")
            .expect("candidate started");
        let completion = scheduler
            .complete_with_commit(
                &job_id,
                JobResult::completed(job_id.clone()),
                1_000,
                CompletionFreshness::playback_generation(PlaybackGeneration::new(2)),
                |_| committed.push(job_id.as_str().to_owned()),
            )
            .expect("stale completion is classified");
        assert!(matches!(completion, JobCompletion::StaleRejected { .. }));
    }

    scheduler
        .submit(timeline_envelope(
            "audio-canceled",
            JobDomain::Audio,
            JobPriority::Realtime,
            ResourceClass::AudioRealtime,
            66_666,
            2,
            2_000,
        ))
        .expect("cancel candidate queues");
    scheduler
        .start_next(2_000)
        .expect("cancel candidate starts")
        .expect("candidate started");
    scheduler
        .cancel(&JobId::new("audio-canceled"))
        .expect("running audio job cancels");
    let completion = scheduler
        .complete_with_commit(
            &JobId::new("audio-canceled"),
            JobResult::completed(JobId::new("audio-canceled")),
            3_000,
            CompletionFreshness::playback_generation(PlaybackGeneration::new(2)),
            |_| committed.push("audio-canceled".to_owned()),
        )
        .expect("cancel completion is classified");

    assert!(matches!(completion, JobCompletion::Cancelled { .. }));
    assert!(
        committed.is_empty(),
        "stale or canceled preview/audio completion mutated visible state: {committed:?}"
    );
    let snapshot = scheduler.telemetry_snapshot();
    assert_eq!(snapshot.stale_rejected_count, 2);
    assert_eq!(snapshot.canceled_count, 1);
    assert_eq!(scheduler.resource_in_flight(ResourceClass::GpuPresent), 0);
    assert_eq!(
        scheduler.resource_in_flight(ResourceClass::AudioRealtime),
        0
    );
}

fn starvation_config() -> TaskRuntimeConfig {
    TaskRuntimeConfig {
        resource_budgets: vec![
            ResourceBudget {
                resource_class: ResourceClass::GpuPresent,
                max_in_flight: 3,
            },
            ResourceBudget {
                resource_class: ResourceClass::AudioRealtime,
                max_in_flight: 1,
            },
            ResourceBudget {
                resource_class: ResourceClass::CpuDecode,
                max_in_flight: 2,
            },
            ResourceBudget {
                resource_class: ResourceClass::FfmpegProcess,
                max_in_flight: 1,
            },
            ResourceBudget {
                resource_class: ResourceClass::BackgroundCpu,
                max_in_flight: 1,
            },
            ResourceBudget {
                resource_class: ResourceClass::ValidationProbe,
                max_in_flight: 1,
            },
            ResourceBudget {
                resource_class: ResourceClass::DiskIo,
                max_in_flight: 1,
            },
        ],
        queue_policies: vec![
            queue_policy(
                JobDomain::InteractivePreview,
                4,
                QueueOverflowPolicy::CoalesceObsolete,
            ),
            queue_policy(
                JobDomain::ScrubSeek,
                4,
                QueueOverflowPolicy::CoalesceObsolete,
            ),
            queue_policy(JobDomain::Audio, 4, QueueOverflowPolicy::CoalesceObsolete),
            queue_policy(JobDomain::Decode, 8, QueueOverflowPolicy::Reject),
            queue_policy(JobDomain::Analysis, 4, QueueOverflowPolicy::Reject),
            queue_policy(JobDomain::Export, 4, QueueOverflowPolicy::Reject),
            queue_policy(
                JobDomain::ArtifactGeneration,
                4,
                QueueOverflowPolicy::Reject,
            ),
            queue_policy(JobDomain::MediaProbe, 4, QueueOverflowPolicy::Reject),
            queue_policy(JobDomain::FilesystemIo, 4, QueueOverflowPolicy::Reject),
        ],
        telemetry_sample_limit: 64,
    }
}

fn queue_policy(
    domain: JobDomain,
    max_queued: usize,
    overflow: QueueOverflowPolicy,
) -> QueuePolicy {
    QueuePolicy {
        domain,
        max_queued,
        overflow,
    }
}

fn timeline_envelope(
    id: &str,
    domain: JobDomain,
    priority: JobPriority,
    resource_class: ResourceClass,
    target_time_us: u64,
    generation: u64,
    submitted_at_us: u64,
) -> JobEnvelope {
    JobEnvelope::new(
        JobId::new(id),
        domain,
        priority,
        resource_class,
        TaskCancellationToken::new(submitted_at_us.saturating_add(1)),
        submitted_at_us,
    )
    .with_freshness(JobFreshness::timeline(
        Microseconds::new(target_time_us),
        PlaybackGeneration::new(generation),
    ))
}

fn background_envelope(
    id: &str,
    domain: JobDomain,
    resource_class: ResourceClass,
    submitted_at_us: u64,
) -> JobEnvelope {
    JobEnvelope::new(
        JobId::new(id),
        domain,
        JobPriority::Background,
        resource_class,
        TaskCancellationToken::new(submitted_at_us.saturating_add(10_000)),
        submitted_at_us,
    )
}
