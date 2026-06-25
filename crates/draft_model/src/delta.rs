use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    DraftId, KeyframeProperty, MaterialId, Microseconds, SegmentId, TargetTimerange, TrackId,
};

/// Semantic command names used in accepted command deltas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CommandDeltaName {
    Ping,
    Version,
    ProbeMediaRuntime,
    ProbeRuntimeCapabilities,
    ImportMaterial,
    ImportTemplate,
    AddSegment,
    AddTimelineSegmentIntent,
    SelectTimelineSegments,
    MoveSegment,
    MoveSelectedSegmentIntent,
    SplitSegment,
    SplitSelectedSegmentIntent,
    TrimSegment,
    TrimSelectedSegmentIntent,
    DeleteSegment,
    UndoTimelineEdit,
    RedoTimelineEdit,
    AddTextSegment,
    AddTextSegmentIntent,
    EditTextSegment,
    ImportSubtitleSrt,
    ImportSubtitleSrtIntent,
    AddAudioSegment,
    AddAudioSegmentIntent,
    SetSegmentVolume,
    UpdateSegmentAudio,
    AddTrack,
    AddTrackIntent,
    RenameTrack,
    SetTrackLock,
    SetTrackVisibility,
    SetTrackMute,
    UpdateDraftCanvasConfig,
    UpdateSegmentVisual,
    ApplySegmentEffect,
    UpdateSegmentEffectParameter,
    RemoveSegmentEffect,
    SetSegmentMask,
    SetSegmentBlendMode,
    SetSegmentRetime,
    ClearSegmentRetime,
    AddTransition,
    UpdateTransitionDuration,
    RemoveTransition,
    SetSegmentKeyframe,
    RemoveSegmentKeyframe,
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

/// Semantic facts emitted after an accepted Rust-owned command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandDelta {
    pub command: CommandDeltaName,
    pub changed_entities: Vec<ChangedEntity>,
    pub changed_domains: Vec<DirtyDomain>,
    pub changed_ranges: Vec<DirtyRange>,
    pub invalidation: InvalidationScope,
    pub reason: String,
}

impl CommandDelta {
    pub fn none(command: CommandDeltaName, reason: impl Into<String>) -> Self {
        Self {
            command,
            changed_entities: Vec::new(),
            changed_domains: Vec::new(),
            changed_ranges: Vec::new(),
            invalidation: InvalidationScope::empty(),
            reason: reason.into(),
        }
    }

    pub fn targeted(
        command: CommandDeltaName,
        changed_entities: Vec<ChangedEntity>,
        changed_domains: Vec<DirtyDomain>,
        changed_ranges: Vec<DirtyRange>,
        invalidation: InvalidationScope,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            command,
            changed_entities,
            changed_domains,
            changed_ranges,
            invalidation,
            reason: reason.into(),
        }
    }

    pub fn full_draft(
        command: CommandDeltaName,
        changed_entities: Vec<ChangedEntity>,
        changed_domains: Vec<DirtyDomain>,
        consumer_domains: Vec<DirtyDomain>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            command,
            changed_entities,
            changed_domains,
            changed_ranges: vec![DirtyRange {
                target_timerange: TargetTimerange::new(Microseconds::ZERO, Microseconds::ZERO),
                source: DirtyRangeSource::FullDraft,
            }],
            invalidation: InvalidationScope {
                full_draft: true,
                material_ids: Vec::new(),
                graph_node_ids: Vec::new(),
                consumer_domains,
            },
            reason: reason.into(),
        }
    }
}

/// Semantic draft entities changed by a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ChangedEntity {
    Draft {
        draft_id: DraftId,
    },
    Material {
        material_id: MaterialId,
    },
    Track {
        track_id: TrackId,
    },
    Segment {
        track_id: TrackId,
        segment_id: SegmentId,
    },
    Keyframe {
        track_id: TrackId,
        segment_id: SegmentId,
        property: KeyframeProperty,
        at: Microseconds,
    },
    Canvas {
        draft_id: DraftId,
    },
    RuntimeCapabilities {
        capability_fingerprint: String,
    },
}

/// Semantic and derived consumer domains affected by a command.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "camelCase")]
pub enum DirtyDomain {
    Track,
    Timing,
    Visual,
    Text,
    Audio,
    Material,
    Effect,
    Filter,
    Transition,
    Canvas,
    OutputProfile,
    RuntimeCapabilities,
    Preview,
    ExportPrep,
    Thumbnail,
    Waveform,
    Proxy,
    GraphSnapshot,
    PreviewCache,
}

/// Integer target timeline span affected by a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirtyRange {
    pub target_timerange: TargetTimerange,
    pub source: DirtyRangeSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum DirtyRangeSource {
    Previous,
    Current,
    PreviousAndCurrent,
    FullDraft,
    MaterialWide,
}

/// Derived invalidation scope. Graph IDs are derived facts, not primary command entities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvalidationScope {
    pub full_draft: bool,
    pub material_ids: Vec<MaterialId>,
    pub graph_node_ids: Vec<String>,
    pub consumer_domains: Vec<DirtyDomain>,
}

impl InvalidationScope {
    pub fn empty() -> Self {
        Self {
            full_draft: false,
            material_ids: Vec::new(),
            graph_node_ids: Vec::new(),
            consumer_domains: Vec::new(),
        }
    }

    pub fn targeted(material_ids: Vec<MaterialId>, consumer_domains: Vec<DirtyDomain>) -> Self {
        Self {
            full_draft: false,
            material_ids,
            graph_node_ids: Vec::new(),
            consumer_domains,
        }
    }
}
