#![recursion_limit = "512"]

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use draft_model::{
    AddTransitionCommandPayload, ArtifactGenerationActionCommandPayload,
    ArtifactGenerationTaskSummary, ArtifactMaintenanceResult, ArtifactQuotaStatus,
    ArtifactStatusSummary, ArtifactTaskStatus, AudioEffectSlot, AudioEffectSlotKind, AudioFade,
    AudioOutputDeviceStatus, AudioOutputDeviceSummary, AudioPanBalance, AudioPreviewCommandPayload,
    AudioPreviewCommandResponse, AudioPreviewPlaybackStatus, AudioPreviewStatusResponse,
    AudioRetimePolicy, BlendModeKind, CancelExportCommandPayload, CanvasAdaptationPolicy,
    CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground, CanvasBackgroundCapability,
    CapabilityCategory, CapabilityReportItem, CapabilitySupport, CapabilitySurface, ChangedEntity,
    ClearSegmentRetimeCommandPayload, CommandDelta, CommandDeltaName, CommandEnvelope,
    CommandError, CommandErrorKind, CommandEvent, CommandHistorySnapshot, CommandName,
    CommandPayload, CommandResultEnvelope, CommandState, DirtyDomain, DirtyRange, DirtyRangeSource,
    DisplayableArtifactRef, Draft, DraftCanvasConfig, DraftId, DraftMetadata, DraftSchemaVersion,
    EffectCapabilityRegistry, EffectKind, EffectParameterUpdate, ExportDiagnostic,
    ExportDiagnosticKind, ExportJobPhase, ExportJobStatusResponse, ExportPrepDirtyFacts,
    ExportPreset, ExportValidationReport, ExternalEffectReference, Filter, FilterKind,
    GetArtifactQuotaStatusCommandPayload,
    GetArtifactStatusCommandPayload, GetExportJobStatusCommandPayload, InvalidationScope, Keyframe,
    KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue, MAX_TEXT_LAYOUT_MILLIS,
    MAX_TEXT_LETTER_SPACING_MILLIS, MAX_TEXT_LINE_HEIGHT_MILLIS, MIN_TEXT_LINE_HEIGHT_MILLIS,
    MainTrackMagnet, MaskKind, Material, MaterialArtifactStatus, MaterialId, MaterialKind,
    MaterialMetadata, MaterialStatus, Microseconds, MissingMaterialCommandDiagnostic,
    MissingMaterialCommandDiagnosticKind, PingCommandPayload, PreviewArtifactResponse,
    PreviewDiagnostic, PreviewDiagnosticKind, PreviewOutputProfile, PreviewStatus,
    ProbeMediaRuntimeCommandPayload, ProbeRuntimeCapabilitiesCommandPayload, RationalFrameRate,
    RefreshArtifactStatusCommandPayload, RemoveTransitionCommandPayload, RetimeMode,
    RunArtifactGarbageCollectionCommandPayload, RuntimeBinaryCapability, RuntimeBinaryKind,
    RuntimeCapabilityReport, RuntimeCapabilityStatus, RuntimeCodecCapability,
    RuntimeColorDiagnostic, RuntimeColorMatrix, RuntimeColorPrimaries, RuntimeColorRange,
    RuntimeColorTransfer, RuntimeDeviceId, RuntimeFallbackDecodePathCapability,
    RuntimeFallbackLadderCapability, RuntimeFeatureCapability, RuntimeFontCapability,
    RuntimeLicensePosture, RuntimeMacosMediaIoCapabilities, RuntimeMediaIoCapabilities,
    RuntimeMediaIoFallbackReason, RuntimePixelFormatCapability, RuntimeSelectedDecodePath,
    RuntimeTextureBackend, RuntimeTextureInteropCapability, RuntimeVideoColorMetadata,
    RuntimeVideoPixelFormat, RuntimeWindowsMediaIoCapabilities, Segment, SegmentAnchor,
    SegmentAudio, SegmentBackgroundFilling, SegmentBlendMode, SegmentCrop, SegmentFitMode,
    SegmentId, SegmentMask, SegmentOpacity, SegmentPosition, SegmentRetiming, SegmentRotation,
    SegmentScale, SegmentTransform, SegmentVisual, SegmentVolume, SetSegmentRetimeCommandPayload,
    SnappingSettings, SourceTimerange, SpeedCurvePoint, SpeedRatio, StartExportCommandPayload,
    TargetTimerange, TextAlignment, TextBackground, TextBox, TextBubbleRef, TextEffectRef,
    TextFont, TextLayoutRegion, TextSegment, TextSegmentSource, TextShadow, TextStroke, TextStyle,
    TextWrapping, TimelineCommandResponse, TimelineSelection, Track, TrackId, TrackKind,
    TrackTransition, Transition, TransitionKind, TransitionReference,
    UpdateTransitionDurationCommandPayload, VersionCommandPayload, WaveformDisplayPeak,
    WaveformDisplayPeaksResponse, WaveformDisplayStatus,
};
use schemars::{Schema, schema_for};
use serde_json::json;
use ts_rs::{Config, TS};

const TEXT_HEX_COLOR_PATTERN: &str = "^#[0-9A-Fa-f]{6}$";
const PUBLIC_TIMELINE_EDIT_PAYLOAD_CONTRACTS: &[&str] = &[
    "AddSegmentCommandPayload",
    "AddTimelineSegmentIntentCommandPayload",
    "SelectTimelineSegmentsCommandPayload",
    "MoveSegmentCommandPayload",
    "MoveSelectedSegmentIntentCommandPayload",
    "SplitSegmentCommandPayload",
    "SplitSelectedSegmentIntentCommandPayload",
    "TrimSegmentCommandPayload",
    "TrimSelectedSegmentIntentCommandPayload",
    "DeleteSegmentCommandPayload",
    "UndoTimelineEditCommandPayload",
    "RedoTimelineEditCommandPayload",
    "AddTextSegmentCommandPayload",
    "AddTextSegmentIntentCommandPayload",
    "EditTextSegmentCommandPayload",
    "ImportSubtitleSrtCommandPayload",
    "ImportSubtitleSrtIntentCommandPayload",
    "AddAudioSegmentCommandPayload",
    "AddAudioSegmentIntentCommandPayload",
    "SetSegmentVolumeCommandPayload",
    "UpdateSegmentAudioCommandPayload",
    "AddTrackCommandPayload",
    "AddTrackIntentCommandPayload",
    "RenameTrackCommandPayload",
    "SetTrackLockCommandPayload",
    "SetTrackVisibilityCommandPayload",
    "SetTrackMuteCommandPayload",
    "UpdateDraftCanvasConfigCommandPayload",
    "UpdateSegmentVisualCommandPayload",
    "SetSegmentRetimeCommandPayload",
    "ClearSegmentRetimeCommandPayload",
    "AddTransitionCommandPayload",
    "UpdateTransitionDurationCommandPayload",
    "RemoveTransitionCommandPayload",
    "SetSegmentKeyframeCommandPayload",
    "RemoveSegmentKeyframeCommandPayload",
];
const PUBLIC_TIMELINE_EDIT_COMMAND_NAMES: &[&str] = &[
    "addSegment",
    "addTimelineSegmentIntent",
    "selectTimelineSegments",
    "moveSegment",
    "moveSelectedSegmentIntent",
    "splitSegment",
    "splitSelectedSegmentIntent",
    "trimSegment",
    "trimSelectedSegmentIntent",
    "deleteSegment",
    "undoTimelineEdit",
    "redoTimelineEdit",
    "addTextSegment",
    "addTextSegmentIntent",
    "editTextSegment",
    "importSubtitleSrt",
    "importSubtitleSrtIntent",
    "addAudioSegment",
    "addAudioSegmentIntent",
    "setSegmentVolume",
    "updateSegmentAudio",
    "addTrack",
    "addTrackIntent",
    "renameTrack",
    "setTrackLock",
    "setTrackVisibility",
    "setTrackMute",
    "updateDraftCanvasConfig",
    "updateSegmentVisual",
    "setSegmentRetime",
    "clearSegmentRetime",
    "addTransition",
    "updateTransitionDuration",
    "removeTransition",
    "setSegmentKeyframe",
    "removeSegmentKeyframe",
];

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("draft_model should live under crates/")
        .to_path_buf()
}

