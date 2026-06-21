//! Node-API binding boundary for the Rust-owned command contracts.
//!
//! The binding crate intentionally exposes only the Phase 1 surface. Editor
//! semantics remain owned by Rust contract crates and later command crates.

use draft_model::{
    ArtifactGenerationActionCommandPayload, AudioPreviewCommandPayload, CancelExportCommandPayload,
    CommandEnvelope, CommandError, CommandErrorKind, CommandName, CommandPayload,
    CommandResultEnvelope, DRAFT_MODEL_VERSION, ExportJobStatusResponse,
    GetArtifactQuotaStatusCommandPayload, GetArtifactStatusCommandPayload,
    GetExportJobStatusCommandPayload, ImportMaterialCommandPayload, ImportMaterialResponse,
    InvalidatePreviewCacheCommandPayload, ListMaterialsCommandPayload, ListMaterialsResponse,
    ListMissingMaterialsCommandPayload, ListMissingMaterialsResponse,
    MissingMaterialCommandDiagnostic, MissingMaterialCommandDiagnosticKind,
    OpenProjectBundleCommandPayload, OpenProjectBundleResponse, PingResponse, PreviewDecodeRequest,
    RefreshArtifactStatusCommandPayload, ReleasePreviewFrameCommandPayload,
    RequestPreviewFrameCommandPayload, RequestPreviewSegmentCommandPayload,
    RunArtifactGarbageCollectionCommandPayload, SaveProjectBundleCommandPayload,
    SaveProjectBundleResponse, StartExportCommandPayload, VersionResponse,
};
use media_runtime::{DiscoveryError, RuntimeConfig, discover_runtime_config};
use media_runtime_desktop::DesktopFfmpegExecutor;
use napi::Env;
use napi::bindgen_prelude::Result;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use project_store::{
    ProjectStoreError, ProjectStoreWarning, StdPlatformFileSystem, open_project_bundle,
    save_project_bundle,
};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::artifact_store_service::handle_artifact_store_command;
use crate::audio_service::{AudioPreviewBindingRegistry, handle_audio_service_command};
use crate::material_service::{
    ImportMaterialRequest, MaterialServiceError, MissingMaterialDiagnostic,
    MissingMaterialDiagnosticKind, import_material_and_save, list_materials,
    list_missing_materials,
};
use crate::preview_export_service::{
    ExportCommandError, PreviewCommandError, compiler_capabilities_from_runtime,
    export_error_diagnostic, global_export_registry, global_preview_frame_handle_registry,
    invalidate_preview_cache_command, request_preview_frame_with_executor,
    request_preview_segment_with_executor,
};
use crate::realtime_preview_service::{
    RealtimePreviewBindingRegistry, RealtimePreviewFrameBindingRequest,
    RealtimePreviewSessionBindingConfig, RealtimePreviewSurfaceBindingDescriptor,
    RealtimePreviewSurfaceBoundsBindingRequest,
};
use crate::runtime_capability_service::probe_runtime_capabilities_command;

