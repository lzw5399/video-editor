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
pub mod delta;
pub mod draft;
pub mod font_registry;
pub mod ids;
pub mod material;
pub mod time;
pub mod timeline;
pub mod validation;

pub use canvas::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground,
    CanvasBackgroundCapability, CanvasPixelPoint, DraftCanvasConfig, NormalizedCanvasPoint,
    canvas_pixel_to_normalized, normalized_to_canvas_pixel, reduce_ratio,
};
pub use delta::{
    ChangedEntity, CommandDelta, CommandDeltaName, DirtyDomain, DirtyRange, DirtyRangeSource,
    InvalidationScope,
};
pub use draft::{Draft, DraftMetadata, DraftSchemaVersion};
pub use font_registry::{
    BUNDLED_TEXT_FONT_COVERAGE_SAMPLE, BUNDLED_TEXT_FONT_FAMILY, BUNDLED_TEXT_FONT_LICENSE_PATH,
    BUNDLED_TEXT_FONT_LICENSE_SPDX, BUNDLED_TEXT_FONT_REF, BUNDLED_TEXT_FONT_RELATIVE_PATH,
    BUNDLED_TEXT_FONT_STYLE, BUNDLED_TEXT_FONT_WEIGHT, BundledFontRegistryEntry,
    BundledFontValidation, FontRegistryError, bundled_font_registry, bundled_text_font,
    bundled_text_font_path, repository_root_from_manifest, resolve_bundled_font,
    validate_bundled_font_registry,
};
pub use ids::{DraftId, MaterialId, SegmentId, TrackId};
pub use material::{
    Material, MaterialKind, MaterialMetadata, MaterialStatus, RationalFrameRate, add_material,
    mark_material_available, mark_material_missing, mark_material_probe_failed, upsert_material,
};
pub use time::Microseconds;
pub use timeline::{
    AudioEffectSlot, AudioEffectSlotKind, AudioFade, AudioPanBalance, Filter, Keyframe,
    KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue,
    MAX_AUDIO_FADE_DURATION_MICROSECONDS, MAX_AUDIO_PAN_BALANCE_MILLIS, MAX_SEGMENT_ANCHOR_MILLIS,
    MAX_SEGMENT_CROP_MILLIS, MAX_SEGMENT_OPACITY_MILLIS, MAX_SEGMENT_VOLUME_MILLIS,
    MAX_TEXT_LAYOUT_MILLIS, MAX_TEXT_LETTER_SPACING_MILLIS, MAX_TEXT_LINE_HEIGHT_MILLIS,
    MIN_AUDIO_PAN_BALANCE_MILLIS, MIN_TEXT_LINE_HEIGHT_MILLIS, MainTrackMagnet, Segment,
    SegmentAnchor, SegmentAudio, SegmentBackgroundFilling, SegmentBlendMode, SegmentCrop,
    SegmentFitMode, SegmentMask, SegmentOpacity, SegmentPosition, SegmentRotation, SegmentScale,
    SegmentTransform, SegmentVisual, SegmentVolume, SourceTimerange, TargetTimerange,
    TextAlignment, TextBackground, TextBox, TextBubbleRef, TextEffectRef, TextFont,
    TextLayoutRegion, TextSegment, TextSegmentSource, TextShadow, TextStroke, TextStyle,
    TextWrapping, Track, TrackKind, Transition,
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
    OpenProjectBundle,
    SaveProjectBundle,
    ImportMaterial,
    ListMaterials,
    ListMissingMaterials,
    RequestPreviewDecode,
    ReleasePreviewFrame,
    RequestPreviewFrame,
    RequestPreviewSegment,
    InvalidatePreviewCache,
    CreateAudioPreviewSession,
    PlayAudioPreview,
    PauseAudioPreview,
    StopAudioPreview,
    SeekAudioPreview,
    CancelAudioPreview,
    GetAudioPreviewStatus,
    ListAudioOutputDevices,
    SelectAudioOutputDevice,
    GetWaveformDisplayPeaks,
    RefreshWaveformStatus,
    GetArtifactStatus,
    RefreshArtifactStatus,
    RetryArtifactGeneration,
    ResumeArtifactGeneration,
    CancelArtifactGeneration,
    GetArtifactQuotaStatus,
    RunArtifactGarbageCollection,
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
    OpenProjectBundle(OpenProjectBundleCommandPayload),
    SaveProjectBundle(SaveProjectBundleCommandPayload),
    ImportMaterial(ImportMaterialCommandPayload),
    ListMaterials(ListMaterialsCommandPayload),
    ListMissingMaterials(ListMissingMaterialsCommandPayload),
    RequestPreviewDecode(PreviewDecodeRequest),
    ReleasePreviewFrame(ReleasePreviewFrameCommandPayload),
    RequestPreviewFrame(RequestPreviewFrameCommandPayload),
    RequestPreviewSegment(RequestPreviewSegmentCommandPayload),
    InvalidatePreviewCache(InvalidatePreviewCacheCommandPayload),
    CreateAudioPreviewSession(AudioPreviewCommandPayload),
    PlayAudioPreview(AudioPreviewCommandPayload),
    PauseAudioPreview(AudioPreviewCommandPayload),
    StopAudioPreview(AudioPreviewCommandPayload),
    SeekAudioPreview(AudioPreviewCommandPayload),
    CancelAudioPreview(AudioPreviewCommandPayload),
    GetAudioPreviewStatus(AudioPreviewCommandPayload),
    ListAudioOutputDevices(AudioPreviewCommandPayload),
    SelectAudioOutputDevice(AudioPreviewCommandPayload),
    GetWaveformDisplayPeaks(AudioPreviewCommandPayload),
    RefreshWaveformStatus(AudioPreviewCommandPayload),
    GetArtifactStatus(GetArtifactStatusCommandPayload),
    RefreshArtifactStatus(RefreshArtifactStatusCommandPayload),
    RetryArtifactGeneration(ArtifactGenerationActionCommandPayload),
    ResumeArtifactGeneration(ArtifactGenerationActionCommandPayload),
    CancelArtifactGeneration(ArtifactGenerationActionCommandPayload),
    GetArtifactQuotaStatus(GetArtifactQuotaStatusCommandPayload),
    RunArtifactGarbageCollection(RunArtifactGarbageCollectionCommandPayload),
    StartExport(StartExportCommandPayload),
    GetExportJobStatus(GetExportJobStatusCommandPayload),
    CancelExport(CancelExportCommandPayload),
}

