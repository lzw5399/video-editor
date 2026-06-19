//! Node-API binding boundary for the Rust-owned command contracts.
//!
//! The binding crate intentionally exposes only the Phase 1 surface. Editor
//! semantics remain owned by Rust contract crates and later command crates.

use draft_model::{
    CancelExportCommandPayload, CommandEnvelope, CommandError, CommandErrorKind, CommandName,
    CommandPayload, CommandResultEnvelope, DRAFT_MODEL_VERSION, ExportJobStatusResponse,
    GetExportJobStatusCommandPayload, ImportMaterialCommandPayload, ImportMaterialResponse,
    InvalidatePreviewCacheCommandPayload, ListMaterialsCommandPayload, ListMaterialsResponse,
    ListMissingMaterialsCommandPayload, ListMissingMaterialsResponse,
    MissingMaterialCommandDiagnostic, MissingMaterialCommandDiagnosticKind, PingResponse,
    PreviewDecodeRequest, ReleasePreviewFrameCommandPayload, RequestPreviewFrameCommandPayload,
    RequestPreviewSegmentCommandPayload, StartExportCommandPayload, VersionResponse,
};
use media_runtime::{DiscoveryError, discover_runtime_config};
use media_runtime_desktop::DesktopFfmpegExecutor;
use napi::bindgen_prelude::Result;
use napi_derive::napi;
use project_store::{ProjectStoreError, StdPlatformFileSystem};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::material_service::{
    ImportMaterialRequest, MaterialServiceError, MissingMaterialDiagnostic,
    MissingMaterialDiagnosticKind, import_material_and_save, list_materials,
    list_missing_materials,
};
use crate::preview_export_service::{
    ExportCommandError, PreviewCommandError, export_error_diagnostic, global_export_registry,
    global_preview_frame_handle_registry, invalidate_preview_cache_command,
    request_preview_frame_with_executor, request_preview_segment_with_executor,
};
use crate::realtime_preview_service::{
    RealtimePreviewBindingRegistry, RealtimePreviewFrameBindingRequest,
    RealtimePreviewSessionBindingConfig, RealtimePreviewSurfaceBindingDescriptor,
    RealtimePreviewSurfaceBoundsBindingRequest,
};
use crate::runtime_capability_service::probe_runtime_capabilities_command;

pub mod material_service;
pub mod preview_export_service;
pub mod realtime_preview_service;
pub mod runtime_capability_service;

const BINDING_VERSION: &str = env!("CARGO_PKG_VERSION");

#[napi]
pub fn ping() -> Result<serde_json::Value> {
    to_js_value(ping_envelope())
}

#[napi]
pub fn version() -> Result<serde_json::Value> {
    to_js_value(version_envelope())
}

