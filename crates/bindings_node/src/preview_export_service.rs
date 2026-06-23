use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Instant;

use draft_model::{
    DirtyDomain, DirtyRange, Draft, ExportDiagnostic, ExportDiagnosticKind, ExportJobPhase,
    ExportJobStatusResponse, ExportPrepDirtyFacts, ExportPreset, ExportValidationReport,
    MaterialId, Microseconds, PreviewArtifactResponse, PreviewCacheEntryRef,
    PreviewCacheInvalidationResponse, PreviewDiagnostic, PreviewDiagnosticKind,
    PreviewOutputProfile, PreviewStatus, StartExportCommandPayload, TargetTimerange,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{
    CompileContext, CompilerCapabilities, FfmpegCompileError, FfmpegJob,
    OutputValidationExpectation as CompileValidation, TextRenderCapability, compile_ffmpeg_job,
};
use media_runtime::FfmpegExecutor;
use media_runtime::{
    CancelToken, FfmpegJobEvent, FfmpegJobResult, FfmpegJobState, FfmpegRuntimeError,
    FfmpegRuntimeJob, OutputValidationError, OutputValidationExpectation, RuntimeCapabilityReport,
    RuntimeConfig, validate_rendered_output,
};
use media_runtime_desktop::{DesktopFfmpegExecutor, probe_desktop_runtime_capabilities};
use preview_service::{
    PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile, PreviewFrameRequest,
    PreviewFrameResponse, PreviewInvalidationRequest, PreviewSegmentRequest,
    PreviewSegmentResponse, PreviewServiceConfig, PreviewServiceError, PreviewServiceErrorKind,
    invalidate_preview_cache, request_preview_frame, request_preview_segment,
};
use render_graph::{
    ExportMp4Preset, OutputDimensions, RenderAudioCodec, RenderContainer, RenderGraphPlan,
    RenderOutputProfile, RenderVideoCodec, build_render_graph,
};
use serde::Serialize;
use task_runtime::{
    CompletionFreshness, JobCompletion, JobDomain, JobEnvelope, JobId, JobPriority, JobResult,
    JobResultKind, ResourceClass, SchedulerTelemetrySnapshot, TaskCancellationToken,
    TaskRuntimeConfig,
};

#[derive(Debug)]
pub enum PreviewCommandError {
    Service(PreviewServiceError),
}

#[derive(Debug)]
pub enum ExportCommandError {
    InvalidOutputPath(String),
    Engine(String),
    RenderGraph(String),
    Compile(FfmpegCompileError),
    Runtime(FfmpegRuntimeError),
    Validation(OutputValidationError),
    Scheduler(String),
    UnknownJob(String),
    Io(String),
}

impl fmt::Display for PreviewCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Service(error) => write!(formatter, "preview service failed: {error}"),
        }
    }
}

impl Error for PreviewCommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Service(error) => Some(error),
        }
    }
}

impl From<PreviewServiceError> for PreviewCommandError {
    fn from(error: PreviewServiceError) -> Self {
        Self::Service(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewFrameArtifactRequest {
    pub draft: Draft,
    pub target_time: Microseconds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSegmentArtifactRequest {
    pub draft: Draft,
    pub target_timerange: TargetTimerange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewCacheInvalidationCommand {
    pub entries: Vec<PreviewCacheEntryRef>,
    pub changed_ranges: Vec<DirtyRange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub changed_graph_node_ids: Vec<String>,
    pub changed_domains: Vec<DirtyDomain>,
    pub runtime_capability_fingerprint: Option<String>,
    pub output_profile_fingerprint: Option<String>,
    pub full_draft: bool,
    pub reason: String,
    pub artifact_schema_version: u32,
    pub generator_version: String,
}

impl fmt::Display for ExportCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOutputPath(message)
            | Self::Engine(message)
            | Self::RenderGraph(message)
            | Self::Scheduler(message)
            | Self::UnknownJob(message)
            | Self::Io(message) => write!(formatter, "{message}"),
            Self::Compile(error) => write!(formatter, "export compile failed: {}", error.message),
            Self::Runtime(error) => write!(formatter, "export runtime failed: {}", error.message),
            Self::Validation(error) => {
                write!(formatter, "export validation failed: {}", error.message)
            }
        }
    }
}

impl Error for ExportCommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Compile(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::Validation(error) => Some(error),
            _ => None,
        }
    }
}

pub fn request_preview_frame_with_executor(
    executor: &impl FfmpegExecutor,
    config: &PreviewServiceConfig,
    payload: PreviewFrameArtifactRequest,
) -> Result<PreviewArtifactResponse, PreviewCommandError> {
    let response = request_preview_frame(
        executor,
        config,
        &PreviewFrameRequest {
            draft: payload.draft,
            target_time: payload.target_time,
        },
    )?;
    Ok(frame_response(response))
}

pub fn request_preview_segment_with_executor(
    executor: &impl FfmpegExecutor,
    config: &PreviewServiceConfig,
    payload: PreviewSegmentArtifactRequest,
) -> Result<PreviewArtifactResponse, PreviewCommandError> {
    let response = request_preview_segment(
        executor,
        config,
        &PreviewSegmentRequest {
            draft: payload.draft,
            target_timerange: payload.target_timerange,
        },
    )?;
    Ok(segment_response(response))
}