pub mod artifact_store_service;
pub mod audio_service;
pub mod material_service;
pub mod native_preview_presenter;
pub mod preview_export_service;
pub mod project_session_service;
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
pub fn configure_bundled_runtime_directory(directory: String) {
    media_runtime::configure_bundled_runtime_directory(PathBuf::from(directory));
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
                | "openProjectBundle"
                | "saveProjectBundle"
                | "importMaterial"
                | "listMaterials"
                | "listMissingMaterials"
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
        CommandName::OpenProjectBundle => match envelope.payload {
            CommandPayload::OpenProjectBundle(payload) => open_project_bundle_command(payload),
            _ => unreachable!("command/payload pair was validated during deserialization"),
        },
        CommandName::SaveProjectBundle => match envelope.payload {
            CommandPayload::SaveProjectBundle(payload) => save_project_bundle_command(payload),
            _ => unreachable!("command/payload pair was validated during deserialization"),
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
        CommandName::CreateAudioPreviewSession
        | CommandName::PlayAudioPreview
        | CommandName::PauseAudioPreview
        | CommandName::StopAudioPreview
        | CommandName::SeekAudioPreview
        | CommandName::CancelAudioPreview
        | CommandName::GetAudioPreviewStatus
        | CommandName::ListAudioOutputDevices
        | CommandName::SelectAudioOutputDevice
        | CommandName::GetWaveformDisplayPeaks
        | CommandName::RefreshWaveformStatus => to_js_value(error_envelope(
            CommandErrorKind::UnsupportedCommand,
            "Audio preview commands require explicit native APIs".to_string(),
            Some(format!("{:?}", envelope.command)),
        )),
        CommandName::GetArtifactStatus
        | CommandName::RefreshArtifactStatus
        | CommandName::RetryArtifactGeneration
        | CommandName::ResumeArtifactGeneration
        | CommandName::CancelArtifactGeneration
        | CommandName::GetArtifactQuotaStatus
        | CommandName::RunArtifactGarbageCollection => to_js_value(error_envelope(
            CommandErrorKind::UnsupportedCommand,
            "Artifact controls require explicit native APIs".to_string(),
            Some(format!("{:?}", envelope.command)),
        )),
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
    }
}

#[napi(js_name = "openProjectSession")]
pub fn open_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::open_project_session(request)
}

#[napi(js_name = "createProjectSession")]
pub fn create_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::create_project_session(request)
}

#[napi(js_name = "closeProjectSession")]
pub fn close_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::close_project_session(request)
}

#[napi(js_name = "executeProjectIntent")]
pub fn execute_project_intent(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::execute_project_intent(request)
}

#[napi(js_name = "listProjectSessionMaterials")]
pub fn list_project_session_materials(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::list_project_session_materials(request)
}

#[napi(js_name = "listProjectSessionMissingMaterials")]
pub fn list_project_session_missing_materials(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    project_session_service::list_project_session_missing_materials(request)
}

#[napi(js_name = "startProjectSessionExport")]
pub fn start_project_session_export(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<StartProjectSessionExportRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid startProjectSessionExport payload: {error}"),
                Some("startProjectSessionExport".to_string()),
            ));
        }
    };
    start_project_session_export_command(request)
}

#[napi(js_name = "getExportJobStatus")]
pub fn get_export_job_status(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<GetExportJobStatusCommandPayload>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid getExportJobStatus payload: {error}"),
                Some("getExportJobStatus".to_string()),
            ));
        }
    };
    get_export_job_status_command(request)
}

#[napi(js_name = "cancelExport")]
pub fn cancel_export(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<CancelExportCommandPayload>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid cancelExport payload: {error}"),
                Some("cancelExport".to_string()),
            ));
        }
    };
    cancel_export_command(request)
}

#[napi(js_name = "createAudioPreviewSession")]
pub fn create_audio_preview_session(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(
        CommandName::CreateAudioPreviewSession,
        "createAudioPreviewSession",
        request,
    )
}

#[napi(js_name = "playAudioPreview")]
pub fn play_audio_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(CommandName::PlayAudioPreview, "playAudioPreview", request)
}

#[napi(js_name = "pauseAudioPreview")]
pub fn pause_audio_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(CommandName::PauseAudioPreview, "pauseAudioPreview", request)
}

#[napi(js_name = "stopAudioPreview")]
pub fn stop_audio_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(CommandName::StopAudioPreview, "stopAudioPreview", request)
}

#[napi(js_name = "seekAudioPreview")]
pub fn seek_audio_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(CommandName::SeekAudioPreview, "seekAudioPreview", request)
}

#[napi(js_name = "cancelAudioPreview")]
pub fn cancel_audio_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(
        CommandName::CancelAudioPreview,
        "cancelAudioPreview",
        request,
    )
}

#[napi(js_name = "getAudioPreviewStatus")]
pub fn get_audio_preview_status(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(
        CommandName::GetAudioPreviewStatus,
        "getAudioPreviewStatus",
        request,
    )
}