#[napi]
pub fn execute_command(command: serde_json::Value) -> Result<serde_json::Value> {
    let command_name = raw_command_name(&command);

    if let Some(name) = command_name.as_deref() {
        if !matches!(
            name,
            "ping"
                | "version"
                | "probeMediaRuntime"
                | "probeRuntimeCapabilities"
                | "importMaterial"
                | "listMaterials"
                | "listMissingMaterials"
                | "addSegment"
                | "selectTimelineSegments"
                | "moveSegment"
                | "splitSegment"
                | "trimSegment"
                | "deleteSegment"
                | "undoTimelineEdit"
                | "redoTimelineEdit"
                | "addTextSegment"
                | "editTextSegment"
                | "importSubtitleSrt"
                | "addAudioSegment"
                | "setSegmentVolume"
                | "setTrackMute"
                | "updateDraftCanvasConfig"
                | "updateSegmentVisual"
                | "setSegmentKeyframe"
                | "removeSegmentKeyframe"
                | "requestPreviewDecode"
                | "releasePreviewFrame"
                | "requestPreviewFrame"
                | "requestPreviewSegment"
                | "invalidatePreviewCache"
                | "startExport"
                | "getExportJobStatus"
                | "cancelExport"
        ) {
            return to_js_value(error_envelope(
                CommandErrorKind::UnsupportedCommand,
                format!("Unsupported command: {name}"),
                Some(name.to_string()),
            ));
        }
    }

    let envelope = match serde_json::from_value::<CommandEnvelope>(command) {
        Ok(envelope) => envelope,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid command envelope: {error}"),
                command_name,
            ));
        }
    };

    match envelope.command {
        CommandName::Ping => to_js_value(ping_envelope()),
        CommandName::Version => to_js_value(version_envelope()),
        CommandName::ProbeMediaRuntime => match discover_runtime_config() {
            Ok(config) => to_js_value(ok_envelope(config)),
            Err(error) => to_js_value(runtime_discovery_error_envelope(error)),
        },
        CommandName::ProbeRuntimeCapabilities => match probe_runtime_capabilities_command() {
            Ok(report) => to_js_value(ok_envelope(report)),
            Err(error) => to_js_value(runtime_capability_error_envelope(error)),
        },
        CommandName::ImportMaterial => match envelope.payload {
            CommandPayload::ImportMaterial(payload) => import_material_command(payload),
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::ListMaterials => match envelope.payload {
            CommandPayload::ListMaterials(payload) => list_materials_command(payload),
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::ListMissingMaterials => match envelope.payload {
            CommandPayload::ListMissingMaterials(payload) => {
                list_missing_materials_command(payload)
            }
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::RequestPreviewDecode => match envelope.payload {
            CommandPayload::RequestPreviewDecode(payload) => {
                request_preview_decode_command(payload)
            }
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::ReleasePreviewFrame => match envelope.payload {
            CommandPayload::ReleasePreviewFrame(payload) => release_preview_frame_command(payload),
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::RequestPreviewFrame => match envelope.payload {
            CommandPayload::RequestPreviewFrame(payload) => request_preview_frame_command(payload),
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::RequestPreviewSegment => match envelope.payload {
            CommandPayload::RequestPreviewSegment(payload) => {
                request_preview_segment_command(payload)
            }
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::InvalidatePreviewCache => match envelope.payload {
            CommandPayload::InvalidatePreviewCache(payload) => {
                invalidate_preview_cache_binding_command(payload)
            }
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::StartExport => match envelope.payload {
            CommandPayload::StartExport(payload) => start_export_command(payload),
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::GetExportJobStatus => match envelope.payload {
            CommandPayload::GetExportJobStatus(payload) => get_export_job_status_command(payload),
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::CancelExport => match envelope.payload {
            CommandPayload::CancelExport(payload) => cancel_export_command(payload),
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::AddSegment
        | CommandName::SelectTimelineSegments
        | CommandName::MoveSegment
        | CommandName::SplitSegment
        | CommandName::TrimSegment
        | CommandName::DeleteSegment
        | CommandName::UndoTimelineEdit
        | CommandName::RedoTimelineEdit
        | CommandName::AddTextSegment
        | CommandName::EditTextSegment
        | CommandName::ImportSubtitleSrt
        | CommandName::AddAudioSegment
        | CommandName::SetSegmentVolume
        | CommandName::SetTrackMute
        | CommandName::UpdateDraftCanvasConfig
        | CommandName::UpdateSegmentVisual
        | CommandName::SetSegmentKeyframe
        | CommandName::RemoveSegmentKeyframe => {
            timeline_command(envelope.command, envelope.payload)
        }
    }
}

#[napi(js_name = "createRealtimePreviewSession")]
pub fn create_realtime_preview_session(config: serde_json::Value) -> Result<serde_json::Value> {
    let config = parse_realtime_preview_payload::<RealtimePreviewSessionBindingConfig>(config)?;
    with_realtime_preview_registry(|registry| registry.create_session(config))
}

#[napi(js_name = "closeRealtimePreviewSession")]
pub fn close_realtime_preview_session(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSessionRequest>(request)?;
    with_realtime_preview_registry(|registry| registry.close_session(&request.session_id))
}

#[napi(js_name = "attachRealtimePreviewSurface")]
pub fn attach_realtime_preview_surface(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSurfaceRequest>(request)?;
    with_realtime_preview_registry(|registry| {
        registry.attach_surface(&request.session_id, request.surface)
    })
}

#[napi(js_name = "updateRealtimePreviewSurfaceBounds")]
pub fn update_realtime_preview_surface_bounds(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSurfaceBoundsRequest>(request)?;
    with_realtime_preview_registry(|registry| {
        registry.update_surface_bounds(&request.session_id, request.bounds)
    })
}

#[napi(js_name = "detachRealtimePreviewSurface")]
pub fn detach_realtime_preview_surface(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSessionRequest>(request)?;
    with_realtime_preview_registry(|registry| registry.detach_surface(&request.session_id))
}

#[napi(js_name = "updateRealtimePreviewDraftSnapshot")]
pub fn update_realtime_preview_draft_snapshot(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewDraftSnapshotRequest>(request)?;
    with_realtime_preview_registry(|registry| {
        registry.update_draft_snapshot(&request.session_id, request.draft)
    })
}

#[napi(js_name = "seekRealtimePreview")]
pub fn seek_realtime_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSeekRequest>(request)?;
    with_realtime_preview_registry(|registry| {
        registry.seek(&request.session_id, request.target_time_microseconds)
    })
}

#[napi(js_name = "requestRealtimePreviewFrame")]
pub fn request_realtime_preview_frame(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewFrameRequest>(request)?;
    with_realtime_preview_registry(|registry| {
        registry.request_frame(&request.session_id, request.frame)
    })
}

#[napi(js_name = "nextRealtimePreviewCancellationToken")]
pub fn next_realtime_preview_cancellation_token(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSessionRequest>(request)?;
    with_realtime_preview_registry(|registry| registry.next_cancellation_token(&request.session_id))
}

#[napi(js_name = "cancelRealtimePreviewRequest")]
pub fn cancel_realtime_preview_request(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewCancellationRequest>(request)?;
    with_realtime_preview_registry(|registry| {
        registry.cancel_request(&request.session_id, request.cancellation_token)
    })
}

#[napi(js_name = "getRealtimePreviewTelemetry")]
pub fn get_realtime_preview_telemetry(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSessionRequest>(request)?;
    with_realtime_preview_registry(|registry| registry.telemetry(&request.session_id))
}

fn ping_envelope() -> CommandResultEnvelope<PingResponse> {
    ok_envelope(PingResponse { pong: true })
}

fn version_envelope() -> CommandResultEnvelope<VersionResponse> {
    ok_envelope(VersionResponse {
        core_version: BINDING_VERSION.to_string(),
        contract_version: DRAFT_MODEL_VERSION.to_string(),
    })
}

fn ok_envelope<T>(data: T) -> CommandResultEnvelope<T> {
    CommandResultEnvelope {
        ok: true,
        data: Some(data),
        error: None,
        events: Vec::new(),
    }
}

fn error_envelope(
    kind: CommandErrorKind,
    message: String,
    command: Option<String>,
) -> CommandResultEnvelope<serde_json::Value> {
    CommandResultEnvelope {
        ok: false,
        data: None,
        error: Some(CommandError {
            kind,
            message,
            command,
        }),
        events: Vec::new(),
    }
}

fn runtime_discovery_error_envelope(
    error: DiscoveryError,
) -> CommandResultEnvelope<serde_json::Value> {
    error_envelope(
        CommandErrorKind::RuntimeDiscoveryFailed,
        runtime_discovery_message(error),
        Some("probeMediaRuntime".to_string()),
    )
}

fn runtime_capability_error_envelope(
    error: DiscoveryError,
) -> CommandResultEnvelope<serde_json::Value> {
    error_envelope(
        CommandErrorKind::RuntimeDiscoveryFailed,
        runtime_capability_message(error),
        Some("probeRuntimeCapabilities".to_string()),
    )
}

fn import_material_command(payload: ImportMaterialCommandPayload) -> Result<serde_json::Value> {
    let fs = StdPlatformFileSystem;
    let executor = DesktopFfmpegExecutor::default();
    let runtime = match discover_runtime_config() {
        Ok(runtime) => runtime,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::MaterialProbeFailed,
                runtime_discovery_message(error),
                Some("importMaterial".to_string()),
            ));
        }
    };
    let mut draft = payload.draft;
    let mut request = ImportMaterialRequest::new(PathBuf::from(payload.material_path));
    if let Some(material_id) = payload.material_id {
        request = request.with_material_id(material_id);
    }
    if let Some(display_name) = payload.display_name {
        request = request.with_display_name(display_name);
    }
    if let Some(kind) = payload.material_kind_hint {
        request = request.with_material_kind_hint(kind);
    }

    match import_material_and_save(
        &mut draft,
        request,
        &fs,
        &executor,
        &runtime,
        PathBuf::from(payload.bundle_path),
    ) {
        Ok(imported) => to_js_value(ok_envelope(ImportMaterialResponse {
            draft,
            material: imported.material,
            diagnostic: imported.diagnostic.map(command_diagnostic),
            bundle_path: imported.bundle_path.display().to_string(),
            project_json_path: imported.project_json_path.display().to_string(),
        })),
        Err(error) => to_js_value(material_service_error_envelope("importMaterial", error)),
    }
}

fn list_materials_command(payload: ListMaterialsCommandPayload) -> Result<serde_json::Value> {
    to_js_value(ok_envelope(ListMaterialsResponse {
        materials: list_materials(&payload.draft),
    }))
}

fn list_missing_materials_command(
    payload: ListMissingMaterialsCommandPayload,
) -> Result<serde_json::Value> {
    let fs = StdPlatformFileSystem;
    match list_missing_materials(&payload.draft, &fs, PathBuf::from(payload.bundle_path)) {
        Ok(diagnostics) => to_js_value(ok_envelope(ListMissingMaterialsResponse {
            diagnostics: diagnostics.into_iter().map(command_diagnostic).collect(),
        })),
        Err(error) => to_js_value(material_service_error_envelope(
            "listMissingMaterials",
            error,
        )),
    }
}

fn request_preview_decode_command(payload: PreviewDecodeRequest) -> Result<serde_json::Value> {
    match global_preview_frame_handle_registry().request_decode(payload) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(preview_error_envelope("requestPreviewDecode", error)),
    }
}

fn release_preview_frame_command(
    payload: ReleasePreviewFrameCommandPayload,
) -> Result<serde_json::Value> {
    match global_preview_frame_handle_registry().release(payload) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(preview_error_envelope("releasePreviewFrame", error)),
    }
}

fn request_preview_frame_command(
    payload: RequestPreviewFrameCommandPayload,
) -> Result<serde_json::Value> {
    let executor = DesktopFfmpegExecutor::default();
    let runtime = match discover_runtime_config() {
        Ok(runtime) => runtime,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::PreviewServiceFailed,
                runtime_discovery_message(error),
                Some("requestPreviewFrame".to_string()),
            ));
        }
    };
    let config =
        preview_service::PreviewServiceConfig::new(&payload.cache_root, runtime.ffmpeg.path);
    match request_preview_frame_with_executor(&executor, &config, payload) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(preview_error_envelope("requestPreviewFrame", error)),
    }
}

