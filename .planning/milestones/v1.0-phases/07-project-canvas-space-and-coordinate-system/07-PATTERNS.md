# Phase 07: Project Canvas Space And Coordinate System - Pattern Map

**Mapped:** 2026-06-18
**Files analyzed:** 25
**Analogs found:** 25 / 25

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/draft_model/src/canvas.rs` | model | transform | `crates/draft_model/src/material.rs` | role-match |
| `crates/draft_model/src/draft.rs` | model | CRUD | `crates/draft_model/src/draft.rs` | exact |
| `crates/draft_model/src/validation.rs` | model | transform | `crates/draft_model/src/validation.rs` | exact |
| `crates/draft_model/src/lib.rs` | model/config | request-response | `crates/draft_model/src/lib.rs` | exact |
| `crates/draft_model/tests/schema_exports.rs` | test | batch | `crates/draft_model/tests/schema_exports.rs` | exact |
| `crates/draft_model/tests/draft_schema.rs` | test | batch | `crates/draft_model/tests/draft_schema.rs` | exact |
| `fixtures/draft/positive/*.json` | config | file-I/O | `fixtures/draft/positive/minimal-draft/project.json` | role-match |
| `fixtures/draft/negative/*canvas*/project.json` | config | file-I/O | `fixtures/draft/negative/invalid-unknown-field/project.json` | role-match |
| `crates/draft_commands/src/canvas.rs` | service | CRUD | `crates/draft_commands/src/timeline.rs` | role-match |
| `crates/draft_commands/src/lib.rs` | config | request-response | `crates/draft_commands/src/lib.rs` | exact |
| `crates/draft_commands/tests/*canvas*.rs` | test | CRUD | `crates/draft_commands/tests/timeline_commands.rs` | role-match |
| `crates/bindings_node/src/lib.rs` | route | request-response | `crates/bindings_node/src/lib.rs` | exact |
| `crates/bindings_node/tests/binding_smoke.rs` | test | request-response | `crates/bindings_node/tests/binding_smoke.rs` | exact |
| `crates/engine_core/src/normalize.rs` | service | transform | `crates/engine_core/src/normalize.rs` | exact |
| `crates/engine_core/tests/normalization.rs` | test | transform | `crates/engine_core/tests/normalization.rs` | exact |
| `crates/render_graph/src/graph.rs` | service | transform | `crates/render_graph/src/graph.rs` | exact |
| `crates/render_graph/tests/*canvas*.rs` | test | transform | `crates/ffmpeg_compiler/tests/common/mod.rs` | role-match |
| `crates/ffmpeg_compiler/src/job.rs` | service | transform | `crates/ffmpeg_compiler/src/job.rs` | exact |
| `crates/preview_service/src/service.rs` | service | file-I/O | `crates/preview_service/src/service.rs` | exact |
| `crates/bindings_node/src/preview_export_service.rs` | service | request-response | `crates/bindings_node/src/preview_export_service.rs` | exact |
| `apps/desktop-electron/src/renderer/commandHelpers.ts` | utility | request-response | `apps/desktop-electron/src/renderer/commandHelpers.ts` | exact |
| `apps/desktop-electron/src/renderer/App.tsx` | component/provider | event-driven | `apps/desktop-electron/src/renderer/App.tsx` | exact |
| `apps/desktop-electron/src/renderer/viewModel.ts` | store/utility | transform | `apps/desktop-electron/src/renderer/viewModel.ts` | exact |
| `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` | component | event-driven | `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` | exact |
| `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` | component | event-driven | `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` | exact |
| `apps/desktop-electron/tests/workspace.spec.ts` | test | event-driven | `apps/desktop-electron/tests/workspace.spec.ts` | exact |
| `scripts/phase7-source-guards.sh` | utility | batch | `scripts/phase4-source-guards.sh`, `scripts/phase5-source-guards.sh` | role-match |

## Pattern Assignments

### `crates/draft_model/src/canvas.rs` (model, transform)

**Analog:** `crates/draft_model/src/material.rs`

**Imports and type derives pattern** (lines 1-3, 25-39):
```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RationalFrameRate {
    pub numerator: u32,
    pub denominator: u32,
}
```

**Constructor/default helper pattern** (lines 32-39, 69-82):
```rust
impl RationalFrameRate {
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self { numerator, denominator }
    }
}

impl MaterialMetadata {
    pub fn empty() -> Self {
        Self { duration: None, width: None, height: None, frame_rate: None, has_video: false, has_audio: false, audio_sample_rate: None, audio_channels: None, probe_error: None }
    }
}
```

**Apply to Phase 07:** define `DraftCanvasConfig`, `CanvasAspectRatio`, `CanvasBackground`, and capability/status enums with `Serialize`, `Deserialize`, `JsonSchema`, `TS`, `rename_all = "camelCase"`, and `deny_unknown_fields`. Use existing `RationalFrameRate`; do not add float fps.

---

### `crates/draft_model/src/draft.rs` (model, CRUD)

**Analog:** `crates/draft_model/src/draft.rs`

**Draft field/default pattern** (lines 41-60):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub schema_version: DraftSchemaVersion,
    pub draft_id: DraftId,
    pub metadata: DraftMetadata,
    pub materials: Vec<Material>,
    pub tracks: Vec<Track>,
}

impl Draft {
    pub fn new(draft_id: impl Into<DraftId>, name: impl Into<String>) -> Self {
        Self {
            schema_version: DraftSchemaVersion::current(),
            draft_id: draft_id.into(),
            metadata: DraftMetadata::new(name),
            materials: Vec::new(),
            tracks: Vec::new(),
        }
    }
}
```

**Apply to Phase 07:** add required `canvas_config` serialized as `canvasConfig` to `Draft`, initialized in `Draft::new` with 1920 x 1080, 30/1, 16:9, black background.

---

### `crates/draft_model/src/validation.rs` (model, transform)

**Analog:** `crates/draft_model/src/validation.rs`

**Error enum/display pattern** (lines 14-24, 26-57):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DraftValidationError {
    InvalidSchemaVersion { found: String, expected: u32 },
    MissingRequiredSemanticField { field: String },
    InvalidTimerange { field: String, reason: String },
    InvalidRationalFrameRate { field: String, reason: String },
    DuplicateId { id_kind: String, id: String },
    DerivedArtifactLeakage { field: String },
    InvalidDraftJson { message: String },
}
```

**Required field and validation pattern** (lines 62-87, 90-120):
```rust
pub fn migrate_draft_json(value: serde_json::Value) -> Result<Draft, DraftValidationError> {
    reject_derived_artifact_fields(&value)?;
    let schema_version = value.get("schemaVersion").ok_or_else(|| missing_field("schemaVersion"))?;
    let version = schema_version_u32(schema_version)?;
    if version != DraftSchemaVersion::CURRENT_VALUE {
        return Err(DraftValidationError::InvalidSchemaVersion { found: version.to_string(), expected: DraftSchemaVersion::CURRENT_VALUE });
    }

    for field in ["draftId", "metadata", "materials", "tracks"] {
        if !value.get(field).is_some() {
            return Err(missing_field(field));
        }
    }

    let draft: Draft = serde_json::from_value(value).map_err(|error| DraftValidationError::InvalidDraftJson { message: error.to_string() })?;
    validate_draft(&draft)?;
    Ok(draft)
}

pub fn validate_draft(draft: &Draft) -> Result<(), DraftValidationError> {
    if !draft.schema_version.is_current() { /* current error pattern */ }
    if draft.draft_id.is_empty() { return Err(missing_field("draftId")); }
    if draft.metadata.name.trim().is_empty() { return Err(missing_field("metadata.name")); }
    for material in &draft.materials {
        if let Some(frame_rate) = &material.metadata.frame_rate {
            validate_frame_rate("materials[].metadata.frameRate", frame_rate)?;
        }
    }
}
```

**Apply to Phase 07:** include `canvasConfig` in required migrated fields, add validation helpers for positive `width`/`height`, rational `frameRate`, aspect ratio consistency, solid color hex, image background material references, and explicit unsupported/degraded background capability.

---

### `crates/draft_model/src/lib.rs` (model/config, request-response)

**Analog:** `crates/draft_model/src/lib.rs`

**Module/export pattern** (lines 13-32):
```rust
pub mod draft;
pub mod ids;
pub mod material;
pub mod time;
pub mod timeline;
pub mod validation;