#[test]
fn schema_exports_generated_contract_artifacts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/command.schema.json");
    let draft_schema_path = root.join("schemas/draft.schema.json");
    let generated_dir = root.join("apps/desktop-electron/src/generated");

    let schema_json = command_schema_json();
    assert_command_schema_rejects_zero_frame_rates(&schema_json);
    assert_command_schema_rejects_invalid_canvas_config(&schema_json);
    assert_command_schema_rejects_invalid_text_contracts(&schema_json);
    assert_command_schema_rejects_invalid_keyframe_contracts(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));

    let draft_schema_json = draft_schema_json();
    assert_draft_schema_rejects_zero_frame_rates(&draft_schema_json);
    assert_draft_schema_rejects_invalid_canvas_config(&draft_schema_json);
    assert_draft_schema_rejects_invalid_text_contracts(&draft_schema_json);
    assert_draft_schema_rejects_invalid_keyframe_contracts(&draft_schema_json);
    assert_draft_schema_includes_phase19_effect_contracts(&draft_schema_json);
    assert_or_update_contract_file(&draft_schema_path, &format!("{draft_schema_json}\n"));

    let command_envelope_ts = ts_contract_with_prelude(
        "import type { Draft, MaterialId, MaterialKind, Microseconds, SegmentId, TargetTimerange, TrackId } from \"./Draft\";\n\n",
        &[
            export_decl::<CommandName>(),
            export_decl::<PingCommandPayload>(),
            export_decl::<VersionCommandPayload>(),
            export_decl::<ProbeMediaRuntimeCommandPayload>(),
            export_decl::<ProbeRuntimeCapabilitiesCommandPayload>(),
            export_decl::<PreviewOutputProfile>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<DirtyRange>(),
            export_decl::<AudioPreviewCommandPayload>(),
            export_decl::<GetArtifactStatusCommandPayload>(),
            export_decl::<RefreshArtifactStatusCommandPayload>(),
            export_decl::<ArtifactGenerationActionCommandPayload>(),
            export_decl::<GetArtifactQuotaStatusCommandPayload>(),
            export_decl::<RunArtifactGarbageCollectionCommandPayload>(),
            export_decl::<ExportPreset>(),
            export_decl::<ExportPrepDirtyFacts>(),
            export_decl::<StartExportCommandPayload>(),
            export_decl::<GetExportJobStatusCommandPayload>(),
            export_decl::<CancelExportCommandPayload>(),
            export_decl::<TimelineSelection>(),
            export_decl::<SnappingSettings>(),
            export_decl::<CommandHistorySnapshot>(),
            export_decl::<CommandState>(),
            export_decl::<CommandPayload>(),
            export_decl::<CommandEnvelope>(),
        ],
    );
    assert_or_update_contract_file(
        generated_dir.join("CommandEnvelope.ts"),
        &command_envelope_ts,
    );

    let command_result_ts = ts_contract_with_prelude(
        "import type { Draft, DraftId, KeyframeProperty, Material, MaterialId, MaterialStatus, Microseconds, RationalFrameRate, SegmentId, TargetTimerange, TrackId } from \"./Draft\";\nimport type { CommandState, ExportPreset, PreviewOutputProfile, TimelineSelection } from \"./CommandEnvelope\";\n\n",
        &[
            export_decl::<CommandErrorKind>(),
            export_decl::<CommandError>(),
            export_decl::<CommandEvent>(),
            export_decl::<CommandResultEnvelope<()>>(),
            export_decl::<PreviewStatus>(),
            export_decl::<PreviewDiagnosticKind>(),
            export_decl::<PreviewDiagnostic>(),
            export_decl::<PreviewArtifactResponse>(),
            export_decl::<AudioPreviewPlaybackStatus>(),
            export_decl::<AudioOutputDeviceStatus>(),
            export_decl::<WaveformDisplayStatus>(),
            export_decl::<AudioOutputDeviceSummary>(),
            export_decl::<AudioPreviewStatusResponse>(),
            export_decl::<AudioPreviewCommandResponse>(),
            export_decl::<WaveformDisplayPeak>(),
            export_decl::<WaveformDisplayPeaksResponse>(),
            export_decl::<ExportJobPhase>(),
            export_decl::<ExportDiagnosticKind>(),
            export_decl::<ExportDiagnostic>(),
            export_decl::<ExportValidationReport>(),
            export_decl::<ExportPrepDirtyFacts>(),
            export_decl::<ExportJobStatusResponse>(),
            export_decl::<RuntimeCapabilityStatus>(),
            export_decl::<RuntimeBinaryKind>(),
            export_decl::<RuntimeBinaryCapability>(),
            export_decl::<RuntimeFeatureCapability>(),
            export_decl::<RuntimeFontCapability>(),
            export_decl::<RuntimeLicensePosture>(),
            export_decl::<RuntimeMediaIoFallbackReason>(),
            export_decl::<RuntimeSelectedDecodePath>(),
            export_decl::<RuntimeTextureBackend>(),
            export_decl::<RuntimeVideoPixelFormat>(),
            export_decl::<RuntimeColorPrimaries>(),
            export_decl::<RuntimeColorTransfer>(),
            export_decl::<RuntimeColorMatrix>(),
            export_decl::<RuntimeColorRange>(),
            export_decl::<RuntimeColorDiagnostic>(),
            export_decl::<RuntimeVideoColorMetadata>(),
            export_decl::<RuntimeDeviceId>(),
            export_decl::<RuntimeWindowsMediaIoCapabilities>(),
            export_decl::<RuntimeMacosMediaIoCapabilities>(),
            export_decl::<RuntimeCodecCapability>(),
            export_decl::<RuntimePixelFormatCapability>(),
            export_decl::<RuntimeTextureInteropCapability>(),
            export_decl::<RuntimeFallbackDecodePathCapability>(),
            export_decl::<RuntimeFallbackLadderCapability>(),
            export_decl::<RuntimeMediaIoCapabilities>(),
            export_decl::<RuntimeCapabilityReport>(),
            export_decl::<MissingMaterialCommandDiagnosticKind>(),
            export_decl::<MissingMaterialCommandDiagnostic>(),
            export_decl::<ChangedEntity>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRange>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<InvalidationScope>(),
            export_decl::<CommandDeltaName>(),
            export_decl::<CommandDelta>(),
            export_decl::<TimelineCommandResponse>(),
            export_decl::<ArtifactTaskStatus>(),
            export_decl::<DisplayableArtifactRef>(),
            export_decl::<MaterialArtifactStatus>(),
            export_decl::<ArtifactGenerationTaskSummary>(),
            export_decl::<ArtifactQuotaStatus>(),
            export_decl::<ArtifactStatusSummary>(),
            export_decl::<ArtifactMaintenanceResult>(),
        ],
    );
    assert_or_update_contract_file(
        generated_dir.join("CommandResultEnvelope.ts"),
        &command_result_ts,
    );

    let draft_ts = ts_contract(&[
        export_decl::<DraftId>(),
        export_decl::<MaterialId>(),
        export_decl::<TrackId>(),
        export_decl::<SegmentId>(),
        export_decl::<Microseconds>(),
        export_decl::<DraftSchemaVersion>(),
        export_decl::<DraftMetadata>(),
        export_decl::<RationalFrameRate>(),
        export_decl::<CanvasAspectRatioPreset>(),
        export_decl::<CanvasAspectRatio>(),
        export_decl::<CanvasBackgroundCapability>(),
        export_decl::<CanvasBackground>(),
        export_decl::<CanvasAdaptationPolicy>(),
        export_decl::<DraftCanvasConfig>(),
        export_decl::<MaterialKind>(),
        export_decl::<MaterialStatus>(),
        export_decl::<MaterialMetadata>(),
        export_decl::<Material>(),
        export_decl::<TrackKind>(),
        export_decl::<MainTrackMagnet>(),
        export_decl::<SourceTimerange>(),
        export_decl::<TargetTimerange>(),
        export_decl::<KeyframeProperty>(),
        export_decl::<KeyframeValue>(),
        export_decl::<KeyframeInterpolation>(),
        export_decl::<KeyframeEasing>(),
        export_decl::<Keyframe>(),
        export_decl::<CapabilitySurface>(),
        export_decl::<ExternalEffectReference>(),
        export_decl::<CapabilitySupport>(),
        export_decl::<CapabilityCategory>(),
        export_decl::<CapabilityReportItem>(),
        export_decl::<EffectCapabilityRegistry>(),
        export_decl::<EffectKind>(),
        export_decl::<FilterKind>(),
        export_decl::<Filter>(),
        export_decl::<EffectParameterUpdate>(),
        export_decl::<TransitionKind>(),
        export_decl::<TransitionReference>(),
        export_decl::<Transition>(),
        export_decl::<TrackTransition>(),
        export_decl::<SpeedRatio>(),
        export_decl::<SpeedCurvePoint>(),
        export_decl::<RetimeMode>(),
        export_decl::<AudioRetimePolicy>(),
        export_decl::<SegmentRetiming>(),
        export_decl::<MaskKind>(),
        export_decl::<BlendModeKind>(),
        export_decl::<TextAlignment>(),
        export_decl::<TextSegmentSource>(),
        export_decl::<TextFont>(),
        export_decl::<TextStroke>(),
        export_decl::<TextShadow>(),
        export_decl::<TextBackground>(),
        export_decl::<TextStyle>(),
        export_decl::<TextBox>(),
        export_decl::<TextLayoutRegion>(),
        export_decl::<TextWrapping>(),
        export_decl::<TextBubbleRef>(),
        export_decl::<TextEffectRef>(),
        export_decl::<TextSegment>(),
        export_decl::<SegmentVolume>(),
        export_decl::<AudioPanBalance>(),
        export_decl::<AudioFade>(),
        export_decl::<AudioEffectSlotKind>(),
        export_decl::<AudioEffectSlot>(),
        export_decl::<SegmentAudio>(),
        export_decl::<SegmentPosition>(),
        export_decl::<SegmentScale>(),
        export_decl::<SegmentRotation>(),
        export_decl::<SegmentOpacity>(),
        export_decl::<SegmentCrop>(),
        export_decl::<SegmentAnchor>(),
        export_decl::<SegmentTransform>(),
        export_decl::<SegmentFitMode>(),
        export_decl::<SegmentBackgroundFilling>(),
        export_decl::<SegmentBlendMode>(),
        export_decl::<SegmentMask>(),
        export_decl::<SegmentVisual>(),
        export_decl::<Segment>(),
        export_decl::<Track>(),
        export_decl::<Draft>(),
    ]);
    assert!(
        draft_ts.contains("export type Microseconds = number;"),
        "Microseconds must match the JSON IPC representation"
    );
    assert!(
        !draft_ts.contains("export type Microseconds = bigint;"),
        "Microseconds must not advertise bigint over the JSON IPC boundary"
    );
    for expected_contract in [
        "CapabilitySurface",
        "CapabilitySupport",
        "CapabilityReportItem",
        "EffectCapabilityRegistry",
        "ExternalEffectReference",
        "FilterKind",
        "TransitionReference",
        "TrackTransition",
        "SpeedRatio",
        "SpeedCurvePoint",
        "RetimeMode",
        "SegmentRetiming",
        "MaskKind",
        "BlendModeKind",
    ] {
        assert!(
            draft_ts.contains(&format!("export type {expected_contract}")),
            "Draft.ts should export Phase 19 contract {expected_contract}"
        );
    }
    for forbidden in [
        "speedSeconds",
        "durationSeconds",
        "targetTimeSeconds",
        "radius: number",
        "opacity: number",
    ] {
        assert!(
            !draft_ts.contains(forbidden),
            "Phase 19 generated TS must not expose naked float-style field {forbidden}"
        );
    }
    assert_or_update_contract_file(generated_dir.join("Draft.ts"), &draft_ts);
}

#[test]
fn schema_exports_include_timeline_command_session_contracts() {
    let schema_json = command_schema_json();
    assert!(
        !schema_json.contains("TimelineCommandResponse")
            && !schema_json.contains("CommandDeltaName"),
        "public command schema must not include timeline session response contracts"
    );

    let command_envelope_ts = ts_contract_with_prelude(
        "import type { Draft, MaterialId, MaterialKind, Microseconds, SegmentId, TargetTimerange, TrackId } from \"./Draft\";\n\n",
        &[
            export_decl::<CommandName>(),
            export_decl::<PingCommandPayload>(),
            export_decl::<VersionCommandPayload>(),
            export_decl::<ProbeMediaRuntimeCommandPayload>(),
            export_decl::<ProbeRuntimeCapabilitiesCommandPayload>(),
            export_decl::<PreviewOutputProfile>(),
            export_decl::<ExportPreset>(),
            export_decl::<StartExportCommandPayload>(),
            export_decl::<GetExportJobStatusCommandPayload>(),
            export_decl::<CancelExportCommandPayload>(),
            export_decl::<TimelineSelection>(),
            export_decl::<SnappingSettings>(),
            export_decl::<CommandHistorySnapshot>(),
            export_decl::<CommandState>(),
            export_decl::<CommandPayload>(),
            export_decl::<CommandEnvelope>(),
        ],
    );
    let command_result_ts = ts_contract_with_prelude(
        "import type { Draft, DraftId, KeyframeProperty, Material, MaterialId, MaterialStatus, Microseconds, RationalFrameRate, SegmentId, TargetTimerange, TrackId } from \"./Draft\";\nimport type { CommandState, ExportPreset, PreviewOutputProfile, TimelineSelection } from \"./CommandEnvelope\";\n\n",
        &[
            export_decl::<CommandErrorKind>(),
            export_decl::<CommandError>(),
            export_decl::<CommandEvent>(),
            export_decl::<CommandResultEnvelope<()>>(),
            export_decl::<PreviewStatus>(),
            export_decl::<PreviewDiagnosticKind>(),
            export_decl::<PreviewDiagnostic>(),
            export_decl::<PreviewArtifactResponse>(),
            export_decl::<ExportJobPhase>(),
            export_decl::<ExportDiagnosticKind>(),
            export_decl::<ExportDiagnostic>(),
            export_decl::<ExportValidationReport>(),
            export_decl::<ExportJobStatusResponse>(),
            export_decl::<RuntimeCapabilityStatus>(),
            export_decl::<RuntimeBinaryKind>(),
            export_decl::<RuntimeBinaryCapability>(),
            export_decl::<RuntimeFeatureCapability>(),
            export_decl::<RuntimeFontCapability>(),
            export_decl::<RuntimeLicensePosture>(),
            export_decl::<RuntimeMediaIoFallbackReason>(),
            export_decl::<RuntimeSelectedDecodePath>(),
            export_decl::<RuntimeTextureBackend>(),
            export_decl::<RuntimeVideoPixelFormat>(),
            export_decl::<RuntimeColorPrimaries>(),
            export_decl::<RuntimeColorTransfer>(),
            export_decl::<RuntimeColorMatrix>(),
            export_decl::<RuntimeColorRange>(),
            export_decl::<RuntimeColorDiagnostic>(),
            export_decl::<RuntimeVideoColorMetadata>(),
            export_decl::<RuntimeDeviceId>(),
            export_decl::<RuntimeWindowsMediaIoCapabilities>(),
            export_decl::<RuntimeMacosMediaIoCapabilities>(),
            export_decl::<RuntimeCodecCapability>(),
            export_decl::<RuntimePixelFormatCapability>(),
            export_decl::<RuntimeTextureInteropCapability>(),
            export_decl::<RuntimeFallbackDecodePathCapability>(),
            export_decl::<RuntimeFallbackLadderCapability>(),
            export_decl::<RuntimeMediaIoCapabilities>(),
            export_decl::<RuntimeCapabilityReport>(),
            export_decl::<MissingMaterialCommandDiagnosticKind>(),
            export_decl::<MissingMaterialCommandDiagnostic>(),
            export_decl::<ChangedEntity>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRange>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<InvalidationScope>(),
            export_decl::<CommandDeltaName>(),
            export_decl::<CommandDelta>(),
            export_decl::<TimelineCommandResponse>(),
        ],
    );

    for expected_contract in [
        "TimelineSelection",
        "CommandState",
        "CommandHistorySnapshot",
        "SnappingSettings",
        "CommandDeltaName",
        "TimelineCommandResponse",
    ] {
        assert!(
            command_envelope_ts.contains(expected_contract)
                || command_result_ts.contains(expected_contract),
            "generated TypeScript contracts should include {expected_contract}"
        );
    }
}

#[test]
fn schema_exports_include_phase13_incremental_harness_anchors() {
    let draft_schema = draft_schema_json();
    let command_result_ts = command_result_ts_contract();
    let draft_ts = ts_contract(&[
        export_decl::<Microseconds>(),
        export_decl::<TargetTimerange>(),
        export_decl::<MaterialId>(),
    ]);

    assert!(
        command_result_ts.contains("TimelineCommandResponse"),
        "Phase 13 timeline response contract should be exported from command result TS"
    );

    for expected_contract in ["Microseconds", "TargetTimerange", "MaterialId"] {
        assert!(
            draft_schema.contains(expected_contract) || draft_ts.contains(expected_contract),
            "Phase 13 dirty range contracts should keep exporting {expected_contract}"
        );
    }

    for forbidden in [
        "artifactStore",
        "graphSnapshot",
        "previewCacheKey",
        "dirtyRangeSeconds",
        "targetTimeSeconds",
    ] {
        assert!(
            !draft_schema.contains(forbidden),
            "canonical draft schema must not grow derived Phase 13 metadata field {forbidden}"
        );
    }
}

#[test]
fn schema_exports_include_phase13_delta_contracts() {
    let command_result_ts = command_result_ts_contract();
    for expected_export in [
        "export type ChangedEntity",
        "export type DirtyDomain",
        "export type DirtyRange",
        "export type DirtyRangeSource",
        "export type InvalidationScope",
        "export type CommandDeltaName",
        "export type CommandDelta",
        "export type TimelineCommandResponse",
        "delta: CommandDelta",
    ] {
        assert!(
            command_result_ts.contains(expected_export),
            "generated TypeScript response contract should include {expected_export}"
        );
    }
}