/// Rust-internal timeline edit payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineEditPayload {
    AddSegment(AddSegmentCommandPayload),
    AddTimelineSegmentIntent(AddTimelineSegmentIntentCommandPayload),
    SelectTimelineSegments(SelectTimelineSegmentsCommandPayload),
    MoveSegment(MoveSegmentCommandPayload),
    MoveSelectedSegmentIntent(MoveSelectedSegmentIntentCommandPayload),
    SplitSegment(SplitSegmentCommandPayload),
    SplitSelectedSegmentIntent(SplitSelectedSegmentIntentCommandPayload),
    TrimSegment(TrimSegmentCommandPayload),
    TrimSelectedSegmentIntent(TrimSelectedSegmentIntentCommandPayload),
    DeleteSegment(DeleteSegmentCommandPayload),
    UndoTimelineEdit(UndoTimelineEditCommandPayload),
    RedoTimelineEdit(RedoTimelineEditCommandPayload),
    AddTextSegment(AddTextSegmentCommandPayload),
    AddTextSegmentIntent(AddTextSegmentIntentCommandPayload),
    EditTextSegment(EditTextSegmentCommandPayload),
    ImportSubtitleSrt(ImportSubtitleSrtCommandPayload),
    ImportSubtitleSrtIntent(ImportSubtitleSrtIntentCommandPayload),
    AddAudioSegment(AddAudioSegmentCommandPayload),
    AddAudioSegmentIntent(AddAudioSegmentIntentCommandPayload),
    SetSegmentVolume(SetSegmentVolumeCommandPayload),
    UpdateSegmentAudio(UpdateSegmentAudioCommandPayload),
    AddTrack(AddTrackCommandPayload),
    AddTrackIntent(AddTrackIntentCommandPayload),
    RenameTrack(RenameTrackCommandPayload),
    SetTrackLock(SetTrackLockCommandPayload),
    SetTrackVisibility(SetTrackVisibilityCommandPayload),
    SetTrackMute(SetTrackMuteCommandPayload),
    UpdateDraftCanvasConfig(UpdateDraftCanvasConfigCommandPayload),
    UpdateSegmentVisual(UpdateSegmentVisualCommandPayload),
    SetSegmentKeyframe(SetSegmentKeyframeCommandPayload),
    RemoveSegmentKeyframe(RemoveSegmentKeyframeCommandPayload),
}