fn request_preview_segment_command(
    payload: RequestPreviewSegmentCommandPayload,
) -> Result<serde_json::Value> {
    let executor = DesktopFfmpegExecutor::default();
    let runtime = match discover_runtime_config() {
        Ok(runtime) => runtime,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::PreviewServiceFailed,
                runtime_discovery_message(error),
                Some("requestPreviewSegment".to_string()),
            ));
        }
    };
    let config =
        preview_service::PreviewServiceConfig::new(&payload.cache_root, runtime.ffmpeg.path);
    match request_preview_segment_with_executor(&executor, &config, payload) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(preview_error_envelope("requestPreviewSegment", error)),
    }
}

fn invalidate_preview_cache_binding_command(
    payload: InvalidatePreviewCacheCommandPayload,
) -> Result<serde_json::Value> {
    to_js_value(ok_envelope(invalidate_preview_cache_command(payload)))
}

fn start_export_command(payload: StartExportCommandPayload) -> Result<serde_json::Value> {
    let runtime = match discover_runtime_config() {
        Ok(runtime) => runtime,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::ExportServiceFailed,
                runtime_discovery_message(error),
                Some("startExport".to_string()),
            ));
        }
    };
    match global_export_registry().start_export(runtime, payload) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(export_error_envelope("startExport", error)),
    }
}