#[test]
fn schema_exports_include_phase13_preview_export_dirty_fact_contracts() {
    let command_schema: serde_json::Value =
        serde_json::from_str(&command_schema_json()).expect("command schema should parse");
    let defs = command_schema
        .get("$defs")
        .and_then(|defs| defs.as_object())
        .expect("command schema should expose definitions");

    for expected_contract in [
        "ExportPrepDirtyFacts",
        "StartExportCommandPayload",
        "ExportJobStatusResponse",
    ] {
        assert!(
            defs.contains_key(expected_contract),
            "command schema should include Phase 13 dirty fact contract {expected_contract}"
        );
    }

    for removed_contract in [
        "PreviewCacheEntryRef",
        "InvalidatePreviewCacheCommandPayload",
    ] {
        assert!(
            !defs.contains_key(removed_contract),
            "generic preview cache command contract {removed_contract} must not be public"
        );
    }

    let export_dirty_facts = defs
        .get("ExportPrepDirtyFacts")
        .expect("ExportPrepDirtyFacts should be generated");
    assert_eq!(
        export_dirty_facts
            .pointer("/properties/dirtyRanges/items/$ref")
            .and_then(|value| value.as_str()),
        Some("#/$defs/DirtyRange"),
        "export dirtyFacts dirtyRanges must use DirtyRange transport"
    );
    for expected_field in [
        "changedMaterialIds",
        "changedGraphNodeIds",
        "changedDomains",
        "runtimeCapabilityFingerprint",
        "outputProfileFingerprint",
        "fullDraft",
        "reason",
        "artifactSchemaVersion",
        "generatorVersion",
    ] {
        assert!(
            export_dirty_facts
                .pointer(&format!("/properties/{expected_field}"))
                .is_some(),
            "export dirtyFacts should expose {expected_field}"
        );
    }

    for (contract_name, field_name) in [
        ("StartExportCommandPayload", "dirtyFacts"),
        ("ExportJobStatusResponse", "dirtyFacts"),
    ] {
        assert!(
            property_references_def(
                defs.get(contract_name).expect("contract should exist"),
                field_name,
                "#/$defs/ExportPrepDirtyFacts",
            ),
            "{contract_name}.{field_name} should reference ExportPrepDirtyFacts"
        );
    }

    let command_envelope_ts = command_envelope_ts_contract();
    let command_result_ts = command_result_ts_contract();
    for expected_export in [
        "export type ExportPrepDirtyFacts",
        "dirtyRanges: Array<DirtyRange>",
        "changedGraphNodeIds: Array<string>",
        "runtimeCapabilityFingerprint",
        "fullDraft: boolean",
        "artifactSchemaVersion: number",
        "generatorVersion: string",
        "dirtyFacts",
    ] {
        assert!(
            command_envelope_ts.contains(expected_export)
                || command_result_ts.contains(expected_export),
            "generated TypeScript contracts should include dirty fact surface {expected_export}"
        );
    }

    for forbidden in [
        "previewCacheKey",
        "cacheKeyFormula",
        "graphDiff",
        "ffmpegArgs",
        "filterComplex",
        concat!("artifactStore", "Sqlite"),
        concat!("priority", "Queue"),
    ] {
        assert!(
            !command_envelope_ts.contains(forbidden) && !command_result_ts.contains(forbidden),
            "dirty fact contracts must not expose renderer-owned or later-phase {forbidden}"
        );
    }
}