pub use draft::{Draft, DraftMetadata, DraftSchemaVersion};
pub use material::{Material, MaterialKind, MaterialMetadata, MaterialStatus, RationalFrameRate};
pub use validation::{DraftValidationError, migrate_draft_json, validate_draft};
```

**Command enum/envelope pattern** (lines 48-110, 112-143):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CommandName {
    AddSegment,
    SetTrackMute,
    StartExport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandPayload {
    AddSegment(AddSegmentCommandPayload),
    SetTrackMute(SetTrackMuteCommandPayload),
    StartExport(StartExportCommandPayload),
}

impl CommandPayload {
    pub fn command_name(&self) -> CommandName {
        match self {
            Self::AddSegment(_) => CommandName::AddSegment,
            Self::SetTrackMute(_) => CommandName::SetTrackMute,
            Self::StartExport(_) => CommandName::StartExport,
        }
    }
}
```

**Apply to Phase 07:** add `pub mod canvas;`, re-export canvas types, add `UpdateDraftCanvasConfig` command name/payload variant, and keep command/payload mismatch protection.

---

### `crates/draft_commands/src/canvas.rs` (service, CRUD)

**Analog:** `crates/draft_commands/src/timeline.rs`

**Imports pattern** (lines 3-15):
```rust
use draft_model::{
    CommandEvent, CommandPayload, CommandState, Draft, TimelineCommandResponse,
    TimelineSelection, validate_draft,
};

use crate::{
    TimelineCommandError,
    history::{push_undo_snapshot, redo_timeline_edit, undo_timeline_edit},
};
```

