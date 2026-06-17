# Phase 03: Timeline Command Core - Pattern Map

**Mapped:** 2026-06-17
**Files analyzed:** 14 new/modified file targets
**Analogs found:** 14 / 14

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/draft_commands/Cargo.toml` | config | request-response | `crates/bindings_node/Cargo.toml` / existing workspace manifests | role-match |
| `crates/draft_commands/src/lib.rs` | service | request-response | `crates/draft_model/src/material.rs` | role-match |
| `crates/draft_commands/src/error.rs` | utility | request-response | `crates/draft_model/src/validation.rs` | role-match |
| `crates/draft_commands/src/history.rs` | store | event-driven | `crates/draft_model/src/material.rs` | partial |
| `crates/draft_commands/src/selection.rs` | model | event-driven | `crates/draft_model/src/timeline.rs` | role-match |
| `crates/draft_commands/src/snapping.rs` | service | transform | `crates/draft_model/src/validation.rs` | partial |
| `crates/draft_commands/src/timeline.rs` | service | CRUD | `crates/draft_model/src/material.rs` | role-match |
| `crates/draft_commands/tests/timeline_commands.rs` | test | CRUD | `crates/draft_model/tests/draft_fixtures.rs` | role-match |
| `crates/draft_model/src/lib.rs` | model | request-response | `crates/draft_model/src/lib.rs` | exact |
| `crates/draft_model/src/timeline.rs` | model | CRUD | `crates/draft_model/src/timeline.rs` | exact |
| `crates/draft_model/src/validation.rs` | utility | transform | `crates/draft_model/src/validation.rs` | exact |
| `crates/draft_model/tests/schema_exports.rs` | test | transform | `crates/draft_model/tests/schema_exports.rs` | exact |
| `crates/bindings_node/src/lib.rs` | route | request-response | `crates/bindings_node/src/lib.rs` | exact |
| `fixtures/draft/**timeline-command*.json` | test | request-response | `fixtures/draft/minimal-command.json` + `crates/draft_model/tests/schema_exports.rs` | role-match |

## Pattern Assignments

### `crates/draft_commands/Cargo.toml` (config, request-response)

**Analog:** existing crate manifests, especially `crates/draft_commands/Cargo.toml` and `crates/bindings_node/Cargo.toml`

**Dependency pattern:** keep this crate pure. Add `draft_model = { path = "../draft_model" }` only; do not add `bindings_node`, `project_store`, `media_runtime`, `media_runtime_desktop`, Electron, filesystem, preview, render, or FFmpeg dependencies.

**Boundary source:** `AGENTS.md` and `docs/runtime-boundaries.md`: pure semantic crates must own draft/timeline semantics and remain independent of platform/runtime concerns.

---

### `crates/draft_commands/src/lib.rs` (service, request-response)

**Analog:** `crates/draft_model/src/material.rs`

**Imports pattern** (`crates/draft_model/src/material.rs` lines 1-5):
```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Draft, DraftValidationError, MaterialId, Microseconds, validate_draft};
```

For `draft_commands`, copy the local-crate style but import from `draft_model`, not platform crates:
```rust
use draft_model::{Draft, validate_draft};
```

**Transactional mutation pattern** (`crates/draft_model/src/material.rs` lines 114-124):
```rust
pub fn add_material(draft: &mut Draft, material: Material) -> Result<(), DraftValidationError> {
    let original_materials = draft.materials.clone();
    draft.materials.push(material);

    if let Err(error) = validate_draft(draft) {
        draft.materials = original_materials;
        return Err(error);
    }

    Ok(())
}
```

Phase 3 should generalize this to clone the whole `Draft` plus session state, apply add/move/split/trim/delete/text/audio edits to the clone, run command-level validation and `validate_draft`, then commit by returning the cloned draft/state. Failed commands must return rejection/error events without mutating input draft or history.

---

### `crates/draft_commands/src/error.rs` (utility, request-response)

**Analog:** `crates/draft_model/src/validation.rs`

**Error enum pattern** (`crates/draft_model/src/validation.rs` lines 13-23):
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

**Display/source pattern** (`crates/draft_model/src/validation.rs` lines 25-59):
```rust
impl fmt::Display for DraftValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimerange { field, reason } => {
                write!(formatter, "invalid timerange {field}: {reason}")
            }
            Self::DuplicateId { id_kind, id } => {
                write!(formatter, "duplicate {id_kind} id {id}")
            }
            // ...
        }
    }
}

impl Error for DraftValidationError {}
```

Use the same tagged enum style for `TimelineCommandError`, with variants for locked track, segment not found, track not found, material not found, overlapping segment, incompatible track/material kind, source range exceeds material duration, timerange overflow, zero duration, invalid split point, history empty, and unsupported command. Keep diagnostic strings stable enough for tests, but prefer structured variants over parsing messages.

---

### `crates/draft_commands/src/history.rs` (store, event-driven)

**Analog:** `crates/draft_model/src/material.rs`

**Rollback-before-error pattern** (`crates/draft_model/src/material.rs` lines 126-145):
```rust
pub fn upsert_material(draft: &mut Draft, material: Material) -> Result<(), DraftValidationError> {
    let original_materials = draft.materials.clone();

    if let Some(existing) = draft
        .materials
        .iter_mut()
        .find(|existing| existing.material_id == material.material_id)
    {
        *existing = material;
    } else {
        draft.materials.push(material);
    }

    if let Err(error) = validate_draft(draft) {
        draft.materials = original_materials;
        return Err(error);
    }

    Ok(())
}
```

History should follow the same success-only commit principle: push undo snapshot only after all validations pass, clear redo only after commit, and leave history byte-for-byte unchanged on rejection. Snapshot history is acceptable for Phase 3 if bounded by a named default and covered by tests.

---

### `crates/draft_commands/src/selection.rs` (model, event-driven)

**Analog:** `crates/draft_model/src/timeline.rs`

**Struct and serde/TS pattern** (`crates/draft_model/src/timeline.rs` lines 19-49):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTimerange {
    pub start: Microseconds,
    pub duration: Microseconds,
}

impl SourceTimerange {
    pub fn new(start: impl Into<Microseconds>, duration: impl Into<Microseconds>) -> Self {
        Self {
            start: start.into(),
            duration: duration.into(),
        }
    }
}
```

Selection/session structs should use the same derive set and `camelCase`/`deny_unknown_fields` contract style when they cross the command boundary. Use `SegmentId` and `TrackId`; avoid UI-only indices as canonical state.

---

### `crates/draft_commands/src/snapping.rs` (service, transform)

**Analog:** `crates/draft_model/src/validation.rs` and `crates/draft_model/src/timeline.rs`

**Timerange validation pattern** (`crates/draft_model/src/validation.rs` lines 227-249):
```rust
fn validate_source_timerange(
    field: &str,
    timerange: &SourceTimerange,
) -> Result<(), DraftValidationError> {
    validate_duration(&format!("{field}.duration"), timerange.duration)
}

fn validate_duration(field: &str, duration: Microseconds) -> Result<(), DraftValidationError> {
    if duration.get() == 0 {
        return Err(DraftValidationError::InvalidTimerange {
            field: field.to_owned(),
            reason: "duration must be greater than zero microseconds".to_owned(),
        });
    }
    Ok(())
}
```

**MainTrackMagnet model pattern** (`crates/draft_model/src/timeline.rs` lines 51-65):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MainTrackMagnet {
    pub enabled: bool,
}

impl MainTrackMagnet {
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}
```

Snapping must use integer microseconds and checked `start + duration` helpers. Return command events for snapped and magnetized edits; do not make the UI recompute snap candidates.

---

### `crates/draft_commands/src/timeline.rs` (service, CRUD)

**Analog:** `crates/draft_model/src/material.rs` and `crates/draft_model/src/timeline.rs`

**Domain model to mutate** (`crates/draft_model/src/timeline.rs` lines 89-146):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Segment {
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub main_track_magnet: MainTrackMagnet,
    pub keyframes: Vec<Keyframe>,
    pub filters: Vec<Filter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub transition: Option<Transition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Track {
    pub track_id: TrackId,
    pub kind: TrackKind,
    pub name: String,
    pub muted: bool,
    pub locked: bool,
    pub segments: Vec<Segment>,
}
```

**Material metadata to enforce source bounds** (`crates/draft_model/src/material.rs` lines 41-67):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaterialMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub duration: Option<Microseconds>,
    // ...
    pub has_video: bool,
    pub has_audio: bool,
}
```

Command implementation should mutate `Draft.tracks` only through typed commands. Enforce locked tracks, no overlap on a single track, track/material compatibility, explicit `SourceTimerange` and `TargetTimerange` updates, material duration bounds when present, and exact source/target updates for split/trim/move.

---

### `crates/draft_commands/tests/timeline_commands.rs` (test, CRUD)

**Analog:** `crates/draft_model/tests/draft_fixtures.rs`

**Fixture classification pattern** (`crates/draft_model/tests/draft_fixtures.rs` lines 17-35):
```rust
#[test]
fn draft_fixtures_are_explicitly_classified() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/draft");
    let positive = positive_project_fixtures();
    let negative = negative_project_fixtures();

    let actual = project_fixture_paths(&fixture_dir);
    let expected = positive
        .iter()
        .copied()
        .chain(negative.iter().map(|(path, _)| *path))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "every .veproj-style project.json fixture must be explicitly classified"
    );
}
```

**Negative exact-error pattern** (`crates/draft_model/tests/draft_fixtures.rs` lines 74-91):
```rust
#[test]
fn negative_draft_fixtures_fail_expected_gates() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/draft");
    let schema = draft_schema_validator();

    for (fixture_path, expected_error) in negative_project_fixtures() {
        let value = read_project_fixture(&fixture_dir, fixture_path);
        let error = migrate_draft_json(value.clone())
            .expect_err("negative fixture should fail draft migration or validation");

        assert_eq!(error, expected_error, "{fixture_path}");

        assert!(
            schema.validate(&value).is_err(),
            "negative fixture should fail generated draft JSON Schema: {fixture_path}"
        );
    }
}
```

Tests should compare exact draft state, command state, selection, and events before/after. Include rejection tests proving invalid edits do not mutate draft or history.

---

### `crates/draft_model/src/lib.rs` (model, request-response)

**Analog:** same file.

**Command envelope pattern** (`crates/draft_model/src/lib.rs` lines 36-69):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandEnvelope {
    pub command: CommandName,
    pub payload: CommandPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CommandName {
    Ping,
    Version,
    ProbeMediaRuntime,
    ImportMaterial,
    ListMaterials,
    ListMissingMaterials,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandPayload {
    Ping(PingCommandPayload),
    Version(VersionCommandPayload),
    ProbeMediaRuntime(ProbeMediaRuntimeCommandPayload),
    ImportMaterial(ImportMaterialCommandPayload),
    ListMaterials(ListMaterialsCommandPayload),
    ListMissingMaterials(ListMissingMaterialsCommandPayload),
}
```

**Payload/command pairing guard** (`crates/draft_model/src/lib.rs` lines 71-111):
```rust
impl CommandPayload {
    pub fn command_name(&self) -> CommandName {
        match self {
            Self::Ping(_) => CommandName::Ping,
            Self::Version(_) => CommandName::Version,
            Self::ProbeMediaRuntime(_) => CommandName::ProbeMediaRuntime,
            Self::ImportMaterial(_) => CommandName::ImportMaterial,
            Self::ListMaterials(_) => CommandName::ListMaterials,
            Self::ListMissingMaterials(_) => CommandName::ListMissingMaterials,
        }
    }
}
```

**Result envelope/event pattern** (`crates/draft_model/src/lib.rs` lines 213-252):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandResultEnvelope<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<CommandError>,
    pub events: Vec<CommandEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandEvent {
    pub kind: String,
    pub message: Option<String>,
}
```

Add Phase 3 timeline command names and tagged payload variants here. New command response types should include updated `Draft`, command state/history, selection where relevant, and events. Keep `serde(rename_all = "camelCase", deny_unknown_fields)` and generated TS/schema derives.

---

### `crates/draft_model/src/timeline.rs` (model, CRUD)

**Analog:** same file.

**Jianying-aligned timeline type pattern** (`crates/draft_model/src/timeline.rs` lines 9-17):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum TrackKind {
    Video,
    Audio,
    Text,
    Sticker,
    Filter,
}
```

**Constructor pattern** (`crates/draft_model/src/timeline.rs` lines 104-121 and 135-146):
```rust
impl Segment {
    pub fn new(
        segment_id: impl Into<SegmentId>,
        material_id: impl Into<MaterialId>,
        source_timerange: SourceTimerange,
        target_timerange: TargetTimerange,
    ) -> Self {
        Self {
            segment_id: segment_id.into(),
            material_id: material_id.into(),
            source_timerange,
            target_timerange,
            main_track_magnet: MainTrackMagnet::disabled(),
            keyframes: Vec::new(),
            filters: Vec::new(),
            transition: None,
        }
    }
}
```

Add semantic text/audio fields with the same persisted-model style. Use Jianying terms: `Draft`, `Material`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, `Keyframe`, `Filter`, `Transition`. Do not introduce internal `Asset`/`Clip` vocabulary.

---

### `crates/draft_model/src/validation.rs` (utility, transform)

**Analog:** same file.

**Validation traversal pattern** (`crates/draft_model/src/validation.rs` lines 89-185):
```rust
pub fn validate_draft(draft: &Draft) -> Result<(), DraftValidationError> {
    if !draft.schema_version.is_current() {
        return Err(DraftValidationError::InvalidSchemaVersion {
            found: draft.schema_version.0.to_string(),
            expected: DraftSchemaVersion::CURRENT_VALUE,
        });
    }
    // ...
    for track in &draft.tracks {
        if track.track_id.is_empty() {
            return Err(missing_field("tracks[].trackId"));
        }
        // ...
        for segment in &track.segments {
            if segment.segment_id.is_empty() {
                return Err(missing_field("tracks[].segments[].segmentId"));
            }
            if !material_ids.contains(segment.material_id.as_str()) {
                return Err(DraftValidationError::MissingRequiredSemanticField {
                    field: format!(
                        "tracks[].segments[].materialId references {}",
                        segment.material_id.as_str()
                    ),
                });
            }
            validate_source_timerange(
                "tracks[].segments[].sourceTimerange",
                &segment.source_timerange,
            )?;
            validate_target_timerange(
                "tracks[].segments[].targetTimerange",
                &segment.target_timerange,
            )?;
        }
    }

    Ok(())
}
```

Extend draft validation only for persisted invariants. Keep command-only edit rules in `draft_commands` unless they must be true for any saved `.veproj/project.json`.

---

### `crates/draft_model/tests/schema_exports.rs` (test, transform)

**Analog:** same file.

**Generated contract export pattern** (`crates/draft_model/tests/schema_exports.rs` lines 29-115):
```rust
#[test]
fn schema_exports_generated_contract_artifacts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/command.schema.json");
    let draft_schema_path = root.join("schemas/draft.schema.json");
    let generated_dir = root.join("apps/desktop-electron/src/generated");

    let schema_json = command_schema_json();
    assert_command_schema_rejects_zero_frame_rates(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));

    let command_envelope_ts = ts_contract_with_prelude(
        "import type { Draft, MaterialId, MaterialKind } from \"./Draft\";\n\n",
        &[
            export_decl::<CommandName>(),
            export_decl::<PingCommandPayload>(),
            export_decl::<VersionCommandPayload>(),
            export_decl::<ProbeMediaRuntimeCommandPayload>(),
            export_decl::<ImportMaterialCommandPayload>(),
            export_decl::<ListMaterialsCommandPayload>(),
            export_decl::<ListMissingMaterialsCommandPayload>(),
            export_decl::<CommandPayload>(),
            export_decl::<CommandEnvelope>(),
        ],
    );
    assert_or_update_contract_file(
        generated_dir.join("CommandEnvelope.ts"),
        &command_envelope_ts,
    );
}
```

**Update-or-drift-check pattern** (`crates/draft_model/tests/schema_exports.rs` lines 143-165):
```rust
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
```

Every new command payload/response/state/selection/text/audio type that crosses IPC must be added to this export list. Preserve `Config::new().with_large_int("number")` so `Microseconds` remains JSON-number compatible.

---

### `crates/bindings_node/src/lib.rs` (route, request-response)

**Analog:** same file.

**Import and route pattern** (`crates/bindings_node/src/lib.rs` lines 6-25):
```rust
use draft_model::{
    CommandEnvelope, CommandError, CommandErrorKind, CommandName, CommandPayload,
    CommandResultEnvelope, DRAFT_MODEL_VERSION, ImportMaterialCommandPayload,
    ImportMaterialResponse, ListMaterialsCommandPayload, ListMaterialsResponse,
    ListMissingMaterialsCommandPayload, ListMissingMaterialsResponse,
    MissingMaterialCommandDiagnostic, MissingMaterialCommandDiagnosticKind, PingResponse,
    VersionResponse,
};
```

**Unsupported command preflight and envelope deserialization** (`crates/bindings_node/src/lib.rs` lines 41-72):
```rust
#[napi]
pub fn execute_command(command: serde_json::Value) -> Result<serde_json::Value> {
    let command_name = raw_command_name(&command);

    if let Some(name) = command_name.as_deref() {
        if !matches!(
            name,
            "ping"
                | "version"
                | "probeMediaRuntime"
                | "importMaterial"
                | "listMaterials"
                | "listMissingMaterials"
        ) {
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
```

**Typed route pattern** (`crates/bindings_node/src/lib.rs` lines 74-95):
```rust
match envelope.command {
    CommandName::Ping => to_js_value(ping_envelope()),
    CommandName::Version => to_js_value(version_envelope()),
    CommandName::ImportMaterial => match envelope.payload {
        CommandPayload::ImportMaterial(payload) => import_material_command(payload),
        _ => unreachable!("command/payload pair was validated during deserialization"),
    },
    CommandName::ListMaterials => match envelope.payload {
        CommandPayload::ListMaterials(payload) => list_materials_command(payload),
        _ => unreachable!("command/payload pair was validated during deserialization"),
    },
    // ...
}
```

Timeline commands should route to `draft_commands` from here only after pure `draft_commands` tests pass. Do not let Electron or the binding layer implement snapping, overlap repair, undo/redo inverse logic, or direct `Draft.tracks` mutation.

---

### `fixtures/draft/**timeline-command*.json` (test, request-response)

**Analog:** `crates/draft_model/tests/schema_exports.rs`

**Command fixture classification pattern** (`crates/draft_model/tests/schema_exports.rs` lines 167-235):
```rust
#[test]
fn schema_fixtures_validate_command_contracts() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/draft");
    let positive = BTreeSet::from(["minimal-command.json"]);
    let negative = BTreeSet::from([
        "invalid-mismatched-command-payload.json",
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
    // ...
}
```

Add positive and negative timeline command fixtures only when they materially improve planner/test coverage. Keep explicit positive/negative lists updated so unclassified fixtures fail tests.

## Shared Patterns

### Pure Semantic Boundary
**Source:** `AGENTS.md`, `docs/runtime-boundaries.md`, `crates/draft_commands/src/lib.rs`
**Apply to:** `crates/draft_commands/**`, command model additions in `draft_model`

`draft_commands` may depend on `draft_model` only. It must not import Electron, Node-API, `project_store`, filesystem abstractions, FFmpeg/media runtime, preview service, render graph, or platform traits. Commands inspect only the `Draft` and material metadata already present in it.

### Strict Rust-Owned Contracts
**Source:** `crates/draft_model/src/lib.rs`
**Apply to:** command payloads, responses, command state, selection, text/audio semantic structs

Use `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]`, `#[serde(rename_all = "camelCase", deny_unknown_fields)]`, and tagged command payload enums. Keep `CommandPayload::command_name()` exhaustive and use custom `Deserialize` on `CommandEnvelope` to reject mismatched command/payload pairs.

### Atomic Edit Execution
**Source:** `crates/draft_model/src/material.rs`
**Apply to:** all timeline edit commands

Copy the rollback-on-validation discipline from `add_material` and `upsert_material`, but prefer whole-draft/session clone-apply-validate-return for Phase 3. Invalid edits must not partially mutate `Draft`, command history, redo stack, or selection.

### Validation
**Source:** `crates/draft_model/src/validation.rs`
**Apply to:** `draft_model` persisted checks and `draft_commands` command-level checks

Use structured error variants and exact test assertions. `validate_draft` covers persisted model integrity; command-level validation must additionally enforce non-overlap, locked tracks, material duration bounds, track/material compatibility, snap/magnet rules, and overflow-safe timerange math.

### Events
**Source:** `crates/draft_model/src/lib.rs`
**Apply to:** all command responses

Return stable `CommandEvent { kind, message }` entries for committed/rejected edits and observable behaviors such as `segmentAdded`, `segmentMoved`, `segmentSplit`, `segmentTrimmed`, `segmentDeleted`, `snapped`, `mainTrackMagnetApplied`, `undoCommitted`, and `redoCommitted`. Events are UI synchronization/test assertions, not render semantics.

### Generated Contract Drift
**Source:** `crates/draft_model/tests/schema_exports.rs`
**Apply to:** `schemas/command.schema.json`, `schemas/draft.schema.json`, `apps/desktop-electron/src/generated/*.ts`

Add every exported Rust type to `schema_exports.rs`, regenerate with `VE_UPDATE_GENERATED_CONTRACTS=1`, and keep drift checks failing when committed artifacts are stale.

### Fixture Discipline
**Source:** `crates/draft_model/tests/draft_fixtures.rs`, `crates/draft_model/tests/schema_exports.rs`
**Apply to:** command fixtures and draft fixtures

Every fixture must be explicitly classified positive or negative. Negative fixtures should assert the exact structured error where possible.

## No Analog Found

All planned Phase 3 file targets have at least a role-match or exact analog. The weakest matches are `history.rs` and `snapping.rs` because the codebase does not yet have timeline-specific undo/redo or snapping modules; planner should use the Phase 3 research recommendations plus the transaction/validation patterns above.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| _None_ | _N/A_ | _N/A_ | Existing Phase 1/2 contract, validation, fixture, and binding patterns cover all targets. |

## Metadata

**Analog search scope:** `crates/draft_model`, `crates/draft_commands`, `crates/bindings_node`, `fixtures/draft`, `schemas`, `apps/desktop-electron/src/generated`, `docs/runtime-boundaries.md`, prior phase artifacts.
**Files scanned:** 40 tracked files in relevant crate/schema/fixture/doc scopes, plus Phase 3 context/research.
**Pattern extraction date:** 2026-06-17