#[napi(js_name = "listAudioOutputDevices")]
pub fn list_audio_output_devices(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(
        CommandName::ListAudioOutputDevices,
        "listAudioOutputDevices",
        request,
    )
}

#[napi(js_name = "selectAudioOutputDevice")]
pub fn select_audio_output_device(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(
        CommandName::SelectAudioOutputDevice,
        "selectAudioOutputDevice",
        request,
    )
}

#[napi(js_name = "getWaveformDisplayPeaks")]
pub fn get_waveform_display_peaks(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(
        CommandName::GetWaveformDisplayPeaks,
        "getWaveformDisplayPeaks",
        request,
    )
}

#[napi(js_name = "refreshWaveformStatus")]
pub fn refresh_waveform_status(request: serde_json::Value) -> Result<serde_json::Value> {
    audio_preview_binding_command(
        CommandName::RefreshWaveformStatus,
        "refreshWaveformStatus",
        request,
    )
}

#[napi(js_name = "getArtifactStatus")]
pub fn get_artifact_status(request: serde_json::Value) -> Result<serde_json::Value> {
    let payload = match parse_binding_payload::<GetArtifactStatusCommandPayload>(
        "getArtifactStatus",
        request,
    ) {
        Ok(payload) => payload,
        Err(envelope) => return to_js_value(envelope),
    };
    artifact_store_binding_command(
        CommandName::GetArtifactStatus,
        CommandPayload::GetArtifactStatus(payload),
    )
}

#[napi(js_name = "refreshArtifactStatus")]
pub fn refresh_artifact_status(request: serde_json::Value) -> Result<serde_json::Value> {
    let payload = match parse_binding_payload::<RefreshArtifactStatusCommandPayload>(
        "refreshArtifactStatus",
        request,
    ) {
        Ok(payload) => payload,
        Err(envelope) => return to_js_value(envelope),
    };
    artifact_store_binding_command(
        CommandName::RefreshArtifactStatus,
        CommandPayload::RefreshArtifactStatus(payload),
    )
}

#[napi(js_name = "retryArtifactGeneration")]
pub fn retry_artifact_generation(request: serde_json::Value) -> Result<serde_json::Value> {
    let payload = match parse_binding_payload::<ArtifactGenerationActionCommandPayload>(
        "retryArtifactGeneration",
        request,
    ) {
        Ok(payload) => payload,
        Err(envelope) => return to_js_value(envelope),
    };
    artifact_store_binding_command(
        CommandName::RetryArtifactGeneration,
        CommandPayload::RetryArtifactGeneration(payload),
    )
}

#[napi(js_name = "resumeArtifactGeneration")]
pub fn resume_artifact_generation(request: serde_json::Value) -> Result<serde_json::Value> {
    let payload = match parse_binding_payload::<ArtifactGenerationActionCommandPayload>(
        "resumeArtifactGeneration",
        request,
    ) {
        Ok(payload) => payload,
        Err(envelope) => return to_js_value(envelope),
    };
    artifact_store_binding_command(
        CommandName::ResumeArtifactGeneration,
        CommandPayload::ResumeArtifactGeneration(payload),
    )
}

#[napi(js_name = "cancelArtifactGeneration")]
pub fn cancel_artifact_generation(request: serde_json::Value) -> Result<serde_json::Value> {
    let payload = match parse_binding_payload::<ArtifactGenerationActionCommandPayload>(
        "cancelArtifactGeneration",
        request,
    ) {
        Ok(payload) => payload,
        Err(envelope) => return to_js_value(envelope),
    };
    artifact_store_binding_command(
        CommandName::CancelArtifactGeneration,
        CommandPayload::CancelArtifactGeneration(payload),
    )
}

