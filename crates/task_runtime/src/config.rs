use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

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
    pub const MAX_DEV_RESOURCE_CAPACITY: usize = 8;
    pub const MAX_DEV_QUEUE_DEPTH: usize = 128;
    pub const MAX_DEV_TELEMETRY_SAMPLE_LIMIT: usize = 4096;

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

    pub fn validate_dev_override(&self) -> Result<(), TaskRuntimeConfigValidationError> {
        if self.resource_budgets.is_empty() {
            return Err(TaskRuntimeConfigValidationError::new(
                "resourceBudgets must include at least one resource class",
            ));
        }
        if self.queue_policies.is_empty() {
            return Err(TaskRuntimeConfigValidationError::new(
                "queuePolicies must include at least one scheduler domain",
            ));
        }
        if self.telemetry_sample_limit == 0 {
            return Err(TaskRuntimeConfigValidationError::new(
                "telemetrySampleLimit must be greater than zero",
            ));
        }
        if self.telemetry_sample_limit > Self::MAX_DEV_TELEMETRY_SAMPLE_LIMIT {
            return Err(TaskRuntimeConfigValidationError::new(format!(
                "telemetrySampleLimit must be <= {}",
                Self::MAX_DEV_TELEMETRY_SAMPLE_LIMIT
            )));
        }

        let mut resource_classes = BTreeSet::new();
        for budget in &self.resource_budgets {
            if !resource_classes.insert(budget.resource_class) {
                return Err(TaskRuntimeConfigValidationError::new(format!(
                    "resourceBudgets contains duplicate resourceClass {:?}",
                    budget.resource_class
                )));
            }
            if budget.max_in_flight == 0 {
                return Err(TaskRuntimeConfigValidationError::new(format!(
                    "resourceBudgets {:?}.maxInFlight must be greater than zero",
                    budget.resource_class
                )));
            }
            if budget.max_in_flight > Self::MAX_DEV_RESOURCE_CAPACITY {
                return Err(TaskRuntimeConfigValidationError::new(format!(
                    "resourceBudgets {:?}.maxInFlight must be <= {}",
                    budget.resource_class,
                    Self::MAX_DEV_RESOURCE_CAPACITY
                )));
            }
        }

        let mut domains = BTreeSet::new();
        for policy in &self.queue_policies {
            if !domains.insert(policy.domain) {
                return Err(TaskRuntimeConfigValidationError::new(format!(
                    "queuePolicies contains duplicate domain {:?}",
                    policy.domain
                )));
            }
            if policy.max_queued == 0 {
                return Err(TaskRuntimeConfigValidationError::new(format!(
                    "queuePolicies {:?}.maxQueued must be greater than zero",
                    policy.domain
                )));
            }
            if policy.max_queued > Self::MAX_DEV_QUEUE_DEPTH {
                return Err(TaskRuntimeConfigValidationError::new(format!(
                    "queuePolicies {:?}.maxQueued must be <= {}",
                    policy.domain,
                    Self::MAX_DEV_QUEUE_DEPTH
                )));
            }
        }

        Ok(())
    }
}

impl Default for TaskRuntimeConfig {
    fn default() -> Self {
        Self::portable_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRuntimeConfigValidationError {
    message: String,
}

impl TaskRuntimeConfigValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TaskRuntimeConfigValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for TaskRuntimeConfigValidationError {}