#[test]
fn schema_exports_include_phase14_artifact_status_and_maintenance_contracts() {
    let command_schema: serde_json::Value =
        serde_json::from_str(&command_schema_json()).expect("command schema should parse");
    let defs = command_schema
        .get("$defs")
        .and_then(|defs| defs.as_object())
        .expect("command schema should expose definitions");
    let command_envelope_ts = command_envelope_ts_contract();
    let command_result_ts = command_result_ts_contract();

    for command_name in [
        "getArtifactStatus",
        "refreshArtifactStatus",
        "retryArtifactGeneration",
        "resumeArtifactGeneration",
        "cancelArtifactGeneration",
        "getArtifactQuotaStatus",
        "runArtifactGarbageCollection",
    ] {
        assert!(
            command_schema.to_string().contains(command_name)
                && command_envelope_ts.contains(command_name),
            "generated command contracts should include artifact command {command_name}"
        );
    }

    let command_name_enum = defs
        .get("CommandName")
        .and_then(|schema| schema.get("enum"))
        .and_then(|entries| entries.as_array())
        .expect("CommandName should expose string enum")
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .expect("CommandName enum entry should be a string")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    let paired_command_names = command_schema
        .get("oneOf")
        .and_then(|entries| entries.as_array())
        .expect("CommandEnvelope schema should expose root command/payload pairing constraints")
        .iter()
        .filter_map(|entry| entry.pointer("/properties/command/const"))
        .map(|entry| {
            entry
                .as_str()
                .expect("paired command entry should be a string")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        paired_command_names, command_name_enum,
        "every CommandName variant must appear exactly once in root command/payload pairing constraints"
    );

    for expected_contract in [
        "GetArtifactStatusCommandPayload",
        "RefreshArtifactStatusCommandPayload",
        "ArtifactGenerationActionCommandPayload",
        "GetArtifactQuotaStatusCommandPayload",
        "RunArtifactGarbageCollectionCommandPayload",
        "ArtifactStatusSummary",
        "MaterialArtifactStatus",
        "ArtifactTaskStatus",
        "ArtifactQuotaStatus",
        "ArtifactMaintenanceResult",
        "DisplayableArtifactRef",
    ] {
        assert!(
            defs.contains_key(expected_contract)
                || command_envelope_ts.contains(&format!("export type {expected_contract}"))
                || command_result_ts.contains(&format!("export type {expected_contract}")),
            "artifact contracts should generate {expected_contract}"
        );
    }

    let artifact_contract_text = [
        "GetArtifactStatusCommandPayload",
        "RefreshArtifactStatusCommandPayload",
        "ArtifactGenerationActionCommandPayload",
        "GetArtifactQuotaStatusCommandPayload",
        "RunArtifactGarbageCollectionCommandPayload",
        "ArtifactStatusSummary",
        "MaterialArtifactStatus",
        "ArtifactGenerationTaskSummary",
        "ArtifactTaskStatus",
        "ArtifactQuotaStatus",
        "ArtifactMaintenanceResult",
        "DisplayableArtifactRef",
    ]
    .into_iter()
    .filter_map(|contract_name| {
        command_envelope_ts
            .lines()
            .chain(command_result_ts.lines())
            .find(|line| line.starts_with(&format!("export type {contract_name}")))
            .map(str::to_owned)
    })
    .collect::<Vec<_>>()
    .join("\n");

    for forbidden in [
        "artifactRoot",
        "blobRoot",
        "blobPath",
        "cacheKey",
        "fingerprint",
        "graphNode",
        "dirtyRange",
        "sqlite",
        "SQLite",
        "ffmpegArgs",
        "schedulerPriority",
    ] {
        assert!(
            !artifact_contract_text.contains(forbidden),
            "artifact transport contracts must not expose internal field {forbidden}"
        );
    }

    let mismatched_action = serde_json::json!({
        "command": "retryArtifactGeneration",
        "payload": {
            "kind": "cancelArtifactGeneration",
            "sessionId": "session-a",
            "bundlePath": "/tmp/project.veproj",
            "jobId": "job-a"
        }
    });
    assert!(
        serde_json::from_value::<CommandEnvelope>(mismatched_action).is_err(),
        "artifact generation action payloads must reject mismatched command names"
    );

    let mismatched_gc = serde_json::json!({
        "command": "getArtifactQuotaStatus",
        "payload": {
            "kind": "runArtifactGarbageCollection",
            "sessionId": "session-a",
            "bundlePath": "/tmp/project.veproj",
            "dryRun": true
        }
    });
    assert!(
        serde_json::from_value::<CommandEnvelope>(mismatched_gc).is_err(),
        "artifact maintenance payloads must reject mismatched command names"
    );
}

#[test]
fn schema_exports_exclude_timeline_edit_commands_from_public_envelope_contracts() {
    let command_schema: serde_json::Value =
        serde_json::from_str(&command_schema_json()).expect("command schema should parse");
    let defs = command_schema
        .get("$defs")
        .and_then(|defs| defs.as_object())
        .expect("command schema should expose definitions");
    let command_envelope_ts = command_envelope_ts_contract();

    for forbidden_contract in PUBLIC_TIMELINE_EDIT_PAYLOAD_CONTRACTS {
        assert!(
            !defs.contains_key(*forbidden_contract),
            "public command schema must not expose internal timeline edit payload {forbidden_contract}"
        );
        assert!(
            !command_envelope_ts.contains(&format!("export type {forbidden_contract}")),
            "generated public CommandEnvelope.ts must not export {forbidden_contract}"
        );
    }

    let command_name_enum = defs
        .get("CommandName")
        .and_then(|schema| schema.get("enum"))
        .and_then(|entries| entries.as_array())
        .expect("CommandName should expose string enum");
    for forbidden_command in PUBLIC_TIMELINE_EDIT_COMMAND_NAMES {
        assert!(
            !command_name_enum
                .iter()
                .any(|entry| entry.as_str() == Some(*forbidden_command)),
            "public CommandName must not expose internal timeline edit command {forbidden_command}"
        );
        assert!(
            !command_envelope_ts.contains(&format!("\"{forbidden_command}\"")),
            "generated public CommandEnvelope.ts must not advertise {forbidden_command}"
        );
    }
}

#[test]
fn schema_exports_include_text_command_contracts() {
    let command_envelope_ts = command_envelope_ts_contract();
    let draft_ts = ts_contract(&[
        export_decl::<TextAlignment>(),
        export_decl::<TextSegmentSource>(),
        export_decl::<TextFont>(),
        export_decl::<TextStroke>(),
        export_decl::<TextShadow>(),
        export_decl::<TextBackground>(),
        export_decl::<TextStyle>(),
        export_decl::<TextBox>(),
        export_decl::<TextLayoutRegion>(),
        export_decl::<TextWrapping>(),
        export_decl::<TextBubbleRef>(),
        export_decl::<TextEffectRef>(),
        export_decl::<TextSegment>(),
    ]);

    for expected_contract in [
        "TextSegment",
        "TextSegmentSource",
        "TextFont",
        "TextBox",
        "TextLayoutRegion",
        "TextWrapping",
        "TextBubbleRef",
        "TextEffectRef",
        "TextStyle",
        "TextAlignment",
    ] {
        assert!(
            draft_ts.contains(expected_contract),
            "draft TypeScript should include {expected_contract}"
        );
    }
    for forbidden_contract in [
        "AddTextSegmentCommandPayload",
        "AddTextSegmentIntentCommandPayload",
        "EditTextSegmentCommandPayload",
        "ImportSubtitleSrtCommandPayload",
        "ImportSubtitleSrtIntentCommandPayload",
    ] {
        assert!(
            !command_envelope_ts.contains(&format!("export type {forbidden_contract}")),
            "generated public CommandEnvelope.ts must not export {forbidden_contract}"
        );
    }
}

#[test]
fn schema_exports_include_audio_command_contracts() {
    let command_envelope_ts = command_envelope_ts_contract();
    let draft_ts = ts_contract(&[export_decl::<SegmentVolume>()]);

    for expected_contract in ["SegmentVolume"] {
        assert!(
            draft_ts.contains(expected_contract),
            "draft TypeScript should include {expected_contract}"
        );
    }
    for forbidden_contract in [
        "AddAudioSegmentCommandPayload",
        "AddAudioSegmentIntentCommandPayload",
        "SetSegmentVolumeCommandPayload",
        "SetTrackMuteCommandPayload",
    ] {
        assert!(
            !command_envelope_ts.contains(&format!("export type {forbidden_contract}")),
            "generated public CommandEnvelope.ts must not export {forbidden_contract}"
        );
    }
}

#[test]
fn schema_exports_include_phase15_audio_semantic_contracts() {
    let command_schema: serde_json::Value =
        serde_json::from_str(&command_schema_json()).expect("command schema should parse");
    let defs = command_schema
        .get("$defs")
        .and_then(|defs| defs.as_object())
        .expect("command schema should expose definitions");

    assert!(
        !defs.contains_key("UpdateSegmentAudioCommandPayload"),
        "public command schema must not expose UpdateSegmentAudioCommandPayload"
    );

    let draft_ts = ts_contract(&[
        export_decl::<SegmentAudio>(),
        export_decl::<AudioFade>(),
        export_decl::<AudioPanBalance>(),
        export_decl::<AudioEffectSlot>(),
    ]);
    let command_envelope_ts = command_envelope_ts_contract();
    for expected_export in [
        "export type SegmentAudio",
        "export type AudioFade",
        "export type AudioPanBalance",
        "export type AudioEffectSlot",
    ] {
        assert!(
            draft_ts.contains(expected_export),
            "generated TypeScript contracts should include {expected_export}"
        );
    }
    assert!(
        !command_envelope_ts.contains("UpdateSegmentAudioCommandPayload")
            && !command_envelope_ts.contains("\"updateSegmentAudio\""),
        "public CommandEnvelope.ts must not expose updateSegmentAudio"
    );

    for forbidden in [
        "waveformPath",
        "waveformBlob",
        "waveformPeaks",
        "artifactRoot",
        "sqlite",
        "cacheKey",
        "fingerprint",
        "nativeHandle",
        "outputDeviceHandle",
        "rawBuffer",
        "mixBuffer",
        "ringBuffer",
        "ffmpegAudioFilter",
        "filterComplex",
    ] {
        assert!(
            !draft_ts.contains(forbidden) && !command_envelope_ts.contains(forbidden),
            "audio contracts must not expose renderer-owned or derived field {forbidden}"
        );
    }
}

#[test]
fn schema_exports_include_phase15_audio_preview_binding_contracts() {
    let command_schema: serde_json::Value =
        serde_json::from_str(&command_schema_json()).expect("command schema should parse");
    let defs = command_schema
        .get("$defs")
        .and_then(|defs| defs.as_object())
        .expect("command schema should expose definitions");
    let command_envelope_ts = command_envelope_ts_contract();
    let command_result_ts = command_result_ts_contract();

    for command_name in [
        "createAudioPreviewSession",
        "playAudioPreview",
        "pauseAudioPreview",
        "stopAudioPreview",
        "seekAudioPreview",
        "cancelAudioPreview",
        "getAudioPreviewStatus",
        "listAudioOutputDevices",
        "selectAudioOutputDevice",
        "getWaveformDisplayPeaks",
        "refreshWaveformStatus",
    ] {
        assert_command_pairing_occurs_once(&command_schema, command_name);
        assert!(
            command_envelope_ts.contains(command_name),
            "generated TypeScript command names should include {command_name}"
        );
    }

    for expected_contract in [
        "AudioPreviewCommandPayload",
        "AudioPreviewCommandResponse",
        "AudioPreviewStatusResponse",
        "AudioOutputDeviceSummary",
        "WaveformDisplayPeaksResponse",
    ] {
        assert!(
            defs.contains_key(expected_contract)
                || command_envelope_ts.contains(&format!("export type {expected_contract}"))
                || command_result_ts.contains(&format!("export type {expected_contract}")),
            "audio preview binding contracts should generate {expected_contract}"
        );
    }

    for expected_field in [
        ("AudioPreviewCommandPayload", "projectSessionId"),
        ("AudioPreviewCommandPayload", "expectedRevision"),
        ("AudioPreviewCommandPayload", "sessionId"),
        ("AudioPreviewCommandPayload", "targetTime"),
        ("AudioPreviewCommandPayload", "playbackGeneration"),
        ("AudioPreviewCommandPayload", "deviceSelectionId"),
        ("AudioPreviewCommandPayload", "maxPeakBins"),
        ("AudioPreviewStatusResponse", "sessionId"),
        ("AudioPreviewStatusResponse", "generation"),
        ("AudioPreviewStatusResponse", "status"),
        ("AudioPreviewStatusResponse", "diagnostics"),
        ("AudioOutputDeviceSummary", "displayName"),
        ("AudioOutputDeviceSummary", "statusLabel"),
        ("WaveformDisplayPeaksResponse", "peaks"),
        ("WaveformDisplayPeaksResponse", "requestedPeakBins"),
    ] {
        let (contract_name, field_name) = expected_field;
        assert!(
            defs.get(contract_name)
                .and_then(|schema| schema.pointer(&format!("/properties/{field_name}")))
                .is_some(),
            "{contract_name} should expose safe field {field_name}"
        );
    }

    let audio_ts_text = [
        "AudioPreviewCommandPayload",
        "AudioPreviewCommandResponse",
        "AudioPreviewStatusResponse",
        "AudioOutputDeviceSummary",
        "WaveformDisplayPeak",
        "WaveformDisplayPeaksResponse",
    ]
    .into_iter()
    .filter_map(|contract_name| {
        command_envelope_ts
            .lines()
            .chain(command_result_ts.lines())
            .find(|line| line.starts_with(&format!("export type {contract_name}")))
            .map(str::to_owned)
    })
    .collect::<Vec<_>>()
    .join("\n");
    let audio_schema_text = serde_json::to_string(
        &[
            defs.get("AudioPreviewCommandPayload"),
            defs.get("AudioPreviewCommandResponse"),
            defs.get("AudioPreviewStatusResponse"),
            defs.get("AudioOutputDeviceSummary"),
            defs.get("WaveformDisplayPeak"),
            defs.get("WaveformDisplayPeaksResponse"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>(),
    )
    .expect("audio contract defs should serialize");
    let audio_contract_text = format!("{audio_ts_text}\n{audio_schema_text}");

    for forbidden in [
        "AudioGraph",
        "DSP",
        "mixBuffer",
        "ringBuffer",
        "outputDeviceHandle",
        "CoreAudio",
        "WASAPI",
        "cpal",
        "rubato",
        "FFmpeg",
        "SQLite",
        "blobPath",
        "artifactRoot",
        "cacheKey",
        "fingerprint",
        "dirtyRange",
        "nativeHandle",
        "rawBuffer",
        "sampleBuffer",
        "streamConfig",
        "ffmpegFilter",
        "draft",
        "Draft",
    ] {
        assert!(
            !audio_contract_text.contains(forbidden),
            "audio binding contracts must not expose internal field or implementation term {forbidden}"
        );
    }
}

#[test]
fn schema_exports_include_preview_command_contracts() {
    let schema_json = command_schema_json();
    let command_envelope_ts = command_envelope_ts_contract();
    let command_result_ts = command_result_ts_contract();

    for expected_contract in [
        "PreviewOutputProfile",
        "PreviewArtifactResponse",
        "PreviewStatus",
        "PreviewDiagnosticKind",
    ] {
        assert!(
            schema_json.contains(expected_contract)
                || command_envelope_ts.contains(expected_contract)
                || command_result_ts.contains(expected_contract),
            "preview command contracts should include {expected_contract}"
        );
    }

    for removed_contract in [
        "PreviewDecodeRequest",
        "ReleasePreviewFrameCommandPayload",
        "PreviewFrameStoragePreference",
        "RequestPreviewFrameCommandPayload",
        "RequestPreviewSegmentCommandPayload",
        "InvalidatePreviewCacheCommandPayload",
        "PreviewCacheEntryRef",
        "DecodedPreviewFrameResponse",
        "PreviewFrameReleaseResponse",
        "PreviewFrameStorageKind",
        "PreviewDecodeDiagnostic",
        "PreviewCacheInvalidationResponse",
        "requestPreviewDecode",
        "releasePreviewFrame",
        "requestPreviewFrame",
        "requestPreviewSegment",
        "invalidatePreviewCache",
    ] {
        assert!(
            !schema_json.contains(removed_contract)
                && !command_envelope_ts.contains(removed_contract)
                && !command_result_ts.contains(removed_contract),
            "generic preview command contract {removed_contract} must not be public"
        );
    }

    for forbidden in [
        "ffmpegArgs",
        "filterComplex",
        "cacheKeyFormula",
        "nativePointer",
        "rawHandle",
        "ArrayBuffer",
        "Uint8Array",
        "bytes",
        "pixels",
    ] {
        assert!(
            !command_envelope_ts.contains(forbidden) && !command_result_ts.contains(forbidden),
            "preview contracts must not expose renderer-owned {forbidden}"
        );
    }
}

#[test]
fn schema_exports_include_export_command_contracts() {
    let schema_json = command_schema_json();
    let command_envelope_ts = command_envelope_ts_contract();
    let command_result_ts = command_result_ts_contract();

    for expected_contract in [
        "StartExportCommandPayload",
        "GetExportJobStatusCommandPayload",
        "CancelExportCommandPayload",
        "ExportPreset",
        "ExportJobStatusResponse",
        "ExportJobPhase",
        "ExportValidationReport",
        "ExportDiagnostic",
        "ExportDiagnosticKind",
    ] {
        assert!(
            schema_json.contains(expected_contract)
                || command_envelope_ts.contains(expected_contract)
                || command_result_ts.contains(expected_contract),
            "export command contracts should include {expected_contract}"
        );
    }

    for forbidden in [
        "ffmpegArgs",
        "filterComplex",
        "filterScript",
        "sidecars",
        "processHandle",
        "validationExpectation",
    ] {
        assert!(
            !command_envelope_ts.contains(forbidden) && !command_result_ts.contains(forbidden),
            "export contracts must not expose renderer-owned {forbidden}"
        );
    }
}

#[test]
fn schema_exports_include_runtime_capability_command_contracts() {
    let schema_json = command_schema_json();
    let command_envelope_ts = command_envelope_ts_contract();
    let command_result_ts = command_result_ts_contract();

    for expected_contract in [
        "ProbeRuntimeCapabilitiesCommandPayload",
        "RuntimeCapabilityReport",
        "RuntimeBinaryCapability",
        "RuntimeFeatureCapability",
        "RuntimeFontCapability",
        "RuntimeLicensePosture",
        "RuntimeMediaIoCapabilities",
        "RuntimeWindowsMediaIoCapabilities",
        "RuntimeMacosMediaIoCapabilities",
        "RuntimeCodecCapability",
        "RuntimePixelFormatCapability",
        "RuntimeTextureInteropCapability",
        "RuntimeFallbackLadderCapability",
        "RuntimeFallbackDecodePathCapability",
        "RuntimeMediaIoFallbackReason",
        "RuntimeSelectedDecodePath",
        "RuntimeTextureBackend",
        "RuntimeDeviceId",
        "RuntimeVideoPixelFormat",
        "RuntimeColorPrimaries",
        "RuntimeColorTransfer",
        "RuntimeColorMatrix",
        "RuntimeColorRange",
        "RuntimeColorDiagnostic",
        "RuntimeVideoColorMetadata",
    ] {
        assert!(
            schema_json.contains(expected_contract)
                || command_envelope_ts.contains(expected_contract)
                || command_result_ts.contains(expected_contract),
            "runtime capability contracts should include {expected_contract}"
        );
    }

    for expected_field in [
        "mediaIo",
        "windows",
        "macos",
        "codecs",
        "pixelFormats",
        "textureInterop",
        "fallbackLadder",
        "compatibleWithPreviewDevice",
        "backend",
        "deviceId",
    ] {
        assert!(
            schema_json.contains(expected_field) || command_result_ts.contains(expected_field),
            "runtime media IO contracts should include {expected_field}"
        );
    }

    assert_handle_safe_runtime_contracts_do_not_expose_raw_payloads(&schema_json);
    assert_handle_safe_runtime_contracts_do_not_expose_raw_payloads(&command_result_ts);

    assert!(
        schema_json.contains("probeRuntimeCapabilities")
            && command_envelope_ts.contains("probeRuntimeCapabilities"),
        "runtime capability command should be generated from Rust contracts"
    );
}

#[test]
fn schema_exports_include_phase12_source_guard_script_wiring() {
    let root = project_root();
    let package_json =
        fs::read_to_string(root.join("package.json")).expect("package.json should exist");
    let guard_path = root.join("scripts/phase12-source-guards.sh");

    assert!(
        package_json.contains("\"test:phase12-source-guards\""),
        "package.json should expose the Phase 12 source guard"
    );
    assert!(
        guard_path.exists(),
        "Phase 12 source guard script should exist"
    );

    let guard = fs::read_to_string(guard_path).expect("Phase 12 source guard should be readable");
    for required_boundary in [
        "MediaFoundation",
        "VideoToolbox",
        "AVFoundation",
        "ArrayBuffer",
        "Uint8Array",
        "nativePointer",
        "rawHandle",
    ] {
        assert!(
            guard.contains(required_boundary),
            "Phase 12 source guard should reject {required_boundary}"
        );
    }
}

#[test]
fn schema_exports_include_canvas_config_and_command_contracts() {
    let command_envelope_ts = command_envelope_ts_contract();
    let draft_ts = ts_contract(&[
        export_decl::<CanvasAspectRatioPreset>(),
        export_decl::<CanvasAspectRatio>(),
        export_decl::<CanvasBackgroundCapability>(),
        export_decl::<CanvasBackground>(),
        export_decl::<CanvasAdaptationPolicy>(),
        export_decl::<DraftCanvasConfig>(),
    ]);

    for expected_contract in [
        "DraftCanvasConfig",
        "CanvasAspectRatio",
        "CanvasAspectRatioPreset",
        "CanvasBackground",
        "CanvasBackgroundCapability",
    ] {
        assert!(
            draft_ts.contains(expected_contract),
            "draft TypeScript should include {expected_contract}"
        );
    }

    assert!(
        !command_envelope_ts.contains("UpdateDraftCanvasConfigCommandPayload")
            && !command_envelope_ts.contains("\"updateDraftCanvasConfig\""),
        "public CommandEnvelope.ts must not expose updateDraftCanvasConfig"
    );
}

#[test]
fn schema_exports_include_segment_visual_and_command_contracts() {
    let command_envelope_ts = command_envelope_ts_contract();
    let draft_ts = ts_contract(&[
        export_decl::<SegmentPosition>(),
        export_decl::<SegmentScale>(),
        export_decl::<SegmentRotation>(),
        export_decl::<SegmentOpacity>(),
        export_decl::<SegmentCrop>(),
        export_decl::<SegmentAnchor>(),
        export_decl::<SegmentTransform>(),
        export_decl::<SegmentFitMode>(),
        export_decl::<SegmentBackgroundFilling>(),
        export_decl::<SegmentBlendMode>(),
        export_decl::<SegmentMask>(),
        export_decl::<SegmentVisual>(),
    ]);

    for expected_contract in [
        "SegmentPosition",
        "SegmentScale",
        "SegmentRotation",
        "SegmentOpacity",
        "SegmentCrop",
        "SegmentAnchor",
        "SegmentTransform",
        "SegmentFitMode",
        "SegmentBackgroundFilling",
        "SegmentBlendMode",
        "SegmentMask",
        "SegmentVisual",
    ] {
        assert!(
            draft_ts.contains(expected_contract),
            "draft TypeScript should include {expected_contract}"
        );
    }

    assert!(
        !command_envelope_ts.contains("UpdateSegmentVisualCommandPayload")
            && !command_envelope_ts.contains("\"updateSegmentVisual\""),
        "public CommandEnvelope.ts must not expose updateSegmentVisual"
    );
}

#[test]
fn schema_exports_include_keyframe_command_contracts() {
    let command_envelope_ts = command_envelope_ts_contract();
    let draft_ts = ts_contract(&[
        export_decl::<KeyframeProperty>(),
        export_decl::<KeyframeValue>(),
        export_decl::<KeyframeInterpolation>(),
        export_decl::<KeyframeEasing>(),
        export_decl::<Keyframe>(),
    ]);

    for expected_contract in [
        "KeyframeProperty",
        "KeyframeValue",
        "KeyframeInterpolation",
        "KeyframeEasing",
        "Keyframe",
    ] {
        assert!(
            draft_ts.contains(expected_contract),
            "draft TypeScript should include {expected_contract}"
        );
    }

    assert!(
        !command_envelope_ts.contains("SetSegmentKeyframeCommandPayload")
            && !command_envelope_ts.contains("RemoveSegmentKeyframeCommandPayload")
            && !command_envelope_ts.contains("\"setSegmentKeyframe\"")
            && !command_envelope_ts.contains("\"removeSegmentKeyframe\""),
        "public CommandEnvelope.ts must not expose keyframe edit commands"
    );
}

#[test]
fn schema_exports_include_retime_command_contracts() {
    let command_envelope_ts = command_envelope_ts_contract();
    let command_result_ts = command_result_ts_contract();
    let retime_payload_ts = ts_contract(&[
        export_decl::<SetSegmentRetimeCommandPayload>(),
        export_decl::<ClearSegmentRetimeCommandPayload>(),
    ]);
    let draft_ts = ts_contract(&[
        export_decl::<SpeedRatio>(),
        export_decl::<SpeedCurvePoint>(),
        export_decl::<RetimeMode>(),
        export_decl::<AudioRetimePolicy>(),
        export_decl::<SegmentRetiming>(),
    ]);

    for expected_export in [
        "export type SpeedRatio",
        "export type SpeedCurvePoint",
        "export type RetimeMode",
        "export type AudioRetimePolicy",
        "export type SegmentRetiming",
    ] {
        assert!(
            draft_ts.contains(expected_export),
            "draft TypeScript should include retime semantic contract {expected_export}"
        );
    }

    for expected_payload_contract in [
        "export type SetSegmentRetimeCommandPayload",
        "retiming: SegmentRetiming",
        "export type ClearSegmentRetimeCommandPayload",
    ] {
        assert!(
            retime_payload_ts.contains(expected_payload_contract),
            "internal retime payload contract should generate {expected_payload_contract}"
        );
    }

    for expected_delta in ["setSegmentRetime", "clearSegmentRetime"] {
        assert!(
            command_result_ts.contains(expected_delta),
            "CommandResultEnvelope.ts should expose retime delta name {expected_delta}"
        );
        assert!(
            !command_envelope_ts.contains(expected_delta),
            "public CommandEnvelope.ts must not expose internal timeline retime command {expected_delta}"
        );
    }

    for forbidden_contract in [
        "SetSegmentRetimeCommandPayload",
        "ClearSegmentRetimeCommandPayload",
    ] {
        assert!(
            !command_envelope_ts.contains(forbidden_contract),
            "public CommandEnvelope.ts must not export internal retime payload {forbidden_contract}"
        );
    }
}

#[test]
fn schema_exports_include_transition_command_contracts() {
    let command_envelope_ts = command_envelope_ts_contract();
    let command_result_ts = command_result_ts_contract();
    let transition_payload_ts = ts_contract(&[
        export_decl::<AddTransitionCommandPayload>(),
        export_decl::<UpdateTransitionDurationCommandPayload>(),
        export_decl::<RemoveTransitionCommandPayload>(),
    ]);
    let draft_ts = ts_contract(&[
        export_decl::<TransitionKind>(),
        export_decl::<TransitionReference>(),
        export_decl::<TrackTransition>(),
    ]);

    for expected_export in [
        "export type TransitionKind",
        "export type TransitionReference",
        "export type TrackTransition",
    ] {
        assert!(
            draft_ts.contains(expected_export),
            "draft TypeScript should include transition semantic contract {expected_export}"
        );
    }

    for expected_payload_contract in [
        "export type AddTransitionCommandPayload",
        "reference: TransitionReference",
        "duration: Microseconds",
        "export type UpdateTransitionDurationCommandPayload",
        "export type RemoveTransitionCommandPayload",
    ] {
        assert!(
            transition_payload_ts.contains(expected_payload_contract),
            "internal transition payload contract should generate {expected_payload_contract}"
        );
    }

    for expected_delta in [
        "addTransition",
        "updateTransitionDuration",
        "removeTransition",
    ] {
        assert!(
            command_result_ts.contains(expected_delta),
            "CommandResultEnvelope.ts should expose transition delta name {expected_delta}"
        );
        assert!(
            !command_envelope_ts.contains(expected_delta),
            "public CommandEnvelope.ts must not expose internal transition command {expected_delta}"
        );
    }

    for forbidden_contract in [
        "AddTransitionCommandPayload",
        "UpdateTransitionDurationCommandPayload",
        "RemoveTransitionCommandPayload",
    ] {
        assert!(
            !command_envelope_ts.contains(forbidden_contract),
            "public CommandEnvelope.ts must not export internal transition payload {forbidden_contract}"
        );
    }
}

fn export_decl<T>() -> String
where
    T: TS + 'static,
{
    format!("export {}\n", T::decl(&ts_config()))
}

fn ts_config() -> Config {
    Config::new().with_large_int("number")
}

fn command_envelope_ts_contract() -> String {
    ts_contract_with_prelude(
        "import type { Draft, MaterialId, MaterialKind, Microseconds, SegmentId, TargetTimerange, TrackId } from \"./Draft\";\n\n",
        &[
            export_decl::<CommandName>(),
            export_decl::<PingCommandPayload>(),
            export_decl::<VersionCommandPayload>(),
            export_decl::<ProbeMediaRuntimeCommandPayload>(),
            export_decl::<ProbeRuntimeCapabilitiesCommandPayload>(),
            export_decl::<PreviewOutputProfile>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<DirtyRange>(),
            export_decl::<AudioPreviewCommandPayload>(),
            export_decl::<GetArtifactStatusCommandPayload>(),
            export_decl::<RefreshArtifactStatusCommandPayload>(),
            export_decl::<ArtifactGenerationActionCommandPayload>(),
            export_decl::<GetArtifactQuotaStatusCommandPayload>(),
            export_decl::<RunArtifactGarbageCollectionCommandPayload>(),
            export_decl::<ExportPreset>(),
            export_decl::<ExportPrepDirtyFacts>(),
            export_decl::<StartExportCommandPayload>(),
            export_decl::<GetExportJobStatusCommandPayload>(),
            export_decl::<CancelExportCommandPayload>(),
            export_decl::<TimelineSelection>(),
            export_decl::<SnappingSettings>(),
            export_decl::<CommandHistorySnapshot>(),
            export_decl::<CommandState>(),
            export_decl::<CommandPayload>(),
            export_decl::<CommandEnvelope>(),
        ],
    )
}

fn command_result_ts_contract() -> String {
    ts_contract_with_prelude(
        "import type { Draft, DraftId, KeyframeProperty, Material, MaterialId, MaterialStatus, Microseconds, RationalFrameRate, SegmentId, TargetTimerange, TrackId } from \"./Draft\";\nimport type { CommandState, ExportPreset, PreviewOutputProfile, TimelineSelection } from \"./CommandEnvelope\";\n\n",
        &[
            export_decl::<CommandErrorKind>(),
            export_decl::<CommandError>(),
            export_decl::<CommandEvent>(),
            export_decl::<CommandResultEnvelope<()>>(),
            export_decl::<PreviewStatus>(),
            export_decl::<PreviewDiagnosticKind>(),
            export_decl::<PreviewDiagnostic>(),
            export_decl::<PreviewArtifactResponse>(),
            export_decl::<AudioPreviewPlaybackStatus>(),
            export_decl::<AudioOutputDeviceStatus>(),
            export_decl::<WaveformDisplayStatus>(),
            export_decl::<AudioOutputDeviceSummary>(),
            export_decl::<AudioPreviewStatusResponse>(),
            export_decl::<AudioPreviewCommandResponse>(),
            export_decl::<WaveformDisplayPeak>(),
            export_decl::<WaveformDisplayPeaksResponse>(),
            export_decl::<ExportJobPhase>(),
            export_decl::<ExportDiagnosticKind>(),
            export_decl::<ExportDiagnostic>(),
            export_decl::<ExportValidationReport>(),
            export_decl::<ExportPrepDirtyFacts>(),
            export_decl::<ExportJobStatusResponse>(),
            export_decl::<RuntimeCapabilityStatus>(),
            export_decl::<RuntimeBinaryKind>(),
            export_decl::<RuntimeBinaryCapability>(),
            export_decl::<RuntimeFeatureCapability>(),
            export_decl::<RuntimeFontCapability>(),
            export_decl::<RuntimeLicensePosture>(),
            export_decl::<RuntimeMediaIoFallbackReason>(),
            export_decl::<RuntimeSelectedDecodePath>(),
            export_decl::<RuntimeTextureBackend>(),
            export_decl::<RuntimeVideoPixelFormat>(),
            export_decl::<RuntimeColorPrimaries>(),
            export_decl::<RuntimeColorTransfer>(),
            export_decl::<RuntimeColorMatrix>(),
            export_decl::<RuntimeColorRange>(),
            export_decl::<RuntimeColorDiagnostic>(),
            export_decl::<RuntimeVideoColorMetadata>(),
            export_decl::<RuntimeDeviceId>(),
            export_decl::<RuntimeWindowsMediaIoCapabilities>(),
            export_decl::<RuntimeMacosMediaIoCapabilities>(),
            export_decl::<RuntimeCodecCapability>(),
            export_decl::<RuntimePixelFormatCapability>(),
            export_decl::<RuntimeTextureInteropCapability>(),
            export_decl::<RuntimeFallbackDecodePathCapability>(),
            export_decl::<RuntimeFallbackLadderCapability>(),
            export_decl::<RuntimeMediaIoCapabilities>(),
            export_decl::<RuntimeCapabilityReport>(),
            export_decl::<MissingMaterialCommandDiagnosticKind>(),
            export_decl::<MissingMaterialCommandDiagnostic>(),
            export_decl::<ChangedEntity>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRange>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<InvalidationScope>(),
            export_decl::<CommandDeltaName>(),
            export_decl::<CommandDelta>(),
            export_decl::<TimelineCommandResponse>(),
            export_decl::<ArtifactTaskStatus>(),
            export_decl::<DisplayableArtifactRef>(),
            export_decl::<MaterialArtifactStatus>(),
            export_decl::<ArtifactGenerationTaskSummary>(),
            export_decl::<ArtifactQuotaStatus>(),
            export_decl::<ArtifactStatusSummary>(),
            export_decl::<ArtifactMaintenanceResult>(),
        ],
    )
}

fn assert_handle_safe_runtime_contracts_do_not_expose_raw_payloads(contract: &str) {
    for forbidden in [
        "nativePointer",
        "rawHandle",
        "ArrayBuffer",
        "Uint8Array",
        "\"bytes\"",
        "\"pixels\"",
        "\"rgba\"",
        "\"bgra\"",
        "bytes:",
        "bytes?:",
        "pixels:",
        "pixels?:",
        "rgba:",
        "rgba?:",
        "bgra:",
        "bgra?:",
    ] {
        assert!(
            !contract.contains(forbidden),
            "handle-capable runtime contracts must not expose raw payload field {forbidden}"
        );
    }
}

fn property_references_def(contract: &serde_json::Value, field: &str, expected_ref: &str) -> bool {
    let pointer = format!("/properties/{field}");
    let Some(property) = contract.pointer(&pointer) else {
        return false;
    };

    property
        .get("$ref")
        .and_then(|value| value.as_str())
        .is_some_and(|value| value == expected_ref)
        || property
            .get("anyOf")
            .and_then(|value| value.as_array())
            .is_some_and(|variants| {
                variants.iter().any(|variant| {
                    variant
                        .get("$ref")
                        .and_then(|value| value.as_str())
                        .is_some_and(|value| value == expected_ref)
                })
            })
}

fn assert_command_pairing_occurs_once(command_schema: &serde_json::Value, command_name: &str) {
    let count = command_schema
        .get("oneOf")
        .and_then(|entries| entries.as_array())
        .expect("CommandEnvelope schema should expose root command/payload pairing constraints")
        .iter()
        .filter(|entry| {
            entry
                .pointer("/properties/command/const")
                .and_then(|value| value.as_str())
                .is_some_and(|value| value == command_name)
        })
        .count();
    assert_eq!(
        count, 1,
        "{command_name} should appear exactly once in command/payload pairing constraints"
    );
}

fn ts_contract(declarations: &[String]) -> String {
    ts_contract_with_prelude("", declarations)
}

fn ts_contract_with_prelude(prelude: &str, declarations: &[String]) -> String {
    let mut ts = String::from(
        "// This file was generated by Rust ts-rs declarations. Do not edit this file manually.\n\n",
    );
    ts.push_str(prelude);
    for declaration in declarations {
        ts.push_str(declaration);
    }
    ts
}

fn assert_or_update_contract_file(path: impl AsRef<Path>, expected: &str) {
    let path = path.as_ref();

    if env::var_os("VE_UPDATE_GENERATED_CONTRACTS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        fs::create_dir_all(path.parent().expect("contract path should have parent"))
            .expect("contract directory should be created");
        fs::write(path, expected).expect("contract artifact should be written");
        return;
    }

    let actual = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "committed contract artifact should be readable at {}: {error}",
            path.display()
        )
    });
    assert_eq!(
        actual,
        expected,
        "generated contract artifact is stale: {}. Run with VE_UPDATE_GENERATED_CONTRACTS=1 to refresh.",
        path.display()
    );
}

