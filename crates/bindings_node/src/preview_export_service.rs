use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use draft_model::{
    DecodedPreviewFrameResponse, Draft, ExportDiagnostic, ExportDiagnosticKind, ExportJobPhase,
    ExportJobStatusResponse, ExportPrepDirtyFacts, ExportPreset, ExportValidationReport,
    InvalidatePreviewCacheCommandPayload, Material, MaterialKind, Microseconds,
    PreviewArtifactResponse, PreviewCacheEntryRef, PreviewCacheInvalidationResponse,
    PreviewDecodeDiagnostic, PreviewDecodeRequest, PreviewDiagnostic, PreviewDiagnosticKind,
    PreviewFrameReleaseResponse, PreviewFrameStorageKind, PreviewFrameStoragePreference,
    PreviewOutputProfile, PreviewStatus, ReleasePreviewFrameCommandPayload,
    RequestPreviewFrameCommandPayload, RequestPreviewSegmentCommandPayload,
    RuntimeDecodedFrameHandleMetadata, RuntimeDeviceId, RuntimeFrameDimensions,
    RuntimeMediaIoFallbackReason, RuntimeSelectedDecodePath, RuntimeTextureBackend,
    RuntimeTextureHandleMetadata, RuntimeVideoColorMetadata, RuntimeVideoPixelFormat,
    StartExportCommandPayload,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{
    CompileContext, FfmpegCompileError, FfmpegJob,
    OutputValidationExpectation as CompileValidation, compile_ffmpeg_job,
};
use media_runtime::FfmpegExecutor;
use media_runtime::{
    CancelToken, FfmpegJobEvent, FfmpegJobResult, FfmpegJobState, FfmpegRuntimeError,
    FfmpegRuntimeJob, OutputValidationError, OutputValidationExpectation, RuntimeConfig,
    validate_rendered_output,
};
use media_runtime_desktop::DesktopFfmpegExecutor;
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

#[derive(Debug)]
pub enum PreviewCommandError {
    Service(PreviewServiceError),
    Handle(String),
}

#[derive(Debug)]
pub enum ExportCommandError {
    InvalidOutputPath(String),
    Engine(String),
    RenderGraph(String),
    Compile(FfmpegCompileError),
    Runtime(FfmpegRuntimeError),
    Validation(OutputValidationError),
    UnknownJob(String),
    Io(String),
}

impl fmt::Display for PreviewCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Service(error) => write!(formatter, "preview service failed: {error}"),
            Self::Handle(message) => write!(formatter, "preview service failed: {message}"),
        }
    }
}

impl Error for PreviewCommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Service(error) => Some(error),
            Self::Handle(_) => None,
        }
    }
}

impl From<PreviewServiceError> for PreviewCommandError {
    fn from(error: PreviewServiceError) -> Self {
        Self::Service(error)
    }
}

impl fmt::Display for ExportCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOutputPath(message)
            | Self::Engine(message)
            | Self::RenderGraph(message)
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
    payload: RequestPreviewFrameCommandPayload,
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
    payload: RequestPreviewSegmentCommandPayload,
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
    payload: InvalidatePreviewCacheCommandPayload,
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

#[derive(Debug, Clone)]
struct PreviewFrameHandleEntry {
    frame: RuntimeDecodedFrameHandleMetadata,
}

#[derive(Default)]
struct PreviewFrameHandleRegistryState {
    next_id: u64,
    entries: BTreeMap<String, PreviewFrameHandleEntry>,
}

pub struct PreviewFrameHandleRegistry {
    state: Mutex<PreviewFrameHandleRegistryState>,
}

