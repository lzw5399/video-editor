use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::config::{QueueOverflowPolicy, TaskRuntimeConfig};
use crate::telemetry::{DomainQueueDepth, ResourceUsageSnapshot, SchedulerTelemetry};
use crate::{
    CompletionFreshness, JobDomain, JobEnvelope, JobId, JobResult, ResourceClass,
    SchedulerTelemetrySnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", deny_unknown_fields)]
pub enum SchedulerRejected {
    DuplicateJobId {
        job_id: JobId,
    },
    QueueFull {
        domain: JobDomain,
        max_queued: usize,
    },
    Cancelled {
        job_id: JobId,
    },
    UnknownJob {
        job_id: JobId,
    },
    StaleCompletion {
        job_id: JobId,
    },
}

impl fmt::Display for SchedulerRejected {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateJobId { job_id } => {
                write!(
                    formatter,
                    "scheduler job already exists: {}",
                    job_id.as_str()
                )
            }
            Self::QueueFull { domain, max_queued } => {
                write!(
                    formatter,
                    "scheduler queue for {domain:?} is full at {max_queued} jobs"
                )
            }
            Self::Cancelled { job_id } => {
                write!(
                    formatter,
                    "scheduler job was cancelled: {}",
                    job_id.as_str()
                )
            }
            Self::UnknownJob { job_id } => {
                write!(formatter, "scheduler job is unknown: {}", job_id.as_str())
            }
            Self::StaleCompletion { job_id } => {
                write!(
                    formatter,
                    "scheduler job completion was stale: {}",
                    job_id.as_str()
                )
            }
        }
    }
}

impl Error for SchedulerRejected {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobCancellationState {
    Queued,
    Running,
    AlreadyTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", deny_unknown_fields)]
pub enum JobCompletion {
    Accepted { job_id: JobId },
    Cancelled { job_id: JobId },
    StaleRejected { job_id: JobId },
}

#[derive(Debug, Clone)]
struct QueuedJob {
    envelope: JobEnvelope,
    sequence: u64,
}

#[derive(Debug, Clone)]
struct RunningJob {
    envelope: JobEnvelope,
    started_at_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalJobState {
    Completed,
    Cancelled,
    StaleRejected,
}

#[derive(Debug, Clone)]
pub struct JobScheduler {
    config: TaskRuntimeConfig,
    queue: VecDeque<QueuedJob>,
    running: BTreeMap<JobId, RunningJob>,
    terminal: BTreeMap<JobId, TerminalJobState>,
    resource_in_flight: BTreeMap<ResourceClass, usize>,
    next_sequence: u64,
    telemetry: SchedulerTelemetry,
}

impl JobScheduler {
    pub fn new(config: TaskRuntimeConfig) -> Self {
        let sample_limit = config.telemetry_sample_limit;
        Self {
            config,
            queue: VecDeque::new(),
            running: BTreeMap::new(),
            terminal: BTreeMap::new(),
            resource_in_flight: BTreeMap::new(),
            next_sequence: 0,
            telemetry: SchedulerTelemetry::new(sample_limit),
        }
    }

    pub fn submit(&mut self, envelope: JobEnvelope) -> Result<(), SchedulerRejected> {
        self.telemetry.record_submitted();
        if self.contains_job(&envelope.job_id) {
            self.telemetry.record_rejected();
            return Err(SchedulerRejected::DuplicateJobId {
                job_id: envelope.job_id,
            });
        }
        if envelope.cancellation_token.is_cancelled() {
            self.terminal
                .insert(envelope.job_id.clone(), TerminalJobState::Cancelled);
            self.telemetry.record_canceled();
            return Err(SchedulerRejected::Cancelled {
                job_id: envelope.job_id,
            });
        }

        let policy = self.config.queue_policy_for(envelope.domain);
        let queued_for_domain = self.queued_count_for_domain(envelope.domain);
        if queued_for_domain >= policy.max_queued {
            match policy.overflow {
                QueueOverflowPolicy::CoalesceObsolete => {
                    if let Some(index) = self.coalescing_candidate_index(&envelope) {
                        if let Some(coalesced) = self.queue.remove(index) {
                            self.terminal
                                .insert(coalesced.envelope.job_id, TerminalJobState::Cancelled);
                            self.telemetry.record_coalesced();
                        }
                    } else {
                        self.telemetry.record_rejected();
                        return Err(SchedulerRejected::QueueFull {
                            domain: envelope.domain,
                            max_queued: policy.max_queued,
                        });
                    }
                }
                QueueOverflowPolicy::Reject => {
                    self.telemetry.record_rejected();
                    return Err(SchedulerRejected::QueueFull {
                        domain: envelope.domain,
                        max_queued: policy.max_queued,
                    });
                }
            }
        }

        self.queue.push_back(QueuedJob {
            envelope,
            sequence: self.next_sequence,
        });
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.telemetry.record_admitted();
        self.record_queue_depth();
        Ok(())
    }