impl CommandPayload {
    /// Command name that must accompany this payload variant.
    pub fn command_name(&self) -> CommandName {
        match self {
            Self::Ping(_) => CommandName::Ping,
            Self::Version(_) => CommandName::Version,
            Self::ProbeMediaRuntime(_) => CommandName::ProbeMediaRuntime,
            Self::ProbeRuntimeCapabilities(_) => CommandName::ProbeRuntimeCapabilities,
            Self::OpenProjectBundle(_) => CommandName::OpenProjectBundle,
            Self::SaveProjectBundle(_) => CommandName::SaveProjectBundle,
            Self::ImportMaterial(_) => CommandName::ImportMaterial,
            Self::ListMaterials(_) => CommandName::ListMaterials,
            Self::ListMissingMaterials(_) => CommandName::ListMissingMaterials,
            Self::RequestPreviewDecode(_) => CommandName::RequestPreviewDecode,
            Self::ReleasePreviewFrame(_) => CommandName::ReleasePreviewFrame,
            Self::RequestPreviewFrame(_) => CommandName::RequestPreviewFrame,
            Self::RequestPreviewSegment(_) => CommandName::RequestPreviewSegment,
            Self::InvalidatePreviewCache(_) => CommandName::InvalidatePreviewCache,
            Self::CreateAudioPreviewSession(_) => CommandName::CreateAudioPreviewSession,
            Self::PlayAudioPreview(_) => CommandName::PlayAudioPreview,
            Self::PauseAudioPreview(_) => CommandName::PauseAudioPreview,
            Self::StopAudioPreview(_) => CommandName::StopAudioPreview,
            Self::SeekAudioPreview(_) => CommandName::SeekAudioPreview,
            Self::CancelAudioPreview(_) => CommandName::CancelAudioPreview,
            Self::GetAudioPreviewStatus(_) => CommandName::GetAudioPreviewStatus,
            Self::ListAudioOutputDevices(_) => CommandName::ListAudioOutputDevices,
            Self::SelectAudioOutputDevice(_) => CommandName::SelectAudioOutputDevice,
            Self::GetWaveformDisplayPeaks(_) => CommandName::GetWaveformDisplayPeaks,
            Self::RefreshWaveformStatus(_) => CommandName::RefreshWaveformStatus,
            Self::GetArtifactStatus(_) => CommandName::GetArtifactStatus,
            Self::RefreshArtifactStatus(_) => CommandName::RefreshArtifactStatus,
            Self::RetryArtifactGeneration(_) => CommandName::RetryArtifactGeneration,
            Self::ResumeArtifactGeneration(_) => CommandName::ResumeArtifactGeneration,
            Self::CancelArtifactGeneration(_) => CommandName::CancelArtifactGeneration,
            Self::GetArtifactQuotaStatus(_) => CommandName::GetArtifactQuotaStatus,
            Self::RunArtifactGarbageCollection(_) => CommandName::RunArtifactGarbageCollection,
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

/// Payload accepted by the project bundle open command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenProjectBundleCommandPayload {
    pub bundle_path: String,
}

/// Payload accepted by the project bundle save command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveProjectBundleCommandPayload {
    pub draft: Draft,
    pub bundle_path: String,
}

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

/// Intent-level command for adding a material at the current timeline context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddTimelineSegmentIntentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub material_id: MaterialId,
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

/// Intent-level command for moving the selected segment by a timeline delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveSelectedSegmentIntentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub delta: Microseconds,
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

/// Intent-level command for splitting the selected segment at a timeline time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SplitSelectedSegmentIntentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
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

/// Intent-level command for trimming the selected segment edge by a delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrimSelectedSegmentIntentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub direction: TrimSegmentDirection,
    pub delta: Microseconds,
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

/// Intent-level command for adding text at the current timeline context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddTextSegmentIntentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub text: TextSegment,
    pub duration: Microseconds,
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

/// Payload accepted by the Phase 9 subtitle SRT import command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportSubtitleSrtCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_id: TrackId,
    pub track_name: String,
    pub srt_content: String,
    pub time_offset: Microseconds,
    pub segment_id_prefix: String,
    pub material_id_prefix: String,
    pub style: TextStyle,
    pub text_box: TextBox,
    pub layout_region: TextLayoutRegion,
    pub wrapping: TextWrapping,
}