impl PreviewFrameHandleRegistry {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(PreviewFrameHandleRegistryState {
                next_id: 1,
                entries: BTreeMap::new(),
            }),
        }
    }

    pub fn request_decode(
        &self,
        payload: PreviewDecodeRequest,
    ) -> Result<DecodedPreviewFrameResponse, PreviewCommandError> {
        let material = decode_material(&payload)?;
        let dimensions = decode_dimensions(material)?;
        let color = RuntimeVideoColorMetadata::unknown_with_diagnostic(
            "preview decode binding did not receive source color metadata",
        );
        let decode_path = select_preview_decode_path(&payload);
        let frame_handle_id = self.next_frame_handle_id();
        let frame = RuntimeDecodedFrameHandleMetadata {
            frame_handle_id: frame_handle_id.clone(),
            owner_session: payload.session_id.clone(),
            generation: payload.playback_generation,
            dimensions,
            pixel_format: RuntimeVideoPixelFormat::Nv12,
            color: color.clone(),
        };
        let texture = decode_path.texture_backend.map(|backend| {
            let device_id = payload
                .preview_device
                .clone()
                .unwrap_or_else(|| runtime_device_for_backend(backend));
            RuntimeTextureHandleMetadata {
                texture_handle_id: format!("{frame_handle_id}-texture"),
                owner_session: payload.session_id.clone(),
                generation: payload.playback_generation,
                backend,
                device_id,
                dimensions,
                pixel_format: RuntimeVideoPixelFormat::Nv12,
                color: color.clone(),
            }
        });
        let diagnostic = PreviewDecodeDiagnostic {
            material_id: payload.material_id.clone(),
            selected_path: decode_path.selected_path,
            fallback_reason: decode_path.fallback_reason,
            storage_kind: decode_path.storage_kind,
            texture_compatible: decode_path.texture_compatible,
            preview_device: payload.preview_device.clone(),
            native_device: texture.as_ref().map(|texture| texture.device_id.clone()),
            message: preview_decode_message(
                decode_path.selected_path,
                decode_path.fallback_reason,
                decode_path.storage_kind,
                decode_path.texture_compatible,
            ),
        };
        let response = DecodedPreviewFrameResponse {
            frame: frame.clone(),
            texture,
            storage_kind: decode_path.storage_kind,
            source_time: payload.source_time,
            selected_path: decode_path.selected_path,
            texture_compatible: decode_path.texture_compatible,
            fallback_reason: decode_path.fallback_reason,
            color,
            diagnostics: vec![diagnostic],
        };

        self.state
            .lock()
            .expect("preview frame handle registry lock")
            .entries
            .insert(frame_handle_id, PreviewFrameHandleEntry { frame });

        Ok(response)
    }

    pub fn release(
        &self,
        payload: ReleasePreviewFrameCommandPayload,
    ) -> Result<PreviewFrameReleaseResponse, PreviewCommandError> {
        let mut state = self
            .state
            .lock()
            .expect("preview frame handle registry lock");
        let entry = state.entries.get(&payload.frame_handle_id).ok_or_else(|| {
            PreviewCommandError::Handle(format!(
                "unknown preview frame handle: {}",
                payload.frame_handle_id
            ))
        })?;

        if entry.frame.owner_session != payload.session_id {
            return Err(PreviewCommandError::Handle(format!(
                "preview frame handle {} belongs to session {}, not {}",
                payload.frame_handle_id, entry.frame.owner_session, payload.session_id
            )));
        }
        if entry.frame.generation != payload.playback_generation {
            return Err(PreviewCommandError::Handle(format!(
                "preview frame handle {} generation {} does not match release generation {}",
                payload.frame_handle_id, entry.frame.generation, payload.playback_generation
            )));
        }

        let entry = state
            .entries
            .remove(&payload.frame_handle_id)
            .expect("preview frame handle was just checked");
        Ok(PreviewFrameReleaseResponse {
            frame_handle_id: entry.frame.frame_handle_id,
            owner_session: entry.frame.owner_session,
            generation: entry.frame.generation,
            released: true,
        })
    }

    fn next_frame_handle_id(&self) -> String {
        let mut state = self
            .state
            .lock()
            .expect("preview frame handle registry lock");
        let handle_id = format!("preview-frame-{}", state.next_id);
        state.next_id = state.next_id.saturating_add(1);
        handle_id
    }
}

impl Default for PreviewFrameHandleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn global_preview_frame_handle_registry() -> &'static PreviewFrameHandleRegistry {
    static REGISTRY: OnceLock<PreviewFrameHandleRegistry> = OnceLock::new();
    REGISTRY.get_or_init(PreviewFrameHandleRegistry::new)
}

