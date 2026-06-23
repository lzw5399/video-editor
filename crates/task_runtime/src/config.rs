use serde::{Deserialize, Serialize};

use crate::{JobDomain, ResourceClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueueOverflowPolicy {
    Reject,
    CoalesceObsolete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueuePolicy {
    pub domain: JobDomain,
    pub max_queued: usize,
    pub overflow: QueueOverflowPolicy,
}

impl QueuePolicy {
    pub fn default_for_domain(domain: JobDomain) -> Self {
        let (max_queued, overflow) = match domain {
            JobDomain::InteractivePreview | JobDomain::ScrubSeek | JobDomain::Audio => {
                (4, QueueOverflowPolicy::CoalesceObsolete)
            }
            JobDomain::Decode | JobDomain::MediaProbe | JobDomain::Analysis => {
                (8, QueueOverflowPolicy::Reject)
            }
            JobDomain::ArtifactGeneration | JobDomain::FilesystemIo => {
                (16, QueueOverflowPolicy::Reject)
            }
            JobDomain::Export => (4, QueueOverflowPolicy::Reject),
        };
        Self {
            domain,
            max_queued,
            overflow,
        }
    }

    pub const fn coalesces_obsolete(&self) -> bool {
        matches!(self.overflow, QueueOverflowPolicy::CoalesceObsolete)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceBudget {
    pub resource_class: ResourceClass,
    pub max_in_flight: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRuntimeConfig {
    pub resource_budgets: Vec<ResourceBudget>,
    pub queue_policies: Vec<QueuePolicy>,
    pub telemetry_sample_limit: usize,
}

impl TaskRuntimeConfig {
    pub fn portable_default() -> Self {
        Self {
            resource_budgets: vec![
                ResourceBudget {
                    resource_class: ResourceClass::GpuPresent,
                    max_in_flight: 2,
                },
                ResourceBudget {
                    resource_class: ResourceClass::GpuDecode,
                    max_in_flight: 1,
                },
                ResourceBudget {
                    resource_class: ResourceClass::CpuDecode,
                    max_in_flight: 2,
                },
                ResourceBudget {
                    resource_class: ResourceClass::AudioRealtime,
                    max_in_flight: 2,
                },
                ResourceBudget {
                    resource_class: ResourceClass::FfmpegProcess,
                    max_in_flight: 1,
                },
                ResourceBudget {
                    resource_class: ResourceClass::DiskIo,
                    max_in_flight: 2,
                },
                ResourceBudget {
                    resource_class: ResourceClass::SqliteWrite,
                    max_in_flight: 1,
                },
                ResourceBudget {
                    resource_class: ResourceClass::BackgroundCpu,
                    max_in_flight: 2,
                },
                ResourceBudget {
                    resource_class: ResourceClass::ValidationProbe,
                    max_in_flight: 1,
                },
            ],
            queue_policies: JobDomain::all()
                .iter()
                .copied()
                .map(QueuePolicy::default_for_domain)
                .collect(),
            telemetry_sample_limit: 512,
        }
    }

    pub fn queue_policy_for(&self, domain: JobDomain) -> QueuePolicy {
        self.queue_policies
            .iter()
            .find(|policy| policy.domain == domain)
            .cloned()
            .unwrap_or_else(|| QueuePolicy::default_for_domain(domain))
    }

    pub fn resource_budget_for(&self, resource_class: ResourceClass) -> ResourceBudget {
        self.resource_budgets
            .iter()
            .find(|budget| budget.resource_class == resource_class)
            .copied()
            .unwrap_or(ResourceBudget {
                resource_class,
                max_in_flight: 1,
            })
    }
}

impl Default for TaskRuntimeConfig {
    fn default() -> Self {
        Self::portable_default()
    }
}
