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

pub mod canvas;
pub mod draft;
pub mod ids;
pub mod material;
pub mod time;
pub mod timeline;
pub mod validation;

pub use canvas::{
    CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground, CanvasBackgroundCapability,
    CanvasPixelPoint, DraftCanvasConfig, NormalizedCanvasPoint, canvas_pixel_to_normalized,
    normalized_to_canvas_pixel, reduce_ratio,
};
pub use draft::{Draft, DraftMetadata, DraftSchemaVersion};
pub use ids::{DraftId, MaterialId, SegmentId, TrackId};
pub use material::{
    Material, MaterialKind, MaterialMetadata, MaterialStatus, RationalFrameRate, add_material,
    mark_material_available, mark_material_missing, mark_material_probe_failed, upsert_material,
};
pub use time::Microseconds;
pub use timeline::{
    Filter, Keyframe, MAX_SEGMENT_ANCHOR_MILLIS, MAX_SEGMENT_CROP_MILLIS,
    MAX_SEGMENT_OPACITY_MILLIS, MAX_SEGMENT_VOLUME_MILLIS, MainTrackMagnet, Segment, SegmentAnchor,
    SegmentBackgroundFilling, SegmentBlendMode, SegmentCrop, SegmentFitMode, SegmentMask,
    SegmentOpacity, SegmentPosition, SegmentRotation, SegmentScale, SegmentTransform,
    SegmentVisual, SegmentVolume, SourceTimerange, TargetTimerange, TextAlignment, TextBackground,
    TextSegment, TextShadow, TextStroke, TextStyle, Track, TrackKind, Transition,
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
    ProbeRuntimeCapabilities,
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
    AddAudioSegment,
    SetSegmentVolume,
    SetTrackMute,
    UpdateDraftCanvasConfig,
    UpdateSegmentVisual,
    RequestPreviewFrame,
    RequestPreviewSegment,
    InvalidatePreviewCache,
    StartExport,
    GetExportJobStatus,
    CancelExport,
}

/// Command payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandPayload {
    Ping(PingCommandPayload),
    Version(VersionCommandPayload),
    ProbeMediaRuntime(ProbeMediaRuntimeCommandPayload),
    ProbeRuntimeCapabilities(ProbeRuntimeCapabilitiesCommandPayload),
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
    AddAudioSegment(AddAudioSegmentCommandPayload),
    SetSegmentVolume(SetSegmentVolumeCommandPayload),
    SetTrackMute(SetTrackMuteCommandPayload),
    UpdateDraftCanvasConfig(UpdateDraftCanvasConfigCommandPayload),
    UpdateSegmentVisual(UpdateSegmentVisualCommandPayload),
    RequestPreviewFrame(RequestPreviewFrameCommandPayload),
    RequestPreviewSegment(RequestPreviewSegmentCommandPayload),
    InvalidatePreviewCache(InvalidatePreviewCacheCommandPayload),
    StartExport(StartExportCommandPayload),
    GetExportJobStatus(GetExportJobStatusCommandPayload),
    CancelExport(CancelExportCommandPayload),
}

impl CommandPayload {
    /// Command name that must accompany this payload variant.
    pub fn command_name(&self) -> CommandName {
        match self {
            Self::Ping(_) => CommandName::Ping,
            Self::Version(_) => CommandName::Version,
            Self::ProbeMediaRuntime(_) => CommandName::ProbeMediaRuntime,
            Self::ProbeRuntimeCapabilities(_) => CommandName::ProbeRuntimeCapabilities,
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
            Self::AddAudioSegment(_) => CommandName::AddAudioSegment,
            Self::SetSegmentVolume(_) => CommandName::SetSegmentVolume,
            Self::SetTrackMute(_) => CommandName::SetTrackMute,
            Self::UpdateDraftCanvasConfig(_) => CommandName::UpdateDraftCanvasConfig,
            Self::UpdateSegmentVisual(_) => CommandName::UpdateSegmentVisual,
            Self::RequestPreviewFrame(_) => CommandName::RequestPreviewFrame,
            Self::RequestPreviewSegment(_) => CommandName::RequestPreviewSegment,
            Self::InvalidatePreviewCache(_) => CommandName::InvalidatePreviewCache,
            Self::StartExport(_) => CommandName::StartExport,
            Self::GetExportJobStatus(_) => CommandName::GetExportJobStatus,
            Self::CancelExport(_) => CommandName::CancelExport,
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

/// Payload accepted by the Phase 6 runtime capability report command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeRuntimeCapabilitiesCommandPayload {}

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

/// Payload accepted by the Phase 3 audio segment add command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddAudioSegmentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
}

/// Payload accepted by the Phase 3 segment volume command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetSegmentVolumeCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
    pub volume: SegmentVolume,
}

/// Payload accepted by the Phase 3 track mute command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetTrackMuteCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_id: TrackId,
    pub muted: bool,
}

/// Payload accepted by the Phase 7 draft canvas config update command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateDraftCanvasConfigCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub canvas_config: DraftCanvasConfig,
}

/// Payload accepted by the Phase 8 segment visual update command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSegmentVisualCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
    pub visual: SegmentVisual,
}

/// Preview artifact profile requested through Rust-owned preview services.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum PreviewOutputProfile {
    FramePng,
    SegmentMp4,
}

/// Stable preview command status returned through command envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum PreviewStatus {
    Generated,
    Cached,
    Invalidated,
}

/// Payload accepted by the Phase 5 preview frame command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestPreviewFrameCommandPayload {
    pub draft: Draft,
    pub cache_root: String,
    pub target_time: Microseconds,
}