#[derive(Debug, Clone, Copy)]
struct PreviewDecodePathDecision {
    storage_kind: PreviewFrameStorageKind,
    selected_path: RuntimeSelectedDecodePath,
    fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    texture_compatible: bool,
    texture_backend: Option<RuntimeTextureBackend>,
}

fn decode_material(payload: &PreviewDecodeRequest) -> Result<&Material, PreviewCommandError> {
    let material = payload
        .draft
        .materials
        .iter()
        .find(|material| material.material_id == payload.material_id)
        .ok_or_else(|| {
            PreviewCommandError::Handle(format!(
                "preview decode material {} was not found in draft",
                payload.material_id.as_str()
            ))
        })?;

    if material.kind != MaterialKind::Video && !material.metadata.has_video {
        return Err(PreviewCommandError::Handle(format!(
            "preview decode material {} is not a video material",
            payload.material_id.as_str()
        )));
    }

    Ok(material)
}

fn decode_dimensions(material: &Material) -> Result<RuntimeFrameDimensions, PreviewCommandError> {
    let width = material.metadata.width.ok_or_else(|| {
        PreviewCommandError::Handle(format!(
            "preview decode material {} is missing width metadata",
            material.material_id.as_str()
        ))
    })?;
    let height = material.metadata.height.ok_or_else(|| {
        PreviewCommandError::Handle(format!(
            "preview decode material {} is missing height metadata",
            material.material_id.as_str()
        ))
    })?;

    if width == 0 || height == 0 {
        return Err(PreviewCommandError::Handle(format!(
            "preview decode material {} has invalid dimensions {}x{}",
            material.material_id.as_str(),
            width,
            height
        )));
    }

    Ok(RuntimeFrameDimensions { width, height })
}

fn select_preview_decode_path(payload: &PreviewDecodeRequest) -> PreviewDecodePathDecision {
    match payload.preferred_storage {
        PreviewFrameStoragePreference::Texture => match payload.preview_device.as_ref() {
            Some(device) => PreviewDecodePathDecision {
                storage_kind: PreviewFrameStorageKind::Texture,
                selected_path: RuntimeSelectedDecodePath::NativeHardwareTexture,
                fallback_reason: None,
                texture_compatible: true,
                texture_backend: Some(device.backend),
            },
            None => PreviewDecodePathDecision {
                storage_kind: PreviewFrameStorageKind::Cpu,
                selected_path: RuntimeSelectedDecodePath::FfmpegCpuFrame,
                fallback_reason: Some(RuntimeMediaIoFallbackReason::TextureInteropUnavailable),
                texture_compatible: false,
                texture_backend: None,
            },
        },
        PreviewFrameStoragePreference::Cpu => PreviewDecodePathDecision {
            storage_kind: PreviewFrameStorageKind::Cpu,
            selected_path: RuntimeSelectedDecodePath::NativeSoftwareCpuFrame,
            fallback_reason: None,
            texture_compatible: false,
            texture_backend: None,
        },
        PreviewFrameStoragePreference::Any => match payload.preview_device.as_ref() {
            Some(device) => PreviewDecodePathDecision {
                storage_kind: PreviewFrameStorageKind::Texture,
                selected_path: RuntimeSelectedDecodePath::NativeHardwareTexture,
                fallback_reason: None,
                texture_compatible: true,
                texture_backend: Some(device.backend),
            },
            None => PreviewDecodePathDecision {
                storage_kind: PreviewFrameStorageKind::Cpu,
                selected_path: RuntimeSelectedDecodePath::NativeSoftwareCpuFrame,
                fallback_reason: None,
                texture_compatible: false,
                texture_backend: None,
            },
        },
    }
}

fn runtime_device_for_backend(backend: RuntimeTextureBackend) -> RuntimeDeviceId {
    RuntimeDeviceId {
        backend,
        adapter_id: "unknown-adapter".to_owned(),
        device_id: "unknown-device".to_owned(),
    }
}