#[napi(js_name = "getArtifactQuotaStatus")]
pub fn get_artifact_quota_status(request: serde_json::Value) -> Result<serde_json::Value> {
    let payload = match parse_binding_payload::<GetArtifactQuotaStatusCommandPayload>(
        "getArtifactQuotaStatus",
        request,
    ) {
        Ok(payload) => payload,
        Err(envelope) => return to_js_value(envelope),
    };
    artifact_store_binding_command(
        CommandName::GetArtifactQuotaStatus,
        CommandPayload::GetArtifactQuotaStatus(payload),
    )
}

#[napi(js_name = "runArtifactGarbageCollection")]
pub fn run_artifact_garbage_collection(request: serde_json::Value) -> Result<serde_json::Value> {
    let payload = match parse_binding_payload::<RunArtifactGarbageCollectionCommandPayload>(
        "runArtifactGarbageCollection",
        request,
    ) {
        Ok(payload) => payload,
        Err(envelope) => return to_js_value(envelope),
    };
    artifact_store_binding_command(
        CommandName::RunArtifactGarbageCollection,
        CommandPayload::RunArtifactGarbageCollection(payload),
    )
}

#[napi(js_name = "requestProjectSessionPreviewFrame")]
pub fn request_project_session_preview_frame(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<RequestProjectSessionPreviewFrameRequest>(request)
    {
        Ok(request) => request,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid requestProjectSessionPreviewFrame payload: {error}"),
                Some("requestProjectSessionPreviewFrame".to_string()),
            ));
        }
    };
    request_project_session_preview_frame_command(request)
}

#[napi(js_name = "requestProjectSessionPreviewSegment")]
pub fn request_project_session_preview_segment(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    let request =
        match serde_json::from_value::<RequestProjectSessionPreviewSegmentRequest>(request) {
            Ok(request) => request,
            Err(error) => {
                return to_js_value(error_envelope(
                    CommandErrorKind::InvalidPayload,
                    format!("Invalid requestProjectSessionPreviewSegment payload: {error}"),
                    Some("requestProjectSessionPreviewSegment".to_string()),
                ));
            }
        };
    request_project_session_preview_segment_command(request)
}

#[napi(js_name = "createRealtimePreviewSession")]
pub fn create_realtime_preview_session(config: serde_json::Value) -> Result<serde_json::Value> {
    let config = parse_realtime_preview_payload::<RealtimePreviewSessionBindingConfig>(config)?;
    with_realtime_preview_registry(|registry| registry.create_session(config))
}

#[napi(js_name = "subscribeRealtimePreviewEvents")]
pub fn subscribe_realtime_preview_events(
    env: Env,
    mut callback: ThreadsafeFunction<String>,
) -> Result<serde_json::Value> {
    #[allow(deprecated)]
    callback.unref(&env)?;
    with_realtime_preview_registry(|registry| registry.subscribe_events(callback))
}

#[napi(js_name = "unsubscribeRealtimePreviewEvents")]
pub fn unsubscribe_realtime_preview_events() -> Result<serde_json::Value> {
    with_realtime_preview_registry(|registry| registry.unsubscribe_events())
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

#[napi(js_name = "updateRealtimePreviewProjectSessionSnapshot")]
pub fn update_realtime_preview_project_session_snapshot(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    let request =
        parse_realtime_preview_payload::<RealtimePreviewProjectSessionSnapshotRequest>(request)?;
    let snapshot = project_session_service::realtime_preview_snapshot(
        &request.project_session_id,
        request.expected_revision,
    )
    .map_err(napi::Error::from_reason)?;
    with_realtime_preview_registry(|registry| {
        registry.update_draft_snapshot(
            &request.session_id,
            snapshot.draft,
            Some(snapshot.bundle_path),
        )
    })
}

#[napi(js_name = "seekRealtimePreview")]
pub fn seek_realtime_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSeekRequest>(request)?;
    with_realtime_preview_registry(|registry| {
        registry.seek(&request.session_id, request.target_time_microseconds)
    })
}

#[napi(js_name = "playRealtimePreview")]
pub fn play_realtime_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSessionRequest>(request)?;
    with_realtime_preview_registry(|registry| registry.play(&request.session_id))
}

