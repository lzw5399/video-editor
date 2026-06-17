use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use draft_model::{
    AddAudioSegmentCommandPayload, AddSegmentCommandPayload, AddTextSegmentCommandPayload,
    CancelExportCommandPayload, CommandEnvelope, CommandError, CommandErrorKind, CommandEvent,
    CommandHistorySnapshot, CommandName, CommandPayload, CommandResultEnvelope, CommandState,
    DeleteSegmentCommandPayload, Draft, DraftId, DraftMetadata, DraftSchemaVersion,
    EditTextSegmentCommandPayload, ExportDiagnostic, ExportDiagnosticKind, ExportJobPhase,
    ExportJobStatusResponse, ExportPreset, ExportValidationReport, Filter,
    GetExportJobStatusCommandPayload, ImportMaterialCommandPayload, ImportMaterialResponse,
    InvalidatePreviewCacheCommandPayload, Keyframe, ListMaterialsCommandPayload,
    ListMaterialsResponse, ListMissingMaterialsCommandPayload, ListMissingMaterialsResponse,
    MainTrackMagnet, Material, MaterialId, MaterialKind, MaterialMetadata, MaterialStatus,
    Microseconds, MissingMaterialCommandDiagnostic, MissingMaterialCommandDiagnosticKind,
    MoveSegmentCommandPayload, PingCommandPayload, PreviewArtifactResponse, PreviewCacheEntryRef,
    PreviewCacheInvalidationResponse, PreviewDiagnostic, PreviewDiagnosticKind,
    PreviewOutputProfile, PreviewStatus, ProbeMediaRuntimeCommandPayload,
    ProbeRuntimeCapabilitiesCommandPayload, RationalFrameRate, RedoTimelineEditCommandPayload,
    RequestPreviewFrameCommandPayload, RequestPreviewSegmentCommandPayload,
    RuntimeBinaryCapability, RuntimeBinaryKind, RuntimeCapabilityReport, RuntimeCapabilityStatus,
    RuntimeFeatureCapability, RuntimeFontCapability, RuntimeLicensePosture, Segment, SegmentId,
    SegmentVolume, SelectTimelineSegmentsCommandPayload, SetSegmentVolumeCommandPayload,
    SetTrackMuteCommandPayload, SnappingSettings, SourceTimerange, SplitSegmentCommandPayload,
    StartExportCommandPayload, TargetTimerange, TextAlignment, TextBackground, TextSegment,
    TextShadow, TextStroke, TextStyle, TimelineCommandResponse, TimelineSelection, Track, TrackId,
    TrackKind, Transition, TrimSegmentCommandPayload, UndoTimelineEditCommandPayload,
    VersionCommandPayload,
};
use schemars::{Schema, schema_for};
use serde_json::json;
use ts_rs::{Config, TS};

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
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));

    let draft_schema_json = draft_schema_json();
    assert_draft_schema_rejects_zero_frame_rates(&draft_schema_json);
    assert_or_update_contract_file(&draft_schema_path, &format!("{draft_schema_json}\n"));

    let command_envelope_ts = ts_contract_with_prelude(
        "import type { Draft, MaterialId, MaterialKind, Microseconds, SegmentId, SegmentVolume, SourceTimerange, TargetTimerange, TextSegment, TrackId, TrimSegmentDirection } from \"./Draft\";\n\n",
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
            export_decl::<AddAudioSegmentCommandPayload>(),
            export_decl::<SetSegmentVolumeCommandPayload>(),
            export_decl::<SetTrackMuteCommandPayload>(),
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
    assert_or_update_contract_file(
        generated_dir.join("CommandEnvelope.ts"),
        &command_envelope_ts,
    );

    let command_result_ts = ts_contract_with_prelude(
        "import type { Draft, Material, MaterialId, MaterialStatus, Microseconds, RationalFrameRate, TargetTimerange } from \"./Draft\";\nimport type { CommandState, ExportPreset, PreviewOutputProfile, TimelineSelection } from \"./CommandEnvelope\";\n\n",
        &[
            export_decl::<CommandErrorKind>(),
            export_decl::<CommandError>(),
            export_decl::<CommandEvent>(),
            export_decl::<CommandResultEnvelope<()>>(),
            export_decl::<PreviewStatus>(),
            export_decl::<PreviewDiagnosticKind>(),
            export_decl::<PreviewDiagnostic>(),
            export_decl::<PreviewArtifactResponse>(),
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
            export_decl::<RuntimeCapabilityReport>(),
            export_decl::<MissingMaterialCommandDiagnosticKind>(),
            export_decl::<MissingMaterialCommandDiagnostic>(),
            export_decl::<ImportMaterialResponse>(),
            export_decl::<ListMaterialsResponse>(),
            export_decl::<ListMissingMaterialsResponse>(),
            export_decl::<TimelineCommandResponse>(),
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
        export_decl::<MaterialKind>(),
        export_decl::<MaterialStatus>(),
        export_decl::<MaterialMetadata>(),
        export_decl::<Material>(),
        export_decl::<TrackKind>(),
        export_decl::<MainTrackMagnet>(),
        export_decl::<SourceTimerange>(),
        export_decl::<TargetTimerange>(),
        export_decl::<Keyframe>(),
        export_decl::<Filter>(),
        export_decl::<Transition>(),
        export_decl::<TextAlignment>(),
        export_decl::<TextStroke>(),
        export_decl::<TextShadow>(),
        export_decl::<TextBackground>(),
        export_decl::<TextStyle>(),
        export_decl::<TextSegment>(),
        export_decl::<SegmentVolume>(),
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
        "import type { Draft, MaterialId, MaterialKind, Microseconds, SegmentId, SegmentVolume, SourceTimerange, TargetTimerange, TextSegment, TrackId, TrimSegmentDirection } from \"./Draft\";\n\n",
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
            export_decl::<AddAudioSegmentCommandPayload>(),
            export_decl::<SetSegmentVolumeCommandPayload>(),
            export_decl::<SetTrackMuteCommandPayload>(),
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
        "import type { Draft, Material, MaterialId, MaterialStatus, Microseconds, RationalFrameRate, TargetTimerange } from \"./Draft\";\nimport type { CommandState, ExportPreset, PreviewOutputProfile, TimelineSelection } from \"./CommandEnvelope\";\n\n",
        &[
            export_decl::<CommandErrorKind>(),
            export_decl::<CommandError>(),
            export_decl::<CommandEvent>(),
            export_decl::<CommandResultEnvelope<()>>(),
            export_decl::<PreviewStatus>(),
            export_decl::<PreviewDiagnosticKind>(),
            export_decl::<PreviewDiagnostic>(),
            export_decl::<PreviewArtifactResponse>(),
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
            export_decl::<RuntimeCapabilityReport>(),
            export_decl::<MissingMaterialCommandDiagnosticKind>(),
            export_decl::<MissingMaterialCommandDiagnostic>(),
            export_decl::<ImportMaterialResponse>(),
            export_decl::<ListMaterialsResponse>(),
            export_decl::<ListMissingMaterialsResponse>(),
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
        export_decl::<TextStroke>(),
        export_decl::<TextShadow>(),
        export_decl::<TextBackground>(),
        export_decl::<TextStyle>(),
        export_decl::<TextSegment>(),
    ]);

    for expected_contract in [
        "TextSegment",
        "TextStyle",
        "TextAlignment",
        "AddTextSegmentCommandPayload",
        "EditTextSegmentCommandPayload",
    ] {
        assert!(
            schema_json.contains(expected_contract) || draft_ts.contains(expected_contract),
            "schema or draft TypeScript should include {expected_contract}"
        );
    }
    for expected_contract in [
        "AddTextSegmentCommandPayload",
        "EditTextSegmentCommandPayload",
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
        "RequestPreviewFrameCommandPayload",
        "RequestPreviewSegmentCommandPayload",
        "InvalidatePreviewCacheCommandPayload",
        "PreviewCacheEntryRef",
        "PreviewOutputProfile",
        "PreviewArtifactResponse",
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

    for forbidden in ["ffmpegArgs", "filterComplex", "cacheKeyFormula"] {
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
    ] {
        assert!(
            schema_json.contains(expected_contract)
                || command_envelope_ts.contains(expected_contract)
                || command_result_ts.contains(expected_contract),
            "runtime capability contracts should include {expected_contract}"
        );
    }

    assert!(
        schema_json.contains("probeRuntimeCapabilities")
            && command_envelope_ts.contains("probeRuntimeCapabilities"),
        "runtime capability command should be generated from Rust contracts"
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
        "import type { Draft, MaterialId, MaterialKind, Microseconds, SegmentId, SegmentVolume, SourceTimerange, TargetTimerange, TextSegment, TrackId, TrimSegmentDirection } from \"./Draft\";\n\n",
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
            export_decl::<AddAudioSegmentCommandPayload>(),
            export_decl::<SetSegmentVolumeCommandPayload>(),
            export_decl::<SetTrackMuteCommandPayload>(),
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
    )
}

fn command_result_ts_contract() -> String {
    ts_contract_with_prelude(
        "import type { Draft, Material, MaterialId, MaterialStatus, Microseconds, RationalFrameRate, TargetTimerange } from \"./Draft\";\nimport type { CommandState, ExportPreset, PreviewOutputProfile, TimelineSelection } from \"./CommandEnvelope\";\n\n",
        &[
            export_decl::<CommandErrorKind>(),
            export_decl::<CommandError>(),
            export_decl::<CommandEvent>(),
            export_decl::<CommandResultEnvelope<()>>(),
            export_decl::<PreviewStatus>(),
            export_decl::<PreviewDiagnosticKind>(),
            export_decl::<PreviewDiagnostic>(),
            export_decl::<PreviewArtifactResponse>(),
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
            export_decl::<RuntimeCapabilityReport>(),
            export_decl::<MissingMaterialCommandDiagnosticKind>(),
            export_decl::<MissingMaterialCommandDiagnostic>(),
            export_decl::<ImportMaterialResponse>(),
            export_decl::<ListMaterialsResponse>(),
            export_decl::<ListMissingMaterialsResponse>(),
            export_decl::<TimelineCommandResponse>(),
        ],
    )
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
    include_command_contract_schema::<RuntimeCapabilityReport>(
        &mut schema_value,
        "RuntimeCapabilityReport",
    );
    constrain_current_draft_schema_version(&mut schema_value);
    constrain_rational_frame_rate(&mut schema_value);
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

fn list_materials_command_with_frame_rate(numerator: u32, denominator: u32) -> serde_json::Value {
    json!({
        "command": "listMaterials",
        "payload": {
            "kind": "listMaterials",
            "draft": draft_value_with_frame_rate(numerator, denominator)
        }
    })
}

fn draft_value_with_frame_rate(numerator: u32, denominator: u32) -> serde_json::Value {
    json!({
        "schemaVersion": DraftSchemaVersion::CURRENT_VALUE,
        "draftId": "draft-schema-zero-frame-rate",
        "metadata": { "name": "Schema zero frame rate" },
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