/// Intent-level command for importing SRT subtitles at the current timeline context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportSubtitleSrtIntentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub srt_content: String,
    pub time_offset: Microseconds,
    pub style: TextStyle,
    pub text_box: TextBox,
    pub layout_region: TextLayoutRegion,
    pub wrapping: TextWrapping,
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

/// Intent-level command for adding an audio material at the current timeline context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddAudioSegmentIntentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub material_id: Option<MaterialId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub duration: Option<Microseconds>,
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

/// Payload accepted by the Phase 15 segment audio semantic update command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSegmentAudioCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub gain_millis: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub pan_balance_millis: Option<AudioPanBalance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fade_in_duration: Option<AudioFade>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fade_out_duration: Option<AudioFade>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub effect_slots: Option<Vec<AudioEffectSlot>>,
}

/// Payload accepted by the Phase 15.1 track creation command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddTrackCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_id: TrackId,
    pub track_kind: TrackKind,
    pub name: String,
}

/// Intent-level command for adding a track of a requested kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddTrackIntentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_kind: TrackKind,
}

/// Payload accepted by the Phase 15.1 track rename command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameTrackCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_id: TrackId,
    pub name: String,
}

/// Payload accepted by the Phase 15.1 track lock command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetTrackLockCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_id: TrackId,
    pub locked: bool,
}

/// Payload accepted by the Phase 15.1 visual track visibility command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetTrackVisibilityCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub track_id: TrackId,
    pub visible: bool,
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

/// Payload accepted by the Phase 10 segment keyframe set command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetSegmentKeyframeCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
    pub keyframe: Keyframe,
}

/// Payload accepted by the Phase 10 segment keyframe remove command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveSegmentKeyframeCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub segment_id: SegmentId,
    pub property: KeyframeProperty,
    pub at: Microseconds,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub cache_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub bundle_path: Option<String>,
    pub target_time: Microseconds,
}

/// Payload accepted by the Phase 5 preview segment command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestPreviewSegmentCommandPayload {
    pub draft: Draft,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub cache_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub bundle_path: Option<String>,
    pub target_timerange: TargetTimerange,
}

/// Storage preference requested by a preview decode caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum PreviewFrameStoragePreference {
    Any,
    Cpu,
    Texture,
}

/// Payload accepted by the handle-based preview decode command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewDecodeRequest {
    pub session_id: String,
    pub draft: Draft,
    pub material_id: MaterialId,
    pub source_time: Microseconds,
    pub playback_generation: u64,
    pub preferred_storage: PreviewFrameStoragePreference,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub preview_device: Option<RuntimeDeviceId>,
}