#[napi(js_name = "pauseRealtimePreview")]
pub fn pause_realtime_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSessionRequest>(request)?;
    with_realtime_preview_registry(|registry| registry.pause(&request.session_id))
}

#[napi(js_name = "stopRealtimePreview")]
pub fn stop_realtime_preview(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSessionRequest>(request)?;
    with_realtime_preview_registry(|registry| registry.stop(&request.session_id))
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

#[napi(js_name = "getRealtimePreviewPresentationState")]
pub fn get_realtime_preview_presentation_state(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    let request = parse_realtime_preview_payload::<RealtimePreviewSessionRequest>(request)?;
    with_realtime_preview_registry(|registry| registry.presentation_state(&request.session_id))
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

fn open_project_bundle_command(
    payload: OpenProjectBundleCommandPayload,
) -> Result<serde_json::Value> {
    let fs = StdPlatformFileSystem;
    match open_project_bundle(&fs, PathBuf::from(payload.bundle_path)) {
        Ok(opened) => to_js_value(ok_envelope(OpenProjectBundleResponse {
            draft: opened.bundle.draft,
            bundle_path: opened.bundle.bundle_path.display().to_string(),
            project_json_path: opened.bundle.project_json_path.display().to_string(),
            warnings: opened
                .warnings
                .into_iter()
                .map(project_store_warning_message)
                .collect(),
        })),
        Err(error) => to_js_value(project_store_error_envelope("openProjectBundle", error)),
    }
}

fn save_project_bundle_command(
    payload: SaveProjectBundleCommandPayload,
) -> Result<serde_json::Value> {
    let fs = StdPlatformFileSystem;
    match save_project_bundle(&fs, PathBuf::from(payload.bundle_path), &payload.draft) {
        Ok(saved) => to_js_value(ok_envelope(SaveProjectBundleResponse {
            draft: saved.draft,
            bundle_path: saved.bundle_path.display().to_string(),
            project_json_path: saved.project_json_path.display().to_string(),
        })),
        Err(error) => to_js_value(project_store_error_envelope("saveProjectBundle", error)),
    }
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
    let config = preview_service_config_from_preview_payload(
        payload.cache_root.as_deref(),
        payload.bundle_path.as_deref(),
        &runtime,
    );
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
    let config = preview_service_config_from_preview_payload(
        payload.cache_root.as_deref(),
        payload.bundle_path.as_deref(),
        &runtime,
    );
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

fn audio_service_command(
    command: CommandName,
    payload: CommandPayload,
) -> Result<serde_json::Value> {
    to_js_value(with_audio_preview_registry(|registry| {
        handle_audio_service_command(registry, command, payload)
    })?)
}

fn audio_preview_binding_command(
    command: CommandName,
    command_label: &'static str,
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    let payload = match parse_binding_payload::<AudioPreviewCommandPayload>(command_label, request)
    {
        Ok(payload) => payload,
        Err(envelope) => return to_js_value(envelope),
    };
    let command_payload = match command {
        CommandName::CreateAudioPreviewSession => {
            CommandPayload::CreateAudioPreviewSession(payload)
        }
        CommandName::PlayAudioPreview => CommandPayload::PlayAudioPreview(payload),
        CommandName::PauseAudioPreview => CommandPayload::PauseAudioPreview(payload),
        CommandName::StopAudioPreview => CommandPayload::StopAudioPreview(payload),
        CommandName::SeekAudioPreview => CommandPayload::SeekAudioPreview(payload),
        CommandName::CancelAudioPreview => CommandPayload::CancelAudioPreview(payload),
        CommandName::GetAudioPreviewStatus => CommandPayload::GetAudioPreviewStatus(payload),
        CommandName::ListAudioOutputDevices => CommandPayload::ListAudioOutputDevices(payload),
        CommandName::SelectAudioOutputDevice => CommandPayload::SelectAudioOutputDevice(payload),
        CommandName::GetWaveformDisplayPeaks => CommandPayload::GetWaveformDisplayPeaks(payload),
        CommandName::RefreshWaveformStatus => CommandPayload::RefreshWaveformStatus(payload),
        _ => unreachable!("audio preview binding command called with non-audio command"),
    };
    audio_service_command(command, command_payload)
}

fn artifact_store_binding_command(
    command: CommandName,
    payload: CommandPayload,
) -> Result<serde_json::Value> {
    to_js_value(handle_artifact_store_command(command, payload))
}

fn parse_binding_payload<T>(
    command_label: &'static str,
    request: serde_json::Value,
) -> std::result::Result<T, CommandResultEnvelope<serde_json::Value>>
where
    T: serde::de::DeserializeOwned,
{
    match serde_json::from_value::<T>(request) {
        Ok(payload) => Ok(payload),
        Err(error) => Err(error_envelope(
            CommandErrorKind::InvalidPayload,
            format!("Invalid {command_label} payload: {error}"),
            Some(command_label.to_string()),
        )),
    }
}

fn preview_service_config_from_preview_payload(
    cache_root: Option<&str>,
    bundle_path: Option<&str>,
    runtime: &RuntimeConfig,
) -> preview_service::PreviewServiceConfig {
    let fallback_cache_root = cache_root.unwrap_or(".video-editor-preview-cache");
    let config = preview_service::PreviewServiceConfig::new(
        fallback_cache_root,
        runtime.ffmpeg.path.clone(),
    )
    .with_compiler_capabilities(compiler_capabilities_from_runtime(runtime));
    if let Some(bundle_path) = bundle_path {
        config.with_project_artifact_root(bundle_path)
    } else {
        config
    }
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

fn start_project_session_export_command(
    request: StartProjectSessionExportRequest,
) -> Result<serde_json::Value> {
    let snapshot = match project_session_service::project_session_snapshot(
        &request.session_id,
        request.expected_revision,
    ) {
        Ok(snapshot) => snapshot,
        Err(message) => {
            let kind = if message.contains("not found") {
                CommandErrorKind::InvalidProject
            } else {
                CommandErrorKind::InvalidPayload
            };
            return to_js_value(error_envelope(
                kind,
                message,
                Some("startProjectSessionExport".to_string()),
            ));
        }
    };
    let runtime = match discover_runtime_config() {
        Ok(runtime) => runtime,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::ExportServiceFailed,
                runtime_discovery_message(error),
                Some("startProjectSessionExport".to_string()),
            ));
        }
    };
    let payload = StartExportCommandPayload {
        draft: snapshot.draft,
        output_path: request.output_path,
        preset: request.preset,
        dirty_facts: None,
    };
    match global_export_registry().start_export(runtime, payload) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(export_error_envelope("startProjectSessionExport", error)),
    }
}