#[test]
fn schema_fixtures_validate_command_contracts() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/draft");
    let positive = BTreeSet::from(["minimal-command.json"]);
    let negative = BTreeSet::from([
        "invalid-mismatched-command-payload.json",
        "invalid-timeline-command.json",
        "minimal-timeline-command.json",
        "invalid-unknown-field.json",
    ]);

    let fixture_names = fs::read_dir(&fixture_dir)
        .expect("fixtures/draft directory should exist")
        .filter_map(|entry| {
            let entry = entry.expect("fixture directory entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                return None;
            }
            assert_eq!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("json"),
                "fixtures/draft should only contain JSON fixtures: {}",
                path.display()
            );
            Some(
                entry
                    .file_name()
                    .into_string()
                    .expect("fixture names should be UTF-8"),
            )
        })
        .collect::<BTreeSet<_>>();

    let expected = positive.union(&negative).copied().collect::<BTreeSet<_>>();
    let actual = fixture_names
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "every draft JSON fixture must be explicitly classified"
    );

    let schema_json: serde_json::Value = serde_json::from_str(&command_schema_json())
        .expect("generated command schema should parse");
    let schema =
        jsonschema::validator_for(&schema_json).expect("generated command schema should compile");

    for fixture_name in positive {
        let value = read_fixture(&fixture_dir, fixture_name);
        serde_json::from_value::<CommandEnvelope>(value.clone())
            .expect("positive fixture should deserialize through Rust model");
        schema
            .validate(&value)
            .expect("positive fixture should validate against JSON Schema");
    }

    for fixture_name in negative {
        let value = read_fixture(&fixture_dir, fixture_name);
        assert!(
            serde_json::from_value::<CommandEnvelope>(value.clone()).is_err(),
            "negative fixture should fail Rust model deserialization: {fixture_name}"
        );
        assert!(
            schema.validate(&value).is_err(),
            "negative fixture should fail JSON Schema validation: {fixture_name}"
        );
    }
}