    pub fn start_next(&mut self, now_us: u64) -> Result<Option<JobEnvelope>, SchedulerRejected> {
        self.drain_cancelled_queued();

        let mut best: Option<(usize, u8, u64, u64)> = None;
        let mut saturated = BTreeSet::new();
        for (index, queued) in self.queue.iter().enumerate() {
            if self.has_resource_capacity(queued.envelope.resource_class) {
                let candidate = (
                    index,
                    queued.envelope.priority.rank(),
                    queued.envelope.submitted_at_us,
                    queued.sequence,
                );
                if best.is_none_or(|best| {
                    candidate.1 < best.1
                        || (candidate.1 == best.1 && candidate.2 < best.2)
                        || (candidate.1 == best.1 && candidate.2 == best.2 && candidate.3 < best.3)
                }) {
                    best = Some(candidate);
                }
            } else {
                saturated.insert(queued.envelope.resource_class);
            }
        }

        let Some((index, ..)) = best else {
            for resource_class in saturated {
                self.telemetry.record_resource_saturation(resource_class);
            }
            return Ok(None);
        };

        let queued = self
            .queue
            .remove(index)
            .expect("selected queued index must exist");
        let queue_latency_us = now_us.saturating_sub(queued.envelope.submitted_at_us);
        self.acquire_resource(queued.envelope.resource_class);
        self.telemetry.record_started(queue_latency_us);
        self.record_queue_depth();
        let envelope = queued.envelope;
        self.running.insert(
            envelope.job_id.clone(),
            RunningJob {
                envelope: envelope.clone(),
                started_at_us: now_us,
            },
        );
        Ok(Some(envelope))
    }

    pub fn cancel(&mut self, job_id: &JobId) -> Result<JobCancellationState, SchedulerRejected> {
        self.cancel_at(job_id, 0)
    }

    pub fn cancel_at(
        &mut self,
        job_id: &JobId,
        cancelled_at_us: u64,
    ) -> Result<JobCancellationState, SchedulerRejected> {
        if let Some(index) = self
            .queue
            .iter()
            .position(|queued| queued.envelope.job_id == *job_id)
        {
            let queued = self
                .queue
                .remove(index)
                .expect("selected queued index must exist");
            let wait_time_us = cancelled_at_us.saturating_sub(queued.envelope.submitted_at_us);
            queued.envelope.cancellation_token.cancel();
            self.terminal
                .insert(queued.envelope.job_id, TerminalJobState::Cancelled);
            self.telemetry.record_canceled_wait(wait_time_us);
            self.record_queue_depth();
            return Ok(JobCancellationState::Queued);
        }

        if let Some(running) = self.running.remove(job_id) {
            let run_time_us = cancelled_at_us.saturating_sub(running.started_at_us);
            let job_duration_us = cancelled_at_us.saturating_sub(running.envelope.submitted_at_us);
            running.envelope.cancellation_token.cancel();
            self.release_resource(running.envelope.resource_class);
            self.terminal
                .insert(running.envelope.job_id, TerminalJobState::Cancelled);
            self.telemetry
                .record_canceled_running(run_time_us, job_duration_us);
            return Ok(JobCancellationState::Running);
        }

        if matches!(
            self.terminal.get(job_id),
            Some(
                TerminalJobState::Cancelled
                    | TerminalJobState::Completed
                    | TerminalJobState::StaleRejected
            )
        ) {
            return Ok(JobCancellationState::AlreadyTerminal);
        }

        Err(SchedulerRejected::UnknownJob {
            job_id: job_id.clone(),
        })
    }