fn request_project_session_preview_frame_command(
    request: RequestProjectSessionPreviewFrameRequest,
) -> Result<serde_json::Value> {
    let snapshot = match project_session_service::project_session_snapshot(
        &request.session_id,
        request.expected_revision,
    ) {
        Ok(snapshot) => snapshot,
        Err(message) => {
            let kind = if message.contains("not found") {
                CommandErrorKind::InvalidProject
            } else {
                CommandErrorKind::InvalidPayload
            };
            return to_js_value(error_envelope(
                kind,
                message,
                Some("requestProjectSessionPreviewFrame".to_string()),
            ));
        }
    };
    let runtime = match discover_runtime_config() {
        Ok(runtime) => runtime,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::PreviewServiceFailed,
                runtime_discovery_message(error),
                Some("requestProjectSessionPreviewFrame".to_string()),
            ));
        }
    };
    let bundle_path = snapshot.bundle_path.display().to_string();
    let config = preview_service_config_from_preview_payload(None, Some(&bundle_path), &runtime);
    let payload = RequestPreviewFrameCommandPayload {
        draft: snapshot.draft,
        cache_root: None,
        bundle_path: Some(bundle_path),
        target_time: request.target_time,
    };
    let executor = DesktopFfmpegExecutor::default();
    match request_preview_frame_with_executor(&executor, &config, payload) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(preview_error_envelope(
            "requestProjectSessionPreviewFrame",
            error,
        )),
    }
}