fn read_fixture(fixture_dir: &Path, fixture_name: &str) -> serde_json::Value {
    serde_json::from_slice(
        &fs::read(fixture_dir.join(fixture_name)).expect("fixture should be readable"),
    )
    .expect("fixture should parse as JSON")
}

fn command_schema_json() -> String {
    let schema = schema_for!(CommandEnvelope);
    let mut schema_value =
        serde_json::to_value(schema).expect("command schema should serialize to JSON value");
    include_command_contract_schema::<AudioPreviewCommandPayload>(
        &mut schema_value,
        "AudioPreviewCommandPayload",
    );
    include_command_contract_schema::<AudioPreviewPlaybackStatus>(
        &mut schema_value,
        "AudioPreviewPlaybackStatus",
    );
    include_command_contract_schema::<AudioOutputDeviceStatus>(
        &mut schema_value,
        "AudioOutputDeviceStatus",
    );
    include_command_contract_schema::<WaveformDisplayStatus>(
        &mut schema_value,
        "WaveformDisplayStatus",
    );
    include_command_contract_schema::<AudioOutputDeviceSummary>(
        &mut schema_value,
        "AudioOutputDeviceSummary",
    );
    include_command_contract_schema::<AudioPreviewStatusResponse>(
        &mut schema_value,
        "AudioPreviewStatusResponse",
    );
    include_command_contract_schema::<AudioPreviewCommandResponse>(
        &mut schema_value,
        "AudioPreviewCommandResponse",
    );
    include_command_contract_schema::<WaveformDisplayPeak>(
        &mut schema_value,
        "WaveformDisplayPeak",
    );
    include_command_contract_schema::<WaveformDisplayPeaksResponse>(
        &mut schema_value,
        "WaveformDisplayPeaksResponse",
    );
    include_command_contract_schema::<ExportPrepDirtyFacts>(
        &mut schema_value,
        "ExportPrepDirtyFacts",
    );
    include_command_contract_schema::<StartExportCommandPayload>(
        &mut schema_value,
        "StartExportCommandPayload",
    );
    include_command_contract_schema::<GetExportJobStatusCommandPayload>(
        &mut schema_value,
        "GetExportJobStatusCommandPayload",
    );
    include_command_contract_schema::<CancelExportCommandPayload>(
        &mut schema_value,
        "CancelExportCommandPayload",
    );
    include_command_contract_schema::<ProbeRuntimeCapabilitiesCommandPayload>(
        &mut schema_value,
        "ProbeRuntimeCapabilitiesCommandPayload",
    );
    include_command_contract_schema::<PreviewArtifactResponse>(
        &mut schema_value,
        "PreviewArtifactResponse",
    );
    include_command_contract_schema::<ExportValidationReport>(
        &mut schema_value,
        "ExportValidationReport",
    );
    include_command_contract_schema::<ExportJobStatusResponse>(
        &mut schema_value,
        "ExportJobStatusResponse",
    );
    include_command_contract_schema::<RuntimeCapabilityStatus>(
        &mut schema_value,
        "RuntimeCapabilityStatus",
    );
    include_command_contract_schema::<RuntimeBinaryKind>(&mut schema_value, "RuntimeBinaryKind");
    include_command_contract_schema::<RuntimeBinaryCapability>(
        &mut schema_value,
        "RuntimeBinaryCapability",
    );
    include_command_contract_schema::<RuntimeFeatureCapability>(
        &mut schema_value,
        "RuntimeFeatureCapability",
    );
    include_command_contract_schema::<RuntimeFontCapability>(
        &mut schema_value,
        "RuntimeFontCapability",
    );
    include_command_contract_schema::<RuntimeLicensePosture>(
        &mut schema_value,
        "RuntimeLicensePosture",
    );
    include_command_contract_schema::<RuntimeMediaIoFallbackReason>(
        &mut schema_value,
        "RuntimeMediaIoFallbackReason",
    );
    include_command_contract_schema::<RuntimeSelectedDecodePath>(
        &mut schema_value,
        "RuntimeSelectedDecodePath",
    );
    include_command_contract_schema::<RuntimeTextureBackend>(
        &mut schema_value,
        "RuntimeTextureBackend",
    );
    include_command_contract_schema::<RuntimeVideoPixelFormat>(
        &mut schema_value,
        "RuntimeVideoPixelFormat",
    );
    include_command_contract_schema::<RuntimeColorPrimaries>(
        &mut schema_value,
        "RuntimeColorPrimaries",
    );
    include_command_contract_schema::<RuntimeColorTransfer>(
        &mut schema_value,
        "RuntimeColorTransfer",
    );
    include_command_contract_schema::<RuntimeColorMatrix>(&mut schema_value, "RuntimeColorMatrix");
    include_command_contract_schema::<RuntimeColorRange>(&mut schema_value, "RuntimeColorRange");
    include_command_contract_schema::<RuntimeColorDiagnostic>(
        &mut schema_value,
        "RuntimeColorDiagnostic",
    );
    include_command_contract_schema::<RuntimeVideoColorMetadata>(
        &mut schema_value,
        "RuntimeVideoColorMetadata",
    );
    include_command_contract_schema::<RuntimeDeviceId>(&mut schema_value, "RuntimeDeviceId");
    include_command_contract_schema::<RuntimeWindowsMediaIoCapabilities>(
        &mut schema_value,
        "RuntimeWindowsMediaIoCapabilities",
    );
    include_command_contract_schema::<RuntimeMacosMediaIoCapabilities>(
        &mut schema_value,
        "RuntimeMacosMediaIoCapabilities",
    );
    include_command_contract_schema::<RuntimeCodecCapability>(
        &mut schema_value,
        "RuntimeCodecCapability",
    );
    include_command_contract_schema::<RuntimePixelFormatCapability>(
        &mut schema_value,
        "RuntimePixelFormatCapability",
    );
    include_command_contract_schema::<RuntimeTextureInteropCapability>(
        &mut schema_value,
        "RuntimeTextureInteropCapability",
    );
    include_command_contract_schema::<RuntimeFallbackDecodePathCapability>(
        &mut schema_value,
        "RuntimeFallbackDecodePathCapability",
    );
    include_command_contract_schema::<RuntimeFallbackLadderCapability>(
        &mut schema_value,
        "RuntimeFallbackLadderCapability",
    );
    include_command_contract_schema::<RuntimeMediaIoCapabilities>(
        &mut schema_value,
        "RuntimeMediaIoCapabilities",
    );
    include_command_contract_schema::<RuntimeCapabilityReport>(
        &mut schema_value,
        "RuntimeCapabilityReport",
    );
    constrain_current_draft_schema_version(&mut schema_value);
    constrain_rational_frame_rate(&mut schema_value);
    constrain_canvas_config(&mut schema_value);
    constrain_text_contracts(&mut schema_value);
    constrain_keyframe_contracts(&mut schema_value);
    schema_value
        .as_object_mut()
        .expect("command schema should be a JSON object")
        .insert("oneOf".to_string(), command_payload_pairing_constraints());

    serde_json::to_string_pretty(&schema_value).expect("command schema should serialize")
}

fn draft_schema_json() -> String {
    let mut schema = schema_for!(Draft);
    constrain_current_draft_schema_version_schema(&mut schema);
    constrain_rational_frame_rate_schema(&mut schema);
    constrain_canvas_config_schema(&mut schema);
    constrain_text_contracts_schema(&mut schema);
    constrain_keyframe_contracts_schema(&mut schema);
    include_phase19_effect_contract_schemas(&mut schema);
    serde_json::to_string_pretty(&schema).expect("draft schema should serialize")
}

fn constrain_current_draft_schema_version(schema_value: &mut serde_json::Value) {
    schema_value["$defs"]["DraftSchemaVersion"] = current_draft_schema_version_schema();
}

fn constrain_current_draft_schema_version_schema(schema: &mut Schema) {
    let defs = schema
        .ensure_object()
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("generated draft schema should contain $defs");
    defs.insert(
        "DraftSchemaVersion".to_owned(),
        current_draft_schema_version_schema(),
    );
}

fn include_command_contract_schema<T>(schema_value: &mut serde_json::Value, name: &str)
where
    T: schemars::JsonSchema,
{
    let contract_schema = schema_for!(T);
    let mut contract_value = serde_json::to_value(contract_schema)
        .expect("command contract schema should serialize to JSON value");

    if let Some(contract_defs) = contract_value
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
    {
        let defs = schema_value
            .get_mut("$defs")
            .and_then(serde_json::Value::as_object_mut)
            .expect("command schema should contain $defs");
        for (def_name, def_schema) in std::mem::take(contract_defs) {
            defs.insert(def_name, def_schema);
        }
    }

    let contract_object = contract_value
        .as_object_mut()
        .expect("command contract schema should be an object");
    contract_object.remove("$schema");
    contract_object.remove("$defs");
    schema_value
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("command schema should contain $defs")
        .insert(name.to_owned(), contract_value);
}

