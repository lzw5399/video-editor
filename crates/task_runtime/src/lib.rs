//! Rust-owned task scheduling boundary contracts.
//!
//! `task_runtime` is the portable scheduler ownership boundary for preview,
//! audio, artifact, probe, project IO, and export work. Domain crates keep
//! owning their editing or execution semantics; this crate owns the shared
//! scheduler contracts, freshness vocabulary, and later runtime policy.

pub mod cancellation;
pub mod config;
pub mod freshness;
pub mod job;
pub mod scheduler;
pub mod telemetry;
pub mod testing;

pub use cancellation::TaskCancellationToken;
pub use config::{QueueOverflowPolicy, QueuePolicy, ResourceBudget, TaskRuntimeConfig};
pub use freshness::{
    PlaybackGeneration, PlaybackRate, PlaybackRateError, PlaybackState, TimelineClock,
    TimelineFreshness,
};
pub use job::{
    CompletionFreshness, JobDiagnosticClassification, JobDomain, JobEnvelope, JobFreshness, JobId,
    JobPriority, JobResult, JobResultKind, ResourceClass,
};
pub use scheduler::{JobCancellationState, JobCompletion, JobScheduler, SchedulerRejected};
pub use telemetry::{
    DiagnosticClassificationCount, DomainQueueDepth, ResourceSaturationSnapshot,
    ResourceUsageSnapshot, SchedulerTelemetrySnapshot, SchedulerTelemetrySummary,
};
