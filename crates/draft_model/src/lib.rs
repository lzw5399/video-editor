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
pub use material::{
    Material, MaterialKind, MaterialMetadata, MaterialStatus, RationalFrameRate, add_material,
    mark_material_available, mark_material_missing, mark_material_probe_failed, upsert_material,
};
pub use time::Microseconds;
pub use timeline::{
    Filter, Keyframe, MainTrackMagnet, Segment, SourceTimerange, TargetTimerange, TextAlignment,
    TextBackground, TextSegment, TextShadow, TextStroke, TextStyle, Track, TrackKind, Transition,
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

/// Command names supported by the Rust contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CommandName {
    Ping,
    Version,
    ProbeMediaRuntime,
    ImportMaterial,
    ListMaterials,
    ListMissingMaterials,
    AddSegment,
    SelectTimelineSegments,
    MoveSegment,
    SplitSegment,
    TrimSegment,
    DeleteSegment,
    UndoTimelineEdit,
    RedoTimelineEdit,
    AddTextSegment,
    EditTextSegment,
}

/// Command payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandPayload {
    Ping(PingCommandPayload),
    Version(VersionCommandPayload),
    ProbeMediaRuntime(ProbeMediaRuntimeCommandPayload),
    ImportMaterial(ImportMaterialCommandPayload),
    ListMaterials(ListMaterialsCommandPayload),
    ListMissingMaterials(ListMissingMaterialsCommandPayload),
    AddSegment(AddSegmentCommandPayload),
    SelectTimelineSegments(SelectTimelineSegmentsCommandPayload),
    MoveSegment(MoveSegmentCommandPayload),
    SplitSegment(SplitSegmentCommandPayload),
    TrimSegment(TrimSegmentCommandPayload),
    DeleteSegment(DeleteSegmentCommandPayload),
    UndoTimelineEdit(UndoTimelineEditCommandPayload),
    RedoTimelineEdit(RedoTimelineEditCommandPayload),
    AddTextSegment(AddTextSegmentCommandPayload),
    EditTextSegment(EditTextSegmentCommandPayload),
}

impl CommandPayload {
    /// Command name that must accompany this payload variant.
    pub fn command_name(&self) -> CommandName {
        match self {
            Self::Ping(_) => CommandName::Ping,
            Self::Version(_) => CommandName::Version,
            Self::ProbeMediaRuntime(_) => CommandName::ProbeMediaRuntime,
            Self::ImportMaterial(_) => CommandName::ImportMaterial,
            Self::ListMaterials(_) => CommandName::ListMaterials,
            Self::ListMissingMaterials(_) => CommandName::ListMissingMaterials,
            Self::AddSegment(_) => CommandName::AddSegment,
            Self::SelectTimelineSegments(_) => CommandName::SelectTimelineSegments,
            Self::MoveSegment(_) => CommandName::MoveSegment,
            Self::SplitSegment(_) => CommandName::SplitSegment,
            Self::TrimSegment(_) => CommandName::TrimSegment,
            Self::DeleteSegment(_) => CommandName::DeleteSegment,
            Self::UndoTimelineEdit(_) => CommandName::UndoTimelineEdit,
            Self::RedoTimelineEdit(_) => CommandName::RedoTimelineEdit,
            Self::AddTextSegment(_) => CommandName::AddTextSegment,
            Self::EditTextSegment(_) => CommandName::EditTextSegment,
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

/// Payload accepted by the Phase 2 material import command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportMaterialCommandPayload {
    pub draft: Draft,
    pub bundle_path: String,
    pub material_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub material_id: Option<MaterialId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub material_kind_hint: Option<MaterialKind>,
}

/// Payload accepted by the Phase 2 material list command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListMaterialsCommandPayload {
    pub draft: Draft,
}

/// Payload accepted by the Phase 2 missing-material diagnostic command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListMissingMaterialsCommandPayload {
    pub draft: Draft,
    pub bundle_path: String,
}

/// Payload accepted by the Phase 3 timeline add command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddSegmentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
}

/// Payload accepted by the Phase 3 timeline selection command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectTimelineSegmentsCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_ids: Vec<SegmentId>,
    pub track_ids: Vec<TrackId>,
}