    pub fn complete_with_commit<F>(
        &mut self,
        job_id: &JobId,
        result: JobResult,
        completed_at_us: u64,
        current: CompletionFreshness,
        commit_visible_state: F,
    ) -> Result<JobCompletion, SchedulerRejected>
    where
        F: FnOnce(&JobResult),
    {
        if matches!(self.terminal.get(job_id), Some(TerminalJobState::Cancelled)) {
            return Ok(JobCompletion::Cancelled {
                job_id: job_id.clone(),
            });
        }

        let Some(running) = self.running.remove(job_id) else {
            return Err(SchedulerRejected::UnknownJob {
                job_id: job_id.clone(),
            });
        };
        self.release_resource(running.envelope.resource_class);

        if running.envelope.cancellation_token.is_cancelled() {
            self.terminal
                .insert(running.envelope.job_id.clone(), TerminalJobState::Cancelled);
            self.telemetry.record_canceled();
            return Ok(JobCompletion::Cancelled {
                job_id: running.envelope.job_id,
            });
        }

        if running.envelope.freshness.is_stale_for(current) {
            self.terminal.insert(
                running.envelope.job_id.clone(),
                TerminalJobState::StaleRejected,
            );
            self.telemetry.record_stale_rejected();
            return Ok(JobCompletion::StaleRejected {
                job_id: running.envelope.job_id,
            });
        }

        commit_visible_state(&result);
        let run_time_us = completed_at_us.saturating_sub(running.started_at_us);
        let job_duration_us = completed_at_us.saturating_sub(running.envelope.submitted_at_us);
        self.telemetry
            .record_completed(run_time_us, job_duration_us, &result);
        self.terminal
            .insert(running.envelope.job_id.clone(), TerminalJobState::Completed);
        Ok(JobCompletion::Accepted {
            job_id: running.envelope.job_id,
        })
    }

    pub fn telemetry_snapshot(&self) -> SchedulerTelemetrySnapshot {
        self.telemetry.snapshot(
            self.queue.len(),
            self.queue_depth_by_domain(),
            self.resource_usage_snapshot(),
        )
    }

    pub fn queued_len(&self) -> usize {
        self.queue.len()
    }

    pub fn resource_in_flight(&self, resource_class: ResourceClass) -> usize {
        self.resource_in_flight
            .get(&resource_class)
            .copied()
            .unwrap_or(0)
    }

    fn contains_job(&self, job_id: &JobId) -> bool {
        self.queue
            .iter()
            .any(|queued| queued.envelope.job_id == *job_id)
            || self.running.contains_key(job_id)
            || self.terminal.contains_key(job_id)
    }

    fn queued_count_for_domain(&self, domain: JobDomain) -> usize {
        self.queue
            .iter()
            .filter(|queued| queued.envelope.domain == domain)
            .count()
    }

    fn coalescing_candidate_index(&self, incoming: &JobEnvelope) -> Option<usize> {
        self.queue.iter().position(|queued| {
            queued.envelope.domain == incoming.domain
                && queued
                    .envelope
                    .freshness
                    .is_obsolete_compared_to(&incoming.freshness)
        })
    }

    fn has_resource_capacity(&self, resource_class: ResourceClass) -> bool {
        self.resource_in_flight(resource_class) < self.resource_capacity(resource_class)
    }

    fn resource_capacity(&self, resource_class: ResourceClass) -> usize {
        self.config
            .resource_budget_for(resource_class)
            .max_in_flight
            .max(1)
    }

    fn acquire_resource(&mut self, resource_class: ResourceClass) {
        *self.resource_in_flight.entry(resource_class).or_insert(0) += 1;
    }

    fn release_resource(&mut self, resource_class: ResourceClass) {
        let Some(in_flight) = self.resource_in_flight.get_mut(&resource_class) else {
            return;
        };
        *in_flight = in_flight.saturating_sub(1);
        if *in_flight == 0 {
            self.resource_in_flight.remove(&resource_class);
        }
    }

    fn drain_cancelled_queued(&mut self) {
        let mut index = 0;
        while index < self.queue.len() {
            if self.queue[index].envelope.cancellation_token.is_cancelled() {
                let queued = self.queue.remove(index).expect("queue index exists");
                self.terminal
                    .insert(queued.envelope.job_id, TerminalJobState::Cancelled);
                self.telemetry.record_canceled();
            } else {
                index += 1;
            }
        }
        self.record_queue_depth();
    }

    fn record_queue_depth(&mut self) {
        self.telemetry.record_queue_depth(self.queue.len());
    }

    fn queue_depth_by_domain(&self) -> Vec<DomainQueueDepth> {
        JobDomain::all()
            .iter()
            .copied()
            .filter_map(|domain| {
                let depth = self.queued_count_for_domain(domain);
                (depth > 0).then_some(DomainQueueDepth { domain, depth })
            })
            .collect()
    }

    fn resource_usage_snapshot(&self) -> Vec<ResourceUsageSnapshot> {
        self.config
            .resource_budgets
            .iter()
            .map(|budget| ResourceUsageSnapshot {
                resource_class: budget.resource_class,
                in_flight: self.resource_in_flight(budget.resource_class),
                max_in_flight: budget.max_in_flight.max(1),
            })
            .collect()
    }
}