pub fn invalidate_preview_cache_command(
    payload: PreviewCacheInvalidationCommand,
) -> PreviewCacheInvalidationResponse {
    let artifact_schema_version = payload.artifact_schema_version;
    let generator_version = payload.generator_version.clone();
    let mut request = PreviewInvalidationRequest::new(
        payload.changed_ranges,
        payload.changed_material_ids,
        payload.changed_graph_node_ids,
        if payload.changed_domains.is_empty() {
            vec![draft_model::DirtyDomain::PreviewCache]
        } else {
            payload.changed_domains
        },
        payload.reason,
    );
    request.runtime_capability_fingerprint = payload.runtime_capability_fingerprint;
    request.output_profile_fingerprint = payload.output_profile_fingerprint;
    request.full_draft = payload.full_draft;

    let entries = payload
        .entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| cache_entry_ref(index, entry))
        .collect::<Vec<_>>();
    let result = invalidate_preview_cache(&entries, &request);

    PreviewCacheInvalidationResponse {
        invalidated_count: u32::try_from(result.invalidated.len()).unwrap_or(u32::MAX),
        retained_count: u32::try_from(result.retained.len()).unwrap_or(u32::MAX),
        status: PreviewStatus::Invalidated,
        dirty_ranges: request.dirty_ranges,
        changed_material_ids: request.changed_material_ids,
        changed_graph_node_ids: request.changed_graph_node_keys,
        changed_domains: request.changed_domains,
        runtime_capability_fingerprint: request.runtime_capability_fingerprint,
        output_profile_fingerprint: request.output_profile_fingerprint,
        full_draft: request.full_draft,
        reason: request.reason,
        artifact_schema_version,
        generator_version,
    }
}

#[derive(Clone)]
struct SchedulerExportEntry {
    status: ExportJobStatusResponse,
    export_job_id: JobId,
    export_cancel_token: CancelToken,
    export_task_token: TaskCancellationToken,
    validation_job_id: Option<JobId>,
    validation_task_token: Option<TaskCancellationToken>,
}

#[derive(Clone, Default)]
pub struct SchedulerExportService {
    state: Arc<Mutex<SchedulerExportState>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchedulerExportStatusResponse {
    #[serde(flatten)]
    pub status: ExportJobStatusResponse,
    pub scheduler: SchedulerExportTelemetry,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchedulerExportTelemetry {
    pub job_id: String,
    pub domain: JobDomain,
    pub priority: JobPriority,
    pub resource_class: ResourceClass,
    pub validation_resource_class: ResourceClass,
    pub submitted_count: u64,
    pub admitted_count: u64,
    pub started_count: u64,
    pub completed_count: u64,
    pub rejected_count: u64,
    pub canceled_count: u64,
    pub current_queue_depth: usize,
    pub max_queue_depth: usize,
    pub resource_saturation_count: u64,
    pub queue_latency_us: task_runtime::SchedulerTelemetrySummary,
    pub wait_time_us: task_runtime::SchedulerTelemetrySummary,
    pub run_time_us: task_runtime::SchedulerTelemetrySummary,
    pub job_duration_us: task_runtime::SchedulerTelemetrySummary,
    pub resource_usage: Vec<task_runtime::ResourceUsageSnapshot>,
    pub resource_saturation: Vec<task_runtime::ResourceSaturationSnapshot>,
}

struct SchedulerExportState {
    scheduler: task_runtime::JobScheduler,
    entries: BTreeMap<String, SchedulerExportEntry>,
    pending: BTreeMap<JobId, ScheduledExportWork>,
    started_at: Instant,
    next_token_id: u64,
}

impl Default for SchedulerExportState {
    fn default() -> Self {
        Self {
            scheduler: task_runtime::JobScheduler::new(TaskRuntimeConfig::portable_default()),
            entries: BTreeMap::new(),
            pending: BTreeMap::new(),
            started_at: Instant::now(),
            next_token_id: 1,
        }
    }
}

enum ScheduledExportWork {
    Export {
        prepared: PreparedExportJob,
        runtime: RuntimeConfig,
        validation_executor: BoxedValidationExecutor,
    },
    Validation {
        export_job_id: String,
        runtime: RuntimeConfig,
        output_path: PathBuf,
        validation: OutputValidationExpectation,
        validation_executor: BoxedValidationExecutor,
    },
}

struct BoxedValidationExecutor {
    inner: Box<dyn FfmpegExecutor + Send>,
}

impl BoxedValidationExecutor {
    fn new<E>(executor: E) -> Self
    where
        E: FfmpegExecutor + Send + 'static,
    {
        Self {
            inner: Box::new(executor),
        }
    }
}

impl FfmpegExecutor for BoxedValidationExecutor {
    fn executor_name(&self) -> &'static str {
        self.inner.executor_name()
    }

    fn can_execute(&self, binary: &Path) -> bool {
        self.inner.can_execute(binary)
    }

    fn run_version_probe(&self, binary: &Path) -> std::io::Result<std::process::Output> {
        self.inner.run_version_probe(binary)
    }

    fn run(
        &self,
        binary: &Path,
        args: &[std::ffi::OsString],
    ) -> std::io::Result<std::process::Output> {
        self.inner.run(binary, args)
    }
}

enum StartedExportWork {
    Export {
        job_id: String,
        prepared: PreparedExportJob,
        runtime: RuntimeConfig,
        cancel_token: CancelToken,
        validation_executor: BoxedValidationExecutor,
    },
    Validation {
        validation_job_id: JobId,
        export_job_id: String,
        runtime: RuntimeConfig,
        output_path: PathBuf,
        validation: OutputValidationExpectation,
        validation_executor: BoxedValidationExecutor,
    },
}

impl SchedulerExportService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_export(
        &self,
        runtime: RuntimeConfig,
        payload: StartExportCommandPayload,
    ) -> Result<SchedulerExportStatusResponse, ExportCommandError> {
        self.start_export_with_validation_executor(
            runtime,
            payload,
            DesktopFfmpegExecutor::default(),
        )
    }

