use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use draft_model::{CommandError, CommandErrorKind, CommandResultEnvelope};
use serde::{Deserialize, Serialize};
use task_runtime::{
    DiagnosticClassificationCount, DomainQueueDepth, JobDiagnosticClassification, JobDomain,
    JobScheduler, ResourceClass, ResourceSaturationSnapshot, ResourceUsageSnapshot,
    SchedulerTelemetrySnapshot, SchedulerTelemetrySummary, TaskRuntimeConfig,
};

#[derive(Debug)]
struct TaskRuntimeServiceState {
    config: TaskRuntimeConfig,
    scheduler: JobScheduler,
    config_revision: u64,
}

impl Default for TaskRuntimeServiceState {
    fn default() -> Self {
        let config = TaskRuntimeConfig::portable_default();
        Self {
            scheduler: JobScheduler::new(config.clone()),
            config,
            config_revision: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskRuntimeStatus {
    Ready,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskRuntimeTelemetrySource {
    InteractivePreview,
    AudioPreview,
    Export,
    ArtifactGeneration,
    MediaProbe,
    ProjectIo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRuntimeDiagnosticsRequest {
    #[serde(default)]
    pub diagnostics: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRuntimeStatusResponse {
    pub status: TaskRuntimeStatus,
    pub status_label: String,
    pub work_available: bool,
    pub telemetry_available: bool,
    pub config_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<TaskRuntimeStatusDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRuntimeStatusDetails {
    pub resource_class_count: usize,
    pub domain_policy_count: usize,
    pub telemetry_sample_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRuntimeTelemetryResponse {
    pub status: TaskRuntimeStatus,
    pub status_label: String,
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
    pub first_frame_time_us: Option<u64>,
    pub dropped_frame_count: u64,
    pub repeated_frame_count: u64,
    pub resource_saturation_count: u64,
    pub queue_latency_us: SchedulerTelemetrySummary,
    pub wait_time_us: SchedulerTelemetrySummary,
    pub run_time_us: SchedulerTelemetrySummary,
    pub job_duration_us: SchedulerTelemetrySummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<TaskRuntimeTelemetryDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRuntimeTelemetryDetails {
    pub current_queue_depth: usize,
    pub max_queue_depth: usize,
    pub queue_depth_by_domain: Vec<DomainQueueDepth>,
    pub resource_usage: Vec<ResourceUsageSnapshot>,
    pub resource_saturation: Vec<task_runtime::ResourceSaturationSnapshot>,
    pub fallback_classifications: Vec<DiagnosticClassificationCount>,
    pub unavailable_classifications: Vec<DiagnosticClassificationCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRuntimeDevConfigRequest {
    pub developer_diagnostics: bool,
    pub config: TaskRuntimeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRuntimeDevConfigResponse {
    pub applied: bool,
    pub config_revision: u64,
    pub resource_class_count: usize,
    pub domain_policy_count: usize,
    pub telemetry_sample_limit: usize,
}

pub fn get_task_runtime_status_command(
    request: serde_json::Value,
) -> CommandResultEnvelope<TaskRuntimeStatusResponse> {
    let request = match parse_request::<TaskRuntimeDiagnosticsRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid getTaskRuntimeStatus payload: {error}"),
                "getTaskRuntimeStatus",
            );
        }
    };

    let state = match task_runtime_service_state().lock() {
        Ok(state) => state,
        Err(_) => {
            return error_envelope(
                CommandErrorKind::Internal,
                "task runtime service lock poisoned".to_owned(),
                "getTaskRuntimeStatus",
            );
        }
    };

    ok_envelope(status_response(&state, request.diagnostics))
}

pub fn get_task_runtime_telemetry_command(
    request: serde_json::Value,
) -> CommandResultEnvelope<TaskRuntimeTelemetryResponse> {
    let request = match parse_request::<TaskRuntimeDiagnosticsRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid getTaskRuntimeTelemetry payload: {error}"),
                "getTaskRuntimeTelemetry",
            );
        }
    };
    refresh_task_runtime_scheduler_snapshots();

    let state = match task_runtime_service_state().lock() {
        Ok(state) => state,
        Err(_) => {
            return error_envelope(
                CommandErrorKind::Internal,
                "task runtime service lock poisoned".to_owned(),
                "getTaskRuntimeTelemetry",
            );
        }
    };
    let snapshot = aggregate_task_runtime_telemetry(state.scheduler.telemetry_snapshot());

    ok_envelope(telemetry_response(snapshot, request.diagnostics))
}

pub fn record_task_runtime_scheduler_snapshot(
    source: TaskRuntimeTelemetrySource,
    snapshot: &SchedulerTelemetrySnapshot,
) {
    if !scheduler_snapshot_has_activity(snapshot) {
        return;
    }
    if let Ok(mut snapshots) = task_runtime_scheduler_snapshots().lock() {
        snapshots.insert(source, snapshot.clone());
    }
}

pub fn clear_task_runtime_scheduler_snapshots() {
    if let Ok(mut snapshots) = task_runtime_scheduler_snapshots().lock() {
        snapshots.clear();
    }
}

pub fn apply_task_runtime_dev_config_command(
    request: serde_json::Value,
) -> CommandResultEnvelope<TaskRuntimeDevConfigResponse> {
    let request = match parse_request::<TaskRuntimeDevConfigRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid applyTaskRuntimeDevConfig payload: {error}"),
                "applyTaskRuntimeDevConfig",
            );
        }
    };

    if !request.developer_diagnostics {
        return error_envelope(
            CommandErrorKind::InvalidPayload,
            "applyTaskRuntimeDevConfig requires developerDiagnostics=true".to_owned(),
            "applyTaskRuntimeDevConfig",
        );
    }

    if let Err(error) = request.config.validate_dev_override() {
        return error_envelope(
            CommandErrorKind::InvalidPayload,
            format!("Invalid task runtime dev config: {error}"),
            "applyTaskRuntimeDevConfig",
        );
    }

    let mut state = match task_runtime_service_state().lock() {
        Ok(state) => state,
        Err(_) => {
            return error_envelope(
                CommandErrorKind::Internal,
                "task runtime service lock poisoned".to_owned(),
                "applyTaskRuntimeDevConfig",
            );
        }
    };

    state.config = request.config;
    state.scheduler = JobScheduler::new(state.config.clone());
    state.config_revision = state.config_revision.saturating_add(1);

    ok_envelope(TaskRuntimeDevConfigResponse {
        applied: true,
        config_revision: state.config_revision,
        resource_class_count: state.config.resource_budgets.len(),
        domain_policy_count: state.config.queue_policies.len(),
        telemetry_sample_limit: state.config.telemetry_sample_limit,
    })
}

fn status_response(
    state: &TaskRuntimeServiceState,
    diagnostics: bool,
) -> TaskRuntimeStatusResponse {
    TaskRuntimeStatusResponse {
        status: TaskRuntimeStatus::Ready,
        status_label: "调度服务就绪".to_owned(),
        work_available: true,
        telemetry_available: true,
        config_revision: state.config_revision,
        details: diagnostics.then(|| TaskRuntimeStatusDetails {
            resource_class_count: state.config.resource_budgets.len(),
            domain_policy_count: state.config.queue_policies.len(),
            telemetry_sample_limit: state.config.telemetry_sample_limit,
        }),
    }
}

fn telemetry_response(
    snapshot: SchedulerTelemetrySnapshot,
    diagnostics: bool,
) -> TaskRuntimeTelemetryResponse {
    TaskRuntimeTelemetryResponse {
        status: TaskRuntimeStatus::Ready,
        status_label: "调度服务就绪".to_owned(),
        submitted_count: snapshot.submitted_count,
        admitted_count: snapshot.admitted_count,
        started_count: snapshot.started_count,
        completed_count: snapshot.completed_count,
        rejected_count: snapshot.rejected_count,
        coalesced_count: snapshot.coalesced_count,
        canceled_count: snapshot.canceled_count,
        stale_rejected_count: snapshot.stale_rejected_count,
        fallback_count: snapshot.fallback_count,
        unavailable_count: snapshot.unavailable_count,
        cache_hit_count: snapshot.cache_hit_count,
        first_frame_time_us: snapshot.first_frame_time_us,
        dropped_frame_count: snapshot.dropped_frame_count,
        repeated_frame_count: snapshot.repeated_frame_count,
        resource_saturation_count: snapshot.resource_saturation_count,
        queue_latency_us: snapshot.queue_latency_us,
        wait_time_us: snapshot.wait_time_us,
        run_time_us: snapshot.run_time_us,
        job_duration_us: snapshot.job_duration_us,
        details: diagnostics.then(|| TaskRuntimeTelemetryDetails {
            current_queue_depth: snapshot.current_queue_depth,
            max_queue_depth: snapshot.max_queue_depth,
            queue_depth_by_domain: snapshot.queue_depth_by_domain,
            resource_usage: snapshot.resource_usage,
            resource_saturation: snapshot.resource_saturation,
            fallback_classifications: snapshot.fallback_classifications,
            unavailable_classifications: snapshot.unavailable_classifications,
        }),
    }
}

fn aggregate_task_runtime_telemetry(
    service_snapshot: SchedulerTelemetrySnapshot,
) -> SchedulerTelemetrySnapshot {
    let mut aggregate = TelemetrySnapshotAccumulator::default();
    aggregate.add_snapshot(&service_snapshot);
    if let Ok(snapshots) = task_runtime_scheduler_snapshots().lock() {
        for snapshot in snapshots.values() {
            aggregate.add_snapshot(snapshot);
        }
    }
    aggregate.into_snapshot()
}

fn refresh_task_runtime_scheduler_snapshots() {
    crate::record_realtime_preview_task_runtime_telemetry_snapshots();
    crate::record_audio_preview_task_runtime_telemetry_snapshot();
    crate::preview_export_service::global_export_registry()
        .record_task_runtime_telemetry_snapshot();
    crate::artifact_store_service::record_artifact_task_runtime_telemetry_snapshot();
    crate::project_session_service::record_project_session_task_runtime_telemetry_snapshots();
}

fn scheduler_snapshot_has_activity(snapshot: &SchedulerTelemetrySnapshot) -> bool {
    snapshot.submitted_count > 0
        || snapshot.admitted_count > 0
        || snapshot.started_count > 0
        || snapshot.completed_count > 0
        || snapshot.rejected_count > 0
        || snapshot.coalesced_count > 0
        || snapshot.canceled_count > 0
        || snapshot.stale_rejected_count > 0
        || snapshot.fallback_count > 0
        || snapshot.unavailable_count > 0
        || snapshot.cache_hit_count > 0
        || snapshot.dropped_frame_count > 0
        || snapshot.repeated_frame_count > 0
        || snapshot.current_queue_depth > 0
        || snapshot.max_queue_depth > 0
        || snapshot.resource_saturation_count > 0
        || snapshot.queue_latency_us.sample_count > 0
        || snapshot.wait_time_us.sample_count > 0
        || snapshot.run_time_us.sample_count > 0
        || snapshot.job_duration_us.sample_count > 0
}

#[derive(Debug, Default)]
struct TelemetrySnapshotAccumulator {
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
    current_queue_depth: usize,
    max_queue_depth: usize,
    queue_depth_by_domain: BTreeMap<JobDomain, usize>,
    resource_usage: BTreeMap<ResourceClass, ResourceUsageAccumulator>,
    resource_saturation_count: u64,
    resource_saturation: BTreeMap<ResourceClass, u64>,
    fallback_classifications: BTreeMap<JobDiagnosticClassification, u64>,
    unavailable_classifications: BTreeMap<JobDiagnosticClassification, u64>,
    queue_latency_us: SchedulerSummaryAccumulator,
    wait_time_us: SchedulerSummaryAccumulator,
    run_time_us: SchedulerSummaryAccumulator,
    job_duration_us: SchedulerSummaryAccumulator,
}

impl TelemetrySnapshotAccumulator {
    fn add_snapshot(&mut self, snapshot: &SchedulerTelemetrySnapshot) {
        self.submitted_count = self
            .submitted_count
            .saturating_add(snapshot.submitted_count);
        self.admitted_count = self.admitted_count.saturating_add(snapshot.admitted_count);
        self.started_count = self.started_count.saturating_add(snapshot.started_count);
        self.completed_count = self
            .completed_count
            .saturating_add(snapshot.completed_count);
        self.rejected_count = self.rejected_count.saturating_add(snapshot.rejected_count);
        self.coalesced_count = self
            .coalesced_count
            .saturating_add(snapshot.coalesced_count);
        self.canceled_count = self.canceled_count.saturating_add(snapshot.canceled_count);
        self.stale_rejected_count = self
            .stale_rejected_count
            .saturating_add(snapshot.stale_rejected_count);
        self.fallback_count = self.fallback_count.saturating_add(snapshot.fallback_count);
        self.unavailable_count = self
            .unavailable_count
            .saturating_add(snapshot.unavailable_count);
        self.cache_hit_count = self
            .cache_hit_count
            .saturating_add(snapshot.cache_hit_count);
        self.first_frame_time_us = match (self.first_frame_time_us, snapshot.first_frame_time_us) {
            (Some(current), Some(incoming)) => Some(current.min(incoming)),
            (None, Some(incoming)) => Some(incoming),
            (current, None) => current,
        };
        self.dropped_frame_count = self
            .dropped_frame_count
            .saturating_add(snapshot.dropped_frame_count);
        self.repeated_frame_count = self
            .repeated_frame_count
            .saturating_add(snapshot.repeated_frame_count);
        self.current_queue_depth = self
            .current_queue_depth
            .saturating_add(snapshot.current_queue_depth);
        self.max_queue_depth = self
            .max_queue_depth
            .saturating_add(snapshot.max_queue_depth);
        for item in &snapshot.queue_depth_by_domain {
            let depth = self.queue_depth_by_domain.entry(item.domain).or_default();
            *depth = depth.saturating_add(item.depth);
        }
        for item in &snapshot.resource_usage {
            self.resource_usage
                .entry(item.resource_class)
                .or_default()
                .add(item);
        }
        self.resource_saturation_count = self
            .resource_saturation_count
            .saturating_add(snapshot.resource_saturation_count);
        for item in &snapshot.resource_saturation {
            let count = self
                .resource_saturation
                .entry(item.resource_class)
                .or_default();
            *count = count.saturating_add(item.count);
        }
        for item in &snapshot.fallback_classifications {
            let count = self
                .fallback_classifications
                .entry(item.classification)
                .or_default();
            *count = count.saturating_add(item.count);
        }
        for item in &snapshot.unavailable_classifications {
            let count = self
                .unavailable_classifications
                .entry(item.classification)
                .or_default();
            *count = count.saturating_add(item.count);
        }
        self.queue_latency_us.add(&snapshot.queue_latency_us);
        self.wait_time_us.add(&snapshot.wait_time_us);
        self.run_time_us.add(&snapshot.run_time_us);
        self.job_duration_us.add(&snapshot.job_duration_us);
    }

    fn into_snapshot(self) -> SchedulerTelemetrySnapshot {
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
            current_queue_depth: self.current_queue_depth,
            max_queue_depth: self.max_queue_depth,
            queue_depth_by_domain: self
                .queue_depth_by_domain
                .into_iter()
                .map(|(domain, depth)| DomainQueueDepth { domain, depth })
                .collect(),
            resource_usage: self
                .resource_usage
                .into_iter()
                .map(|(resource_class, usage)| ResourceUsageSnapshot {
                    resource_class,
                    in_flight: usage.in_flight,
                    max_in_flight: usage.max_in_flight,
                })
                .collect(),
            resource_saturation_count: self.resource_saturation_count,
            resource_saturation: self
                .resource_saturation
                .into_iter()
                .map(|(resource_class, count)| ResourceSaturationSnapshot {
                    resource_class,
                    count,
                })
                .collect(),
            fallback_classifications: self
                .fallback_classifications
                .into_iter()
                .map(|(classification, count)| DiagnosticClassificationCount {
                    classification,
                    count,
                })
                .collect(),
            unavailable_classifications: self
                .unavailable_classifications
                .into_iter()
                .map(|(classification, count)| DiagnosticClassificationCount {
                    classification,
                    count,
                })
                .collect(),
            queue_latency_us: self.queue_latency_us.into_summary(),
            wait_time_us: self.wait_time_us.into_summary(),
            run_time_us: self.run_time_us.into_summary(),
            job_duration_us: self.job_duration_us.into_summary(),
        }
    }
}

#[derive(Debug, Default)]
struct ResourceUsageAccumulator {
    in_flight: usize,
    max_in_flight: usize,
}

impl ResourceUsageAccumulator {
    fn add(&mut self, snapshot: &ResourceUsageSnapshot) {
        self.in_flight = self.in_flight.saturating_add(snapshot.in_flight);
        self.max_in_flight = self.max_in_flight.saturating_add(snapshot.max_in_flight);
    }
}

#[derive(Debug, Default)]
struct SchedulerSummaryAccumulator {
    sample_count: u64,
    p50: Option<u64>,
    p95: Option<u64>,
    max: Option<u64>,
}

impl SchedulerSummaryAccumulator {
    fn add(&mut self, summary: &SchedulerTelemetrySummary) {
        self.sample_count = self.sample_count.saturating_add(summary.sample_count);
        self.p50 = max_optional(self.p50, summary.p50);
        self.p95 = max_optional(self.p95, summary.p95);
        self.max = max_optional(self.max, summary.max);
    }