/// Payload accepted by the preview frame release command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleasePreviewFrameCommandPayload {
    pub session_id: String,
    pub frame_handle_id: String,
    pub playback_generation: u64,
}

/// Binding-visible storage returned for a decoded preview frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum PreviewFrameStorageKind {
    Cpu,
    Texture,
    PlatformOpaque,
    ArtifactFallback,
}

/// Decode route diagnostic returned with handle-based preview frames.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewDecodeDiagnostic {
    pub material_id: MaterialId,
    pub selected_path: RuntimeSelectedDecodePath,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    pub storage_kind: PreviewFrameStorageKind,
    pub texture_compatible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub preview_device: Option<RuntimeDeviceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub native_device: Option<RuntimeDeviceId>,
    pub message: String,
}

/// Handle-based preview decode response. Frame payloads remain native/Rust-owned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecodedPreviewFrameResponse {
    pub frame: RuntimeDecodedFrameHandleMetadata,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub texture: Option<RuntimeTextureHandleMetadata>,
    pub storage_kind: PreviewFrameStorageKind,
    pub source_time: Microseconds,
    pub selected_path: RuntimeSelectedDecodePath,
    pub texture_compatible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    pub color: RuntimeVideoColorMetadata,
    pub diagnostics: Vec<PreviewDecodeDiagnostic>,
}

/// Response returned when a retained preview frame handle is released.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewFrameReleaseResponse {
    pub frame_handle_id: String,
    pub owner_session: String,
    pub generation: u64,
    pub released: bool,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graph_node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub semantic_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub input_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub output_profile_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub runtime_capability_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub artifact_schema_version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub generator_version: String,
}

/// Payload accepted by the Phase 5 preview cache invalidation command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvalidatePreviewCacheCommandPayload {
    pub entries: Vec<PreviewCacheEntryRef>,
    pub changed_ranges: Vec<DirtyRange>,
    pub changed_material_ids: Vec<MaterialId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_graph_node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_domains: Vec<DirtyDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub runtime_capability_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub output_profile_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub full_draft: bool,
    pub reason: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub artifact_schema_version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub generator_version: String,
}

/// Shared payload for Rust-owned audio preview, output device, and waveform display commands.
///
/// Individual commands consume the subset they need. Optional fields keep the
/// transport stable while command handlers validate required combinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioPreviewCommandPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub draft: Option<Draft>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub material_id: Option<MaterialId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub target_time: Option<Microseconds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub target_timerange: Option<TargetTimerange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub playback_generation: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub device_selection_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub max_peak_bins: Option<u16>,
}

/// Stable audio preview playback status exposed to production UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum AudioPreviewPlaybackStatus {
    Ready,
    Playing,
    Paused,
    Stopped,
    Buffering,
    Seeking,
    Canceled,
    StaleRejected,
    Unavailable,
    Failed,
}

/// Safe audio output device readiness status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum AudioOutputDeviceStatus {
    Ready,
    Degraded,
    Missing,
    Unavailable,
}

/// Safe waveform display readiness status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum WaveformDisplayStatus {
    Ready,
    Pending,
    Missing,
    Failed,
}

/// Device summary safe for renderer display and selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioOutputDeviceSummary {
    pub selection_id: String,
    pub display_name: String,
    pub status: AudioOutputDeviceStatus,
    pub status_label: String,
    pub is_default: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub sample_rate_hz: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub channel_count: Option<u16>,
    pub diagnostics: Vec<String>,
}

/// Audio preview session status safe for binding responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioPreviewStatusResponse {
    pub session_id: String,
    pub generation: u64,
    pub status: AudioPreviewPlaybackStatus,
    pub status_label: String,
    pub target_time: Microseconds,
    pub buffered_until: Microseconds,
    pub device: AudioOutputDeviceSummary,
    pub diagnostics: Vec<String>,
}