    pub fn start_export_with_validation_executor<E>(
        &self,
        runtime: RuntimeConfig,
        payload: StartExportCommandPayload,
        validation_executor: E,
    ) -> Result<SchedulerExportStatusResponse, ExportCommandError>
    where
        E: FfmpegExecutor + Send + 'static,
    {
        let prepared = prepare_export_job(&runtime, payload)?;
        let response_job_id = prepared.job_id.clone();
        let export_job_id = JobId::new(prepared.job_id.clone());
        let export_cancel_token = CancelToken::new();
        let initial_status = ExportJobStatusResponse {
            job_id: prepared.job_id.clone(),
            phase: ExportJobPhase::Queued,
            output_path: prepared.output_path.display().to_string(),
            preset: prepared.preset,
            progress_per_mille: Some(0),
            out_time: Some(Microseconds::ZERO),
            log_summary: Some("导出任务已进入调度器队列".to_owned()),
            validation: None,
            diagnostic: None,
            dirty_facts: prepared.dirty_facts.clone(),
        };

        {
            let mut state = self.state.lock().expect("scheduler export lock");
            let export_task_token = state.next_task_token();
            let submitted_at_us = state.now_us();
            let envelope = JobEnvelope::new(
                export_job_id.clone(),
                JobDomain::Export,
                JobPriority::UserVisible,
                ResourceClass::FfmpegProcess,
                export_task_token.clone(),
                submitted_at_us,
            );
            state.scheduler.submit(envelope).map_err(|error| {
                ExportCommandError::Scheduler(format!("scheduler export queue rejected: {error}"))
            })?;
            state.entries.insert(
                prepared.job_id.clone(),
                SchedulerExportEntry {
                    status: initial_status.clone(),
                    export_job_id: export_job_id.clone(),
                    export_cancel_token: export_cancel_token.clone(),
                    export_task_token,
                    validation_job_id: None,
                    validation_task_token: None,
                },
            );
            state.pending.insert(
                export_job_id,
                ScheduledExportWork::Export {
                    prepared,
                    runtime,
                    validation_executor: BoxedValidationExecutor::new(validation_executor),
                },
            );
        }

        self.start_ready_jobs()?;
        self.status(&response_job_id)
    }

    pub fn status(
        &self,
        job_id: &str,
    ) -> Result<SchedulerExportStatusResponse, ExportCommandError> {
        let state = self.state.lock().expect("scheduler export lock");
        let entry = state.entries.get(job_id).ok_or_else(|| {
            ExportCommandError::UnknownJob(format!("unknown export job id: {job_id}"))
        })?;
        Ok(state.binding_status(entry))
    }

    pub fn cancel(
        &self,
        job_id: &str,
    ) -> Result<SchedulerExportStatusResponse, ExportCommandError> {
        {
            let mut state = self.state.lock().expect("scheduler export lock");
            let now_us = state.now_us();
            let entry = state.entries.get_mut(job_id).ok_or_else(|| {
                ExportCommandError::UnknownJob(format!("unknown export job id: {job_id}"))
            })?;
            entry.export_cancel_token.cancel();
            entry.export_task_token.cancel();
            if let Some(token) = entry.validation_task_token.as_ref() {
                token.cancel();
            }
            let export_job_id = entry.export_job_id.clone();
            let validation_job_id = entry.validation_job_id.clone();
            state.pending.remove(&export_job_id);
            if let Some(validation_job_id) = validation_job_id.as_ref() {
                state.pending.remove(validation_job_id);
            }
            let _ = state.scheduler.cancel_at(&export_job_id, now_us);
            if let Some(validation_job_id) = validation_job_id.as_ref() {
                let _ = state.scheduler.cancel_at(validation_job_id, now_us);
            }
            state.update_status_if_not_terminal(job_id, |status| {
                status.phase = ExportJobPhase::Cancelled;
                status.log_summary = Some("导出任务已取消".to_owned());
                status.diagnostic = Some(ExportDiagnostic {
                    kind: ExportDiagnosticKind::Cancelled,
                    message: "已请求取消导出任务".to_owned(),
                    stdout_summary: None,
                    stderr_summary: None,
                });
            })?;
        }
        self.start_ready_jobs()?;
        self.status(job_id)
    }

