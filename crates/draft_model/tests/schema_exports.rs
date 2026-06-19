use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use draft_model::{
    AddAudioSegmentCommandPayload, AddSegmentCommandPayload, AddTextSegmentCommandPayload,
    ArtifactGenerationActionCommandPayload, ArtifactGenerationTaskSummary,
    ArtifactMaintenanceResult, ArtifactQuotaStatus, ArtifactStatusSummary, ArtifactTaskStatus,
    CancelExportCommandPayload, CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground,
    CanvasBackgroundCapability, ChangedEntity, CommandDelta, CommandEnvelope, CommandError,
    CommandErrorKind, CommandEvent, CommandHistorySnapshot, CommandName, CommandPayload,
    CommandResultEnvelope, CommandState, DecodedPreviewFrameResponse, DeleteSegmentCommandPayload,
    DirtyDomain, DirtyRange, DirtyRangeSource, DisplayableArtifactRef, Draft, DraftCanvasConfig,
    DraftId, DraftMetadata, DraftSchemaVersion, EditTextSegmentCommandPayload, ExportDiagnostic,
    ExportDiagnosticKind, ExportJobPhase, ExportJobStatusResponse, ExportPrepDirtyFacts,
    ExportPreset, ExportValidationReport, Filter, GetArtifactQuotaStatusCommandPayload,
    GetArtifactStatusCommandPayload, GetExportJobStatusCommandPayload,
    ImportMaterialCommandPayload, ImportMaterialResponse, ImportSubtitleSrtCommandPayload,
    InvalidatePreviewCacheCommandPayload, InvalidationScope, Keyframe, KeyframeEasing,
    KeyframeInterpolation, KeyframeProperty, KeyframeValue, ListMaterialsCommandPayload,
    ListMaterialsResponse, ListMissingMaterialsCommandPayload, ListMissingMaterialsResponse,
    MAX_TEXT_LAYOUT_MILLIS, MAX_TEXT_LETTER_SPACING_MILLIS, MAX_TEXT_LINE_HEIGHT_MILLIS,
    MIN_TEXT_LINE_HEIGHT_MILLIS, MainTrackMagnet, Material, MaterialArtifactStatus, MaterialId,
    MaterialKind, MaterialMetadata, MaterialStatus, Microseconds, MissingMaterialCommandDiagnostic,
    MissingMaterialCommandDiagnosticKind, MoveSegmentCommandPayload, PingCommandPayload,
    PreviewArtifactResponse, PreviewCacheEntryRef, PreviewCacheInvalidationResponse,
    PreviewDecodeDiagnostic, PreviewDecodeRequest, PreviewDiagnostic, PreviewDiagnosticKind,
    PreviewFrameReleaseResponse, PreviewFrameStorageKind, PreviewFrameStoragePreference,
    PreviewOutputProfile, PreviewStatus, ProbeMediaRuntimeCommandPayload,
    ProbeRuntimeCapabilitiesCommandPayload, RationalFrameRate, RedoTimelineEditCommandPayload,
    RefreshArtifactStatusCommandPayload, ReleasePreviewFrameCommandPayload,
    RemoveSegmentKeyframeCommandPayload, RequestPreviewFrameCommandPayload,
    RequestPreviewSegmentCommandPayload, RunArtifactGarbageCollectionCommandPayload,
    RuntimeBinaryCapability, RuntimeBinaryKind, RuntimeCapabilityReport, RuntimeCapabilityStatus,
    RuntimeCodecCapability, RuntimeColorDiagnostic, RuntimeColorMatrix, RuntimeColorPrimaries,
    RuntimeColorRange, RuntimeColorTransfer, RuntimeDecodedFrameHandleMetadata, RuntimeDeviceId,
    RuntimeFallbackDecodePathCapability, RuntimeFallbackLadderCapability, RuntimeFeatureCapability,
    RuntimeFontCapability, RuntimeFrameDimensions, RuntimeLicensePosture,
    RuntimeMacosMediaIoCapabilities, RuntimeMediaIoCapabilities, RuntimeMediaIoFallbackReason,
    RuntimePixelFormatCapability, RuntimeSelectedDecodePath, RuntimeTextureBackend,
    RuntimeTextureHandleMetadata, RuntimeTextureInteropCapability, RuntimeVideoColorMetadata,
    RuntimeVideoPixelFormat, RuntimeWindowsMediaIoCapabilities, Segment, SegmentAnchor,
    SegmentBackgroundFilling, SegmentBlendMode, SegmentCrop, SegmentFitMode, SegmentId,
    SegmentMask, SegmentOpacity, SegmentPosition, SegmentRotation, SegmentScale, SegmentTransform,
    SegmentVisual, SegmentVolume, SelectTimelineSegmentsCommandPayload,
    SetSegmentKeyframeCommandPayload, SetSegmentVolumeCommandPayload, SetTrackMuteCommandPayload,
    SnappingSettings, SourceTimerange, SplitSegmentCommandPayload, StartExportCommandPayload,
    TargetTimerange, TextAlignment, TextBackground, TextBox, TextBubbleRef, TextEffectRef,
    TextFont, TextLayoutRegion, TextSegment, TextSegmentSource, TextShadow, TextStroke, TextStyle,
    TextWrapping, TimelineCommandResponse, TimelineSelection, Track, TrackId, TrackKind,
    Transition, TrimSegmentCommandPayload, UndoTimelineEditCommandPayload,
    UpdateDraftCanvasConfigCommandPayload, UpdateSegmentVisualCommandPayload,
    VersionCommandPayload,
};
use schemars::{Schema, schema_for};
use serde_json::json;
use ts_rs::{Config, TS};