fn get_export_job_status_command(
    payload: GetExportJobStatusCommandPayload,
) -> Result<serde_json::Value> {
    match global_export_registry().status(&payload.job_id) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(export_error_envelope("getExportJobStatus", error)),
    }
}

fn cancel_export_command(payload: CancelExportCommandPayload) -> Result<serde_json::Value> {
    match global_export_registry().cancel(&payload.job_id) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(export_error_envelope("cancelExport", error)),
    }
}

fn timeline_command(command: CommandName, payload: CommandPayload) -> Result<serde_json::Value> {
    let command = command_wire_name(&command);
    match draft_commands::timeline::execute_timeline_edit(payload) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(error_envelope(
            CommandErrorKind::InvalidTimelineEdit,
            error.to_string(),
            command,
        )),
    }
}

fn preview_error_envelope(
    command: &str,
    error: PreviewCommandError,
) -> CommandResultEnvelope<serde_json::Value> {
    error_envelope(
        CommandErrorKind::PreviewServiceFailed,
        error.to_string(),
        Some(command.to_string()),
    )
}

fn export_error_envelope(
    command: &str,
    error: ExportCommandError,
) -> CommandResultEnvelope<serde_json::Value> {
    let diagnostic = export_error_diagnostic(&error);
    CommandResultEnvelope {
        ok: false,
        data: Some(
            serde_json::to_value(ExportJobStatusResponse {
                job_id: "unavailable".to_owned(),
                phase: draft_model::ExportJobPhase::Failed,
                output_path: String::new(),
                preset: draft_model::ExportPreset::H264AacBalanced,
                progress_per_mille: None,
                out_time: None,
                log_summary: None,
                validation: None,
                diagnostic: Some(diagnostic),
                dirty_facts: None,
            })
            .expect("export error status should serialize"),
        ),
        error: Some(CommandError {
            kind: CommandErrorKind::ExportServiceFailed,
            message: error.to_string(),
            command: Some(command.to_string()),
        }),
        events: Vec::new(),
    }
}