/// Generic acknowledgement returned by audio preview transport commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioPreviewCommandResponse {
    pub session_id: String,
    pub generation: u64,
    pub accepted: bool,
    pub status: AudioPreviewPlaybackStatus,
    pub status_label: String,
    pub target_time: Microseconds,
    pub diagnostics: Vec<String>,
}

/// Single display-ready waveform peak bin with bounded integer amplitude units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaveformDisplayPeak {
    pub min_millis: i16,
    pub max_millis: i16,
}

/// Bounded waveform display response. Values are derived display data only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaveformDisplayPeaksResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub material_id: Option<MaterialId>,
    pub status: WaveformDisplayStatus,
    pub status_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub target_timerange: Option<TargetTimerange>,
    pub requested_peak_bins: u16,
    pub returned_peak_bins: u16,
    pub peaks: Vec<WaveformDisplayPeak>,
    pub diagnostics: Vec<String>,
}

/// Common project-scoped artifact status request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetArtifactStatusCommandPayload {
    pub session_id: String,
    pub bundle_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub material_id: Option<MaterialId>,
}

/// Project-scoped artifact refresh request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefreshArtifactStatusCommandPayload {
    pub session_id: String,
    pub bundle_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub material_id: Option<MaterialId>,
}

/// Job action request for retry, resume, and cancellation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactGenerationActionCommandPayload {
    pub session_id: String,
    pub bundle_path: String,
    pub job_id: String,
}

/// Project-scoped quota request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetArtifactQuotaStatusCommandPayload {
    pub session_id: String,
    pub bundle_path: String,
}

/// Project-scoped cache cleanup request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunArtifactGarbageCollectionCommandPayload {
    pub session_id: String,
    pub bundle_path: String,
    pub dry_run: bool,
}

/// Status classes safe for default production UI display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactTaskStatus {
    Waiting,
    Running,
    Ready,
    Dirty,
    Resumable,
    CancelRequested,
    Cancelled,
    Failed,
}

/// Project-relative artifact reference safe to show only where allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisplayableArtifactRef {
    pub label: String,
    pub project_relative_ref: String,
    pub artifact_kind: String,
}

/// Per-material artifact status summary for the resource panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaterialArtifactStatus {
    pub material_id: MaterialId,
    pub material_label: String,
    pub artifact_kind: String,
    pub status: ArtifactTaskStatus,
    pub status_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub progress_per_mille: Option<u16>,
    pub can_refresh: bool,
    pub can_retry: bool,
    pub can_resume: bool,
    pub can_cancel: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub display_ref: Option<DisplayableArtifactRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub error_category: Option<String>,
}

/// Active generation task summary safe for UI transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactGenerationTaskSummary {
    pub job_id: String,
    pub artifact_kind: String,
    pub display_label: String,
    pub status: ArtifactTaskStatus,
    pub status_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub progress_per_mille: Option<u16>,
    pub can_retry: bool,
    pub can_resume: bool,
    pub can_cancel: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub error_category: Option<String>,
}

/// Rust-owned quota status labels and maintenance availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactQuotaStatus {
    pub status_label: String,
    pub severity: String,
    pub used_label: String,
    pub reclaimable_label: String,
    pub released_label: String,
    pub cleanup_available: bool,
}

/// Project artifact status response safe for renderer transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactStatusSummary {
    pub session_id: String,
    pub status_label: String,
    pub materials: Vec<MaterialArtifactStatus>,
    pub tasks: Vec<ArtifactGenerationTaskSummary>,
    pub quota: ArtifactQuotaStatus,
    pub refresh_available: bool,
}