    fn start_ready_jobs(&self) -> Result<(), ExportCommandError> {
        let mut started = Vec::new();
        {
            let mut state = self.state.lock().expect("scheduler export lock");
            loop {
                let now_us = state.now_us();
                let Some(envelope) = state.scheduler.start_next(now_us).map_err(|error| {
                    ExportCommandError::Scheduler(format!("scheduler export start failed: {error}"))
                })?
                else {
                    break;
                };
                let Some(work) = state.pending.remove(&envelope.job_id) else {
                    let completed_at_us = state.now_us();
                    let _ = state.scheduler.complete_with_commit(
                        &envelope.job_id,
                        JobResult::new(envelope.job_id.clone(), JobResultKind::Failed),
                        completed_at_us,
                        CompletionFreshness::none(),
                        |_| {},
                    );
                    continue;
                };
                match work {
                    ScheduledExportWork::Export {
                        prepared,
                        runtime,
                        validation_executor,
                    } => {
                        let job_id = prepared.job_id.clone();
                        let cancel_token = state
                            .entries
                            .get(&job_id)
                            .map(|entry| entry.export_cancel_token.clone())
                            .ok_or_else(|| {
                                ExportCommandError::UnknownJob(format!(
                                    "unknown export job id: {job_id}"
                                ))
                            })?;
                        state.update_status_if_not_terminal(&job_id, |status| {
                            status.phase = ExportJobPhase::Running;
                            status.log_summary = Some("导出调度器已启动 FFmpeg 任务".to_owned());
                        })?;
                        started.push(StartedExportWork::Export {
                            job_id,
                            prepared,
                            runtime,
                            cancel_token,
                            validation_executor,
                        });
                    }
                    ScheduledExportWork::Validation {
                        export_job_id,
                        runtime,
                        output_path,
                        validation,
                        validation_executor,
                    } => {
                        state.update_status_if_not_terminal(&export_job_id, |status| {
                            status.phase = ExportJobPhase::Validating;
                            status.progress_per_mille = Some(1000);
                            status.log_summary = Some("导出完成，正在校验输出".to_owned());
                        })?;
                        started.push(StartedExportWork::Validation {
                            validation_job_id: envelope.job_id,
                            export_job_id,
                            runtime,
                            output_path,
                            validation,
                            validation_executor,
                        });
                    }
                }
            }
        }

        for work in started {
            match work {
                StartedExportWork::Export {
                    job_id,
                    prepared,
                    runtime,
                    cancel_token,
                    validation_executor,
                } => {
                    let service = self.clone();
                    thread::Builder::new()
                        .name("task-runtime-export-driver".to_owned())
                        .spawn(move || {
                            service.run_scheduled_export(
                                job_id,
                                prepared,
                                runtime,
                                cancel_token,
                                validation_executor,
                            );
                        })
                        .map_err(|error| {
                            ExportCommandError::Io(format!(
                                "failed to start scheduler export driver: {error}"
                            ))
                        })?;
                }
                StartedExportWork::Validation {
                    validation_job_id,
                    export_job_id,
                    runtime,
                    output_path,
                    validation,
                    validation_executor,
                } => {
                    let service = self.clone();
                    thread::Builder::new()
                        .name("task-runtime-export-validation".to_owned())
                        .spawn(move || {
                            service.run_scheduled_validation(
                                validation_job_id,
                                export_job_id,
                                runtime,
                                output_path,
                                validation,
                                validation_executor,
                            );
                        })
                        .map_err(|error| {
                            ExportCommandError::Io(format!(
                                "failed to start scheduler export validation: {error}"
                            ))
                        })?;
                }
            }
        }

        Ok(())
    }

    fn run_scheduled_export(
        &self,
        job_id: String,
        prepared: PreparedExportJob,
        runtime: RuntimeConfig,
        cancel_token: CancelToken,
        validation_executor: BoxedValidationExecutor,
    ) {
        let runtime_result =
            media_runtime::run_export_job(&prepared.runtime_job, &cancel_token, |event| {
                self.apply_runtime_event(&job_id, event)
            });

        match runtime_result {
            Ok(result) if result.state == FfmpegJobState::Completed => {
                let accepted = self.complete_scheduler_job(
                    &JobId::new(job_id.clone()),
                    JobResult::completed(JobId::new(job_id.clone())),
                );
                if accepted && !cancel_token.is_cancelled() {
                    self.enqueue_validation(prepared, runtime, validation_executor);
                }
            }
            Ok(result) if result.state == FfmpegJobState::Cancelled => {
                let diagnostic = ExportDiagnostic {
                    kind: ExportDiagnosticKind::Cancelled,
                    message: "导出任务已取消".to_owned(),
                    stdout_summary: result.stdout_summary.clone(),
                    stderr_summary: result.stderr_summary.clone(),
                };
                let _ = self.complete_scheduler_job(
                    &JobId::new(job_id.clone()),
                    JobResult::new(JobId::new(job_id.clone()), JobResultKind::Failed),
                );
                let _ = self.update_status_if_not_terminal(&job_id, |status| {
                    status.phase = ExportJobPhase::Cancelled;
                    status.progress_per_mille = result
                        .final_progress
                        .and_then(|progress| progress.progress_per_mille);
                    status.out_time = result
                        .final_progress
                        .map(|progress| Microseconds::new(progress.out_time_microseconds));
                    status.log_summary = bounded_export_log(&result);
                    status.diagnostic = Some(diagnostic);
                });
                let _ = self.start_ready_jobs();
            }
            Ok(result) => {
                let diagnostic = ExportDiagnostic {
                    kind: ExportDiagnosticKind::RuntimeFailed,
                    message: format!("导出任务结束状态异常：{:?}", result.state),
                    stdout_summary: result.stdout_summary.clone(),
                    stderr_summary: result.stderr_summary.clone(),
                };
                let _ = self.complete_scheduler_job(
                    &JobId::new(job_id.clone()),
                    JobResult::new(JobId::new(job_id.clone()), JobResultKind::Failed),
                );
                let _ = self.update_status_if_not_terminal(&job_id, |status| {
                    status.phase = ExportJobPhase::Failed;
                    status.log_summary = bounded_export_log(&result);
                    status.diagnostic = Some(diagnostic);
                });
                let _ = self.start_ready_jobs();
            }
            Err(error) => {
                let diagnostic = export_runtime_diagnostic(&error);
                let _ = self.complete_scheduler_job(
                    &JobId::new(job_id.clone()),
                    JobResult::new(JobId::new(job_id.clone()), JobResultKind::Failed),
                );
                let _ = self.update_status_if_not_terminal(&job_id, |status| {
                    status.phase = ExportJobPhase::Failed;
                    status.log_summary = Some("导出运行失败".to_owned());
                    status.diagnostic = Some(diagnostic);
                });
                let _ = self.start_ready_jobs();
            }
        }
    }

