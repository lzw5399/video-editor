//! Rust-owned draft and command contract model.
//!
//! This crate is the pure semantic source of truth for Jianying-aligned editor
//! concepts. Later plans add draft, material, track, segment, timerange,
//! keyframe, filter, and transition schema here before any Electron binding or
//! runtime service consumes them.

use schemars::JsonSchema;
use serde::de;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod draft;
pub mod ids;
pub mod material;
pub mod time;
pub mod timeline;
pub mod validation;

pub use draft::{Draft, DraftMetadata, DraftSchemaVersion};
pub use ids::{DraftId, MaterialId, SegmentId, TrackId};
pub use material::{Material, MaterialKind, MaterialMetadata, MaterialStatus, RationalFrameRate};
pub use time::Microseconds;
pub use timeline::{
    Filter, Keyframe, MainTrackMagnet, Segment, SourceTimerange, TargetTimerange, Track, TrackKind,
    Transition,
};
pub use validation::{DraftValidationError, migrate_draft_json, validate_draft};

/// Current version label for the draft model contract surface.
pub const DRAFT_MODEL_VERSION: &str = "0.1.0";

/// Rust-owned command envelope accepted by the Electron binding boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandEnvelope {
    pub command: CommandName,
    pub payload: CommandPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub request_id: Option<String>,
}

/// Phase 1 command names supported by the Rust contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CommandName {
    Ping,
    Version,
    ProbeMediaRuntime,
}

/// Phase 1 command payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandPayload {
    Ping(PingCommandPayload),
    Version(VersionCommandPayload),
    ProbeMediaRuntime(ProbeMediaRuntimeCommandPayload),
}

impl CommandPayload {
    /// Command name that must accompany this payload variant.
    pub fn command_name(&self) -> CommandName {
        match self {
            Self::Ping(_) => CommandName::Ping,
            Self::Version(_) => CommandName::Version,
            Self::ProbeMediaRuntime(_) => CommandName::ProbeMediaRuntime,
        }
    }
}

impl<'de> Deserialize<'de> for CommandEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct CommandEnvelopeFields {
            command: CommandName,
            payload: CommandPayload,
            #[serde(default)]
            request_id: Option<String>,
        }

        let fields = CommandEnvelopeFields::deserialize(deserializer)?;
        if fields.payload.command_name() != fields.command {
            return Err(de::Error::custom(
                "command name does not match payload kind",
            ));
        }

        Ok(Self {
            command: fields.command,
            payload: fields.payload,
            request_id: fields.request_id,
        })
    }
}

/// Payload accepted by the Phase 1 `ping` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PingCommandPayload {}

/// Payload accepted by the Phase 1 `version` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionCommandPayload {}

/// Payload accepted by the Phase 1 non-editing runtime probe command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeMediaRuntimeCommandPayload {}

/// Standard command result envelope used by binding calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandResultEnvelope<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<CommandError>,
    pub events: Vec<CommandEvent>,
}

/// Structured error payload for failed command execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandError {
    pub kind: CommandErrorKind,
    pub message: String,
    pub command: Option<String>,
}

/// Stable command error kinds shared by the binding boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CommandErrorKind {
    UnsupportedCommand,
    InvalidPayload,
    RuntimeDiscoveryFailed,
    Internal,
}

/// Event emitted with a command result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandEvent {
    pub kind: String,
    pub message: Option<String>,
}

/// Response data returned by the `ping` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PingResponse {
    pub pong: bool,
}

/// Response data returned by the `version` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionResponse {
    pub core_version: String,
    pub contract_version: String,
}