/// Result from artifact maintenance actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactMaintenanceResult {
    pub session_id: String,
    pub status_label: String,
    pub mode: String,
    pub affected_count: u32,
    pub reclaimable_label: String,
    pub released_label: String,
    pub completed: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub dirty_facts: Option<ExportPrepDirtyFacts>,
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
    pub delta: CommandDelta,
}

/// Response data returned by opening a project bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenProjectBundleResponse {
    pub draft: Draft,
    pub bundle_path: String,
    pub project_json_path: String,
    pub warnings: Vec<String>,
}

/// Response data returned by saving a project bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveProjectBundleResponse {
    pub draft: Draft,
    pub bundle_path: String,
    pub project_json_path: String,
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
    ArtifactStoreFailed,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dirty_ranges: Vec<DirtyRange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_material_ids: Vec<MaterialId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_graph_node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_domains: Vec<DirtyDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub runtime_capability_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub output_profile_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub full_draft: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub artifact_schema_version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub generator_version: String,
}

/// Binding-safe export-preparation dirty facts produced by Rust services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportPrepDirtyFacts {
    pub dirty_ranges: Vec<DirtyRange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub changed_graph_node_ids: Vec<String>,
    pub changed_domains: Vec<DirtyDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub runtime_capability_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub output_profile_fingerprint: Option<String>,
    pub full_draft: bool,
    pub reason: String,
    pub artifact_schema_version: u32,
    pub generator_version: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub dirty_facts: Option<ExportPrepDirtyFacts>,
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn is_false(value: &bool) -> bool {
    !*value
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub bundled_font_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub bundled_font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub bundled_font_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub bundled_font_license: Option<String>,
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

/// Binding-safe fallback reason for media IO capability reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeMediaIoFallbackReason {
    UnsupportedCodec,
    UnsupportedPixelFormat,
    HardwareDecodeUnavailable,
    TextureInteropUnavailable,
    DeviceMismatch,
    AllocationFailure,
    PlatformApiFailure,
    FfmpegUnavailable,
    UserDisabledHardwareDecode,
    UnsupportedPlatform,
}

/// Ordered decode route exposed as capability metadata, not renderer policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeSelectedDecodePath {
    NativeHardwareTexture,
    NativeHardwareCpuCopy,
    NativeSoftwareCpuFrame,
    FfmpegCpuFrame,
    FfmpegPreviewArtifact,
}

/// Native texture backend identity. Values are metadata only, not native handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeTextureBackend {
    D3d11Texture2D,
    D3d12Resource,
    MetalTexture,
    CoreVideoPixelBuffer,
}

/// Decoded frame pixel format exposed without frame payload data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeVideoPixelFormat {
    Nv12,
    Bgra8,
    Rgba8,
    P010,
    Yuv420P,
    Unknown,
}

/// Runtime video color primaries exposed without platform sample attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeColorPrimaries {
    Bt709,
    Bt2020,
    DisplayP3,
    Unknown,
}

/// Runtime video transfer function exposed without platform sample attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeColorTransfer {
    Bt709,
    Srgb,
    Pq,
    Hlg,
    Unknown,
}

/// Runtime video color matrix exposed without platform sample attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeColorMatrix {
    Bt709,
    Bt2020NonConstant,
    Identity,
    Unknown,
}

/// Runtime video color range exposed without platform sample attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeColorRange {
    Limited,
    Full,
    Unknown,
}

/// Bounded color metadata diagnostic suitable for binding responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeColorDiagnostic {
    pub message: String,
}

/// Binding-safe color metadata for decoded frames and textures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeVideoColorMetadata {
    pub primaries: RuntimeColorPrimaries,
    pub transfer: RuntimeColorTransfer,
    pub matrix: RuntimeColorMatrix,
    pub range: RuntimeColorRange,
    pub diagnostics: Vec<RuntimeColorDiagnostic>,
}