    fn into_summary(self) -> SchedulerTelemetrySummary {
        SchedulerTelemetrySummary {
            sample_count: self.sample_count,
            p50: self.p50,
            p95: self.p95,
            max: self.max,
        }
    }
}

fn max_optional(current: Option<u64>, incoming: Option<u64>) -> Option<u64> {
    match (current, incoming) {
        (Some(current), Some(incoming)) => Some(current.max(incoming)),
        (None, Some(incoming)) => Some(incoming),
        (current, None) => current,
    }
}

fn parse_request<T>(request: serde_json::Value) -> Result<T, serde_json::Error>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value::<T>(request)
}

fn ok_envelope<T>(data: T) -> CommandResultEnvelope<T> {
    CommandResultEnvelope {
        ok: true,
        data: Some(data),
        error: None,
        events: Vec::new(),
    }
}

fn error_envelope<T>(
    kind: CommandErrorKind,
    message: String,
    command: &'static str,
) -> CommandResultEnvelope<T> {
    CommandResultEnvelope {
        ok: false,
        data: None,
        error: Some(CommandError {
            kind,
            message,
            command: Some(command.to_owned()),
        }),
        events: Vec::new(),
    }
}

fn task_runtime_service_state() -> &'static Mutex<TaskRuntimeServiceState> {
    static STATE: OnceLock<Mutex<TaskRuntimeServiceState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TaskRuntimeServiceState::default()))
}

fn task_runtime_scheduler_snapshots()
-> &'static Mutex<BTreeMap<TaskRuntimeTelemetrySource, SchedulerTelemetrySnapshot>> {
    static SNAPSHOTS: OnceLock<
        Mutex<BTreeMap<TaskRuntimeTelemetrySource, SchedulerTelemetrySnapshot>>,
    > = OnceLock::new();
    SNAPSHOTS.get_or_init(|| Mutex::new(BTreeMap::new()))
}