fn preview_decode_message(
    selected_path: RuntimeSelectedDecodePath,
    fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    storage_kind: PreviewFrameStorageKind,
    texture_compatible: bool,
) -> String {
    if let Some(reason) = fallback_reason {
        return format!(
            "preview decode selected {selected_path:?} with {reason:?}; storage={storage_kind:?}; textureCompatible={texture_compatible}"
        );
    }
    format!(
        "preview decode selected {selected_path:?}; storage={storage_kind:?}; textureCompatible={texture_compatible}"
    )
}

#[derive(Clone)]
struct ExportJobEntry {
    status: ExportJobStatusResponse,
    cancel_token: CancelToken,
}

#[derive(Clone, Default)]
pub struct ExportJobRegistry {
    entries: Arc<Mutex<BTreeMap<String, ExportJobEntry>>>,
}

impl ExportJobRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_export(
        &self,
        runtime: RuntimeConfig,
        payload: StartExportCommandPayload,
    ) -> Result<ExportJobStatusResponse, ExportCommandError> {
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
    ) -> Result<ExportJobStatusResponse, ExportCommandError>
    where
        E: FfmpegExecutor + Send + 'static,
    {
        let prepared = prepare_export_job(&runtime, payload)?;
        let cancel_token = CancelToken::new();
        let initial_status = ExportJobStatusResponse {
            job_id: prepared.job_id.clone(),
            phase: ExportJobPhase::Running,
            output_path: prepared.output_path.display().to_string(),
            preset: prepared.preset,
            progress_per_mille: Some(0),
            out_time: Some(Microseconds::ZERO),
            log_summary: Some("导出任务已启动".to_owned()),
            validation: None,
            diagnostic: None,
            dirty_facts: prepared.dirty_facts.clone(),
        };

        self.entries.lock().expect("export registry lock").insert(
            prepared.job_id.clone(),
            ExportJobEntry {
                status: initial_status.clone(),
                cancel_token: cancel_token.clone(),
            },
        );

        let registry = self.clone();
        thread::spawn(move || {
            registry.run_export_thread(prepared, runtime, cancel_token, validation_executor);
        });

        Ok(initial_status)
    }

    pub fn status(&self, job_id: &str) -> Result<ExportJobStatusResponse, ExportCommandError> {
        self.entries
            .lock()
            .expect("export registry lock")
            .get(job_id)
            .map(|entry| entry.status.clone())
            .ok_or_else(|| {
                ExportCommandError::UnknownJob(format!("unknown export job id: {job_id}"))
            })
    }

    pub fn cancel(&self, job_id: &str) -> Result<ExportJobStatusResponse, ExportCommandError> {
        let cancel_token = {
            let entries = self.entries.lock().expect("export registry lock");
            entries
                .get(job_id)
                .map(|entry| entry.cancel_token.clone())
                .ok_or_else(|| {
                    ExportCommandError::UnknownJob(format!("unknown export job id: {job_id}"))
                })?
        };
        cancel_token.cancel();
        self.update_status(job_id, |status| {
            if !matches!(
                status.phase,
                ExportJobPhase::Completed
                    | ExportJobPhase::Failed
                    | ExportJobPhase::ValidationFailed
                    | ExportJobPhase::Cancelled
            ) {
                status.phase = ExportJobPhase::Cancelled;
                status.log_summary = Some("正在取消导出".to_owned());
                status.diagnostic = Some(ExportDiagnostic {
                    kind: ExportDiagnosticKind::Cancelled,
                    message: "已请求取消导出任务".to_owned(),
                    stdout_summary: None,
                    stderr_summary: None,
                });
            }
        })?;
        self.status(job_id)
    }

    fn run_export_thread(
        &self,
        prepared: PreparedExportJob,
        runtime: RuntimeConfig,
        cancel_token: CancelToken,
        validation_executor: impl FfmpegExecutor,
    ) {
        let job_id = prepared.job_id.clone();
        let runtime_result =
            media_runtime::run_export_job(&prepared.runtime_job, &cancel_token, |event| {
                self.apply_runtime_event(&job_id, event)
            });

        match runtime_result {
            Ok(result) if result.state == FfmpegJobState::Completed => {
                if cancel_token.is_cancelled() {
                    return;
                }
                self.mark_validating(&job_id, &result);
                match validate_rendered_output(
                    &validation_executor,
                    &runtime,
                    &prepared.output_path,
                    &prepared.validation,
                ) {
                    Ok(report) => {
                        let validation = export_validation_report(report);
                        let _ = self.update_status_if_not_terminal(&job_id, |status| {
                            status.phase = ExportJobPhase::Completed;
                            status.progress_per_mille = Some(1000);
                            status.log_summary = Some("导出完成，输出校验通过".to_owned());
                            status.validation = Some(validation);
                            status.diagnostic = None;
                        });
                    }
                    Err(error) => {
                        let diagnostic = export_validation_diagnostic(&error);
                        let _ = self.update_status_if_not_terminal(&job_id, |status| {
                            status.phase = ExportJobPhase::ValidationFailed;
                            status.log_summary = Some("导出完成，但输出校验未通过".to_owned());
                            status.diagnostic = Some(diagnostic);
                        });
                    }
                }
            }
            Ok(result) if result.state == FfmpegJobState::Cancelled => {
                let diagnostic = ExportDiagnostic {
                    kind: ExportDiagnosticKind::Cancelled,
                    message: "导出任务已取消".to_owned(),
                    stdout_summary: result.stdout_summary.clone(),
                    stderr_summary: result.stderr_summary.clone(),
                };
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
            }
            Ok(result) => {
                let diagnostic = ExportDiagnostic {
                    kind: ExportDiagnosticKind::RuntimeFailed,
                    message: format!("导出任务结束状态异常：{:?}", result.state),
                    stdout_summary: result.stdout_summary.clone(),
                    stderr_summary: result.stderr_summary.clone(),
                };
                let _ = self.update_status_if_not_terminal(&job_id, |status| {
                    status.phase = ExportJobPhase::Failed;
                    status.log_summary = bounded_export_log(&result);
                    status.diagnostic = Some(diagnostic);
                });
            }
            Err(error) => {
                let diagnostic = export_runtime_diagnostic(&error);
                let _ = self.update_status_if_not_terminal(&job_id, |status| {
                    status.phase = ExportJobPhase::Failed;
                    status.log_summary = Some("导出运行失败".to_owned());
                    status.diagnostic = Some(diagnostic);
                });
            }
        }
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

    fn mark_validating(&self, job_id: &str, result: &FfmpegJobResult) {
        let _ = self.update_status_if_not_terminal(job_id, |status| {
            status.phase = ExportJobPhase::Validating;
            status.progress_per_mille = Some(1000);
            status.out_time = result
                .final_progress
                .map(|progress| Microseconds::new(progress.out_time_microseconds));
            status.log_summary = Some("导出完成，正在校验输出".to_owned());
        });
    }

    fn update_status_if_not_terminal(
        &self,
        job_id: &str,
        update: impl FnOnce(&mut ExportJobStatusResponse),
    ) -> Result<(), ExportCommandError> {
        self.update_status(job_id, |status| {
            if is_terminal_export_phase(status.phase) {
                return;
            }
            update(status);
        })
    }

    fn update_status(
        &self,
        job_id: &str,
        update: impl FnOnce(&mut ExportJobStatusResponse),
    ) -> Result<(), ExportCommandError> {
        let mut entries = self.entries.lock().expect("export registry lock");
        let entry = entries.get_mut(job_id).ok_or_else(|| {
            ExportCommandError::UnknownJob(format!("unknown export job id: {job_id}"))
        })?;
        update(&mut entry.status);
        Ok(())
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

pub fn global_export_registry() -> &'static ExportJobRegistry {
    static REGISTRY: OnceLock<ExportJobRegistry> = OnceLock::new();
    REGISTRY.get_or_init(ExportJobRegistry::new)
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
    let compile_context = CompileContext::new(&output_path, &sidecar_dir);
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
        ExportCommandError::UnknownJob(message) | ExportCommandError::Io(message) => {
            ExportDiagnostic {
                kind: ExportDiagnosticKind::RuntimeFailed,
                message: message.clone(),
                stdout_summary: None,
                stderr_summary: None,
            }
        }
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