/// Payload accepted by the Phase 3 timeline move command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveSegmentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
    pub target_track_id: TrackId,
    pub target_start: Microseconds,
}

/// Payload accepted by the Phase 3 timeline split command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SplitSegmentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
    pub right_segment_id: SegmentId,
    pub split_at: Microseconds,
}

/// Trim edge controlled by the Phase 3 timeline trim command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum TrimSegmentDirection {
    Left,
    Right,
}

/// Payload accepted by the Phase 3 timeline trim command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrimSegmentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
    pub direction: TrimSegmentDirection,
    pub target_timerange: TargetTimerange,
}

/// Payload accepted by the Phase 3 timeline delete command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSegmentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
}

/// Payload accepted by the Phase 3 undo command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UndoTimelineEditCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
}

/// Payload accepted by the Phase 3 redo command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RedoTimelineEditCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
}

/// Payload accepted by the Phase 3 text segment add command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddTextSegmentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub text: TextSegment,
}

/// Payload accepted by the Phase 3 text segment edit command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditTextSegmentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
    pub text: TextSegment,
}

/// Segment and track selection returned by Rust-owned timeline commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineSelection {
    pub segment_ids: Vec<SegmentId>,
    pub track_ids: Vec<TrackId>,
}

impl TimelineSelection {
    pub fn empty() -> Self {
        Self {
            segment_ids: Vec::new(),
            track_ids: Vec::new(),
        }
    }
}

/// Deterministic snapping settings passed to Rust timeline semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnappingSettings {
    pub enabled: bool,
    pub threshold: Microseconds,
}

impl SnappingSettings {
    pub const DEFAULT_THRESHOLD: Microseconds = Microseconds(100_000);

    pub fn enabled() -> Self {
        Self {
            enabled: true,
            threshold: Self::DEFAULT_THRESHOLD,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            threshold: Self::DEFAULT_THRESHOLD,
        }
    }
}

/// Session-only command history snapshot; never persisted to `.veproj/project.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandHistorySnapshot {
    pub draft: Draft,
    pub selection: TimelineSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub label: Option<String>,
}

/// Session-only command state passed through Electron as opaque Rust-owned data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandState {
    pub undo_stack: Vec<CommandHistorySnapshot>,
    pub redo_stack: Vec<CommandHistorySnapshot>,
    pub max_history_entries: u32,
    pub snapping: SnappingSettings,
}

impl CommandState {
    pub const DEFAULT_MAX_HISTORY_ENTRIES: u32 = 100;

    pub fn empty() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history_entries: Self::DEFAULT_MAX_HISTORY_ENTRIES,
            snapping: SnappingSettings::enabled(),
        }
    }
}

/// Response returned by Rust-owned timeline command execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineCommandResponse {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub events: Vec<CommandEvent>,
}

/// Response data returned by the Phase 2 material import command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportMaterialResponse {
    pub draft: Draft,
    pub material: Material,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<MissingMaterialCommandDiagnostic>,
    pub bundle_path: String,
    pub project_json_path: String,
}

/// Response data returned by the Phase 2 material list command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListMaterialsResponse {
    pub materials: Vec<Material>,
}

/// Response data returned by the Phase 2 missing-material diagnostic command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListMissingMaterialsResponse {
    pub diagnostics: Vec<MissingMaterialCommandDiagnostic>,
}

/// Binding-safe missing-material diagnostic returned through command envelopes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MissingMaterialCommandDiagnostic {
    pub material_id: MaterialId,
    pub kind: MissingMaterialCommandDiagnosticKind,
    pub original_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub last_known_resolved_path: Option<String>,
    pub status: MaterialStatus,
    pub message: String,
}

/// Stable classes of missing-material diagnostics exposed to Electron.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum MissingMaterialCommandDiagnosticKind {
    MissingFile,
    MarkedMissing,
    ProbeFailed,
    UnresolvedExternalUri,
}

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
    InvalidProject,
    ProjectIoFailed,
    MaterialProbeFailed,
    MissingMaterial,
    InvalidTimelineEdit,
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