/// Payload accepted by the Phase 5 preview segment command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestPreviewSegmentCommandPayload {
    pub draft: Draft,
    pub cache_root: String,
    pub target_timerange: TargetTimerange,
}

/// Renderer-provided reference to an existing derived preview cache entry.
///
/// This intentionally contains no cache-key formula, FFmpeg args, render graph,
/// or derived script content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewCacheEntryRef {
    pub profile: PreviewOutputProfile,
    pub target_timerange: TargetTimerange,
    pub material_dependencies: Vec<MaterialId>,
    pub artifact_path: String,
}

/// Payload accepted by the Phase 5 preview cache invalidation command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvalidatePreviewCacheCommandPayload {
    pub entries: Vec<PreviewCacheEntryRef>,
    pub changed_ranges: Vec<TargetTimerange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub reason: String,
}

/// H.264/AAC export presets exposed through Rust-owned export services.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum ExportPreset {
    H264AacDraft,
    H264AacBalanced,
}

/// Payload accepted by the Phase 5 export start command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartExportCommandPayload {
    pub draft: Draft,
    pub output_path: String,
    pub preset: ExportPreset,
}

/// Payload accepted by the Phase 5 export status command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetExportJobStatusCommandPayload {
    pub job_id: String,
}

/// Payload accepted by the Phase 5 export cancellation command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelExportCommandPayload {
    pub job_id: String,
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
    PreviewServiceFailed,
    ExportServiceFailed,
    Internal,
}

/// Event emitted with a command result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandEvent {
    pub kind: String,
    pub message: Option<String>,
}

/// Stable classes of preview diagnostics returned through command envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum PreviewDiagnosticKind {
    EngineFailed,
    RenderGraphFailed,
    CompileFailed,
    IoFailed,
    RuntimeUnavailable,
    RuntimeFailed,
}

/// Preview diagnostic details suitable for UI display and logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewDiagnostic {
    pub kind: PreviewDiagnosticKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub stdout_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub stderr_summary: Option<String>,
}

/// Preview artifact response returned by frame and segment preview commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewArtifactResponse {
    pub profile: PreviewOutputProfile,
    pub path: String,
    pub mime_type: String,
    pub status: PreviewStatus,
    pub target_timerange: TargetTimerange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<PreviewDiagnostic>,
}

/// Preview cache invalidation response returned by the invalidation command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewCacheInvalidationResponse {
    pub invalidated_count: u32,
    pub retained_count: u32,
    pub status: PreviewStatus,
}

/// Stable export job phase displayed by desktop clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum ExportJobPhase {
    Queued,
    Running,
    Validating,
    Completed,
    Cancelled,
    Failed,
    ValidationFailed,
}

/// Stable classes of export diagnostics returned through command envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum ExportDiagnosticKind {
    InvalidOutputPath,
    EngineFailed,
    RenderGraphFailed,
    CompileFailed,
    RuntimeUnavailable,
    RuntimeFailed,
    Cancelled,
    ValidationFailed,
}

/// Export diagnostic details suitable for UI display and bounded logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportDiagnostic {
    pub kind: ExportDiagnosticKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub stdout_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub stderr_summary: Option<String>,
}

/// Binding-safe validation report for a rendered export output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportValidationReport {
    pub path: String,
    pub file_size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub duration: Option<Microseconds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub frame_rate: Option<RationalFrameRate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub height: Option<u32>,
    pub has_audio: bool,
}

/// Export job state returned by start/status/cancel commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportJobStatusResponse {
    pub job_id: String,
    pub phase: ExportJobPhase,
    pub output_path: String,
    pub preset: ExportPreset,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub progress_per_mille: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub out_time: Option<Microseconds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub log_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub validation: Option<ExportValidationReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<ExportDiagnostic>,
}

/// Stable readiness state for runtime diagnostics displayed by desktop clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeCapabilityStatus {
    Ready,
    Warning,
    Unavailable,
}

/// FFmpeg-family binary kind in runtime capability reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeBinaryKind {
    Ffmpeg,
    Ffprobe,
}

/// Binding-safe FFmpeg/ffprobe binary capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeBinaryCapability {
    pub kind: RuntimeBinaryKind,
    pub path: String,
    pub source: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub configure_summary: Option<String>,
    pub status: RuntimeCapabilityStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<String>,
}

/// Binding-safe feature capability such as H.264/AAC/ASS support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeFeatureCapability {
    pub name: String,
    pub available: bool,
    pub status: RuntimeCapabilityStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<String>,
}

/// Binding-safe deterministic font readiness summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeFontCapability {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub env_text_font_path: Option<String>,
    pub available_font_paths: Vec<String>,
    pub status: RuntimeCapabilityStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<String>,
}

/// Binding-safe FFmpeg runtime redistribution posture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeLicensePosture {
    pub external_runtime: bool,
    pub redistributable_build: bool,
    pub source: String,
    pub message: String,
}

/// Runtime readiness report returned by Rust-owned capability probing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeCapabilityReport {
    pub status: RuntimeCapabilityStatus,
    pub executor_name: String,
    pub ffmpeg: RuntimeBinaryCapability,
    pub ffprobe: RuntimeBinaryCapability,
    pub h264_encoder: RuntimeFeatureCapability,
    pub aac_encoder: RuntimeFeatureCapability,
    pub ass_filter: RuntimeFeatureCapability,
    pub subtitles_filter: RuntimeFeatureCapability,
    pub font_readiness: RuntimeFontCapability,
    pub license_posture: RuntimeLicensePosture,
    pub diagnostics: Vec<String>,
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