    fn enqueue_validation(
        &self,
        prepared: PreparedExportJob,
        runtime: RuntimeConfig,
        validation_executor: BoxedValidationExecutor,
    ) {
        let export_job_id = prepared.job_id.clone();
        let validation_job_id = JobId::new(format!("{export_job_id}:validation"));
        let output_path = prepared.output_path.clone();
        let validation = prepared.validation.clone();
        {
            let mut state = self.state.lock().expect("scheduler export lock");
            if state.is_terminal(&export_job_id) {
                return;
            }
            let submitted_at_us = state.now_us();
            let token = state.next_task_token();
            let envelope = JobEnvelope::new(
                validation_job_id.clone(),
                JobDomain::Export,
                JobPriority::Background,
                ResourceClass::ValidationProbe,
                token.clone(),
                submitted_at_us,
            );
            if state.scheduler.submit(envelope).is_err() {
                let _ = state.update_status_if_not_terminal(&export_job_id, |status| {
                    status.phase = ExportJobPhase::Failed;
                    status.log_summary = Some("导出输出校验未能进入调度器".to_owned());
                    status.diagnostic = Some(ExportDiagnostic {
                        kind: ExportDiagnosticKind::RuntimeFailed,
                        message: "scheduler export validation queue rejected".to_owned(),
                        stdout_summary: None,
                        stderr_summary: None,
                    });
                });
                return;
            }
            if let Some(entry) = state.entries.get_mut(&export_job_id) {
                entry.validation_job_id = Some(validation_job_id.clone());
                entry.validation_task_token = Some(token);
            }
            state.pending.insert(
                validation_job_id,
                ScheduledExportWork::Validation {
                    export_job_id,
                    runtime,
                    output_path,
                    validation,
                    validation_executor,
                },
            );
        }
        let _ = self.start_ready_jobs();
    }

    fn run_scheduled_validation(
        &self,
        validation_job_id: JobId,
        export_job_id: String,
        runtime: RuntimeConfig,
        output_path: PathBuf,
        validation: OutputValidationExpectation,
        validation_executor: BoxedValidationExecutor,
    ) {
        let result =
            validate_rendered_output(&validation_executor, &runtime, &output_path, &validation);
        match result {
            Ok(report) => {
                let validation = export_validation_report(report);
                let accepted = self.complete_scheduler_job(
                    &validation_job_id,
                    JobResult::completed(validation_job_id.clone()),
                );
                if accepted {
                    let _ = self.update_status_if_not_terminal(&export_job_id, |status| {
                        status.phase = ExportJobPhase::Completed;
                        status.progress_per_mille = Some(1000);
                        status.log_summary = Some("导出完成，输出校验通过".to_owned());
                        status.validation = Some(validation);
                        status.diagnostic = None;
                    });
                }
            }
            Err(error) => {
                let diagnostic = export_validation_diagnostic(&error);
                let accepted = self.complete_scheduler_job(
                    &validation_job_id,
                    JobResult::new(validation_job_id.clone(), JobResultKind::Failed),
                );
                if accepted {
                    let _ = self.update_status_if_not_terminal(&export_job_id, |status| {
                        status.phase = ExportJobPhase::ValidationFailed;
                        status.log_summary = Some("导出完成，但输出校验未通过".to_owned());
                        status.diagnostic = Some(diagnostic);
                    });
                }
            }
        }
        let _ = self.start_ready_jobs();
    }

    fn complete_scheduler_job(&self, job_id: &JobId, result: JobResult) -> bool {
        let mut accepted = false;
        let completion = {
            let mut state = self.state.lock().expect("scheduler export lock");
            let completed_at_us = state.now_us();
            state.scheduler.complete_with_commit(
                job_id,
                result,
                completed_at_us,
                CompletionFreshness::none(),
                |_| accepted = true,
            )
        };
        matches!(completion, Ok(JobCompletion::Accepted { .. })) && accepted
    }

    fn apply_runtime_event(&self, job_id: &str, event: FfmpegJobEvent) {
        match event {
            FfmpegJobEvent::Started { .. } => {
                let _ = self.update_status_if_not_terminal(job_id, |status| {
                    status.phase = ExportJobPhase::Running;
                    status.log_summary = Some("导出运行中".to_owned());
                });
            }
            FfmpegJobEvent::Progress { progress } => {
                let _ = self.update_status_if_not_terminal(job_id, |status| {
                    status.phase = ExportJobPhase::Running;
                    status.progress_per_mille = progress.progress_per_mille;
                    status.out_time = Some(Microseconds::new(progress.out_time_microseconds));
                    status.log_summary = Some(format!(
                        "导出进度 {} / {} 微秒",
                        progress.out_time_microseconds,
                        progress.expected_duration_microseconds.unwrap_or_default()
                    ));
                });
            }
            FfmpegJobEvent::Completed { state } => {
                let _ = self.update_status_if_not_terminal(job_id, |status| {
                    if state == FfmpegJobState::Cancelled {
                        status.phase = ExportJobPhase::Cancelled;
                    }
                });
            }
        }
    }

    fn update_status_if_not_terminal(
        &self,
        job_id: &str,
        update: impl FnOnce(&mut ExportJobStatusResponse),
    ) -> Result<(), ExportCommandError> {
        let mut state = self.state.lock().expect("scheduler export lock");
        state.update_status_if_not_terminal(job_id, update)
    }
}

