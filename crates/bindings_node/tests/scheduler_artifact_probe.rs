use std::collections::BTreeSet;

use draft_model::Microseconds;
use task_runtime::{
    CompletionFreshness, JobDomain, JobEnvelope, JobFreshness, JobId, JobPriority, JobResult,
    JobScheduler, PlaybackGeneration, QueueOverflowPolicy, QueuePolicy, ResourceBudget,
    ResourceClass, TaskCancellationToken, TaskRuntimeConfig,
};

#[test]
fn scheduler_artifact_probe_artifact_refresh_uses_task_runtime_not_inline_generation() {
    let source = include_str!("../src/artifact_store_service.rs");

    for forbidden in [
        "generate_thumbnail_artifact(",
        "DesktopFfmpegExecutor::with_timeout",
        "executor.run(&self.runtime.ffmpeg.path",
    ] {
        assert!(
            !source.contains(forbidden),
            "artifact refresh must not keep binding-inline heavy work: {forbidden}"
        );
    }

    for required in [
        "task_runtime::JobScheduler",
        "JobDomain::ArtifactGeneration",
        "JobPriority::Background",
        "ResourceClass::BackgroundCpu",
        "task-runtime-artifact-driver",
        "complete_with_commit",
    ] {
        assert!(
            source.contains(required),
            "artifact refresh must expose scheduler-backed admission evidence: {required}"
        );
    }
}

#[test]
fn scheduler_artifact_probe_material_probe_returns_queued_status_without_direct_wait() {
    let material_source = include_str!("../src/material_service.rs");
    let session_source = include_str!("../src/project_session_service.rs");

    for forbidden in [
        "probe_material_metadata(",
        "submit_and_wait",
        "wait_for_probe",
        "block_on_probe",
    ] {
        assert!(
            !material_source.contains(forbidden) && !session_source.contains(forbidden),
            "material import/probe must not block the product command path: {forbidden}"
        );
    }

    for required in [
        "JobDomain::MediaProbe",
        "ResourceClass::ValidationProbe",
        "probe_status",
        "probe_job_id",
        "task-runtime-media-probe",
        "complete_material_probe_job",
    ] {
        assert!(
            session_source.contains(required),
            "material probe must return scheduler pending status and async completion evidence: {required}"
        );
    }
}

#[test]
fn scheduler_artifact_probe_project_session_io_has_filesystem_resource_and_revision_gate() {
    let project_store_source = include_str!("../../project_store/src/lib.rs");
    let session_source = include_str!("../src/project_session_service.rs");

    for required in [
        "JobDomain::FilesystemIo",
        "ResourceClass::DiskIo",
        "project_io_scheduler_envelope",
    ] {
        assert!(
            project_store_source.contains(required) || session_source.contains(required),
            "project filesystem IO must expose scheduler admission evidence: {required}"
        );
    }

    for required in [
        "complete_project_io_job",
        "CompletionFreshness::none().with_expected_revision",
        "stale project session revision",
    ] {
        assert!(
            session_source.contains(required),
            "project session-visible IO mutation must be protected by stale commit gates: {required}"
        );
    }
}