fn include_phase19_effect_contract_schemas(schema: &mut Schema) {
    let mut schema_value = schema.as_value().clone();
    for (name, include) in [
        (
            "CapabilitySurface",
            include_contract_schema::<CapabilitySurface> as fn(&mut serde_json::Value, &str),
        ),
        (
            "CapabilitySupport",
            include_contract_schema::<CapabilitySupport>,
        ),
        (
            "CapabilityCategory",
            include_contract_schema::<CapabilityCategory>,
        ),
        (
            "CapabilityReportItem",
            include_contract_schema::<CapabilityReportItem>,
        ),
        (
            "EffectCapabilityRegistry",
            include_contract_schema::<EffectCapabilityRegistry>,
        ),
        ("EffectKind", include_contract_schema::<EffectKind>),
        (
            "ExternalEffectReference",
            include_contract_schema::<ExternalEffectReference>,
        ),
        ("FilterKind", include_contract_schema::<FilterKind>),
        ("Filter", include_contract_schema::<Filter>),
        ("TransitionKind", include_contract_schema::<TransitionKind>),
        (
            "TransitionReference",
            include_contract_schema::<TransitionReference>,
        ),
        ("Transition", include_contract_schema::<Transition>),
        (
            "TrackTransition",
            include_contract_schema::<TrackTransition>,
        ),
        ("SpeedRatio", include_contract_schema::<SpeedRatio>),
        (
            "SpeedCurvePoint",
            include_contract_schema::<SpeedCurvePoint>,
        ),
        ("RetimeMode", include_contract_schema::<RetimeMode>),
        (
            "AudioRetimePolicy",
            include_contract_schema::<AudioRetimePolicy>,
        ),
        (
            "SegmentRetiming",
            include_contract_schema::<SegmentRetiming>,
        ),
        ("MaskKind", include_contract_schema::<MaskKind>),
        ("BlendModeKind", include_contract_schema::<BlendModeKind>),
    ] {
        include(&mut schema_value, name);
    }
    constrain_speed_ratio_contract(&mut schema_value);
    *schema = Schema::try_from(schema_value).expect("patched draft schema should remain valid");
}

fn include_contract_schema<T>(schema_value: &mut serde_json::Value, name: &str)
where
    T: schemars::JsonSchema,
{
    include_command_contract_schema::<T>(schema_value, name);
}

fn current_draft_schema_version_schema() -> serde_json::Value {
    json!({
        "type": "integer",
        "const": DraftSchemaVersion::CURRENT_VALUE
    })
}

fn constrain_rational_frame_rate(schema_value: &mut serde_json::Value) {
    let frame_rate = rational_frame_rate_schema_object(schema_value);
    frame_rate["properties"]["numerator"]["minimum"] = json!(1);
    frame_rate["properties"]["denominator"]["minimum"] = json!(1);
    assert_eq!(frame_rate["properties"]["numerator"]["minimum"], json!(1));
    assert_eq!(frame_rate["properties"]["denominator"]["minimum"], json!(1));
}

fn constrain_rational_frame_rate_schema(schema: &mut Schema) {
    let mut schema_value = schema.as_value().clone();
    constrain_rational_frame_rate(&mut schema_value);
    *schema = Schema::try_from(schema_value).expect("patched draft schema should remain valid");
}

fn constrain_speed_ratio_contract(schema_value: &mut serde_json::Value) {
    let defs = schema_value
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("generated schema should contain $defs");
    let speed_ratio = defs
        .get_mut("SpeedRatio")
        .expect("generated schema should contain SpeedRatio");
    speed_ratio["properties"]["numerator"]["minimum"] = json!(1);
    speed_ratio["properties"]["denominator"]["minimum"] = json!(1);
    assert_eq!(speed_ratio["properties"]["numerator"]["minimum"], json!(1));
    assert_eq!(
        speed_ratio["properties"]["denominator"]["minimum"],
        json!(1)
    );
}

fn constrain_canvas_config(schema_value: &mut serde_json::Value) {
    let defs = schema_value
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("generated schema should contain $defs");

    let canvas_config = defs
        .get_mut("DraftCanvasConfig")
        .expect("generated schema should contain DraftCanvasConfig");
    canvas_config["properties"]["width"]["minimum"] = json!(1);
    canvas_config["properties"]["height"]["minimum"] = json!(1);
    assert_eq!(canvas_config["properties"]["width"]["minimum"], json!(1));
    assert_eq!(canvas_config["properties"]["height"]["minimum"], json!(1));

    let aspect_ratio = defs
        .get_mut("CanvasAspectRatio")
        .expect("generated schema should contain CanvasAspectRatio");
    let custom_ratio = aspect_ratio
        .get_mut("oneOf")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|variants| {
            variants.iter_mut().find(|variant| {
                variant["properties"]["kind"]["const"] == serde_json::Value::String("custom".into())
            })
        })
        .expect("CanvasAspectRatio should contain custom ratio variant");
    custom_ratio["properties"]["numerator"]["minimum"] = json!(1);
    custom_ratio["properties"]["denominator"]["minimum"] = json!(1);
    assert_eq!(custom_ratio["properties"]["numerator"]["minimum"], json!(1));
    assert_eq!(
        custom_ratio["properties"]["denominator"]["minimum"],
        json!(1)
    );
}

fn constrain_canvas_config_schema(schema: &mut Schema) {
    let mut schema_value = schema.as_value().clone();
    constrain_canvas_config(&mut schema_value);
    *schema = Schema::try_from(schema_value).expect("patched draft schema should remain valid");
}

fn constrain_text_contracts(schema_value: &mut serde_json::Value) {
    let defs = schema_value
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("generated schema should contain $defs");

    let text_box = defs
        .get_mut("TextBox")
        .expect("generated schema should contain TextBox");
    constrain_uint_property(text_box, "widthMillis", 1, MAX_TEXT_LAYOUT_MILLIS);
    constrain_uint_property(text_box, "heightMillis", 1, MAX_TEXT_LAYOUT_MILLIS);

    let text_layout_region = defs
        .get_mut("TextLayoutRegion")
        .expect("generated schema should contain TextLayoutRegion");
    constrain_uint_property(text_layout_region, "xMillis", 0, MAX_TEXT_LAYOUT_MILLIS);
    constrain_uint_property(text_layout_region, "yMillis", 0, MAX_TEXT_LAYOUT_MILLIS);
    constrain_uint_property(text_layout_region, "widthMillis", 1, MAX_TEXT_LAYOUT_MILLIS);
    constrain_uint_property(
        text_layout_region,
        "heightMillis",
        1,
        MAX_TEXT_LAYOUT_MILLIS,
    );
    constrain_layout_region_sum_property(text_layout_region, "xMillis", "widthMillis");
    constrain_layout_region_sum_property(text_layout_region, "yMillis", "heightMillis");

    let text_style = defs
        .get_mut("TextStyle")
        .expect("generated schema should contain TextStyle");
    constrain_uint_min_property(text_style, "fontSize", 1);
    constrain_uint_property(
        text_style,
        "lineHeightMillis",
        MIN_TEXT_LINE_HEIGHT_MILLIS,
        MAX_TEXT_LINE_HEIGHT_MILLIS,
    );
    constrain_uint_property(
        text_style,
        "letterSpacingMillis",
        0,
        MAX_TEXT_LETTER_SPACING_MILLIS,
    );
    constrain_string_pattern_property(text_style, "color", TEXT_HEX_COLOR_PATTERN);

    let text_stroke = defs
        .get_mut("TextStroke")
        .expect("generated schema should contain TextStroke");
    constrain_string_pattern_property(text_stroke, "color", TEXT_HEX_COLOR_PATTERN);
    constrain_uint_min_property(text_stroke, "width", 1);

    let text_shadow = defs
        .get_mut("TextShadow")
        .expect("generated schema should contain TextShadow");
    constrain_string_pattern_property(text_shadow, "color", TEXT_HEX_COLOR_PATTERN);

    let text_background = defs
        .get_mut("TextBackground")
        .expect("generated schema should contain TextBackground");
    constrain_string_pattern_property(text_background, "color", TEXT_HEX_COLOR_PATTERN);
}

fn constrain_text_contracts_schema(schema: &mut Schema) {
    let mut schema_value = schema.as_value().clone();
    constrain_text_contracts(&mut schema_value);
    *schema = Schema::try_from(schema_value).expect("patched draft schema should remain valid");
}

fn constrain_keyframe_contracts(schema_value: &mut serde_json::Value) {
    let defs = schema_value
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("generated schema should contain $defs");

    let keyframe_value = defs
        .get_mut("KeyframeValue")
        .expect("generated schema should contain KeyframeValue");
    let color_variant = keyframe_value
        .get_mut("oneOf")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|variants| {
            variants.iter_mut().find(|variant| {
                variant["properties"]["kind"]["const"] == serde_json::Value::String("color".into())
            })
        })
        .expect("KeyframeValue should contain color variant");
    constrain_string_pattern_property(color_variant, "value", TEXT_HEX_COLOR_PATTERN);
}

fn constrain_keyframe_contracts_schema(schema: &mut Schema) {
    let mut schema_value = schema.as_value().clone();
    constrain_keyframe_contracts(&mut schema_value);
    *schema = Schema::try_from(schema_value).expect("patched draft schema should remain valid");
}

fn constrain_uint_property(
    object_schema: &mut serde_json::Value,
    property: &str,
    minimum: u32,
    maximum: u32,
) {
    object_schema["properties"][property]["minimum"] = json!(minimum);
    object_schema["properties"][property]["maximum"] = json!(maximum);
    assert_eq!(
        object_schema["properties"][property]["minimum"],
        json!(minimum)
    );
    assert_eq!(
        object_schema["properties"][property]["maximum"],
        json!(maximum)
    );
}

fn constrain_uint_min_property(
    object_schema: &mut serde_json::Value,
    property: &str,
    minimum: u32,
) {
    object_schema["properties"][property]["minimum"] = json!(minimum);
    assert_eq!(
        object_schema["properties"][property]["minimum"],
        json!(minimum)
    );
}

fn constrain_string_pattern_property(
    object_schema: &mut serde_json::Value,
    property: &str,
    pattern: &str,
) {
    object_schema["properties"][property]["pattern"] = json!(pattern);
    assert_eq!(
        object_schema["properties"][property]["pattern"],
        json!(pattern)
    );
}

fn constrain_layout_region_sum_property(
    object_schema: &mut serde_json::Value,
    offset_property: &str,
    size_property: &str,
) {
    let invalid_ranges = (1..=MAX_TEXT_LAYOUT_MILLIS)
        .map(|offset| {
            json!({
                "properties": {
                    offset_property: { "minimum": offset },
                    size_property: { "minimum": MAX_TEXT_LAYOUT_MILLIS + 1 - offset }
                }
            })
        })
        .collect::<Vec<_>>();

    object_schema
        .as_object_mut()
        .expect("object schema should be an object")
        .entry("allOf")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .expect("allOf should be an array")
        .push(json!({
            "not": {
                "anyOf": invalid_ranges
            }
        }));
}

fn rational_frame_rate_schema_object(
    schema_value: &mut serde_json::Value,
) -> &mut serde_json::Value {
    schema_value
        .get_mut("$defs")
        .and_then(|defs| defs.get_mut("RationalFrameRate"))
        .expect("generated schema should contain RationalFrameRate")
}