impl SchedulerExportState {
    fn now_us(&self) -> u64 {
        u64::try_from(self.started_at.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    fn next_task_token(&mut self) -> TaskCancellationToken {
        let token = TaskCancellationToken::new(self.next_token_id);
        self.next_token_id = self.next_token_id.saturating_add(1);
        token
    }

    fn binding_status(&self, entry: &SchedulerExportEntry) -> SchedulerExportStatusResponse {
        SchedulerExportStatusResponse {
            status: entry.status.clone(),
            scheduler: scheduler_export_telemetry(
                entry.export_job_id.as_str(),
                self.scheduler.telemetry_snapshot(),
            ),
        }
    }

    fn update_status_if_not_terminal(
        &mut self,
        job_id: &str,
        update: impl FnOnce(&mut ExportJobStatusResponse),
    ) -> Result<(), ExportCommandError> {
        let entry = self.entries.get_mut(job_id).ok_or_else(|| {
            ExportCommandError::UnknownJob(format!("unknown export job id: {job_id}"))
        })?;
        if !is_terminal_export_phase(entry.status.phase) {
            update(&mut entry.status);
        }
        Ok(())
    }

    fn is_terminal(&self, job_id: &str) -> bool {
        self.entries
            .get(job_id)
            .is_some_and(|entry| is_terminal_export_phase(entry.status.phase))
    }
}

fn scheduler_export_telemetry(
    job_id: &str,
    snapshot: SchedulerTelemetrySnapshot,
) -> SchedulerExportTelemetry {
    SchedulerExportTelemetry {
        job_id: job_id.to_owned(),
        domain: JobDomain::Export,
        priority: JobPriority::UserVisible,
        resource_class: ResourceClass::FfmpegProcess,
        validation_resource_class: ResourceClass::ValidationProbe,
        submitted_count: snapshot.submitted_count,
        admitted_count: snapshot.admitted_count,
        started_count: snapshot.started_count,
        completed_count: snapshot.completed_count,
        rejected_count: snapshot.rejected_count,
        canceled_count: snapshot.canceled_count,
        current_queue_depth: snapshot.current_queue_depth,
        max_queue_depth: snapshot.max_queue_depth,
        resource_saturation_count: snapshot.resource_saturation_count,
        queue_latency_us: snapshot.queue_latency_us,
        wait_time_us: snapshot.wait_time_us,
        run_time_us: snapshot.run_time_us,
        job_duration_us: snapshot.job_duration_us,
        resource_usage: snapshot.resource_usage,
        resource_saturation: snapshot.resource_saturation,
    }
}

fn is_terminal_export_phase(phase: ExportJobPhase) -> bool {
    matches!(
        phase,
        ExportJobPhase::Completed
            | ExportJobPhase::Failed
            | ExportJobPhase::ValidationFailed
            | ExportJobPhase::Cancelled
    )
}

pub fn global_export_registry() -> &'static SchedulerExportService {
    static REGISTRY: OnceLock<SchedulerExportService> = OnceLock::new();
    REGISTRY.get_or_init(SchedulerExportService::new)
}

struct PreparedExportJob {
    job_id: String,
    output_path: PathBuf,
    preset: ExportPreset,
    runtime_job: FfmpegRuntimeJob,
    validation: OutputValidationExpectation,
    dirty_facts: Option<ExportPrepDirtyFacts>,
}

fn prepare_export_job(
    runtime: &RuntimeConfig,
    payload: StartExportCommandPayload,
) -> Result<PreparedExportJob, ExportCommandError> {
    let output_path = validate_output_path(&payload.output_path)?;
    let sidecar_dir = export_sidecar_dir(&output_path);
    let dirty_facts = payload.dirty_facts.clone();
    let draft = payload.draft;
    let engine_profile = EngineProfile::from_draft_canvas(&draft).map_err(|error| {
        ExportCommandError::Engine(format!("export engine profile resolution failed: {error}"))
    })?;
    let normalized = normalize_draft(&draft, &engine_profile).map_err(|error| {
        ExportCommandError::Engine(format!("export engine normalization failed: {error}"))
    })?;
    let target_timerange = draft_export_timerange(&draft, normalized.duration)?;
    let range = resolve_render_range(&normalized, target_timerange.clone()).map_err(|error| {
        ExportCommandError::Engine(format!("export range resolution failed: {error}"))
    })?;
    let graph = build_render_graph(&normalized, &range).map_err(|error| {
        ExportCommandError::RenderGraph(format!("export render graph failed: {error}"))
    })?;
    let output_profile = RenderOutputProfile::export_mp4(
        OutputDimensions::new(engine_profile.canvas_width, engine_profile.canvas_height),
        range.frame_rate.clone(),
        target_timerange,
        export_preset(payload.preset),
    );
    let plan = RenderGraphPlan::new(graph, output_profile).map_err(|error| {
        ExportCommandError::RenderGraph(format!("export output profile failed: {error}"))
    })?;
    let compile_context = CompileContext::new(&output_path, &sidecar_dir)
        .with_capabilities(compiler_capabilities_from_runtime(runtime));
    let ffmpeg_job =
        compile_ffmpeg_job(&plan, &compile_context).map_err(ExportCommandError::Compile)?;
    write_export_sidecars(&ffmpeg_job)?;
    let validation = runtime_validation(&ffmpeg_job.validation);
    let runtime_job = FfmpegRuntimeJob::new(
        ffmpeg_job.job_id.clone(),
        runtime.ffmpeg.path.clone(),
        ffmpeg_job.args,
        output_path.clone(),
    )
    .with_expected_duration_microseconds(ffmpeg_job.validation.expected_duration.get());

    Ok(PreparedExportJob {
        job_id: ffmpeg_job.job_id,
        output_path,
        preset: payload.preset,
        runtime_job,
        validation,
        dirty_facts,
    })
}

pub(crate) fn compiler_capabilities_from_runtime(runtime: &RuntimeConfig) -> CompilerCapabilities {
    let executor = DesktopFfmpegExecutor::default();
    let report = probe_desktop_runtime_capabilities(&executor, runtime).ffmpeg;
    CompilerCapabilities {
        supports_h264_encoder: report.h264_encoder.available,
        supports_aac_encoder: report.aac_encoder.available,
        text: text_capability_from_runtime(&report),
    }
}

fn text_capability_from_runtime(report: &RuntimeCapabilityReport) -> TextRenderCapability {
    TextRenderCapability {
        supports_ass_filter: report.ass_filter.available,
        supports_subtitles_filter: report.subtitles_filter.available,
        env_text_font_path: report
            .font_readiness
            .env_text_font_path
            .as_ref()
            .map(|path| path.display().to_string()),
        available_font_paths: report
            .font_readiness
            .available_font_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        bundled_font_ref: report.font_readiness.bundled_font_ref.clone(),
        bundled_font_family: report.font_readiness.bundled_font_family.clone(),
        bundled_font_path: report
            .font_readiness
            .bundled_font_path
            .as_ref()
            .map(|path| path.display().to_string()),
        bundled_font_license: report.font_readiness.bundled_font_license.clone(),
    }
}

fn frame_response(response: PreviewFrameResponse) -> PreviewArtifactResponse {
    artifact_response(
        response.artifact,
        response.cache_entry.key.target_timerange,
        response.from_cache,
    )
}

fn segment_response(response: PreviewSegmentResponse) -> PreviewArtifactResponse {
    artifact_response(
        response.artifact,
        response.cache_entry.key.target_timerange,
        response.from_cache,
    )
}

fn artifact_response(
    artifact: PreviewArtifact,
    target_timerange: draft_model::TargetTimerange,
    from_cache: bool,
) -> PreviewArtifactResponse {
    PreviewArtifactResponse {
        profile: output_profile(artifact.profile),
        path: artifact.path,
        mime_type: artifact.mime_type,
        status: if from_cache {
            PreviewStatus::Cached
        } else {
            PreviewStatus::Generated
        },
        target_timerange,
        diagnostic: None,
    }
}

fn validate_output_path(value: &str) -> Result<PathBuf, ExportCommandError> {
    if value.trim().is_empty() {
        return Err(ExportCommandError::InvalidOutputPath(
            "export output path must not be empty".to_owned(),
        ));
    }
    let path = PathBuf::from(value);
    if path.extension().and_then(|extension| extension.to_str()) != Some("mp4") {
        return Err(ExportCommandError::InvalidOutputPath(
            "export output path must end with .mp4".to_owned(),
        ));
    }
    if path.parent().is_none() {
        return Err(ExportCommandError::InvalidOutputPath(
            "export output path must include a parent directory".to_owned(),
        ));
    }
    Ok(path)
}

fn draft_export_timerange(
    _draft: &Draft,
    duration: Microseconds,
) -> Result<draft_model::TargetTimerange, ExportCommandError> {
    if duration == Microseconds::ZERO {
        return Err(ExportCommandError::Engine(
            "export draft has no renderable timeline duration".to_owned(),
        ));
    }
    Ok(draft_model::TargetTimerange::new(
        Microseconds::ZERO,
        duration,
    ))
}

fn export_preset(preset: ExportPreset) -> ExportMp4Preset {
    match preset {
        ExportPreset::H264AacBalanced => ExportMp4Preset::h264_aac_balanced(),
        ExportPreset::H264AacDraft => ExportMp4Preset {
            preset_id: "h264-aac-draft".to_owned(),
            container: RenderContainer::Mp4,
            video_codec: RenderVideoCodec::H264,
            audio_codec: RenderAudioCodec::Aac,
            crf: 28,
            audio_bitrate_kbps: 128,
        },
    }
}

fn export_sidecar_dir(output_path: &Path) -> PathBuf {
    output_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".ve-export-sidecars")
}