#[test]
fn scheduler_artifact_probe_background_domains_do_not_block_interactive_lanes() {
    let mut scheduler = JobScheduler::new(artifact_probe_io_config());

    for (job_id, domain, resource) in [
        (
            "artifact-running",
            JobDomain::ArtifactGeneration,
            ResourceClass::BackgroundCpu,
        ),
        (
            "probe-running",
            JobDomain::MediaProbe,
            ResourceClass::ValidationProbe,
        ),
        ("io-running", JobDomain::FilesystemIo, ResourceClass::DiskIo),
    ] {
        scheduler
            .submit(background_envelope(job_id, domain, resource, 0))
            .expect("background work queues");
        scheduler
            .start_next(0)
            .expect("background start succeeds")
            .expect("background work starts");
        scheduler
            .submit(background_envelope(
                &format!("{job_id}-queued"),
                domain,
                resource,
                1,
            ))
            .expect("same resource queues behind running job");
    }

    for (job_id, domain, resource) in [
        (
            "preview-frame",
            JobDomain::InteractivePreview,
            ResourceClass::GpuPresent,
        ),
        (
            "scrub-seek",
            JobDomain::ScrubSeek,
            ResourceClass::GpuPresent,
        ),
        (
            "audio-refill",
            JobDomain::Audio,
            ResourceClass::AudioRealtime,
        ),
    ] {
        scheduler
            .submit(interactive_envelope(job_id, domain, resource, 2))
            .expect("interactive work queues");
    }

    let mut started = BTreeSet::new();
    for _ in 0..3 {
        let envelope = scheduler
            .start_next(3)
            .expect("interactive start scan succeeds")
            .expect("interactive lane starts while background resources are saturated");
        started.insert(envelope.job_id.as_str().to_owned());
    }

    assert_eq!(
        started,
        BTreeSet::from([
            "audio-refill".to_owned(),
            "preview-frame".to_owned(),
            "scrub-seek".to_owned(),
        ])
    );
    assert!(
        scheduler
            .start_next(4)
            .expect("saturated background scan succeeds")
            .is_none(),
        "queued background work should remain blocked by its saturated resource classes"
    );
    let snapshot = scheduler.telemetry_snapshot();
    assert!(snapshot.queue_latency_us.p95.is_some_and(|p95| p95 <= 3));
    assert!(
        snapshot.resource_saturation_count >= 1,
        "saturated background resources should be visible in telemetry: {snapshot:#?}"
    );
}

#[test]
fn scheduler_artifact_probe_stale_revision_completion_rejects_visible_commit() {
    let mut scheduler = JobScheduler::new(artifact_probe_io_config());
    let mut committed = false;
    let job_id = JobId::new("probe-stale");
    scheduler
        .submit(
            JobEnvelope::new(
                job_id.clone(),
                JobDomain::MediaProbe,
                JobPriority::UserVisible,
                ResourceClass::ValidationProbe,
                TaskCancellationToken::new(77),
                0,
            )
            .with_freshness(
                JobFreshness::timeline(Microseconds::ZERO, PlaybackGeneration::new(1))
                    .with_project_session("session-a", 4),
            ),
        )
        .expect("probe queues");
    scheduler
        .start_next(1)
        .expect("probe starts")
        .expect("probe running");

    let completion = scheduler
        .complete_with_commit(
            &job_id,
            JobResult::completed(job_id.clone()),
            10,
            CompletionFreshness::playback_generation(PlaybackGeneration::new(1))
                .with_expected_revision(5),
            |_| committed = true,
        )
        .expect("stale completion is classified");

    assert!(matches!(
        completion,
        task_runtime::JobCompletion::StaleRejected { .. }
    ));
    assert!(!committed, "stale completion must not commit visible state");
    assert_eq!(scheduler.telemetry_snapshot().stale_rejected_count, 1);
}

fn artifact_probe_io_config() -> TaskRuntimeConfig {
    TaskRuntimeConfig {
        resource_budgets: vec![
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
            ResourceBudget {
                resource_class: ResourceClass::GpuPresent,
                max_in_flight: 2,
            },
            ResourceBudget {
                resource_class: ResourceClass::AudioRealtime,
                max_in_flight: 1,
            },
        ],
        queue_policies: vec![
            queue_policy(
                JobDomain::ArtifactGeneration,
                4,
                QueueOverflowPolicy::Reject,
            ),
            queue_policy(JobDomain::MediaProbe, 4, QueueOverflowPolicy::Reject),
            queue_policy(JobDomain::FilesystemIo, 4, QueueOverflowPolicy::Reject),
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

fn background_envelope(
    job_id: &str,
    domain: JobDomain,
    resource_class: ResourceClass,
    submitted_at_us: u64,
) -> JobEnvelope {
    JobEnvelope::new(
        JobId::new(job_id),
        domain,
        JobPriority::Background,
        resource_class,
        TaskCancellationToken::new(submitted_at_us.saturating_add(10)),
        submitted_at_us,
    )
}

fn interactive_envelope(
    job_id: &str,
    domain: JobDomain,
    resource_class: ResourceClass,
    submitted_at_us: u64,
) -> JobEnvelope {
    JobEnvelope::new(
        JobId::new(job_id),
        domain,
        JobPriority::Realtime,
        resource_class,
        TaskCancellationToken::new(submitted_at_us.saturating_add(20)),
        submitted_at_us,
    )
    .with_freshness(JobFreshness::timeline(
        Microseconds::new(33_333),
        PlaybackGeneration::new(1),
    ))
}