fn request_project_session_preview_segment_command(
    request: RequestProjectSessionPreviewSegmentRequest,
) -> Result<serde_json::Value> {
    let snapshot = match project_session_service::project_session_snapshot(
        &request.session_id,
        request.expected_revision,
    ) {
        Ok(snapshot) => snapshot,
        Err(message) => {
            let kind = if message.contains("not found") {
                CommandErrorKind::InvalidProject
            } else {
                CommandErrorKind::InvalidPayload
            };
            return to_js_value(error_envelope(
                kind,
                message,
                Some("requestProjectSessionPreviewSegment".to_string()),
            ));
        }
    };
    let runtime = match discover_runtime_config() {
        Ok(runtime) => runtime,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::PreviewServiceFailed,
                runtime_discovery_message(error),
                Some("requestProjectSessionPreviewSegment".to_string()),
            ));
        }
    };
    let bundle_path = snapshot.bundle_path.display().to_string();
    let config = preview_service_config_from_preview_payload(None, Some(&bundle_path), &runtime);
    let payload = RequestPreviewSegmentCommandPayload {
        draft: snapshot.draft,
        cache_root: None,
        bundle_path: Some(bundle_path),
        target_timerange: request.target_timerange,
    };
    let executor = DesktopFfmpegExecutor::default();
    match request_preview_segment_with_executor(&executor, &config, payload) {
        Ok(response) => to_js_value(ok_envelope(response)),
        Err(error) => to_js_value(preview_error_envelope(
            "requestProjectSessionPreviewSegment",
            error,
        )),
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

fn project_store_error_envelope(
    command: &str,
    error: ProjectStoreError,
) -> CommandResultEnvelope<serde_json::Value> {
    let kind = match &error {
        ProjectStoreError::Io { .. } => CommandErrorKind::ProjectIoFailed,
        _ => CommandErrorKind::InvalidProject,
    };

    error_envelope(kind, error.to_string(), Some(command.to_string()))
}

fn project_store_warning_message(warning: ProjectStoreWarning) -> String {
    match warning {
        ProjectStoreWarning::MissingMaterial {
            material_id,
            uri,
            resolved_path,
        } => {
            let resolved = resolved_path
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unresolved".to_owned());
            format!("missing material {material_id}: {uri} -> {resolved}")
        }
    }
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
            "未找到内置 FFmpeg，请检查应用打包的 runtime/ffmpeg 资源。".to_owned()
        }
        media_runtime::BinaryKind::Ffprobe => {
            "未找到内置 ffprobe，请检查应用打包的 runtime/ffmpeg 资源。".to_owned()
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

fn global_audio_preview_registry() -> &'static Mutex<AudioPreviewBindingRegistry> {
    static REGISTRY: OnceLock<Mutex<AudioPreviewBindingRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(AudioPreviewBindingRegistry::new()))
}

fn with_audio_preview_registry<T>(
    action: impl FnOnce(&mut AudioPreviewBindingRegistry) -> T,
) -> Result<T> {
    let mut registry = global_audio_preview_registry()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio preview registry lock poisoned"))?;
    Ok(action(&mut registry))
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
struct RealtimePreviewProjectSessionSnapshotRequest {
    session_id: String,
    project_session_id: String,
    expected_revision: u64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartProjectSessionExportRequest {
    session_id: String,
    expected_revision: u64,
    output_path: String,
    preset: draft_model::ExportPreset,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RequestProjectSessionPreviewFrameRequest {
    session_id: String,
    expected_revision: u64,
    target_time: draft_model::Microseconds,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RequestProjectSessionPreviewSegmentRequest {
    session_id: String,
    expected_revision: u64,
    target_timerange: draft_model::TargetTimerange,
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