fn write_export_sidecars(job: &FfmpegJob) -> Result<(), ExportCommandError> {
    for sidecar in &job.sidecars {
        let path = Path::new(&sidecar.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                ExportCommandError::Io(format!("failed to create export sidecar dir: {error}"))
            })?;
        }
        fs::write(path, sidecar.contents.as_bytes()).map_err(|error| {
            ExportCommandError::Io(format!("failed to write export sidecar: {error}"))
        })?;
    }
    Ok(())
}

fn runtime_validation(compile: &CompileValidation) -> OutputValidationExpectation {
    OutputValidationExpectation::new()
        .with_expected_duration_microseconds(compile.expected_duration.get(), 33_334)
        .with_expected_frame_rate(media_runtime::RationalFrameRate {
            numerator: compile.expected_frame_rate.numerator,
            denominator: compile.expected_frame_rate.denominator,
        })
        .with_expected_dimensions(compile.expected_width, compile.expected_height)
        .with_audio_stream(compile.expect_audio_stream)
}

fn export_validation_report(
    report: media_runtime::OutputValidationReport,
) -> ExportValidationReport {
    ExportValidationReport {
        path: report.path.display().to_string(),
        file_size_bytes: report.file_size_bytes,
        duration: report.metadata.duration_microseconds.map(Microseconds::new),
        frame_rate: report
            .metadata
            .frame_rate
            .map(|frame_rate| draft_model::RationalFrameRate {
                numerator: frame_rate.numerator,
                denominator: frame_rate.denominator,
            }),
        width: report.metadata.width,
        height: report.metadata.height,
        has_audio: report.metadata.has_audio_stream,
    }
}

fn bounded_export_log(result: &FfmpegJobResult) -> Option<String> {
    result
        .stderr_summary
        .clone()
        .or_else(|| result.stdout_summary.clone())
}