fn assert_draft_schema_rejects_zero_frame_rates(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("draft schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("draft schema should compile");

    assert!(
        schema.validate(&draft_value_with_frame_rate(0, 1)).is_err(),
        "draft schema should reject zero frame-rate numerator"
    );
    assert!(
        schema
            .validate(&draft_value_with_frame_rate(24, 0))
            .is_err(),
        "draft schema should reject zero frame-rate denominator"
    );
}

fn assert_draft_schema_rejects_invalid_canvas_config(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("draft schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("draft schema should compile");

    assert!(
        schema
            .validate(&draft_value_with_canvas_config(0, 1080, 16, 9))
            .is_err(),
        "draft schema should reject zero canvas width"
    );
    assert!(
        schema
            .validate(&draft_value_with_canvas_config(1920, 0, 16, 9))
            .is_err(),
        "draft schema should reject zero canvas height"
    );
    assert!(
        schema
            .validate(&draft_value_with_canvas_config(1920, 1080, 0, 9))
            .is_err(),
        "draft schema should reject zero custom aspect-ratio numerator"
    );
    assert!(
        schema
            .validate(&draft_value_with_canvas_config(1920, 1080, 16, 0))
            .is_err(),
        "draft schema should reject zero custom aspect-ratio denominator"
    );
}

fn assert_draft_schema_rejects_invalid_text_contracts(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("draft schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("draft schema should compile");

    for (case, value) in invalid_text_contract_drafts() {
        assert!(
            schema.validate(&value).is_err(),
            "draft schema should reject invalid text contract: {case}"
        );
    }
}

fn assert_draft_schema_rejects_invalid_keyframe_contracts(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("draft schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("draft schema should compile");

    for (case, value) in invalid_keyframe_contract_drafts() {
        assert!(
            schema.validate(&value).is_err(),
            "draft schema should reject invalid keyframe contract: {case}"
        );
    }
}

fn assert_draft_schema_includes_phase19_effect_contracts(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("draft schema should parse");
    let defs = schema_value
        .get("$defs")
        .and_then(serde_json::Value::as_object)
        .expect("draft schema should expose definitions");

    for expected_contract in [
        "CapabilitySurface",
        "CapabilitySupport",
        "CapabilityReportItem",
        "EffectCapabilityRegistry",
        "ExternalEffectReference",
        "FilterKind",
        "Filter",
        "TransitionReference",
        "Transition",
        "TrackTransition",
        "SpeedRatio",
        "SpeedCurvePoint",
        "RetimeMode",
        "SegmentRetiming",
        "MaskKind",
        "BlendModeKind",
    ] {
        assert!(
            defs.contains_key(expected_contract),
            "draft schema should include Phase 19 definition {expected_contract}"
        );
    }

    let segment = defs
        .get("Segment")
        .expect("Segment definition should exist");
    assert!(
        property_references_def(segment, "retiming", "#/$defs/SegmentRetiming"),
        "Segment.retiming should reference typed SegmentRetiming"
    );
    let speed_ratio = defs
        .get("SpeedRatio")
        .expect("SpeedRatio definition should exist");
    assert_eq!(
        speed_ratio.pointer("/properties/numerator/type"),
        Some(&json!("integer")),
        "speed ratio numerator must be an integer field"
    );
    assert_eq!(
        speed_ratio.pointer("/properties/denominator/type"),
        Some(&json!("integer")),
        "speed ratio denominator must be an integer field"
    );

    let forbidden_schema_text = schema_json;
    for forbidden in [
        "speedSeconds",
        "durationSeconds",
        "targetTimeSeconds",
        "radiusSeconds",
        "opacitySeconds",
    ] {
        assert!(
            !forbidden_schema_text.contains(forbidden),
            "draft schema must not persist naked floating-time/effect field {forbidden}"
        );
    }
}

fn assert_command_schema_rejects_zero_frame_rates(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("command schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("command schema should compile");

    assert!(
        schema
            .validate(&export_command_with_frame_rate(0, 1))
            .is_err(),
        "command schema should reject zero frame-rate numerator"
    );
    assert!(
        schema
            .validate(&export_command_with_frame_rate(24, 0))
            .is_err(),
        "command schema should reject zero frame-rate denominator"
    );
}

fn assert_command_schema_rejects_invalid_canvas_config(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("command schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("command schema should compile");

    assert!(
        schema
            .validate(&export_command_with_canvas_config(0, 1080, 16, 9))
            .is_err(),
        "command schema should reject zero canvas width"
    );
    assert!(
        schema
            .validate(&export_command_with_canvas_config(1920, 0, 16, 9))
            .is_err(),
        "command schema should reject zero canvas height"
    );
    assert!(
        schema
            .validate(&export_command_with_canvas_config(1920, 1080, 0, 9))
            .is_err(),
        "command schema should reject zero custom aspect-ratio numerator"
    );
    assert!(
        schema
            .validate(&export_command_with_canvas_config(1920, 1080, 16, 0))
            .is_err(),
        "command schema should reject zero custom aspect-ratio denominator"
    );
}

fn assert_command_schema_rejects_invalid_text_contracts(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("command schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("command schema should compile");

    for (case, draft) in invalid_text_contract_drafts() {
        let value = export_command_with_draft(draft);
        assert!(
            schema.validate(&value).is_err(),
            "command schema should reject invalid text contract: {case}"
        );
    }
}

fn assert_command_schema_rejects_invalid_keyframe_contracts(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("command schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("command schema should compile");

    for (case, draft) in invalid_keyframe_contract_drafts() {
        let value = export_command_with_draft(draft);
        assert!(
            schema.validate(&value).is_err(),
            "command schema should reject invalid keyframe contract: {case}"
        );
    }
}

fn export_command_with_frame_rate(numerator: u32, denominator: u32) -> serde_json::Value {
    json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": draft_value_with_frame_rate(numerator, denominator),
            "outputPath": "/tmp/video-editor-schema-export.mp4",
            "preset": "h264AacBalanced"
        }
    })
}

fn export_command_with_draft(draft: serde_json::Value) -> serde_json::Value {
    json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": draft,
            "outputPath": "/tmp/video-editor-schema-export.mp4",
            "preset": "h264AacBalanced"
        }
    })
}

fn export_command_with_canvas_config(
    width: u32,
    height: u32,
    numerator: u32,
    denominator: u32,
) -> serde_json::Value {
    json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": draft_value_with_canvas_config(width, height, numerator, denominator),
            "outputPath": "/tmp/video-editor-schema-export.mp4",
            "preset": "h264AacBalanced"
        }
    })
}

fn invalid_text_contract_drafts() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "text box width must be greater than zero",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/textBox/widthMillis",
                json!(0),
            ),
        ),
        (
            "text box height must be <= 1000",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/textBox/heightMillis",
                json!(1001),
            ),
        ),
        (
            "layout width must be greater than zero",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/layoutRegion/widthMillis",
                json!(0),
            ),
        ),
        (
            "layout x must be <= 1000",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/layoutRegion/xMillis",
                json!(1001),
            ),
        ),
        (
            "layout x plus width must be <= 1000",
            draft_value_with_text_layout_region(900, 100, 200, 800),
        ),
        (
            "layout y plus height must be <= 1000",
            draft_value_with_text_layout_region(100, 900, 800, 200),
        ),
        (
            "text color must be #RRGGBB",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/style/color",
                json!("ffffff"),
            ),
        ),
        (
            "font size must be greater than zero",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/style/fontSize",
                json!(0),
            ),
        ),
        (
            "line height must be >= 500",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/style/lineHeightMillis",
                json!(499),
            ),
        ),
        (
            "line height must be <= 3000",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/style/lineHeightMillis",
                json!(3001),
            ),
        ),
        (
            "letter spacing must be <= 2000",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/style/letterSpacingMillis",
                json!(2001),
            ),
        ),
        (
            "stroke color must be #RRGGBB",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/style/stroke/color",
                json!("red"),
            ),
        ),
        (
            "stroke width must be greater than zero",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/style/stroke/width",
                json!(0),
            ),
        ),
        (
            "shadow color must be #RRGGBB",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/style/shadow/color",
                json!("#fffff"),
            ),
        ),
        (
            "background color must be #RRGGBB",
            draft_value_with_text_contract_field(
                "/tracks/0/segments/0/text/style/background/color",
                json!("#gggggg"),
            ),
        ),
    ]
}

fn invalid_keyframe_contract_drafts() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "unknown keyframe property",
            draft_value_with_keyframe_contract_field(
                "/tracks/0/segments/0/keyframes/0/property",
                json!("opacity"),
            ),
        ),
        (
            "keyframe color must be #RRGGBB",
            draft_value_with_keyframe_contract_field(
                "/tracks/0/segments/0/keyframes/1/value/value",
                json!("ffcc00"),
            ),
        ),
    ]
}

fn draft_value_with_keyframe_contract_field(
    pointer: &str,
    replacement: serde_json::Value,
) -> serde_json::Value {
    let mut value = draft_value_with_keyframe_contract();
    *value
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("keyframe contract pointer should exist: {pointer}")) =
        replacement;
    value
}

fn draft_value_with_keyframe_contract() -> serde_json::Value {
    let mut draft = Draft::new("draft-schema-keyframe-contract", "Schema keyframe contract");
    draft.materials.push(Material::new(
        "material-video-001",
        MaterialKind::Video,
        "media/video.mp4",
        "video.mp4",
    ));

    let mut segment = Segment::new(
        "segment-video-001",
        "material-video-001",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    segment.keyframes = vec![
        Keyframe {
            at: Microseconds::new(100_000),
            property: KeyframeProperty::VisualOpacity,
            value: KeyframeValue::Uint { value: 800 },
            interpolation: KeyframeInterpolation::Linear,
            easing: KeyframeEasing::None,
        },
        Keyframe {
            at: Microseconds::new(500_000),
            property: KeyframeProperty::TextColor,
            value: KeyframeValue::Color {
                value: "#ffcc00".to_owned(),
            },
            interpolation: KeyframeInterpolation::Hold,
            easing: KeyframeEasing::None,
        },
    ];

    let mut track = Track::new("track-video-001", TrackKind::Video, "视频");
    track.segments.push(segment);
    draft.tracks.push(track);

    serde_json::to_value(draft).expect("keyframe contract draft should serialize")
}

fn draft_value_with_text_contract_field(
    pointer: &str,
    replacement: serde_json::Value,
) -> serde_json::Value {
    let mut value = draft_value_with_text_contract();
    *value
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("text contract pointer should exist: {pointer}")) = replacement;
    value
}

fn draft_value_with_text_layout_region(
    x_millis: u32,
    y_millis: u32,
    width_millis: u32,
    height_millis: u32,
) -> serde_json::Value {
    let mut value = draft_value_with_text_contract();
    *value
        .pointer_mut("/tracks/0/segments/0/text/layoutRegion/xMillis")
        .expect("text contract xMillis pointer should exist") = json!(x_millis);
    *value
        .pointer_mut("/tracks/0/segments/0/text/layoutRegion/yMillis")
        .expect("text contract yMillis pointer should exist") = json!(y_millis);
    *value
        .pointer_mut("/tracks/0/segments/0/text/layoutRegion/widthMillis")
        .expect("text contract widthMillis pointer should exist") = json!(width_millis);
    *value
        .pointer_mut("/tracks/0/segments/0/text/layoutRegion/heightMillis")
        .expect("text contract heightMillis pointer should exist") = json!(height_millis);
    value
}

fn draft_value_with_text_contract() -> serde_json::Value {
    let mut draft = Draft::new("draft-schema-text-contract", "Schema text contract");
    draft.materials.push(Material::new(
        "text-material",
        MaterialKind::Text,
        "text://title",
        "Title",
    ));

    let mut segment = Segment::new(
        "text-a",
        "text-material",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    segment.text = Some(TextSegment {
        content: "标题".to_owned(),
        source: TextSegmentSource::Text,
        style: TextStyle {
            font: TextFont::system_default(),
            font_size: 32,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            line_height_millis: 1_200,
            letter_spacing_millis: 0,
            stroke: Some(TextStroke {
                color: "#000000".to_owned(),
                width: 1,
            }),
            shadow: Some(TextShadow {
                color: "#101010".to_owned(),
                offset_x: 1,
                offset_y: 1,
                blur: 2,
            }),
            background: Some(TextBackground {
                color: "#202020".to_owned(),
            }),
        },
        text_box: TextBox {
            width_millis: 800,
            height_millis: 200,
        },
        layout_region: TextLayoutRegion {
            x_millis: 100,
            y_millis: 100,
            width_millis: 800,
            height_millis: 800,
        },
        wrapping: TextWrapping::Auto,
        bubble: None,
        effect: None,
    });

    let mut track = Track::new("text-track", TrackKind::Text, "文字");
    track.segments.push(segment);
    draft.tracks.push(track);

    serde_json::to_value(draft).expect("text contract draft should serialize")
}

fn draft_value_with_canvas_config(
    width: u32,
    height: u32,
    numerator: u32,
    denominator: u32,
) -> serde_json::Value {
    json!({
        "schemaVersion": DraftSchemaVersion::CURRENT_VALUE,
        "draftId": "draft-schema-invalid-canvas-config",
        "metadata": { "name": "Schema invalid canvas config" },
        "canvasConfig": {
            "aspectRatio": {
                "kind": "custom",
                "numerator": numerator,
                "denominator": denominator
            },
            "width": width,
            "height": height,
            "frameRate": {
                "numerator": 30,
                "denominator": 1
            },
            "background": { "kind": "black" }
        },
        "materials": [],
        "tracks": []
    })
}

fn draft_value_with_frame_rate(numerator: u32, denominator: u32) -> serde_json::Value {
    json!({
        "schemaVersion": DraftSchemaVersion::CURRENT_VALUE,
        "draftId": "draft-schema-zero-frame-rate",
        "metadata": { "name": "Schema zero frame rate" },
        "canvasConfig": {
            "aspectRatio": { "kind": "preset", "preset": "ratio16x9" },
            "width": 1920,
            "height": 1080,
            "frameRate": {
                "numerator": numerator,
                "denominator": denominator
            },
            "background": { "kind": "black" }
        },
        "materials": [{
            "materialId": "material-video-001",
            "kind": "video",
            "uri": "media/video.mp4",
            "displayName": "video.mp4",
            "metadata": {
                "duration": 1_000_000,
                "width": 160,
                "height": 90,
                "frameRate": {
                    "numerator": numerator,
                    "denominator": denominator
                },
                "hasVideo": true,
                "hasAudio": false
            },
            "status": "available"
        }],
        "tracks": []
    })
}

fn command_payload_pairing_constraints() -> serde_json::Value {
    let command_names = [
        "ping",
        "version",
        "probeMediaRuntime",
        "probeRuntimeCapabilities",
        "createAudioPreviewSession",
        "playAudioPreview",
        "pauseAudioPreview",
        "stopAudioPreview",
        "seekAudioPreview",
        "cancelAudioPreview",
        "getAudioPreviewStatus",
        "listAudioOutputDevices",
        "selectAudioOutputDevice",
        "getWaveformDisplayPeaks",
        "refreshWaveformStatus",
        "getArtifactStatus",
        "refreshArtifactStatus",
        "retryArtifactGeneration",
        "resumeArtifactGeneration",
        "cancelArtifactGeneration",
        "getArtifactQuotaStatus",
        "runArtifactGarbageCollection",
        "startExport",
        "getExportJobStatus",
        "cancelExport",
    ];

    serde_json::Value::Array(
        command_names
            .iter()
            .map(|name| {
                json!({
                    "properties": {
                        "command": { "const": name },
                        "payload": {
                            "properties": {
                                "kind": { "const": name }
                            },
                            "required": ["kind"]
                        }
                    }
                })
            })
            .collect(),
    )
}