**Dispatch pattern** (lines 136-149):
```rust
pub fn execute_timeline_edit(payload: CommandPayload) -> Result<TimelineCommandResponse, TimelineCommandError> {
    match payload {
        CommandPayload::AddSegment(payload) => add_segment(
            &payload.draft,
            &payload.command_state,
            &payload.selection,
            payload.track_id,
            payload.segment_id,
            payload.material_id,
            payload.source_timerange,
            payload.target_timerange,
        ),
        _ => unreachable!("command/payload pair was validated during deserialization"),
    }
}
```

**Clone/validate/respond pattern** (lines 543-571, 608-627, 648-664):
```rust
pub fn delete_segment(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    next_draft.tracks[track_index].segments.remove(segment_index);
    validate_timeline_rules(&next_draft)?;

    Ok(response_with_events(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "deleteSegment"),
        next_selection,
        "segmentDeleted",
        extra_events,
    ))
}

fn response_with_events(...) -> TimelineCommandResponse {
    TimelineCommandResponse { draft, command_state: command_state.state, selection, events }
}

fn command_state_after_commit(...) -> CommandStateWithEvents {
    let (state, pruned) = push_undo_snapshot(command_state, draft, selection, label);
    CommandStateWithEvents { state, events }
}
```

**Apply to Phase 07:** canvas command should clone draft, replace `canvas_config`, call `validate_draft` or shared timeline validation, push undo label `updateDraftCanvasConfig`, keep selection unchanged, and emit `draftCanvasConfigUpdated`.

---

### `crates/draft_commands/src/history.rs` (service, event-driven)

**Analog:** `crates/draft_commands/src/history.rs`

**Undo/redo snapshot pattern** (lines 12-30, 52-78, 80-111):
```rust
pub fn push_undo_snapshot(command_state: &CommandState, draft: &Draft, selection: &TimelineSelection, label: impl Into<String>) -> (CommandState, bool) {
    let mut next_state = command_state.clone();
    if next_state.max_history_entries == 0 {
        next_state.max_history_entries = DEFAULT_HISTORY_LIMIT;
    }
    next_state.undo_stack.push(CommandHistorySnapshot {
        draft: draft.clone(),
        selection: selection.clone(),
        label: Some(label.into()),
    });
    clear_redo_after_commit(&mut next_state);
    let pruned = prune_history_to_limit(&mut next_state);
    (next_state, pruned)
}

pub fn undo_timeline_edit(...) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_state = command_state.clone();
    let Some(snapshot) = next_state.undo_stack.pop() else { /* HistoryEmpty */ };
    next_state.redo_stack.push(CommandHistorySnapshot { draft: draft.clone(), selection: selection.clone(), label: Some("redo snapshot".to_owned()) });
    Ok(TimelineCommandResponse { draft: snapshot.draft, command_state: next_state, selection: snapshot.selection, events: vec![event("undoCommitted")] })
}
```

**Apply to Phase 07:** do not create a separate canvas history. Reuse the same `CommandState` stacks.

---

### `crates/bindings_node/src/lib.rs` (route, request-response)

**Analog:** `crates/bindings_node/src/lib.rs`

**Allowlist and envelope parsing pattern** (lines 52-104):
```rust
pub fn execute_command(command: serde_json::Value) -> Result<serde_json::Value> {
    let command_name = raw_command_name(&command);

    if let Some(name) = command_name.as_deref() {
        if !matches!(name, "ping" | "version" | "addSegment" | "startExport") {
            return to_js_value(error_envelope(
                CommandErrorKind::UnsupportedCommand,
                format!("Unsupported command: {name}"),
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
        CommandName::ImportMaterial => match envelope.payload { /* payload match */ },
        _ => timeline_command(envelope.command, envelope.payload),
    }
}
```

**Timeline route grouping pattern** (lines 157-170):
```rust
CommandName::AddSegment
| CommandName::SelectTimelineSegments
| CommandName::MoveSegment
| CommandName::SplitSegment
| CommandName::TrimSegment
| CommandName::DeleteSegment
| CommandName::UndoTimelineEdit
| CommandName::RedoTimelineEdit
| CommandName::AddTextSegment
| CommandName::EditTextSegment
| CommandName::AddAudioSegment
| CommandName::SetSegmentVolume
| CommandName::SetTrackMute => timeline_command(envelope.command, envelope.payload),
```

**Apply to Phase 07:** add `"updateDraftCanvasConfig"` to the raw allowlist and route it through the same Rust command response path as other undoable draft edits.

---

### `crates/engine_core/src/normalize.rs` (service, transform)

**Analog:** `crates/engine_core/src/normalize.rs`