fn export_runtime_diagnostic(error: &FfmpegRuntimeError) -> ExportDiagnostic {
    ExportDiagnostic {
        kind: match error.kind {
            media_runtime::FfmpegRuntimeErrorKind::RuntimeUnavailable
            | media_runtime::FfmpegRuntimeErrorKind::ProcessLaunchFailed => {
                ExportDiagnosticKind::RuntimeUnavailable
            }
            media_runtime::FfmpegRuntimeErrorKind::Timeout
            | media_runtime::FfmpegRuntimeErrorKind::NonZeroExit
            | media_runtime::FfmpegRuntimeErrorKind::MissingEncoder
            | media_runtime::FfmpegRuntimeErrorKind::MissingFilter
            | media_runtime::FfmpegRuntimeErrorKind::MalformedProgress => {
                ExportDiagnosticKind::RuntimeFailed
            }
        },
        message: error.message.clone(),
        stdout_summary: error.stdout_summary.clone(),
        stderr_summary: error.stderr_summary.clone(),
    }
}

fn export_validation_diagnostic(error: &OutputValidationError) -> ExportDiagnostic {
    ExportDiagnostic {
        kind: ExportDiagnosticKind::ValidationFailed,
        message: error.message.clone(),
        stdout_summary: error.stdout_summary.clone(),
        stderr_summary: error.stderr_summary.clone(),
    }
}

pub fn export_error_diagnostic(error: &ExportCommandError) -> ExportDiagnostic {
    match error {
        ExportCommandError::InvalidOutputPath(message) => ExportDiagnostic {
            kind: ExportDiagnosticKind::InvalidOutputPath,
            message: message.clone(),
            stdout_summary: None,
            stderr_summary: None,
        },
        ExportCommandError::Engine(message) => ExportDiagnostic {
            kind: ExportDiagnosticKind::EngineFailed,
            message: message.clone(),
            stdout_summary: None,
            stderr_summary: None,
        },
        ExportCommandError::RenderGraph(message) => ExportDiagnostic {
            kind: ExportDiagnosticKind::RenderGraphFailed,
            message: message.clone(),
            stdout_summary: None,
            stderr_summary: None,
        },
        ExportCommandError::Compile(error) => ExportDiagnostic {
            kind: ExportDiagnosticKind::CompileFailed,
            message: error.message.clone(),
            stdout_summary: None,
            stderr_summary: None,
        },
        ExportCommandError::Runtime(error) => export_runtime_diagnostic(error),
        ExportCommandError::Validation(error) => export_validation_diagnostic(error),
        ExportCommandError::Scheduler(message)
        | ExportCommandError::UnknownJob(message)
        | ExportCommandError::Io(message) => ExportDiagnostic {
            kind: ExportDiagnosticKind::RuntimeFailed,
            message: message.clone(),
            stdout_summary: None,
            stderr_summary: None,
        },
    }
}

fn cache_entry_ref(index: usize, entry: PreviewCacheEntryRef) -> PreviewCacheEntry {
    let profile = cache_profile(entry.profile);
    PreviewCacheEntry {
        key: PreviewCacheKey {
            key_id: format!("binding-entry-{index}"),
            profile,
            target_timerange: entry.target_timerange,
            graph_node_keys: entry.graph_node_ids,
            semantic_fingerprint: entry
                .semantic_fingerprint
                .unwrap_or_else(|| "binding-provided".to_owned()),
            input_fingerprint: entry.input_fingerprint.unwrap_or_default(),
            output_profile_fingerprint: entry.output_profile_fingerprint.unwrap_or_default(),
            runtime_capability_fingerprint: entry
                .runtime_capability_fingerprint
                .unwrap_or_default(),
            material_dependencies: entry.material_dependencies,
            artifact_schema_version: entry.artifact_schema_version,
            generator_version: entry.generator_version,
        },
        artifact: PreviewArtifact {
            profile,
            path: entry.artifact_path,
            mime_type: profile.mime_type().to_owned(),
        },
    }
}

pub fn preview_diagnostic(error: &PreviewServiceError) -> PreviewDiagnostic {
    PreviewDiagnostic {
        kind: match error.kind {
            PreviewServiceErrorKind::EngineFailed => PreviewDiagnosticKind::EngineFailed,
            PreviewServiceErrorKind::RenderGraphFailed => PreviewDiagnosticKind::RenderGraphFailed,
            PreviewServiceErrorKind::CompileFailed => PreviewDiagnosticKind::CompileFailed,
            PreviewServiceErrorKind::IoFailed => PreviewDiagnosticKind::IoFailed,
            PreviewServiceErrorKind::RuntimeUnavailable => {
                PreviewDiagnosticKind::RuntimeUnavailable
            }
            PreviewServiceErrorKind::RuntimeFailed => PreviewDiagnosticKind::RuntimeFailed,
        },
        message: error.message.clone(),
        stdout_summary: error.stdout_summary.clone(),
        stderr_summary: error.stderr_summary.clone(),
    }
}

fn cache_profile(profile: PreviewOutputProfile) -> PreviewCacheProfile {
    match profile {
        PreviewOutputProfile::FramePng => PreviewCacheProfile::FramePng,
        PreviewOutputProfile::SegmentMp4 => PreviewCacheProfile::SegmentMp4,
    }
}

fn output_profile(profile: PreviewCacheProfile) -> PreviewOutputProfile {
    match profile {
        PreviewCacheProfile::FramePng => PreviewOutputProfile::FramePng,
        PreviewCacheProfile::SegmentMp4 => PreviewOutputProfile::SegmentMp4,
    }
}
