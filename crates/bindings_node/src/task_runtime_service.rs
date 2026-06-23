use std::sync::{Mutex, OnceLock};

use draft_model::{CommandError, CommandErrorKind, CommandResultEnvelope};
use serde::{Deserialize, Serialize};
use task_runtime::{
    DiagnosticClassificationCount, DomainQueueDepth, JobScheduler, ResourceUsageSnapshot,
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
    let snapshot = state.scheduler.telemetry_snapshot();

    ok_envelope(telemetry_response(snapshot, request.diagnostics))
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