**Profile/default/validation pattern** (lines 14-51):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineProfile {
    pub frame_rate: RationalFrameRate,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub text_layout: Option<TextLayoutProfile>,
}

impl EngineProfile {
    pub fn mvp_default() -> Self {
        Self {
            frame_rate: RationalFrameRate::new(30, 1),
            canvas_width: 1920,
            canvas_height: 1080,
            text_layout: Some(TextLayoutProfile::mvp_default()),
        }
    }

    pub fn validate(&self) -> Result<(), EngineError> {
        if self.frame_rate.numerator == 0 || self.frame_rate.denominator == 0 { /* InvalidFrameRate */ }
        if self.canvas_width == 0 || self.canvas_height == 0 { /* InvalidEngineProfile */ }
        if let Some(text_layout) = &self.text_layout {
            text_layout.validate(self.canvas_width, self.canvas_height)?;
        }
        Ok(())
    }
}
```

**Normalize pattern** (lines 215-221, 273-280):
```rust
pub fn normalize_draft(draft: &Draft, profile: &EngineProfile) -> Result<NormalizedDraft, EngineError> {
    profile.validate()?;
    validate_draft(draft)?;
    /* normalize tracks/materials */
    tracks.push(NormalizedTrack {
        track_id: track.track_id.clone(),
        kind: track.kind,
        name: track.name.clone(),
        muted: track.muted,
        stack_index,
        segments,
    });
}
```

**Apply to Phase 07:** add `EngineProfile::from_draft_canvas(&Draft)` and migrate production callers from `mvp_default()` to draft-owned profile resolution. Keep `mvp_default()` only for tests/convenience.

---

### `crates/render_graph/src/graph.rs` (service, transform)

**Analog:** `crates/render_graph/src/graph.rs`

**Canvas/profile propagation pattern** (lines 15-34, 173-183):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraph {
    pub draft_id: DraftId,
    pub canvas: RenderCanvas,
    pub target_timerange: TargetTimerange,
    pub frame_rate: RationalFrameRate,
    pub materials: Vec<RenderMaterial>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderCanvas {
    pub width: u32,
    pub height: u32,
}

pub fn build_render_graph(normalized: &NormalizedDraft, range: &RenderRangeState) -> Result<RenderGraph, RenderGraphError> {
    if range.frames.is_empty() { /* EmptyRenderRange */ }
}
```