fn command_diagnostic(diagnostic: MissingMaterialDiagnostic) -> MissingMaterialCommandDiagnostic {
    MissingMaterialCommandDiagnostic {
        material_id: diagnostic.material_id,
        kind: command_diagnostic_kind(diagnostic.kind),
        original_uri: diagnostic.original_uri,
        last_known_resolved_path: diagnostic
            .last_known_resolved_path
            .map(|path| path.display().to_string()),
        status: diagnostic.status,
        message: diagnostic.message,
    }
}

fn command_diagnostic_kind(
    kind: MissingMaterialDiagnosticKind,
) -> MissingMaterialCommandDiagnosticKind {
    match kind {
        MissingMaterialDiagnosticKind::MissingFile => {
            MissingMaterialCommandDiagnosticKind::MissingFile
        }
        MissingMaterialDiagnosticKind::MarkedMissing => {
            MissingMaterialCommandDiagnosticKind::MarkedMissing
        }
        MissingMaterialDiagnosticKind::ProbeFailed => {
            MissingMaterialCommandDiagnosticKind::ProbeFailed
        }
        MissingMaterialDiagnosticKind::UnresolvedExternalUri => {
            MissingMaterialCommandDiagnosticKind::UnresolvedExternalUri
        }
    }
}

fn material_service_error_envelope(
    command: &str,
    error: MaterialServiceError,
) -> CommandResultEnvelope<serde_json::Value> {
    let kind = match &error {
        MaterialServiceError::ProjectStore(ProjectStoreError::Io { .. }) => {
            CommandErrorKind::ProjectIoFailed
        }
        MaterialServiceError::ProjectStore(_) | MaterialServiceError::Draft(_) => {
            CommandErrorKind::InvalidProject
        }
    };

    error_envelope(kind, error.to_string(), Some(command.to_string()))
}

