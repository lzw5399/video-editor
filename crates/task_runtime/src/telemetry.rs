use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{JobDiagnosticClassification, JobDomain, JobResult, JobResultKind, ResourceClass};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchedulerTelemetrySummary {
    pub sample_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p50: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p95: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
}

impl SchedulerTelemetrySummary {
    fn from_samples(samples: &VecDeque<u64>) -> Self {
        let mut values = samples.iter().copied().collect::<Vec<_>>();
        values.sort_unstable();
        Self {
            sample_count: samples.len() as u64,
            p50: percentile(&values, 50),
            p95: percentile(&values, 95),
            max: values.last().copied(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DomainQueueDepth {
    pub domain: JobDomain,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceUsageSnapshot {
    pub resource_class: ResourceClass,
    pub in_flight: usize,
    pub max_in_flight: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceSaturationSnapshot {
    pub resource_class: ResourceClass,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticClassificationCount {
    pub classification: JobDiagnosticClassification,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchedulerTelemetrySnapshot {
    pub submitted_count: u64,
    pub admitted_count: u64,
    pub started_count: u64,
    pub completed_count: u64,
    pub rejected_count: u64,
    pub coalesced_count: u64,
    pub canceled_count: u64,
    pub stale_rejected_count: u64,
    pub fallback_count: u64,
    pub unavailable_count: u64,
    pub cache_hit_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_frame_time_us: Option<u64>,
    pub dropped_frame_count: u64,
    pub repeated_frame_count: u64,
    pub current_queue_depth: usize,
    pub max_queue_depth: usize,
    pub queue_depth_by_domain: Vec<DomainQueueDepth>,
    pub resource_usage: Vec<ResourceUsageSnapshot>,
    pub resource_saturation_count: u64,
    pub resource_saturation: Vec<ResourceSaturationSnapshot>,
    pub fallback_classifications: Vec<DiagnosticClassificationCount>,
    pub unavailable_classifications: Vec<DiagnosticClassificationCount>,
    pub queue_latency_us: SchedulerTelemetrySummary,
    pub wait_time_us: SchedulerTelemetrySummary,
    pub run_time_us: SchedulerTelemetrySummary,
    pub job_duration_us: SchedulerTelemetrySummary,
}

#[derive(Debug, Clone)]
pub(crate) struct SchedulerTelemetry {
    sample_limit: usize,
    submitted_count: u64,
    admitted_count: u64,
    started_count: u64,
    completed_count: u64,
    rejected_count: u64,
    coalesced_count: u64,
    canceled_count: u64,
    stale_rejected_count: u64,
    fallback_count: u64,
    unavailable_count: u64,
    cache_hit_count: u64,
    first_frame_time_us: Option<u64>,
    dropped_frame_count: u64,
    repeated_frame_count: u64,
    max_queue_depth: usize,
    resource_saturation_count: u64,
    resource_saturation: BTreeMap<ResourceClass, u64>,
    fallback_classifications: BTreeMap<JobDiagnosticClassification, u64>,
    unavailable_classifications: BTreeMap<JobDiagnosticClassification, u64>,
    queue_latency_samples: VecDeque<u64>,
    wait_time_samples: VecDeque<u64>,
    run_time_samples: VecDeque<u64>,
    job_duration_samples: VecDeque<u64>,
}

impl SchedulerTelemetry {
    pub(crate) fn new(sample_limit: usize) -> Self {
        Self {
            sample_limit: sample_limit.max(1),
            submitted_count: 0,
            admitted_count: 0,
            started_count: 0,
            completed_count: 0,
            rejected_count: 0,
            coalesced_count: 0,
            canceled_count: 0,
            stale_rejected_count: 0,
            fallback_count: 0,
            unavailable_count: 0,
            cache_hit_count: 0,
            first_frame_time_us: None,
            dropped_frame_count: 0,
            repeated_frame_count: 0,
            max_queue_depth: 0,
            resource_saturation_count: 0,
            resource_saturation: BTreeMap::new(),
            fallback_classifications: BTreeMap::new(),
            unavailable_classifications: BTreeMap::new(),
            queue_latency_samples: VecDeque::new(),
            wait_time_samples: VecDeque::new(),
            run_time_samples: VecDeque::new(),
            job_duration_samples: VecDeque::new(),
        }
    }

    pub(crate) fn record_submitted(&mut self) {
        self.submitted_count = self.submitted_count.saturating_add(1);
    }

    pub(crate) fn record_admitted(&mut self) {
        self.admitted_count = self.admitted_count.saturating_add(1);
    }

    pub(crate) fn record_started(&mut self, queue_latency_us: u64) {
        self.started_count = self.started_count.saturating_add(1);
        push_sample(
            &mut self.queue_latency_samples,
            queue_latency_us,
            self.sample_limit,
        );
        push_sample(
            &mut self.wait_time_samples,
            queue_latency_us,
            self.sample_limit,
        );
    }

    pub(crate) fn record_completed(
        &mut self,
        run_time_us: u64,
        job_duration_us: u64,
        result: &JobResult,
    ) {
        self.completed_count = self.completed_count.saturating_add(1);
        push_sample(&mut self.run_time_samples, run_time_us, self.sample_limit);
        push_sample(
            &mut self.job_duration_samples,
            job_duration_us,
            self.sample_limit,
        );
        if result.cache_hit {
            self.cache_hit_count = self.cache_hit_count.saturating_add(1);
        }
        if let Some(first_frame_time_us) = result.first_frame_time_us {
            self.first_frame_time_us = Some(
                self.first_frame_time_us
                    .map_or(first_frame_time_us, |current| {
                        current.min(first_frame_time_us)
                    }),
            );
        }
        self.dropped_frame_count = self
            .dropped_frame_count
            .saturating_add(result.dropped_frame_count);
        self.repeated_frame_count = self
            .repeated_frame_count
            .saturating_add(result.repeated_frame_count);

        match result.kind {
            JobResultKind::Fallback { classification } => {
                self.fallback_count = self.fallback_count.saturating_add(1);
                increment_classification(&mut self.fallback_classifications, classification);
            }
            JobResultKind::Unavailable { classification } => {
                self.unavailable_count = self.unavailable_count.saturating_add(1);
                increment_classification(&mut self.unavailable_classifications, classification);
            }
            JobResultKind::Completed | JobResultKind::Failed => {}
        }
    }

    pub(crate) fn record_rejected(&mut self) {
        self.rejected_count = self.rejected_count.saturating_add(1);
    }

    pub(crate) fn record_coalesced(&mut self) {
        self.coalesced_count = self.coalesced_count.saturating_add(1);
    }

    pub(crate) fn record_canceled(&mut self) {
        self.canceled_count = self.canceled_count.saturating_add(1);
    }

    pub(crate) fn record_canceled_wait(&mut self, wait_time_us: u64) {
        self.record_canceled();
        push_sample(&mut self.wait_time_samples, wait_time_us, self.sample_limit);
    }

    pub(crate) fn record_canceled_running(&mut self, run_time_us: u64, job_duration_us: u64) {
        self.record_canceled();
        push_sample(&mut self.run_time_samples, run_time_us, self.sample_limit);
        push_sample(
            &mut self.job_duration_samples,
            job_duration_us,
            self.sample_limit,
        );
    }

    pub(crate) fn record_stale_rejected(&mut self) {
        self.stale_rejected_count = self.stale_rejected_count.saturating_add(1);
    }

    pub(crate) fn record_queue_depth(&mut self, queue_depth: usize) {
        self.max_queue_depth = self.max_queue_depth.max(queue_depth);
    }

    pub(crate) fn record_resource_saturation(&mut self, resource_class: ResourceClass) {
        self.resource_saturation_count = self.resource_saturation_count.saturating_add(1);
        *self.resource_saturation.entry(resource_class).or_insert(0) += 1;
    }

    pub(crate) fn snapshot(
        &self,
        current_queue_depth: usize,
        queue_depth_by_domain: Vec<DomainQueueDepth>,
        resource_usage: Vec<ResourceUsageSnapshot>,
    ) -> SchedulerTelemetrySnapshot {
        SchedulerTelemetrySnapshot {
            submitted_count: self.submitted_count,
            admitted_count: self.admitted_count,
            started_count: self.started_count,
            completed_count: self.completed_count,
            rejected_count: self.rejected_count,
            coalesced_count: self.coalesced_count,
            canceled_count: self.canceled_count,
            stale_rejected_count: self.stale_rejected_count,
            fallback_count: self.fallback_count,
            unavailable_count: self.unavailable_count,
            cache_hit_count: self.cache_hit_count,
            first_frame_time_us: self.first_frame_time_us,
            dropped_frame_count: self.dropped_frame_count,
            repeated_frame_count: self.repeated_frame_count,
            current_queue_depth,
            max_queue_depth: self.max_queue_depth,
            queue_depth_by_domain,
            resource_usage,
            resource_saturation_count: self.resource_saturation_count,
            resource_saturation: self
                .resource_saturation
                .iter()
                .map(|(resource_class, count)| ResourceSaturationSnapshot {
                    resource_class: *resource_class,
                    count: *count,
                })
                .collect(),
            fallback_classifications: classification_counts(&self.fallback_classifications),
            unavailable_classifications: classification_counts(&self.unavailable_classifications),
            queue_latency_us: SchedulerTelemetrySummary::from_samples(&self.queue_latency_samples),
            wait_time_us: SchedulerTelemetrySummary::from_samples(&self.wait_time_samples),
            run_time_us: SchedulerTelemetrySummary::from_samples(&self.run_time_samples),
            job_duration_us: SchedulerTelemetrySummary::from_samples(&self.job_duration_samples),
        }
    }
}

fn push_sample(samples: &mut VecDeque<u64>, sample: u64, sample_limit: usize) {
    samples.push_back(sample);
    while samples.len() > sample_limit {
        samples.pop_front();
    }
}

fn increment_classification(
    counts: &mut BTreeMap<JobDiagnosticClassification, u64>,
    classification: JobDiagnosticClassification,
) {
    *counts.entry(classification).or_insert(0) += 1;
}

fn classification_counts(
    counts: &BTreeMap<JobDiagnosticClassification, u64>,
) -> Vec<DiagnosticClassificationCount> {
    counts
        .iter()
        .map(|(classification, count)| DiagnosticClassificationCount {
            classification: *classification,
            count: *count,
        })
        .collect()
}

fn percentile(sorted_values: &[u64], percentile: usize) -> Option<u64> {
    if sorted_values.is_empty() {
        return None;
    }
    let last_index = sorted_values.len().saturating_sub(1);
    let index = last_index.saturating_mul(percentile).saturating_add(99) / 100;
    sorted_values.get(index).copied()
}