const TEXT_HEX_COLOR_PATTERN: &str = "^#[0-9A-Fa-f]{6}$";

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
    assert_or_update_contract_file(&draft_schema_path, &format!("{draft_schema_json}\n"));

    let command_envelope_ts = ts_contract_with_prelude(
        "import type { Draft, DraftCanvasConfig, Keyframe, KeyframeProperty, MaterialId, MaterialKind, Microseconds, SegmentId, SegmentVisual, SegmentVolume, SourceTimerange, TargetTimerange, TextBox, TextLayoutRegion, TextSegment, TextStyle, TextWrapping, TrackId, TrimSegmentDirection } from \"./Draft\";\n\n",
        &[
            export_decl::<CommandName>(),
            export_decl::<PingCommandPayload>(),
            export_decl::<VersionCommandPayload>(),
            export_decl::<ProbeMediaRuntimeCommandPayload>(),
            export_decl::<ProbeRuntimeCapabilitiesCommandPayload>(),
            export_decl::<ImportMaterialCommandPayload>(),
            export_decl::<ListMaterialsCommandPayload>(),
            export_decl::<ListMissingMaterialsCommandPayload>(),
            export_decl::<AddSegmentCommandPayload>(),
            export_decl::<SelectTimelineSegmentsCommandPayload>(),
            export_decl::<MoveSegmentCommandPayload>(),
            export_decl::<SplitSegmentCommandPayload>(),
            export_decl::<TrimSegmentCommandPayload>(),
            export_decl::<DeleteSegmentCommandPayload>(),
            export_decl::<UndoTimelineEditCommandPayload>(),
            export_decl::<RedoTimelineEditCommandPayload>(),
            export_decl::<AddTextSegmentCommandPayload>(),
            export_decl::<EditTextSegmentCommandPayload>(),
            export_decl::<ImportSubtitleSrtCommandPayload>(),
            export_decl::<AddAudioSegmentCommandPayload>(),
            export_decl::<SetSegmentVolumeCommandPayload>(),
            export_decl::<SetTrackMuteCommandPayload>(),
            export_decl::<UpdateDraftCanvasConfigCommandPayload>(),
            export_decl::<UpdateSegmentVisualCommandPayload>(),
            export_decl::<SetSegmentKeyframeCommandPayload>(),
            export_decl::<RemoveSegmentKeyframeCommandPayload>(),
            export_decl::<PreviewFrameStoragePreference>(),
            export_decl::<PreviewDecodeRequest>(),
            export_decl::<ReleasePreviewFrameCommandPayload>(),
            export_decl::<PreviewOutputProfile>(),
            export_decl::<RequestPreviewFrameCommandPayload>(),
            export_decl::<RequestPreviewSegmentCommandPayload>(),
            export_decl::<PreviewCacheEntryRef>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<DirtyRange>(),
            export_decl::<InvalidatePreviewCacheCommandPayload>(),
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
        "import type { Draft, DraftId, KeyframeProperty, Material, MaterialId, MaterialStatus, Microseconds, RationalFrameRate, SegmentId, TargetTimerange, TrackId } from \"./Draft\";\nimport type { CommandName, CommandState, ExportPreset, PreviewOutputProfile, TimelineSelection } from \"./CommandEnvelope\";\n\n",
        &[
            export_decl::<CommandErrorKind>(),
            export_decl::<CommandError>(),
            export_decl::<CommandEvent>(),
            export_decl::<CommandResultEnvelope<()>>(),
            export_decl::<PreviewStatus>(),
            export_decl::<PreviewDiagnosticKind>(),
            export_decl::<PreviewDiagnostic>(),
            export_decl::<PreviewArtifactResponse>(),
            export_decl::<PreviewFrameStorageKind>(),
            export_decl::<PreviewDecodeDiagnostic>(),
            export_decl::<DecodedPreviewFrameResponse>(),
            export_decl::<PreviewFrameReleaseResponse>(),
            export_decl::<PreviewCacheInvalidationResponse>(),
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
            export_decl::<RuntimeFrameDimensions>(),
            export_decl::<RuntimeDecodedFrameHandleMetadata>(),
            export_decl::<RuntimeTextureHandleMetadata>(),
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
            export_decl::<ImportMaterialResponse>(),
            export_decl::<ListMaterialsResponse>(),
            export_decl::<ListMissingMaterialsResponse>(),
            export_decl::<ChangedEntity>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRange>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<InvalidationScope>(),
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
        export_decl::<Filter>(),
        export_decl::<Transition>(),
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
    assert_or_update_contract_file(generated_dir.join("Draft.ts"), &draft_ts);
}

#[test]
fn schema_exports_include_timeline_command_session_contracts() {
    let schema_json = command_schema_json();
    for expected_contract in [
        "TimelineSelection",
        "CommandState",
        "CommandHistorySnapshot",
        "SnappingSettings",
        "TimelineCommandResponse",
    ] {
        assert!(
            schema_json.contains(expected_contract),
            "command schema should include {expected_contract}"
        );
    }

    let command_envelope_ts = ts_contract_with_prelude(
        "import type { Draft, DraftCanvasConfig, Keyframe, KeyframeProperty, MaterialId, MaterialKind, Microseconds, SegmentId, SegmentVisual, SegmentVolume, SourceTimerange, TargetTimerange, TextBox, TextLayoutRegion, TextSegment, TextStyle, TextWrapping, TrackId, TrimSegmentDirection } from \"./Draft\";\n\n",
        &[
            export_decl::<CommandName>(),
            export_decl::<PingCommandPayload>(),
            export_decl::<VersionCommandPayload>(),
            export_decl::<ProbeMediaRuntimeCommandPayload>(),
            export_decl::<ProbeRuntimeCapabilitiesCommandPayload>(),
            export_decl::<ImportMaterialCommandPayload>(),
            export_decl::<ListMaterialsCommandPayload>(),
            export_decl::<ListMissingMaterialsCommandPayload>(),
            export_decl::<AddSegmentCommandPayload>(),
            export_decl::<SelectTimelineSegmentsCommandPayload>(),
            export_decl::<MoveSegmentCommandPayload>(),
            export_decl::<SplitSegmentCommandPayload>(),
            export_decl::<TrimSegmentCommandPayload>(),
            export_decl::<DeleteSegmentCommandPayload>(),
            export_decl::<UndoTimelineEditCommandPayload>(),
            export_decl::<RedoTimelineEditCommandPayload>(),
            export_decl::<AddTextSegmentCommandPayload>(),
            export_decl::<EditTextSegmentCommandPayload>(),
            export_decl::<ImportSubtitleSrtCommandPayload>(),
            export_decl::<AddAudioSegmentCommandPayload>(),
            export_decl::<SetSegmentVolumeCommandPayload>(),
            export_decl::<SetTrackMuteCommandPayload>(),
            export_decl::<UpdateDraftCanvasConfigCommandPayload>(),
            export_decl::<UpdateSegmentVisualCommandPayload>(),
            export_decl::<SetSegmentKeyframeCommandPayload>(),
            export_decl::<RemoveSegmentKeyframeCommandPayload>(),
            export_decl::<PreviewFrameStoragePreference>(),
            export_decl::<PreviewDecodeRequest>(),
            export_decl::<ReleasePreviewFrameCommandPayload>(),
            export_decl::<PreviewOutputProfile>(),
            export_decl::<RequestPreviewFrameCommandPayload>(),
            export_decl::<RequestPreviewSegmentCommandPayload>(),
            export_decl::<PreviewCacheEntryRef>(),
            export_decl::<InvalidatePreviewCacheCommandPayload>(),
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
        "import type { Draft, DraftId, KeyframeProperty, Material, MaterialId, MaterialStatus, Microseconds, RationalFrameRate, SegmentId, TargetTimerange, TrackId } from \"./Draft\";\nimport type { CommandName, CommandState, ExportPreset, PreviewOutputProfile, TimelineSelection } from \"./CommandEnvelope\";\n\n",
        &[
            export_decl::<CommandErrorKind>(),
            export_decl::<CommandError>(),
            export_decl::<CommandEvent>(),
            export_decl::<CommandResultEnvelope<()>>(),
            export_decl::<PreviewStatus>(),
            export_decl::<PreviewDiagnosticKind>(),
            export_decl::<PreviewDiagnostic>(),
            export_decl::<PreviewArtifactResponse>(),
            export_decl::<PreviewFrameStorageKind>(),
            export_decl::<PreviewDecodeDiagnostic>(),
            export_decl::<DecodedPreviewFrameResponse>(),
            export_decl::<PreviewFrameReleaseResponse>(),
            export_decl::<PreviewCacheInvalidationResponse>(),
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
            export_decl::<RuntimeFrameDimensions>(),
            export_decl::<RuntimeDecodedFrameHandleMetadata>(),
            export_decl::<RuntimeTextureHandleMetadata>(),
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
            export_decl::<ImportMaterialResponse>(),
            export_decl::<ListMaterialsResponse>(),
            export_decl::<ListMissingMaterialsResponse>(),
            export_decl::<ChangedEntity>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRange>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<InvalidationScope>(),
            export_decl::<CommandDelta>(),
            export_decl::<TimelineCommandResponse>(),
        ],
    );

    for expected_contract in [
        "TimelineSelection",
        "CommandState",
        "CommandHistorySnapshot",
        "SnappingSettings",
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
    let command_schema = command_schema_json();
    let draft_schema = draft_schema_json();
    let command_result_ts = command_result_ts_contract();
    let draft_ts = ts_contract(&[
        export_decl::<Microseconds>(),
        export_decl::<TargetTimerange>(),
        export_decl::<MaterialId>(),
    ]);

    for expected_contract in [
        "TimelineCommandResponse",
        "InvalidatePreviewCacheCommandPayload",
        "PreviewCacheInvalidationResponse",
    ] {
        assert!(
            command_schema.contains(expected_contract)
                || command_result_ts.contains(expected_contract),
            "Phase 13 downstream contract assertions should attach to {expected_contract}"
        );
    }

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
    let command_schema: serde_json::Value =
        serde_json::from_str(&command_schema_json()).expect("command schema should parse");
    let defs = command_schema
        .get("$defs")
        .and_then(|defs| defs.as_object())
        .expect("command schema should expose definitions");

    for expected_contract in [
        "ChangedEntity",
        "DirtyDomain",
        "DirtyRange",
        "DirtyRangeSource",
        "InvalidationScope",
        "CommandDelta",
        "TimelineCommandResponse",
    ] {
        assert!(
            defs.contains_key(expected_contract),
            "command schema should include Phase 13 delta contract {expected_contract}"
        );
    }

    let timeline_response = defs
        .get("TimelineCommandResponse")
        .expect("TimelineCommandResponse should be generated");
    assert_eq!(
        timeline_response
            .pointer("/properties/delta/$ref")
            .and_then(|value| value.as_str()),
        Some("#/$defs/CommandDelta"),
        "TimelineCommandResponse.delta must directly reference CommandDelta"
    );
    assert!(
        timeline_response
            .get("required")
            .and_then(|value| value.as_array())
            .is_some_and(|required| required.iter().any(|field| field == "delta")),
        "TimelineCommandResponse.delta must be required in the generated schema"
    );

    let command_result_ts = command_result_ts_contract();
    for expected_export in [
        "export type ChangedEntity",
        "export type DirtyDomain",
        "export type DirtyRange",
        "export type DirtyRangeSource",
        "export type InvalidationScope",
        "export type CommandDelta",
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
        "PreviewCacheEntryRef",
        "InvalidatePreviewCacheCommandPayload",
        "PreviewCacheInvalidationResponse",
        "ExportPrepDirtyFacts",
        "StartExportCommandPayload",
        "ExportJobStatusResponse",
    ] {
        assert!(
            defs.contains_key(expected_contract),
            "command schema should include Phase 13 dirty fact contract {expected_contract}"
        );
    }

    let invalidation_payload = defs
        .get("InvalidatePreviewCacheCommandPayload")
        .expect("InvalidatePreviewCacheCommandPayload should be generated");
    assert_eq!(
        invalidation_payload
            .pointer("/properties/changedRanges/items/$ref")
            .and_then(|value| value.as_str()),
        Some("#/$defs/DirtyRange"),
        "preview invalidation changedRanges must use DirtyRange transport"
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
            invalidation_payload
                .pointer(&format!("/properties/{expected_field}"))
                .is_some(),
            "preview invalidation payload should expose {expected_field}"
        );
    }

    let entry_ref = defs
        .get("PreviewCacheEntryRef")
        .expect("PreviewCacheEntryRef should be generated");
    for expected_field in [
        "graphNodeIds",
        "semanticFingerprint",
        "inputFingerprint",
        "outputProfileFingerprint",
        "runtimeCapabilityFingerprint",
        "artifactSchemaVersion",
        "generatorVersion",
    ] {
        assert!(
            entry_ref
                .pointer(&format!("/properties/{expected_field}"))
                .is_some(),
            "preview cache entry refs should expose v2 key fact field {expected_field}"
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
fn schema_exports_include_timeline_edit_command_contracts() {
    let schema_json = command_schema_json();
    let command_envelope_ts = command_envelope_ts_contract();

    for expected_contract in [
        "AddSegmentCommandPayload",
        "SelectTimelineSegmentsCommandPayload",
        "MoveSegmentCommandPayload",
        "SplitSegmentCommandPayload",
        "TrimSegmentCommandPayload",
        "DeleteSegmentCommandPayload",
    ] {
        assert!(
            schema_json.contains(expected_contract),
            "command schema should include {expected_contract}"
        );
        assert!(
            command_envelope_ts.contains(&format!("export type {expected_contract}")),
            "generated TypeScript contracts should export {expected_contract}"
        );
    }
}

#[test]
fn schema_exports_include_undo_redo_command_contracts() {
    let schema_json = command_schema_json();
    let command_envelope_ts = command_envelope_ts_contract();

    for expected_contract in [
        "UndoTimelineEditCommandPayload",
        "RedoTimelineEditCommandPayload",
    ] {
        assert!(
            schema_json.contains(expected_contract),
            "command schema should include {expected_contract}"
        );
        assert!(
            command_envelope_ts.contains(&format!("export type {expected_contract}")),
            "generated TypeScript contracts should export {expected_contract}"
        );
    }
}

#[test]
fn schema_exports_include_text_command_contracts() {
    let schema_json = command_schema_json();
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
        "AddTextSegmentCommandPayload",
        "EditTextSegmentCommandPayload",
        "ImportSubtitleSrtCommandPayload",
    ] {
        assert!(
            schema_json.contains(expected_contract) || draft_ts.contains(expected_contract),
            "schema or draft TypeScript should include {expected_contract}"
        );
    }
    for expected_contract in [
        "AddTextSegmentCommandPayload",
        "EditTextSegmentCommandPayload",
        "ImportSubtitleSrtCommandPayload",
    ] {
        assert!(
            command_envelope_ts.contains(&format!("export type {expected_contract}")),
            "generated TypeScript contracts should export {expected_contract}"
        );
    }
}

#[test]
fn schema_exports_include_audio_command_contracts() {
    let schema_json = command_schema_json();
    let command_envelope_ts = command_envelope_ts_contract();
    let draft_ts = ts_contract(&[export_decl::<SegmentVolume>()]);

    for expected_contract in [
        "SegmentVolume",
        "AddAudioSegmentCommandPayload",
        "SetSegmentVolumeCommandPayload",
        "SetTrackMuteCommandPayload",
    ] {
        assert!(
            schema_json.contains(expected_contract) || draft_ts.contains(expected_contract),
            "schema or draft TypeScript should include {expected_contract}"
        );
    }
    for expected_contract in [
        "AddAudioSegmentCommandPayload",
        "SetSegmentVolumeCommandPayload",
        "SetTrackMuteCommandPayload",
    ] {
        assert!(
            command_envelope_ts.contains(&format!("export type {expected_contract}")),
            "generated TypeScript contracts should export {expected_contract}"
        );
    }
}

#[test]
fn schema_exports_include_preview_command_contracts() {
    let schema_json = command_schema_json();
    let command_envelope_ts = command_envelope_ts_contract();
    let command_result_ts = command_result_ts_contract();

    for expected_contract in [
        "PreviewDecodeRequest",
        "ReleasePreviewFrameCommandPayload",
        "PreviewFrameStoragePreference",
        "RequestPreviewFrameCommandPayload",
        "RequestPreviewSegmentCommandPayload",
        "InvalidatePreviewCacheCommandPayload",
        "PreviewCacheEntryRef",
        "PreviewOutputProfile",
        "PreviewArtifactResponse",
        "DecodedPreviewFrameResponse",
        "PreviewFrameReleaseResponse",
        "PreviewFrameStorageKind",
        "PreviewDecodeDiagnostic",
        "PreviewCacheInvalidationResponse",
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
        "RuntimeFrameDimensions",
        "RuntimeDecodedFrameHandleMetadata",
        "RuntimeTextureHandleMetadata",
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
        "ownerSession",
        "generation",
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
    let schema_json = command_schema_json();
    let command_envelope_ts = command_envelope_ts_contract();
    let draft_ts = ts_contract(&[
        export_decl::<CanvasAspectRatioPreset>(),
        export_decl::<CanvasAspectRatio>(),
        export_decl::<CanvasBackgroundCapability>(),
        export_decl::<CanvasBackground>(),
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

    for expected_contract in ["UpdateDraftCanvasConfigCommandPayload"] {
        assert!(
            schema_json.contains(expected_contract),
            "command schema should include {expected_contract}"
        );
        assert!(
            command_envelope_ts.contains(&format!("export type {expected_contract}")),
            "generated TypeScript contracts should export {expected_contract}"
        );
    }

    assert!(
        schema_json.contains("updateDraftCanvasConfig")
            && command_envelope_ts.contains("updateDraftCanvasConfig"),
        "draft canvas update command should be generated from Rust contracts"
    );
}

#[test]
fn schema_exports_include_segment_visual_and_command_contracts() {
    let schema_json = command_schema_json();
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
        schema_json.contains("UpdateSegmentVisualCommandPayload"),
        "command schema should include UpdateSegmentVisualCommandPayload"
    );
    assert!(
        command_envelope_ts.contains("export type UpdateSegmentVisualCommandPayload"),
        "generated TypeScript contracts should export UpdateSegmentVisualCommandPayload"
    );
    assert!(
        schema_json.contains("updateSegmentVisual")
            && command_envelope_ts.contains("updateSegmentVisual"),
        "segment visual update command should be generated from Rust contracts"
    );
}

#[test]
fn schema_exports_include_keyframe_command_contracts() {
    let schema_json = command_schema_json();
    let command_envelope_ts = command_envelope_ts_contract();

    for expected_contract in [
        "SetSegmentKeyframeCommandPayload",
        "RemoveSegmentKeyframeCommandPayload",
    ] {
        assert!(
            schema_json.contains(expected_contract),
            "command schema should include {expected_contract}"
        );
        assert!(
            command_envelope_ts.contains(&format!("export type {expected_contract}")),
            "generated TypeScript contracts should export {expected_contract}"
        );
    }

    assert!(
        schema_json.contains("setSegmentKeyframe")
            && schema_json.contains("removeSegmentKeyframe")
            && command_envelope_ts.contains("setSegmentKeyframe")
            && command_envelope_ts.contains("removeSegmentKeyframe"),
        "keyframe commands should be generated from Rust contracts"
    );
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
        "import type { Draft, DraftCanvasConfig, Keyframe, KeyframeProperty, MaterialId, MaterialKind, Microseconds, SegmentId, SegmentVisual, SegmentVolume, SourceTimerange, TargetTimerange, TextBox, TextLayoutRegion, TextSegment, TextStyle, TextWrapping, TrackId, TrimSegmentDirection } from \"./Draft\";\n\n",
        &[
            export_decl::<CommandName>(),
            export_decl::<PingCommandPayload>(),
            export_decl::<VersionCommandPayload>(),
            export_decl::<ProbeMediaRuntimeCommandPayload>(),
            export_decl::<ProbeRuntimeCapabilitiesCommandPayload>(),
            export_decl::<ImportMaterialCommandPayload>(),
            export_decl::<ListMaterialsCommandPayload>(),
            export_decl::<ListMissingMaterialsCommandPayload>(),
            export_decl::<AddSegmentCommandPayload>(),
            export_decl::<SelectTimelineSegmentsCommandPayload>(),
            export_decl::<MoveSegmentCommandPayload>(),
            export_decl::<SplitSegmentCommandPayload>(),
            export_decl::<TrimSegmentCommandPayload>(),
            export_decl::<DeleteSegmentCommandPayload>(),
            export_decl::<UndoTimelineEditCommandPayload>(),
            export_decl::<RedoTimelineEditCommandPayload>(),
            export_decl::<AddTextSegmentCommandPayload>(),
            export_decl::<EditTextSegmentCommandPayload>(),
            export_decl::<ImportSubtitleSrtCommandPayload>(),
            export_decl::<AddAudioSegmentCommandPayload>(),
            export_decl::<SetSegmentVolumeCommandPayload>(),
            export_decl::<SetTrackMuteCommandPayload>(),
            export_decl::<UpdateDraftCanvasConfigCommandPayload>(),
            export_decl::<UpdateSegmentVisualCommandPayload>(),
            export_decl::<SetSegmentKeyframeCommandPayload>(),
            export_decl::<RemoveSegmentKeyframeCommandPayload>(),
            export_decl::<PreviewFrameStoragePreference>(),
            export_decl::<PreviewDecodeRequest>(),
            export_decl::<ReleasePreviewFrameCommandPayload>(),
            export_decl::<PreviewOutputProfile>(),
            export_decl::<RequestPreviewFrameCommandPayload>(),
            export_decl::<RequestPreviewSegmentCommandPayload>(),
            export_decl::<PreviewCacheEntryRef>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<DirtyRange>(),
            export_decl::<InvalidatePreviewCacheCommandPayload>(),
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
        "import type { Draft, DraftId, KeyframeProperty, Material, MaterialId, MaterialStatus, Microseconds, RationalFrameRate, SegmentId, TargetTimerange, TrackId } from \"./Draft\";\nimport type { CommandName, CommandState, ExportPreset, PreviewOutputProfile, TimelineSelection } from \"./CommandEnvelope\";\n\n",
        &[
            export_decl::<CommandErrorKind>(),
            export_decl::<CommandError>(),
            export_decl::<CommandEvent>(),
            export_decl::<CommandResultEnvelope<()>>(),
            export_decl::<PreviewStatus>(),
            export_decl::<PreviewDiagnosticKind>(),
            export_decl::<PreviewDiagnostic>(),
            export_decl::<PreviewArtifactResponse>(),
            export_decl::<PreviewFrameStorageKind>(),
            export_decl::<PreviewDecodeDiagnostic>(),
            export_decl::<DecodedPreviewFrameResponse>(),
            export_decl::<PreviewFrameReleaseResponse>(),
            export_decl::<PreviewCacheInvalidationResponse>(),
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
            export_decl::<RuntimeFrameDimensions>(),
            export_decl::<RuntimeDecodedFrameHandleMetadata>(),
            export_decl::<RuntimeTextureHandleMetadata>(),
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
            export_decl::<ImportMaterialResponse>(),
            export_decl::<ListMaterialsResponse>(),
            export_decl::<ListMissingMaterialsResponse>(),
            export_decl::<ChangedEntity>(),
            export_decl::<DirtyDomain>(),
            export_decl::<DirtyRange>(),
            export_decl::<DirtyRangeSource>(),
            export_decl::<InvalidationScope>(),
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
    let positive = BTreeSet::from(["minimal-command.json", "minimal-timeline-command.json"]);
    let negative = BTreeSet::from([
        "invalid-mismatched-command-payload.json",
        "invalid-timeline-command.json",
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
    include_command_contract_schema::<TimelineSelection>(&mut schema_value, "TimelineSelection");
    include_command_contract_schema::<SnappingSettings>(&mut schema_value, "SnappingSettings");
    include_command_contract_schema::<CommandHistorySnapshot>(
        &mut schema_value,
        "CommandHistorySnapshot",
    );
    include_command_contract_schema::<CommandState>(&mut schema_value, "CommandState");
    include_command_contract_schema::<TimelineCommandResponse>(
        &mut schema_value,
        "TimelineCommandResponse",
    );
    include_command_contract_schema::<AddSegmentCommandPayload>(
        &mut schema_value,
        "AddSegmentCommandPayload",
    );
    include_command_contract_schema::<SelectTimelineSegmentsCommandPayload>(
        &mut schema_value,
        "SelectTimelineSegmentsCommandPayload",
    );
    include_command_contract_schema::<MoveSegmentCommandPayload>(
        &mut schema_value,
        "MoveSegmentCommandPayload",
    );
    include_command_contract_schema::<SplitSegmentCommandPayload>(
        &mut schema_value,
        "SplitSegmentCommandPayload",
    );
    include_command_contract_schema::<TrimSegmentCommandPayload>(
        &mut schema_value,
        "TrimSegmentCommandPayload",
    );
    include_command_contract_schema::<DeleteSegmentCommandPayload>(
        &mut schema_value,
        "DeleteSegmentCommandPayload",
    );
    include_command_contract_schema::<UndoTimelineEditCommandPayload>(
        &mut schema_value,
        "UndoTimelineEditCommandPayload",
    );
    include_command_contract_schema::<RedoTimelineEditCommandPayload>(
        &mut schema_value,
        "RedoTimelineEditCommandPayload",
    );
    include_command_contract_schema::<AddTextSegmentCommandPayload>(
        &mut schema_value,
        "AddTextSegmentCommandPayload",
    );
    include_command_contract_schema::<EditTextSegmentCommandPayload>(
        &mut schema_value,
        "EditTextSegmentCommandPayload",
    );
    include_command_contract_schema::<ImportSubtitleSrtCommandPayload>(
        &mut schema_value,
        "ImportSubtitleSrtCommandPayload",
    );
    include_command_contract_schema::<AddAudioSegmentCommandPayload>(
        &mut schema_value,
        "AddAudioSegmentCommandPayload",
    );
    include_command_contract_schema::<SetSegmentVolumeCommandPayload>(
        &mut schema_value,
        "SetSegmentVolumeCommandPayload",
    );
    include_command_contract_schema::<SetTrackMuteCommandPayload>(
        &mut schema_value,
        "SetTrackMuteCommandPayload",
    );
    include_command_contract_schema::<UpdateDraftCanvasConfigCommandPayload>(
        &mut schema_value,
        "UpdateDraftCanvasConfigCommandPayload",
    );
    include_command_contract_schema::<UpdateSegmentVisualCommandPayload>(
        &mut schema_value,
        "UpdateSegmentVisualCommandPayload",
    );
    include_command_contract_schema::<SetSegmentKeyframeCommandPayload>(
        &mut schema_value,
        "SetSegmentKeyframeCommandPayload",
    );
    include_command_contract_schema::<RemoveSegmentKeyframeCommandPayload>(
        &mut schema_value,
        "RemoveSegmentKeyframeCommandPayload",
    );
    include_command_contract_schema::<PreviewFrameStoragePreference>(
        &mut schema_value,
        "PreviewFrameStoragePreference",
    );
    include_command_contract_schema::<PreviewDecodeRequest>(
        &mut schema_value,
        "PreviewDecodeRequest",
    );
    include_command_contract_schema::<ReleasePreviewFrameCommandPayload>(
        &mut schema_value,
        "ReleasePreviewFrameCommandPayload",
    );
    include_command_contract_schema::<RequestPreviewFrameCommandPayload>(
        &mut schema_value,
        "RequestPreviewFrameCommandPayload",
    );
    include_command_contract_schema::<RequestPreviewSegmentCommandPayload>(
        &mut schema_value,
        "RequestPreviewSegmentCommandPayload",
    );
    include_command_contract_schema::<PreviewCacheEntryRef>(
        &mut schema_value,
        "PreviewCacheEntryRef",
    );
    include_command_contract_schema::<InvalidatePreviewCacheCommandPayload>(
        &mut schema_value,
        "InvalidatePreviewCacheCommandPayload",
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
    include_command_contract_schema::<PreviewFrameStorageKind>(
        &mut schema_value,
        "PreviewFrameStorageKind",
    );
    include_command_contract_schema::<PreviewDecodeDiagnostic>(
        &mut schema_value,
        "PreviewDecodeDiagnostic",
    );
    include_command_contract_schema::<DecodedPreviewFrameResponse>(
        &mut schema_value,
        "DecodedPreviewFrameResponse",
    );
    include_command_contract_schema::<PreviewFrameReleaseResponse>(
        &mut schema_value,
        "PreviewFrameReleaseResponse",
    );
    include_command_contract_schema::<PreviewCacheInvalidationResponse>(
        &mut schema_value,
        "PreviewCacheInvalidationResponse",
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
    include_command_contract_schema::<RuntimeFrameDimensions>(
        &mut schema_value,
        "RuntimeFrameDimensions",
    );
    include_command_contract_schema::<RuntimeDecodedFrameHandleMetadata>(
        &mut schema_value,
        "RuntimeDecodedFrameHandleMetadata",
    );
    include_command_contract_schema::<RuntimeTextureHandleMetadata>(
        &mut schema_value,
        "RuntimeTextureHandleMetadata",
    );
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

fn assert_command_schema_rejects_zero_frame_rates(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("command schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("command schema should compile");

    assert!(
        schema
            .validate(&list_materials_command_with_frame_rate(0, 1))
            .is_err(),
        "command schema should reject zero frame-rate numerator"
    );
    assert!(
        schema
            .validate(&list_materials_command_with_frame_rate(24, 0))
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
            .validate(&list_materials_command_with_canvas_config(0, 1080, 16, 9))
            .is_err(),
        "command schema should reject zero canvas width"
    );
    assert!(
        schema
            .validate(&list_materials_command_with_canvas_config(1920, 0, 16, 9))
            .is_err(),
        "command schema should reject zero canvas height"
    );
    assert!(
        schema
            .validate(&list_materials_command_with_canvas_config(1920, 1080, 0, 9))
            .is_err(),
        "command schema should reject zero custom aspect-ratio numerator"
    );
    assert!(
        schema
            .validate(&list_materials_command_with_canvas_config(
                1920, 1080, 16, 0
            ))
            .is_err(),
        "command schema should reject zero custom aspect-ratio denominator"
    );
}

fn assert_command_schema_rejects_invalid_text_contracts(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("command schema should parse");
    let schema = jsonschema::validator_for(&schema_value).expect("command schema should compile");

    for (case, draft) in invalid_text_contract_drafts() {
        let value = list_materials_command_with_draft(draft);
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
        let value = list_materials_command_with_draft(draft);
        assert!(
            schema.validate(&value).is_err(),
            "command schema should reject invalid keyframe contract: {case}"
        );
    }
}

fn list_materials_command_with_frame_rate(numerator: u32, denominator: u32) -> serde_json::Value {
    json!({
        "command": "listMaterials",
        "payload": {
            "kind": "listMaterials",
            "draft": draft_value_with_frame_rate(numerator, denominator)
        }
    })
}

fn list_materials_command_with_draft(draft: serde_json::Value) -> serde_json::Value {
    json!({
        "command": "listMaterials",
        "payload": {
            "kind": "listMaterials",
            "draft": draft
        }
    })
}

fn list_materials_command_with_canvas_config(
    width: u32,
    height: u32,
    numerator: u32,
    denominator: u32,
) -> serde_json::Value {
    json!({
        "command": "listMaterials",
        "payload": {
            "kind": "listMaterials",
            "draft": draft_value_with_canvas_config(width, height, numerator, denominator)
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
    json!([
        {
            "properties": {
                "command": { "const": "ping" },
                "payload": {
                    "properties": {
                        "kind": { "const": "ping" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "version" },
                "payload": {
                    "properties": {
                        "kind": { "const": "version" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "probeMediaRuntime" },
                "payload": {
                    "properties": {
                        "kind": { "const": "probeMediaRuntime" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "probeRuntimeCapabilities" },
                "payload": {
                    "properties": {
                        "kind": { "const": "probeRuntimeCapabilities" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "importMaterial" },
                "payload": {
                    "properties": {
                        "kind": { "const": "importMaterial" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "listMaterials" },
                "payload": {
                    "properties": {
                        "kind": { "const": "listMaterials" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "listMissingMaterials" },
                "payload": {
                    "properties": {
                        "kind": { "const": "listMissingMaterials" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "addSegment" },
                "payload": {
                    "properties": {
                        "kind": { "const": "addSegment" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "selectTimelineSegments" },
                "payload": {
                    "properties": {
                        "kind": { "const": "selectTimelineSegments" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "moveSegment" },
                "payload": {
                    "properties": {
                        "kind": { "const": "moveSegment" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "splitSegment" },
                "payload": {
                    "properties": {
                        "kind": { "const": "splitSegment" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "trimSegment" },
                "payload": {
                    "properties": {
                        "kind": { "const": "trimSegment" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "deleteSegment" },
                "payload": {
                    "properties": {
                        "kind": { "const": "deleteSegment" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "undoTimelineEdit" },
                "payload": {
                    "properties": {
                        "kind": { "const": "undoTimelineEdit" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "redoTimelineEdit" },
                "payload": {
                    "properties": {
                        "kind": { "const": "redoTimelineEdit" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "addTextSegment" },
                "payload": {
                    "properties": {
                        "kind": { "const": "addTextSegment" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "editTextSegment" },
                "payload": {
                    "properties": {
                        "kind": { "const": "editTextSegment" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "importSubtitleSrt" },
                "payload": {
                    "properties": {
                        "kind": { "const": "importSubtitleSrt" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "addAudioSegment" },
                "payload": {
                    "properties": {
                        "kind": { "const": "addAudioSegment" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "setSegmentVolume" },
                "payload": {
                    "properties": {
                        "kind": { "const": "setSegmentVolume" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "setTrackMute" },
                "payload": {
                    "properties": {
                        "kind": { "const": "setTrackMute" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "updateDraftCanvasConfig" },
                "payload": {
                    "properties": {
                        "kind": { "const": "updateDraftCanvasConfig" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "updateSegmentVisual" },
                "payload": {
                    "properties": {
                        "kind": { "const": "updateSegmentVisual" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "setSegmentKeyframe" },
                "payload": {
                    "properties": {
                        "kind": { "const": "setSegmentKeyframe" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "removeSegmentKeyframe" },
                "payload": {
                    "properties": {
                        "kind": { "const": "removeSegmentKeyframe" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "requestPreviewDecode" },
                "payload": {
                    "properties": {
                        "kind": { "const": "requestPreviewDecode" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "releasePreviewFrame" },
                "payload": {
                    "properties": {
                        "kind": { "const": "releasePreviewFrame" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "requestPreviewFrame" },
                "payload": {
                    "properties": {
                        "kind": { "const": "requestPreviewFrame" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "requestPreviewSegment" },
                "payload": {
                    "properties": {
                        "kind": { "const": "requestPreviewSegment" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "invalidatePreviewCache" },
                "payload": {
                    "properties": {
                        "kind": { "const": "invalidatePreviewCache" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "startExport" },
                "payload": {
                    "properties": {
                        "kind": { "const": "startExport" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "getExportJobStatus" },
                "payload": {
                    "properties": {
                        "kind": { "const": "getExportJobStatus" }
                    },
                    "required": ["kind"]
                }
            }
        },
        {
            "properties": {
                "command": { "const": "cancelExport" },
                "payload": {
                    "properties": {
                        "kind": { "const": "cancelExport" }
                    },
                    "required": ["kind"]
                }
            }
        }
    ])
}