fn runtime_discovery_message(error: DiscoveryError) -> String {
    let kind = serde_json::to_value(error.kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{:?}", error.kind));
    let checked_paths = error
        .checked_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let mut message = format!(
        "Media runtime discovery failed ({kind}) for {}. {}",
        error.binary.binary_name(),
        error.remediation
    );

    if !checked_paths.is_empty() {
        message.push_str(" Checked paths: ");
        message.push_str(&checked_paths);
        message.push('.');
    }
    if let Some(stdout) = error.stdout_summary {
        message.push_str(" stdout: ");
        message.push_str(&stdout);
    }
    if let Some(stderr) = error.stderr_summary {
        message.push_str(" stderr: ");
        message.push_str(&stderr);
    }

    message
}

fn runtime_capability_message(error: DiscoveryError) -> String {
    let mut message = match error.binary {
        media_runtime::BinaryKind::Ffmpeg => {
            "未找到 FFmpeg，请配置 VE_FFMPEG_PATH 或加入 PATH。".to_owned()
        }
        media_runtime::BinaryKind::Ffprobe => {
            "未找到 ffprobe，请配置 VE_FFPROBE_PATH 或加入 PATH。".to_owned()
        }
    };
    let checked_paths = error
        .checked_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    if !checked_paths.is_empty() {
        message.push_str(" 已检查路径：");
        message.push_str(&checked_paths);
        message.push('。');
    }
    if let Some(stdout) = error.stdout_summary {
        message.push_str(" stdout：");
        message.push_str(&stdout);
    }
    if let Some(stderr) = error.stderr_summary {
        message.push_str(" stderr：");
        message.push_str(&stderr);
    }

    message
}

fn command_wire_name(command: &CommandName) -> Option<String> {
    serde_json::to_value(command)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
}

fn raw_command_name(command: &serde_json::Value) -> Option<String> {
    command
        .get("command")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn to_js_value<T: serde::Serialize>(value: CommandResultEnvelope<T>) -> Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|error| napi::Error::from_reason(error.to_string()))
}

fn global_realtime_preview_registry() -> &'static Mutex<RealtimePreviewBindingRegistry> {
    static REGISTRY: OnceLock<Mutex<RealtimePreviewBindingRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(RealtimePreviewBindingRegistry::new()))
}

fn with_realtime_preview_registry<T: serde::Serialize>(
    action: impl FnOnce(
        &mut RealtimePreviewBindingRegistry,
    )
        -> std::result::Result<T, realtime_preview_service::RealtimePreviewBindingError>,
) -> Result<serde_json::Value> {
    let mut registry = global_realtime_preview_registry()
        .lock()
        .map_err(|_| napi::Error::from_reason("realtime preview registry lock poisoned"))?;
    let value =
        action(&mut registry).map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_value(value).map_err(|error| napi::Error::from_reason(error.to_string()))
}

fn parse_realtime_preview_payload<T: serde::de::DeserializeOwned>(
    payload: serde_json::Value,
) -> Result<T> {
    serde_json::from_value(payload).map_err(|error| {
        napi::Error::from_reason(format!("Invalid realtime preview payload: {error}"))
    })
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RealtimePreviewSessionRequest {
    session_id: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RealtimePreviewSurfaceRequest {
    session_id: String,
    surface: RealtimePreviewSurfaceBindingDescriptor,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RealtimePreviewSurfaceBoundsRequest {
    session_id: String,
    bounds: RealtimePreviewSurfaceBoundsBindingRequest,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RealtimePreviewDraftSnapshotRequest {
    session_id: String,
    draft: draft_model::Draft,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RealtimePreviewSeekRequest {
    session_id: String,
    target_time_microseconds: u64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RealtimePreviewFrameRequest {
    session_id: String,
    frame: RealtimePreviewFrameBindingRequest,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RealtimePreviewCancellationRequest {
    session_id: String,
    cancellation_token: realtime_preview_runtime::PreviewCancellationToken,
}
