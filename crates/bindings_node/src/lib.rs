//! Node-API binding boundary for the Rust-owned command contracts.
//!
//! The binding crate intentionally exposes only the Phase 1 surface. Editor
//! semantics remain owned by Rust contract crates and later command crates.

use draft_model::{
    CommandEnvelope, CommandError, CommandErrorKind, CommandName, CommandResultEnvelope,
    DRAFT_MODEL_VERSION, PingResponse, VersionResponse,
};
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
        if name != "ping" && name != "version" {
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

fn raw_command_name(command: &serde_json::Value) -> Option<String> {
    command
        .get("command")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn to_js_value<T: serde::Serialize>(value: CommandResultEnvelope<T>) -> Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|error| napi::Error::from_reason(error.to_string()))
}
