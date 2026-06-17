//! Node-API binding boundary for the Rust-owned command contracts.
//!
//! The binding crate intentionally exposes only the Phase 1 surface. Editor
//! semantics remain owned by Rust contract crates and later command crates.

use draft_model::{
    CommandEnvelope, CommandError, CommandErrorKind, CommandName, CommandPayload,
    CommandResultEnvelope, DRAFT_MODEL_VERSION, ImportMaterialCommandPayload,
    ImportMaterialResponse, ListMaterialsCommandPayload, ListMaterialsResponse,
    ListMissingMaterialsCommandPayload, ListMissingMaterialsResponse,
    MissingMaterialCommandDiagnostic, MissingMaterialCommandDiagnosticKind, PingResponse,
    VersionResponse,
};
use media_runtime::{DiscoveryError, discover_runtime_config};
use media_runtime_desktop::DesktopFfmpegExecutor;
use napi::bindgen_prelude::Result;
use napi_derive::napi;
use project_store::{ProjectStoreError, StdPlatformFileSystem};
use std::path::PathBuf;

use crate::material_service::{
    ImportMaterialRequest, MaterialServiceError, MissingMaterialDiagnostic,
    MissingMaterialDiagnosticKind, import_material_and_save, list_materials,
    list_missing_materials,
};

pub mod material_service;

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
                | "addAudioSegment"
                | "setSegmentVolume"
                | "setTrackMute"
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
        | CommandName::AddAudioSegment
        | CommandName::SetSegmentVolume
        | CommandName::SetTrackMute => timeline_command(envelope.command, envelope.payload),
    }
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