impl RuntimeVideoColorMetadata {
    pub fn unknown_with_diagnostic(message: impl Into<String>) -> Self {
        Self {
            primaries: RuntimeColorPrimaries::Unknown,
            transfer: RuntimeColorTransfer::Unknown,
            matrix: RuntimeColorMatrix::Unknown,
            range: RuntimeColorRange::Unknown,
            diagnostics: vec![RuntimeColorDiagnostic {
                message: message.into(),
            }],
        }
    }
}

/// Binding-safe runtime device identity for texture compatibility checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeDeviceId {
    pub backend: RuntimeTextureBackend,
    pub adapter_id: String,
    pub device_id: String,
}

/// Binding-safe frame dimensions for decoded frame and texture metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeFrameDimensions {
    pub width: u32,
    pub height: u32,
}

/// Binding-safe decoded frame metadata. The frame payload stays owned by Rust/native runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeDecodedFrameHandleMetadata {
    pub frame_handle_id: String,
    pub owner_session: String,
    pub generation: u64,
    pub dimensions: RuntimeFrameDimensions,
    pub pixel_format: RuntimeVideoPixelFormat,
    pub color: RuntimeVideoColorMetadata,
}

/// Binding-safe texture metadata. Native pointers and GPU objects never cross this contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeTextureHandleMetadata {
    pub texture_handle_id: String,
    pub owner_session: String,
    pub generation: u64,
    pub backend: RuntimeTextureBackend,
    pub device_id: RuntimeDeviceId,
    pub dimensions: RuntimeFrameDimensions,
    pub pixel_format: RuntimeVideoPixelFormat,
    pub color: RuntimeVideoColorMetadata,
}

/// Binding-safe native media IO capability report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeMediaIoCapabilities {
    pub windows: RuntimeWindowsMediaIoCapabilities,
    pub macos: RuntimeMacosMediaIoCapabilities,
    pub codecs: Vec<RuntimeCodecCapability>,
    pub pixel_formats: Vec<RuntimePixelFormatCapability>,
    pub texture_interop: RuntimeTextureInteropCapability,
    pub fallback_ladder: RuntimeFallbackLadderCapability,
}

/// Windows native media IO capability domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeWindowsMediaIoCapabilities {
    pub status: RuntimeCapabilityStatus,
    pub media_foundation: RuntimeFeatureCapability,
    pub dxva: RuntimeFeatureCapability,
    pub d3d_texture_interop: RuntimeFeatureCapability,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<String>,
}

/// macOS native media IO capability domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeMacosMediaIoCapabilities {
    pub status: RuntimeCapabilityStatus,
    pub av_foundation: RuntimeFeatureCapability,
    pub video_toolbox: RuntimeFeatureCapability,
    pub core_video: RuntimeFeatureCapability,
    pub metal_texture_interop: RuntimeFeatureCapability,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<String>,
}

/// Codec readiness and first native target metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeCodecCapability {
    pub codec: String,
    pub containers: Vec<String>,
    pub first_native_hardware_decode_target: bool,
    pub status: RuntimeCapabilityStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<String>,
}

/// Pixel-format readiness metadata without pixel payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePixelFormatCapability {
    pub pixel_format: RuntimeVideoPixelFormat,
    pub status: RuntimeCapabilityStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<String>,
}

/// Texture interop capability metadata for preview-device compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeTextureInteropCapability {
    pub status: RuntimeCapabilityStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub backend: Option<RuntimeTextureBackend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub device_id: Option<RuntimeDeviceId>,
    pub compatible_with_preview_device: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<String>,
}

/// Supported decode path ladder exposed as data, not renderer-owned selection logic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeFallbackLadderCapability {
    pub paths: Vec<RuntimeFallbackDecodePathCapability>,
}

/// One decode path capability entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeFallbackDecodePathCapability {
    pub path: RuntimeSelectedDecodePath,
    pub status: RuntimeCapabilityStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fallback_reason: Option<RuntimeMediaIoFallbackReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub diagnostic: Option<String>,
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
    pub media_io: RuntimeMediaIoCapabilities,
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
