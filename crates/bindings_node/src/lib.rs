//! Node-API binding boundary for the Rust-owned command contracts.
//!
//! The binding crate intentionally exposes only the Phase 1 surface. Editor
//! semantics remain owned by Rust contract crates and later command crates.

use draft_model::{
    CommandEnvelope, CommandError, CommandErrorKind, CommandName, CommandResultEnvelope,
    DRAFT_MODEL_VERSION, PingResponse, VersionResponse,
};
use media_runtime::{DiscoveryError, discover_runtime_config};
use napi::bindgen_prelude::Result;
use napi_derive::napi;

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
        if name != "ping" && name != "version" && name != "probeMediaRuntime" {
            return to_js_value(error_envelope(
                CommandErrorKind::UnsupportedCommand,
                format!("Unsupported Phase 1 command: {name}"),
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

fn raw_command_name(command: &serde_json::Value) -> Option<String> {
    command
        .get("command")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn to_js_value<T: serde::Serialize>(value: CommandResultEnvelope<T>) -> Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|error| napi::Error::from_reason(error.to_string()))
}