**Support/degraded intent pattern** (lines 87-110, 164-171):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderFilterIntent {
    pub name: String,
    pub parameters: BTreeMap<String, String>,
    pub support: RenderIntentSupport,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderIntentSupport {
    Supported,
    Degraded,
}
```

**Apply to Phase 07:** propagate canvas background/capability from normalized state. Use explicit `Supported`/`Degraded` style diagnostics for blur/image background rather than silent fallback.

---

### `crates/ffmpeg_compiler/src/job.rs` (service, transform)

**Analog:** `crates/ffmpeg_compiler/src/job.rs`

**Job validation profile pattern** (lines 97-109, 160-189):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FfmpegJob {
    pub job_id: String,
    pub output_kind: FfmpegOutputKind,
    pub output_path: String,
    pub inputs: Vec<FfmpegInput>,
    pub sidecars: Vec<FfmpegSidecar>,
    pub filter_script: String,
    pub encode_settings: EncodeSettings,
    pub validation: OutputValidationExpectation,
    pub args: Vec<OsString>,
}

pub struct EncodeSettings {
    pub dimensions: OutputDimensionsSnapshot,
    pub frame_rate: RationalFrameRate,
}

pub struct OutputValidationExpectation {
    pub expected_duration: Microseconds,
    pub expected_frame_rate: RationalFrameRate,
    pub expected_width: u32,
    pub expected_height: u32,
    pub expect_audio_stream: bool,
}
```

**Apply to Phase 07:** ensure compiler output dimensions/frame rate come from `RenderOutputProfile` built from draft canvas, and add tests that vertical/non-1920 profiles reach `EncodeSettings` and validation.

---

### `crates/preview_service/src/service.rs` (service, file-I/O)

**Analog:** `crates/preview_service/src/service.rs`

**Service request/response/error pattern** (lines 17-41, 43-73, 75-118):
```rust
pub struct PreviewServiceConfig {
    pub cache_root: PathBuf,
    pub ffmpeg_path: PathBuf,
    pub compiler_capabilities: CompilerCapabilities,
    pub preview_frame_dimensions: OutputDimensions,
    pub preview_segment_dimensions: OutputDimensions,
}

pub struct PreviewFrameRequest {
    pub draft: Draft,
    pub target_time: Microseconds,
}

pub struct PreviewFrameResponse {
    pub artifact: PreviewArtifact,
    pub cache_entry: PreviewCacheEntry,
    pub ffmpeg_job: FfmpegJob,
    pub from_cache: bool,
}
```

**Current hard-coded profile path to replace** (lines 177-231):
```rust
fn prepare_preview(...) -> Result<PreparedPreview, PreviewServiceError> {
    let normalized = normalize_draft(draft, &EngineProfile::mvp_default()).map_err(|error| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::EngineFailed,
            format!("preview engine normalization failed: {error}"),
        )
    })?;
    let range = resolve_render_range(&normalized, target_timerange.clone()).map_err(|error| { /* EngineFailed */ })?;
    let graph = build_render_graph(&normalized, &range).map_err(|error| { /* RenderGraphFailed */ })?;

    let output_profile = match profile {
        PreviewCacheProfile::FramePng => RenderOutputProfile::preview_frame_png(
            config.preview_frame_dimensions,
            range.frame_rate.clone(),
            target_timerange,
        ),
        PreviewCacheProfile::SegmentMp4 => RenderOutputProfile::preview_segment_mp4(
            config.preview_segment_dimensions,
            range.frame_rate.clone(),
            target_timerange,
        ),
    };
}
```

**Apply to Phase 07:** resolve `EngineProfile` from `draft.canvasConfig`; preview frame/segment dimensions should follow draft canvas profile or an explicit Rust-owned preview policy derived from it, not fixed `960 x 540` semantic defaults.

---

### `crates/bindings_node/src/preview_export_service.rs` (service, request-response)

**Analog:** `crates/bindings_node/src/preview_export_service.rs`

**Current export profile path to replace** (lines 427-457, 545-550, 588-596):
```rust
fn prepare_export_job(runtime: &RuntimeConfig, payload: StartExportCommandPayload) -> Result<PreparedExportJob, ExportCommandError> {
    let output_path = validate_output_path(&payload.output_path)?;
    let draft = payload.draft;
    let normalized = normalize_draft(&draft, &EngineProfile::mvp_default()).map_err(|error| {
        ExportCommandError::Engine(format!("export engine normalization failed: {error}"))
    })?;
    let target_timerange = draft_export_timerange(&draft, normalized.duration)?;
    let range = resolve_render_range(&normalized, target_timerange.clone()).map_err(|error| { /* Engine */ })?;
    let graph = build_render_graph(&normalized, &range).map_err(|error| { /* RenderGraph */ })?;
    let output_profile = RenderOutputProfile::export_mp4(
        export_dimensions(payload.preset),
        range.frame_rate,
        target_timerange,
        export_preset(payload.preset),
    );
    let validation = runtime_validation(&ffmpeg_job.validation);
}

fn export_dimensions(preset: ExportPreset) -> OutputDimensions {
    match preset {
        ExportPreset::H264AacDraft => OutputDimensions::new(1280, 720),
        ExportPreset::H264AacBalanced => OutputDimensions::new(1920, 1080),
    }
}

fn runtime_validation(compile: &CompileValidation) -> OutputValidationExpectation {
    OutputValidationExpectation::new()
        .with_expected_frame_rate(media_runtime::RationalFrameRate { numerator: compile.expected_frame_rate.numerator, denominator: compile.expected_frame_rate.denominator })
        .with_expected_dimensions(compile.expected_width, compile.expected_height)
}
```

**Apply to Phase 07:** export validation should expect draft canvas width/height/frame rate. Presets may still choose codec/quality, but not canonical dimensions/fps.

---

### `apps/desktop-electron/src/renderer/commandHelpers.ts` (utility, request-response)

**Analog:** `apps/desktop-electron/src/renderer/commandHelpers.ts`

**Generated type imports and context pattern** (lines 1-58):
```typescript
import type {
  AddSegmentCommandPayload,
  CommandEnvelope,
  CommandState,
  TimelineSelection
} from "../generated/CommandEnvelope";
import type { TimelineCommandResponse } from "../generated/CommandResultEnvelope";
import type { Draft } from "../generated/Draft";

export type CommandContext = {
  draft: Draft;
  commandState: CommandState;
  selection: TimelineSelection;
};
```

**Command builder pattern** (lines 117-130, 219-238):
```typescript
export function buildAddSegmentCommand(options: AddSegmentOptions): CommandEnvelope {
  const payload = {
    kind: "addSegment",
    draft: options.context.draft,
    commandState: options.context.commandState,
    selection: options.context.selection,
    trackId: options.trackId,
    segmentId: options.segmentId,
    materialId: options.materialId,
    sourceTimerange: options.sourceTimerange,
    targetTimerange: options.targetTimerange
  } satisfies AddSegmentCommandPayload & { kind: "addSegment" };

  return envelope("addSegment", payload);
}
```

**Apply to Phase 07:** add `buildUpdateDraftCanvasConfigCommand(context, canvasConfig)` using generated payload types. Renderer may build envelopes only; it must not mutate `draft.canvasConfig` locally.

---

### `apps/desktop-electron/src/renderer/App.tsx` (component/provider, event-driven)

**Analog:** `apps/desktop-electron/src/renderer/App.tsx`

**Command execution/apply pattern** (lines 166-239):
```typescript
async function executeDraftCommand<T>(
  buildCommand: DraftCommandBuilder,
  pendingCommand: string,
  applyResult: DraftCommandResultApplier<T>
): Promise<void> {
  if (commandInFlightRef.current) {
    setWorkspace((current) => ({ ...current, commandError: commandErrorMessage("上一个操作仍在执行，请等待剪辑核心返回") }));
    return;
  }

  commandInFlightRef.current = true;
  setWorkspace((current) => ({ ...current, pendingCommand, commandError: null }));

  try {
    const command = buildCommand(workspaceRef.current);
    const result = await window.videoEditorCore.executeCommand<T>(command);
    setWorkspace((current) => applyResult(current, result));
  } finally {
    commandInFlightRef.current = false;
  }
}

async function executeTimelineCommand(buildCommand: DraftCommandBuilder, pendingCommand: string): Promise<void> {
  await executeDraftCommand<TimelineCommandResponse>(buildCommand, pendingCommand, (current, result) => {
    const applied = applyTimelineCommandResult({ draft: current.draft, commandState: current.commandState, selection: current.selection }, result);
    return { ...current, draft: applied.state.draft, commandState: applied.state.commandState, selection: applied.state.selection, materials: applied.state.draft.materials, pendingCommand: null, commandError: applied.errorMessage };
  });
}
```

**Apply to Phase 07:** add `handleUpdateDraftCanvasConfig` that uses the same pending/error/apply flow and passes result state into `Inspector`.

---

### `apps/desktop-electron/src/renderer/viewModel.ts` (store/utility, transform)

**Analog:** `apps/desktop-electron/src/renderer/viewModel.ts`

**Workspace state and initial draft pattern** (lines 62-86, 169-190):
```typescript
export type WorkspaceState = {
  draft: Draft;
  commandState: CommandState;
  selection: TimelineSelection;
  materials: Material[];
  preview: PreviewDisplayState;
  export: ExportDisplayState;
  pendingCommand: string | null;
  commandError: string | null;
};

export const initialWorkspaceDraft: Draft = {
  schemaVersion: 1,
  draftId: "draft-phase-04-workspace",
  metadata: {
    name: "未命名草稿",
    description: "阶段四桌面工作区展示草稿"
  },
  materials: [
    {
      metadata: {
        width: 1920,
        height: 1080,
        frameRate: { numerator: 30, denominator: 1 }
      }
    }
  ],
  tracks: []
};
```

**Apply to Phase 07:** update `initialWorkspaceDraft` with generated `canvasConfig`, and add pure formatters for canvas ratio, size, fps, background state, and preview readout. Keep these display-only.

---

### `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` (component, event-driven)

**Analog:** `apps/desktop-electron/src/renderer/workspace/Inspector.tsx`

**Local form state pattern** (lines 17-43, 60-102, 104-119):
```typescript
type TextFormState = {
  content: string;
  fontSize: number;
  color: string;
  alignment: TextAlignment;
};

const DEFAULT_TEXT_STATE: TextFormState = {
  content: "",
  fontSize: 36,
  color: "#ffffff",
  alignment: "center",
};

useEffect(() => {
  if (selected === null) {
    setTextState(DEFAULT_TEXT_STATE);
    return;
  }
  setTextState({
    content: selected.segment.text.content,
    fontSize: selected.segment.text.style.fontSize,
    color: selected.segment.text.style.color,
    alignment: selected.segment.text.style.alignment,
  });
}, [selected?.segment.segmentId, selected?.segment.text?.content]);
```

**Current no-selection hard-code to replace** (lines 142-163):
```tsx
{selected === null ? (
  <>
    {activeTab === "画面" ? (
      <section className="inspector-section" aria-label="草稿参数" role="tabpanel">
        <div className="inspector-section-title">
          <h3>草稿参数</h3>
        </div>
        <div className="empty-state compact-empty">
          <strong>未选择片段</strong>
          <span>选择时间线片段后，可在这里调整画面、音频、文字和关键帧参数。</span>
        </div>
        <dl className="inspector-list compact">
          <InspectorDatum label="草稿名称" value={workspace.draft.metadata.name} />
          <InspectorDatum label="画布比例" value="16:9" />
          <InspectorDatum label="画布尺寸" value="1920 x 1080" />
          <InspectorDatum label="序列时长" value={formatMicroseconds(sequenceDuration)} />
        </dl>
      </section>
    ) : null}
  </>
) : null}
```

**Apply to Phase 07:** replace hard-coded readouts with compact controls for `画布比例`, `画布尺寸`, `帧率`, `画布背景`, `应用草稿参数`; keep local form state temporary and commit through handler from `App.tsx`.

---

### `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` (component, event-driven)

**Analog:** `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx`

**Current monitor hard-code to replace** (lines 41-49, 77-89):
```tsx
const MONITOR_CONTROLS: readonly MonitorControl[] = [
  { label: "播放", symbol: "▶" },
  { label: "停止", symbol: "■" },
  { label: "适应窗口", symbol: "□" },
  { label: "画面比例", symbol: "16:9" },
  { label: "全屏", symbol: "⛶" }
];

return (
  <div className="preview-shell">
    <div className="preview-titlebar">
      <strong>{draftName}</strong>
      <span>预览命令已接入</span>
    </div>

    <div className="preview-canvas" aria-label="预览画面">
      <div className="preview-placeholder">
        <span>{preview.frameArtifactPath === null ? "等待请求预览帧" : "预览帧已返回"}</span>
      </div>
    </div>
  </div>
);
```

**Apply to Phase 07:** pass canvas display state from `App`/`viewModel`, set `preview-canvas` aspect ratio from draft canvas width/height, update readout to `画布 {ratio} · {width} x {height} · {fps}`, and keep ratio button display-only unless routed through Rust command.

---

### `apps/desktop-electron/tests/workspace.spec.ts` (test, event-driven)

**Analog:** `apps/desktop-electron/tests/workspace.spec.ts`

**Electron launch and command spy pattern** (lines 40-57, 68-93):
```typescript
async function launchWorkspaceApp(options = {}): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: "1",
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await expectVisibleWorkspaceRegions(page);
  return { app, page };
}

async function expectCommandCall(app: ElectronApplication, command: CommandName): Promise<void> {
  await expect.poll(async () => (await readExecuteCommandCalls(app)).some((call) => call.command === command)).toBe(true);
}
```

**Layout/no-overlap pattern** (lines 95-141, 227-232):
```typescript
async function setViewportSizeAndVerifyLayout(app: ElectronApplication, page: Page, width: number, height: number): Promise<void> {
  await app.evaluate(async ({ BrowserWindow }, size) => {
    const window = BrowserWindow.getAllWindows()[0];
    window.setSize(size.width, size.height);
  }, { width, height });
  await page.setViewportSize({ width, height });
  await expectVisibleWorkspaceRegions(page);

  const boxes = {
    left: await expectStableBox(page.locator('[aria-label="素材面板"]'), `素材面板 ${width}x${height}`),
    preview: await expectStableBox(page.locator('[aria-label="预览窗口"]'), `预览窗口 ${width}x${height}`),
    inspector: await expectStableBox(page.locator('[aria-label="属性检查器"]'), `属性检查器 ${width}x${height}`),
  };
  expectNoOverlap(boxes.preview, boxes.inspector, "预览窗口", "属性检查器");
}

async function expectPreviewCanvasAspectRatio(page: Page): Promise<void> {
  const canvas = await expectStableBox(page.locator(".preview-canvas"), "预览画面 16:9");
  const ratio = canvas.width / canvas.height;
  expect(Math.abs(ratio - 16 / 9), "预览画面保持 16:9").toBeLessThanOrEqual(0.04);
}
```

**Apply to Phase 07:** add tests at `1280x800` and `1120x720` for visible `草稿参数`, `画布比例`, `画布尺寸`, `帧率`, `画布背景`, no overflow, and recorded `updateDraftCanvasConfig` command after applying changes. Update aspect ratio assertion to use draft canvas.

---

### `scripts/phase7-source-guards.sh` (utility, batch)

**Analogs:** `scripts/phase4-source-guards.sh`, `scripts/phase5-source-guards.sh`

**Guard helper pattern** (phase4 lines 1-26):
```bash
#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase4-source-guards: rg is required" >&2
  exit 1
fi

fail_if_matches() {
  local description="$1"
  local pattern="$2"
  shift 2

  local output
  if output=$(rg -n --pcre2 "$pattern" "$@" 2>/dev/null); then
    echo "phase4-source-guards: ${description}" >&2
    echo "$output" >&2
    exit 1
  fi
}
```

**Renderer ownership guard pattern** (phase4 lines 40-74):
```bash
fail_if_matches \
  "renderer must not mutate Draft.tracks or Track.segments arrays directly" \
  '(?:draft|current|nextDraft|workspace\.draft)\.tracks\s*=|\.tracks\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(' \
  "${renderer_files[@]}"

fail_if_matches \
  "renderer must not construct FFmpeg, render graph, preview/export, or media artifact behavior" \
  '\b(?:ffmpeg|ffprobe|filter_complex|renderGraph|render_graph|ffmpegCompiler|ffmpeg_compiler|ffmpegScripts|exportScript|previewCache|previewFrame|thumbnail|waveform|proxy)\b|render graph' \
  --glob '!commandHelpers.ts' \
  "${renderer_files[@]}"
```

**Phase 5 contract drift pattern** (phase5 lines 39-59):
```bash
rg -n "requestPreviewFrame|requestPreviewSegment|invalidatePreviewCache" \
  apps/desktop-electron/src/generated/CommandEnvelope.ts >/dev/null

rg -n "durationSeconds|duration_seconds|seconds: f32|seconds: f64|\\bf32\\b|\\bf64\\b" \
  crates/draft_model/src schemas/command.schema.json apps/desktop-electron/src/generated && {
    echo "preview/export command contracts must use integer microseconds, not naked floating time" >&2
    exit 1
  } || true
```

**Apply to Phase 07:** block renderer direct assignment/mutation of `draft.canvasConfig`, `aspectRatio`, `frameRate`, `canvasBackground`, output dimensions, normalized coordinate semantics, and hard-coded production `1920 x 1080`/`16:9` display outside generated helpers/tests where explicitly allowed. Require generated contracts contain `updateDraftCanvasConfig` and canvas types.

## Shared Patterns

### Rust Contract Generation

**Source:** `crates/draft_model/tests/schema_exports.rs`
**Apply to:** `canvas.rs`, `lib.rs`, generated TS/schema files

**Generated artifact pattern** (lines 45-58, 60-104, 143-173, 708-722):
```rust
#[test]
fn schema_exports_generated_contract_artifacts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/command.schema.json");
    let draft_schema_path = root.join("schemas/draft.schema.json");
    let generated_dir = root.join("apps/desktop-electron/src/generated");

    let schema_json = command_schema_json();
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));

    let draft_ts = ts_contract(&[
        export_decl::<DraftSchemaVersion>(),
        export_decl::<DraftMetadata>(),
        export_decl::<RationalFrameRate>(),
        export_decl::<Draft>(),
    ]);
    assert_or_update_contract_file(generated_dir.join("Draft.ts"), &draft_ts);
}

fn command_schema_json() -> String {
    let schema = schema_for!(CommandEnvelope);
    let mut schema_value = serde_json::to_value(schema).expect("command schema should serialize to JSON value");
    include_command_contract_schema::<TimelineCommandResponse>(&mut schema_value, "TimelineCommandResponse");
}
```

Add `DraftCanvasConfig`, aspect/background types, and `UpdateDraftCanvasConfigCommandPayload` to the same export lists and schema includes.

### Command Error And Response Shape

**Source:** `crates/bindings_node/src/lib.rs`, `crates/draft_commands/src/timeline.rs`
**Apply to:** all canvas command routes

```rust
fn error_envelope(kind: CommandErrorKind, message: String, command: Option<String>) -> CommandResultEnvelope<serde_json::Value> {
    CommandResultEnvelope {
        ok: false,
        data: None,
        error: Some(CommandError { kind, message, command }),
        events: Vec::new(),
    }
}

TimelineCommandResponse {
    draft,
    command_state: command_state.state,
    selection,
    events,
}
```

### Preview/Export Semantic Ownership

**Source:** `crates/preview_service/src/service.rs`, `crates/bindings_node/src/preview_export_service.rs`
**Apply to:** preview frame, preview segment, export

Replace these production usages:
```rust
normalize_draft(draft, &EngineProfile::mvp_default())
normalize_draft(&draft, &EngineProfile::mvp_default())
export_dimensions(payload.preset)
```

with a Rust-owned draft canvas resolution path, e.g. `EngineProfile::from_draft_canvas(&draft)` followed by `normalize_draft(&draft, &profile)`.

### Renderer Command Ownership

**Source:** `apps/desktop-electron/src/renderer/App.tsx`, `commandHelpers.ts`
**Apply to:** inspector canvas controls

```typescript
const command = buildCommand(workspaceRef.current);
const result = await window.videoEditorCore.executeCommand<T>(command);
const applied = applyTimelineCommandResult(
  { draft: current.draft, commandState: current.commandState, selection: current.selection },
  result
);
return {
  ...current,
  draft: applied.state.draft,
  commandState: applied.state.commandState,
  selection: applied.state.selection,
  pendingCommand: null,
  commandError: applied.errorMessage
};
```

Local inspector state may validate inputs, but committed draft state must come back through this Rust command response.

### UI Copy And Layout

**Source:** `apps/desktop-electron/src/renderer/workspace/Inspector.tsx`, `PreviewMonitor.tsx`, `workspace.spec.ts`
**Apply to:** canvas UI and tests

Required visible Chinese labels from the UI spec should be asserted in Playwright: `草稿参数`, `画布比例`, `画布尺寸`, `帧率`, `画布背景`, `黑色`, `纯色`, `模糊填充`, `图片背景`, `未接入`, `应用草稿参数`, and `坐标以画布中心为原点`.

## No Analog Found

All Phase 07 files have close analogs in the current codebase. New `canvas.rs` files are new domain files, but their model/command patterns map directly to existing `material.rs`, `draft.rs`, `validation.rs`, and `timeline.rs`.

## Metadata

**Analog search scope:** `crates/`, `apps/desktop-electron/src/renderer/`, `apps/desktop-electron/tests/`, `scripts/`, `fixtures/draft/`, `schemas/`
**Files scanned:** 100+
**Pattern extraction date:** 2026-06-18
