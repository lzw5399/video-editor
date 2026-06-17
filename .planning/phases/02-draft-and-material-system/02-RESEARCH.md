# Phase 02: Draft And Material System - Research

**Researched:** 2026-06-17
**Domain:** Rust-owned draft schema, `.veproj` persistence, material probing, generated contracts
**Confidence:** HIGH for codebase boundaries and validation gates; MEDIUM for optional package choices because Rust crate slopcheck was unavailable

## User Constraints (from CONTEXT.md)

### Locked Decisions

## Implementation Decisions

### Draft Bundle And Canonical Semantics
- **D-01:** `.veproj/project.json` is the only persisted semantic source of truth for Phase 2. Derived artifacts such as thumbnails, waveforms, preview caches, render graphs, FFmpeg scripts, probe JSON, and exports must live outside the semantic draft model.
- **D-02:** A new draft starts as a valid, saveable bundle with explicit draft metadata, schema version, stable IDs, materials registry, tracks, and an empty sequence/timeline shell if needed for schema integrity. Do not wait for Phase 3 commands to make `.veproj` round trips testable.
- **D-03:** Project persistence belongs in `project_store`, but semantic schema, migration versioning, IDs, time ranges, material/track/segment structs, and validation live in `draft_model`. `project_store` may serialize/deserialize the model through `PlatformFileSystem`; it must not decide editing semantics.
- **D-04:** Use relative paths inside saved drafts when a material is inside or beneath the project bundle where feasible. Preserve absolute/external URIs for media outside the bundle, and centralize path resolution in `project_store` rather than UI code.
- **D-05:** Round-trip tests must compare semantic equality after save/open, not raw JSON byte equality. Formatting and stable ordering should still be deterministic enough for fixtures and schema drift review.

### Jianying-Aligned Schema Vocabulary
- **D-06:** Internal Rust domain types, JSON schema, generated TypeScript contracts, IPC payloads, docs, and tests should use Jianying-aligned English concepts directly: `Draft`, `Material`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, `Keyframe`, `Filter`, and `Transition`. Avoid reintroducing `Asset`/`Clip` as internal aliases.
- **D-07:** Persisted time values use integer microseconds for Phase 2. Later frame-index/rational-rate helpers can be layered on top, but persisted draft semantics must not use naked floating-point seconds.
- **D-08:** Keep Phase 2 schema broad enough for later video/audio/text tracks and segments, but only implement validation needed for draft/material integrity. Timeline command behavior, overlap rules, snapping, and undo/redo remain Phase 3.
- **D-09:** Schema versioning and migration hooks must exist in Phase 2 even if only version `1` is supported. Unknown future versions should fail with a structured, recoverable error rather than silently loading.

### Material Import And Probing
- **D-10:** Material import is a Rust-owned command/API path, not a UI-only mutation. The binding may expose Phase 2 commands, but all material IDs, metadata storage, and validation are owned by Rust.
- **D-11:** Material metadata should capture enough ffprobe-derived facts for the material bin and future timeline/rendering: material type, URI/path, display name, duration, width/height, fps/rational frame rate, stream presence, audio sample rate/channel count where available, and probe status/errors.
- **D-12:** Use the existing `media_runtime` discovery/process boundary for ffprobe. Do not call FFmpeg/ffprobe from Electron renderer or construct process strings in UI code.
- **D-13:** Thumbnails are allowed as derived artifacts for Phase 2 only if they remain cache outputs outside `project.json`. If thumbnail generation is too large for the phase, material import should still store metadata and leave thumbnail generation as a later derived cache path without blocking draft integrity.

### Missing Material And Recovery State
- **D-14:** Missing materials should not corrupt or delete draft semantics. Store the material entry, mark it as missing/unresolved through a recoverable status, and surface enough path/URI information for future relink UI.
- **D-15:** Open/save should preserve missing material entries exactly. A missing file is a warning/recoverable state, not a load failure unless `project.json` itself is invalid.
- **D-16:** Recovery/relink UI is out of Phase 2 scope, but Rust APIs should return classified missing-material information so Phase 4 UI can present it without reparsing paths itself.

### Testing And Gates
- **D-17:** Phase 2 must add `.veproj` fixtures under `fixtures/draft` or a dedicated project-fixture folder and classify them as positive/negative in tests, extending the Phase 1 fixture discipline.
- **D-18:** Required gates should include Rust model/schema tests, project-store save/open round-trip tests, ffprobe-backed material metadata tests using generated tiny media, missing-material tests, generated schema/TypeScript drift checks, and the existing `just build` / `just test` path.
- **D-19:** Electron can remain a smoke surface in Phase 2, but the key acceptance proof is Rust-owned draft/material behavior plus generated contracts. Rich material-bin UI waits until Phase 4.

### the agent's Discretion

- The planner may choose exact module/file names and whether Phase 2 commands are added as `executeCommand` variants or narrower exported binding calls, as long as Rust remains the source of truth and generated contracts stay synchronized.
- The planner may decide whether thumbnail generation is in Phase 2 or deferred, but material metadata import and missing-material recovery must be implemented and tested.
- The planner may choose fixture directory layout if it remains deterministic, documented, and covered by schema/model tests.

### Deferred Ideas (OUT OF SCOPE)

- Timeline edit commands, segment overlap rules, snapping/main-track magnet behavior, undo/redo, and invalid edit rejection belong to Phase 3.
- Full Jianying-style material bin UI, inspector, timeline interactions, and visual layout checks belong to Phase 4.
- Preview frames, waveform/preview cache generation, render graph snapshots, and export jobs belong to Phase 5.
- Material relink UI and advanced compatibility import/export adapters are later work; Phase 2 only needs recoverable missing-material state and API evidence.

## Summary

Phase 2 should extend the Phase 1 Rust-owned contract model instead of creating a parallel draft system. `draft_model` already owns serde, schemars, and ts-rs contract generation, and `project_store`, `media_runtime`, and `testkit` already expose the filesystem, ffprobe, and generated-media boundaries this phase needs. [VERIFIED: crates/draft_model/src/lib.rs; crates/project_store/src/lib.rs; crates/media_runtime/src/lib.rs; crates/testkit/src/lib.rs]

The correct planning split is: `draft_model` owns schema, IDs, timeranges, validation, and migration hooks; `project_store` owns `.veproj/project.json` path resolution and save/open/autosave IO; `media_runtime` owns ffprobe execution and parsing; `testkit` owns generated video/image/audio fixtures; `bindings_node` only exposes typed Rust commands/results after the Rust behavior is tested. [VERIFIED: docs/runtime-boundaries.md; .planning/phases/02-draft-and-material-system/02-CONTEXT.md]

**Primary recommendation:** Implement Phase 2 as a durability and metadata layer first: schema and migrations, deterministic project-store round trips, ffprobe-backed material import, missing-material diagnostics, generated schema/TypeScript drift checks, then optional thumbnail cache work only if it stays outside `project.json`. [VERIFIED: .planning/phases/02-draft-and-material-system/02-CONTEXT.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Draft schema and semantic validation | Rust `draft_model` | Generated JSON Schema/TS contracts | The phase context assigns schema, IDs, timeranges, validation, and migrations to `draft_model`; generated artifacts must follow Rust types. [VERIFIED: 02-CONTEXT.md; crates/draft_model/tests/schema_exports.rs] |
| `.veproj` create/open/save/autosave | Rust `project_store` | `draft_model` | `project_store` owns `PlatformFileSystem`; `draft_model` owns semantic meaning and validation. [VERIFIED: crates/project_store/src/lib.rs; docs/runtime-boundaries.md] |
| Material import command semantics | Rust command/API layer | `draft_model`, `media_runtime` | Material IDs and metadata are Rust-owned; ffprobe must remain behind `media_runtime`. [VERIFIED: 02-CONTEXT.md; crates/media_runtime/src/lib.rs] |
| ffprobe process execution | Rust `media_runtime` | `media_runtime_desktop`, `testkit` | The runtime boundary already exposes `FfmpegExecutor` and argument-array execution, with desktop implementation injected at the boundary. [VERIFIED: crates/media_runtime/src/lib.rs; docs/runtime-boundaries.md] |
| Material bin display | Electron renderer | Generated contracts | Phase 2 may expose smoke-level metadata, but rich material-bin UI is deferred to Phase 4. [VERIFIED: 02-CONTEXT.md; apps/desktop-electron/src/renderer/App.tsx] |
| Thumbnails | `preview_service` or derived cache path | `testkit` | Thumbnails are allowed only as derived artifacts outside `project.json`; the preview boundary is reserved for thumbnails and cache work. [VERIFIED: docs/runtime-boundaries.md; crates/preview_service/src/lib.rs] |
| Missing material recovery state | Rust `project_store` plus `draft_model` status model | Electron UI later | Missing files are recoverable warnings and must preserve draft entries; relink UI is later. [VERIFIED: 02-CONTEXT.md] |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DRAFT-01 | User can create a new `.veproj` draft bundle. | Plan `project_store::create_project_bundle` around `PlatformFileSystem::write_string` and a valid `Draft::new` semantic model. [VERIFIED: .planning/REQUIREMENTS.md; crates/project_store/src/lib.rs] |
| DRAFT-02 | User can open and save a draft without semantic changes in a round trip. | Use semantic equality tests after serde load/save, not byte equality. [VERIFIED: 02-CONTEXT.md D-05] |
| DRAFT-03 | Draft schema uses Jianying-aligned concepts. | Extend Rust model with `Draft`, `Material`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, `Keyframe`, `Filter`, `Transition`; do not use internal `Asset`/`Clip`. [VERIFIED: 02-CONTEXT.md D-06; .planning/REQUIREMENTS.md] |
| DRAFT-04 | Draft stores semantic state only in `project.json`. | Keep thumbnails, waveforms, probe JSON, render graphs, FFmpeg scripts, caches, and exports outside the semantic draft model. [VERIFIED: AGENTS.md; 02-CONTEXT.md D-01] |
| DRAFT-05 | Draft versioning and migration hooks exist. | Add schema version `1`, migration entrypoint, and structured unknown-version error in `draft_model`. [VERIFIED: 02-CONTEXT.md D-09] |
| MAT-01 | User can import video, image, and audio materials. | Add Rust-owned import API that probes media through `media_runtime`; generated media fixtures should cover one video, one image, and one audio file. [VERIFIED: 02-CONTEXT.md D-10/D-12; crates/testkit/src/lib.rs] |
| MAT-02 | Imported materials receive stable IDs and retain metadata. | Persist ID, URI/path, duration microseconds, dimensions, rational fps, stream flags, sample rate, channel count, and probe status/errors. [VERIFIED: 02-CONTEXT.md D-11] |
| MAT-03 | Material bin displays imported materials with basic metadata and thumbnails where applicable. | In Phase 2, expose metadata through generated contracts and keep thumbnail files derived; rich UI waits for Phase 4. [VERIFIED: 02-CONTEXT.md D-13/D-19] |
| MAT-04 | App detects missing material files and presents recovery/error state without corrupting the draft. | Preserve entries on open/save and return classified missing-material diagnostics from Rust. [VERIFIED: 02-CONTEXT.md D-14/D-15/D-16] |

## Project Constraints (from AGENTS.md)

- UI emits commands; Rust core owns project and timeline semantics; UI code must not construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is canonical; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived. [VERIFIED: AGENTS.md]
- Product language, desktop code, Rust domain types, IPC commands, docs, schema, and tests must prefer Jianying concepts such as draft/material/track/segment/keyframe/filter/transition. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates and avoid naked floating-point persisted time. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors but does not decide editing behavior. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are conceptual references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md]
- External drafts go through adapters and compatibility reports; proprietary IDs are external references, not internal render semantics. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution license review is required before distribution, but Phase 2 context defers licensing and says not to block on it. [VERIFIED: AGENTS.md; 02-CONTEXT.md]
- GSD workflow requires using GSD entry points before file-changing work unless explicitly bypassed. [VERIFIED: AGENTS.md]

## Existing Codebase Anchors

| Area | Current Anchor | Planning Implication |
|------|----------------|----------------------|
| Rust contract model | `crates/draft_model/src/lib.rs` defines command/result envelopes with serde, schemars, and ts-rs derives. [VERIFIED: crates/draft_model/src/lib.rs] | Add draft/material schema in this crate and extend schema/TS export tests rather than hand-writing TypeScript contracts. [VERIFIED: crates/draft_model/tests/schema_exports.rs] |
| Contract generator | `crates/draft_model/tests/schema_exports.rs` writes/compares `schemas/command.schema.json` and `apps/desktop-electron/src/generated/*.ts`. [VERIFIED: crates/draft_model/tests/schema_exports.rs] | Add `draft.schema.json` and generated Draft/Material TS declarations through the same test-driven generator pattern. [VERIFIED: crates/draft_model/tests/schema_exports.rs] |
| Project IO boundary | `project_store::PlatformFileSystem` has read/write/exists methods and `StdPlatformFileSystem` creates parent directories. [VERIFIED: crates/project_store/src/lib.rs] | Implement bundle APIs here; do not let `draft_model` touch filesystem paths except semantic URI/path values. [VERIFIED: docs/runtime-boundaries.md] |
| FFmpeg boundary | `media_runtime::FfmpegExecutor` runs binaries with explicit argument arrays, and discovery uses `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, then PATH. [VERIFIED: crates/media_runtime/src/lib.rs; crates/media_runtime/src/discovery.rs] | Material probe must add ffprobe calls here or via a runtime service using this trait; no renderer or shell string construction. [VERIFIED: docs/runtime-boundaries.md] |
| Tiny media test harness | `testkit` already generates a tiny lavfi MP4 and parses ffprobe JSON into integer microsecond metadata. [VERIFIED: crates/testkit/src/lib.rs] | Extend testkit with generated image/audio fixtures and reusable material-probe assertions. [VERIFIED: crates/testkit/src/lib.rs] |
| Electron smoke | Renderer currently only calls ping/version/executeCommand and shows a placeholder material row. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx] | Keep Electron Phase 2 work to smoke-level contract exercise unless a plan explicitly includes minimal material metadata display. [VERIFIED: 02-CONTEXT.md D-19] |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust workspace crates | Rust 1.95.0 | Source of truth for draft/material semantics and service boundaries. [VERIFIED: Cargo.toml; rustc --version] | Existing project architecture pins Rust and already compiles Phase 1 crates. [VERIFIED: Cargo.toml; .planning/phases/01-foundation-and-golden-harness/01-VERIFICATION.md] |
| `serde` | 1.0.228 | Serialize/deserialize persisted draft JSON and command payloads. [VERIFIED: cargo info serde; crates/draft_model/Cargo.toml] | Already used by `draft_model` and `media_runtime`; official crate metadata identifies it as serialization/deserialization framework. [VERIFIED: cargo info serde] |
| `serde_json` | 1.0.150 | Read/write `project.json`, test fixture JSON, and ffprobe JSON parsing. [VERIFIED: cargo info serde_json; crates/draft_model/Cargo.toml; crates/testkit/Cargo.toml] | Already used by contract tests and testkit; official crate metadata identifies it as JSON serialization format. [VERIFIED: cargo info serde_json] |
| `schemars` | 1.2.1 | Generate JSON Schema from Rust draft and command types. [VERIFIED: cargo info schemars; crates/draft_model/Cargo.toml] | Existing schema export test uses `schema_for!` and compares committed schema output. [VERIFIED: crates/draft_model/tests/schema_exports.rs] |
| `ts-rs` | 12.0.1 | Generate TypeScript declarations from Rust contract types. [VERIFIED: cargo info ts-rs; crates/draft_model/Cargo.toml] | Existing generator exports Rust declarations consumed by Electron. [VERIFIED: crates/draft_model/tests/schema_exports.rs; apps/desktop-electron/src/renderer/App.tsx] |
| `media_runtime` | local crate | Discover and execute ffprobe/FFmpeg-family binaries through a bounded runtime boundary. [VERIFIED: crates/media_runtime/src/lib.rs] | Phase 2 context explicitly requires use of this boundary for ffprobe. [VERIFIED: 02-CONTEXT.md D-12] |
| `project_store` | local crate | `.veproj` filesystem persistence through `PlatformFileSystem`. [VERIFIED: crates/project_store/src/lib.rs] | Phase 2 context assigns project persistence here. [VERIFIED: 02-CONTEXT.md D-03] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | 2.0.18 | Structured error enums for project-store and material-probe errors. [VERIFIED: cargo info thiserror; crates/media_runtime/Cargo.toml] | Use when Phase 2 adds recoverable unknown-version, invalid-project, missing-material, and probe-failed errors. [VERIFIED: crates/media_runtime/src/error.rs pattern] |
| `tempfile` | 3.27.0 | Temporary project bundles and generated media in tests. [VERIFIED: cargo info tempfile; crates/testkit/Cargo.toml] | Use in project-store round-trip tests and testkit media generation. [VERIFIED: crates/testkit/src/lib.rs] |
| `uuid` | 1.23.3 | Optional stable generated IDs for Draft/Material/Track/Segment values. [VERIFIED: cargo info uuid] [ASSUMED: package legitimacy for Rust slopcheck] | Use if planner chooses generated UUIDs instead of deterministic caller/test-supplied IDs; gate with human package verification because slopcheck could not validate Rust crates. [ASSUMED] |
| `jsonschema` | 0.46.5 | Dev-only validation of fixtures against generated schema. [VERIFIED: crates/draft_model/Cargo.toml; crates/draft_model/tests/schema_exports.rs] | Extend existing fixture validation tests for draft/project fixtures. [VERIFIED: crates/draft_model/tests/schema_exports.rs] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `uuid` generated IDs | Deterministic string newtypes with caller-supplied IDs | Avoids new dependency and makes fixtures stable, but app/runtime must supply uniqueness. [ASSUMED] |
| Derived JSON Schema from Rust | Hand-maintained schema files | Hand-maintained schema would contradict Phase 1 Rust-owned contract pattern and drift checks. [VERIFIED: crates/draft_model/tests/schema_exports.rs] |
| `project_store` path resolution | Electron renderer path logic | Renderer path logic would violate the Rust-owned semantics and project-store centralization decisions. [VERIFIED: AGENTS.md; 02-CONTEXT.md D-04] |
| Persisting ffprobe JSON | Persist normalized material metadata only | Raw probe JSON is a derived artifact and should not become canonical project semantics. [VERIFIED: 02-CONTEXT.md D-01/D-11] |

**Installation:**

```bash
# Required only if planner adds UUID generation instead of caller-supplied IDs.
cargo add uuid --features serde,v4

# Existing crates should be reused where already present in Cargo.toml.
```

**Version verification:** `cargo search` and `cargo info` were run for `serde`, `serde_json`, `schemars`, `ts-rs`, `thiserror`, `tempfile`, and `uuid`; current versions are listed above. [VERIFIED: cargo search; cargo info]

## Package Legitimacy Audit

> Slopcheck 0.6.1 is installed, but `slopcheck install ... --json` is unsupported in this environment and `slopcheck install` checks npm packages, not crates.io packages. [VERIFIED: slopcheck command output] Because Rust-crate slopcheck could not run, any new external crate must be treated as `[ASSUMED]` for package legitimacy even when crates.io existence is verified. [ASSUMED: applying project package protocol to unavailable Rust slopcheck]

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `serde` | crates.io | Existing mature crate; exact age not retrieved due crates.io API 403. [VERIFIED: cargo info serde; ASSUMED: age] | Not retrieved. [ASSUMED] | `github.com/serde-rs/serde` [VERIFIED: cargo info serde] | Unavailable for Rust crate [ASSUMED] | Existing approved dependency; do not add duplicate. |
| `serde_json` | crates.io | Existing mature crate; exact age not retrieved due crates.io API 403. [VERIFIED: cargo info serde_json; ASSUMED: age] | Not retrieved. [ASSUMED] | `github.com/serde-rs/json` [VERIFIED: cargo info serde_json] | Unavailable for Rust crate [ASSUMED] | Existing approved dependency; reuse. |
| `schemars` | crates.io | Existing crate; exact age not retrieved due crates.io API 403. [VERIFIED: cargo info schemars; ASSUMED: age] | Not retrieved. [ASSUMED] | `github.com/GREsau/schemars` [VERIFIED: cargo info schemars] | Unavailable for Rust crate [ASSUMED] | Existing approved dependency; reuse. |
| `ts-rs` | crates.io | Existing crate; exact age not retrieved due crates.io API 403. [VERIFIED: cargo info ts-rs; ASSUMED: age] | Not retrieved. [ASSUMED] | `github.com/Aleph-Alpha/ts-rs` [VERIFIED: cargo info ts-rs] | Unavailable for Rust crate [ASSUMED] | Existing approved dependency; reuse. |
| `thiserror` | crates.io | Existing crate; exact age not retrieved due crates.io API 403. [VERIFIED: cargo info thiserror; ASSUMED: age] | Not retrieved. [ASSUMED] | `github.com/dtolnay/thiserror` [VERIFIED: cargo info thiserror] | Unavailable for Rust crate [ASSUMED] | Reuse if Phase 2 adds errors. |
| `tempfile` | crates.io | Existing crate; exact age not retrieved due crates.io API 403. [VERIFIED: cargo info tempfile; ASSUMED: age] | Not retrieved. [ASSUMED] | `github.com/Stebalien/tempfile` [VERIFIED: cargo info tempfile] | Unavailable for Rust crate [ASSUMED] | Reuse for tests. |
| `uuid` | crates.io | Existing crate; exact age not retrieved due crates.io API 403. [VERIFIED: cargo info uuid; ASSUMED: age] | Not retrieved. [ASSUMED] | `github.com/uuid-rs/uuid` [VERIFIED: cargo info uuid] | Unavailable for Rust crate [ASSUMED] | Optional; planner should insert `checkpoint:human-verify` before adding. |

**Packages removed due to slopcheck [SLOP] verdict:** none for Rust crates; npm slopcheck output was not applicable to crates.io. [VERIFIED: slopcheck command output]
**Packages flagged as suspicious [SUS]:** `uuid` only as an optional new dependency because Rust slopcheck verification was unavailable. [ASSUMED]

## Architecture Patterns

### System Architecture Diagram

```text
User command / test fixture / Electron IPC
        |
        v
bindings_node executeCommand (optional Phase 2 surface)
        |
        v
Rust command handler / project API
        |
        +--> draft_model: validate Draft schema, IDs, timeranges, material statuses, schema version
        |
        +--> project_store: resolve .veproj paths, read/write project.json, preserve derived-artifact boundary
        |
        +--> media_runtime: discover ffprobe, run argument-array probe, parse bounded JSON output
        |
        v
Draft result / Material import result / Missing material diagnostics
        |
        v
Generated JSON Schema + TypeScript contracts + Rust tests + fixture gates
```

This flow matches the Phase 1 service-boundary architecture and keeps filesystem, ffprobe, and Electron concerns out of pure semantic crates. [VERIFIED: docs/runtime-boundaries.md]

### Recommended Project Structure

```text
crates/draft_model/src/
  lib.rs              # public exports and command/result contract extensions
  draft.rs            # Draft, DraftMetadata, schema version, migration entrypoint
  material.rs         # Material, MaterialKind, MaterialProbeMetadata, MaterialStatus
  timeline.rs         # Track, Segment, SourceTimerange, TargetTimerange shells
  ids.rs              # DraftId, MaterialId, TrackId, SegmentId newtypes/helpers
  validation.rs       # semantic validation, missing references, unknown-version handling

crates/project_store/src/
  lib.rs              # public create/open/save/autosave APIs
  bundle.rs           # .veproj path construction and project.json constant
  paths.rs            # relative/external URI resolution
  error.rs            # recoverable project store errors

crates/media_runtime/src/
  probe.rs            # ffprobe metadata command and normalized probe output

crates/testkit/src/
  media.rs            # generated video/image/audio fixture helpers
  project.rs          # temp .veproj fixture helpers

fixtures/draft/
  positive/*.veproj/project.json or *.json
  negative/*.json
```

The exact module names are discretionary, but this responsibility split follows locked Phase 2 decisions and existing crate boundaries. [VERIFIED: 02-CONTEXT.md D-03; docs/runtime-boundaries.md]

### Pattern 1: Rust-Derived Contracts

**What:** Derive serde, schemars, and ts-rs on Rust model types; generate schema and TS from tests; fail when committed artifacts drift. [VERIFIED: crates/draft_model/tests/schema_exports.rs]

**When to use:** Every persisted draft type, IPC command payload/result, and material metadata payload that Electron consumes. [VERIFIED: 02-CONTEXT.md D-06/D-18]

**Example:**

```rust
// Source: crates/draft_model/src/lib.rs and crates/draft_model/tests/schema_exports.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub schema_version: u32,
    pub draft_id: DraftId,
    pub materials: Vec<Material>,
    pub tracks: Vec<Track>,
}
```

### Pattern 2: Semantic Equality Round Trips

**What:** Save a `Draft`, open it, and compare the loaded Rust value to the original semantic model rather than comparing raw JSON bytes. [VERIFIED: 02-CONTEXT.md D-05]

**When to use:** Project-store tests, autosave tests, missing-material preservation tests, and fixture normalization checks. [VERIFIED: 02-CONTEXT.md D-18]

**Example:**

```rust
// Source: Phase 2 context D-05; project_store filesystem boundary in crates/project_store/src/lib.rs
let original = Draft::new("Untitled");
save_project(&fs, project_path, &original)?;
let reopened = open_project(&fs, project_path)?;
assert_eq!(reopened.draft, original);
```

### Pattern 3: Runtime Probe Normalization

**What:** Use ffprobe JSON as input, but persist normalized material metadata, not raw probe JSON. [VERIFIED: 02-CONTEXT.md D-01/D-11; crates/testkit/src/lib.rs]

**When to use:** Material import tests and `media_runtime` probe APIs. [VERIFIED: 02-CONTEXT.md D-12]

**Example:**

```rust
// Source: crates/testkit/src/lib.rs ffprobe pattern
let args = vec![
    OsString::from("-v"),
    OsString::from("error"),
    OsString::from("-output_format"),
    OsString::from("json"),
    OsString::from("-show_entries"),
    OsString::from("stream=codec_type,width,height,r_frame_rate,duration,sample_rate,channels:format=duration"),
    path.as_os_str().to_owned(),
];
```

### Anti-Patterns to Avoid

- **UI-owned material mutation:** Renderer-side arrays of imported materials would violate Rust-owned semantics and later command consistency. [VERIFIED: AGENTS.md; 02-CONTEXT.md D-10]
- **Raw FFmpeg strings:** Shell-concatenated ffprobe commands risk quoting bugs and violate the existing argument-array runtime pattern. [VERIFIED: crates/media_runtime/src/process.rs; .planning/phases/01-foundation-and-golden-harness/01-VERIFICATION.md]
- **Persisted floats for time:** Floating-point seconds in `project.json` would violate the locked integer microsecond time model. [VERIFIED: AGENTS.md; 02-CONTEXT.md D-07]
- **Derived artifacts in `project.json`:** Persisting thumbnails, waveform samples, render graph snapshots, or raw probe JSON as canonical semantics would violate DRAFT-04. [VERIFIED: .planning/REQUIREMENTS.md; 02-CONTEXT.md D-01]
- **Timeline behavior in Phase 2 validation:** Overlap rules, snapping, undo/redo, and invalid edit rejection are Phase 3 work. [VERIFIED: 02-CONTEXT.md Deferred Ideas]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON parsing/formatting | Custom parser or string templates | `serde` / `serde_json` | Existing Rust contracts already use serde, and custom JSON would break strict schema/fixture validation. [VERIFIED: crates/draft_model/src/lib.rs] |
| JSON Schema generation | Manually maintained schema | `schemars` from Rust types | Existing test generator compares Rust-derived schema to committed artifacts. [VERIFIED: crates/draft_model/tests/schema_exports.rs] |
| TypeScript contracts | Hand-written TS interfaces | `ts-rs` from Rust types | Electron already imports generated Rust-owned types. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx] |
| Process launching | Shell command strings | `media_runtime::FfmpegExecutor` with `OsString` args | Existing runtime process runner uses `Command::new(binary).args(args)` and bounded waits. [VERIFIED: crates/media_runtime/src/process.rs] |
| Media metadata detection | Extension-only detection | ffprobe normalization through `media_runtime` | Phase 2 requires ffprobe-derived duration, stream, fps, size, and audio metadata. [VERIFIED: 02-CONTEXT.md D-11/D-12] |
| Temporary media fixtures | Checked-in binary fixtures | `testkit` generated tiny media | Phase 1 established generated media tests and no committed binary media fixtures. [VERIFIED: .planning/phases/01-foundation-and-golden-harness/01-VERIFICATION.md] |

**Key insight:** Phase 2 is a durability contract; hand-rolled schema, process, or path behavior will create drift between Rust semantics, saved drafts, generated contracts, and later UI/render phases. [VERIFIED: 02-CONTEXT.md; docs/runtime-boundaries.md]

## Common Pitfalls

### Pitfall 1: Duplicating Canonical State
**What goes wrong:** Material thumbnails, raw ffprobe JSON, render graph data, or preview caches become treated as draft truth. [VERIFIED: 02-CONTEXT.md D-01]
**Why it happens:** It is tempting to persist whatever Phase 2 computes during import. [ASSUMED]
**How to avoid:** Persist only normalized semantic material metadata and derived-artifact references/status where needed; keep cache files outside `project.json`. [VERIFIED: 02-CONTEXT.md D-01/D-13]
**Warning signs:** `project.json` contains thumbnail bytes, waveform arrays, FFmpeg script text, or raw probe documents. [VERIFIED: .planning/REQUIREMENTS.md DRAFT-04]

### Pitfall 2: Terminology Drift
**What goes wrong:** `Asset`, `Clip`, or generic media names leak into schema/API where Jianying-aligned `Material` and `Segment` should be used. [VERIFIED: 02-CONTEXT.md D-06]
**Why it happens:** English editing ecosystem docs often use clip/asset terms. [ASSUMED]
**How to avoid:** Add grep-style validation for forbidden internal terms in domain schema/API files, with exceptions only for external reference docs or explanatory comments. [ASSUMED]
**Warning signs:** Generated TS or JSON Schema contains `Asset`, `Clip`, or `sourceRange` instead of `Material`, `Segment`, `SourceTimerange`, or `TargetTimerange`. [VERIFIED: 02-CONTEXT.md D-06]

### Pitfall 3: Missing Media Treated As Load Failure
**What goes wrong:** Opening a draft with missing external media fails or deletes the material entry. [VERIFIED: 02-CONTEXT.md D-14/D-15]
**Why it happens:** Project-store path checks get mixed with draft validity checks. [ASSUMED]
**How to avoid:** Separate invalid `project.json` errors from recoverable material resolution diagnostics. [VERIFIED: 02-CONTEXT.md D-15/D-16]
**Warning signs:** `open_project` returns an error solely because a referenced material path does not exist. [VERIFIED: 02-CONTEXT.md D-15]

### Pitfall 4: Probe Metadata Uses Floats Internally
**What goes wrong:** ffprobe decimal seconds become persisted floats, causing round-trip and frame-boundary drift. [VERIFIED: AGENTS.md; 02-CONTEXT.md D-07]
**Why it happens:** ffprobe reports some durations as decimal strings. [VERIFIED: crates/testkit/src/lib.rs]
**How to avoid:** Parse decimal strings into integer microseconds and fps strings into rational numerator/denominator values, following the existing testkit helper pattern. [VERIFIED: crates/testkit/src/lib.rs]
**Warning signs:** `durationSeconds: number` appears in generated contracts for persisted semantics. [VERIFIED: AGENTS.md]

### Pitfall 5: Contract Drift Hidden By Manual TypeScript
**What goes wrong:** Electron compiles against a different shape than Rust saves/loads. [VERIFIED: .planning/phases/01-foundation-and-golden-harness/01-VERIFICATION.md]
**Why it happens:** Manual TS interfaces are faster to add during UI smoke work. [ASSUMED]
**How to avoid:** Extend `schema_exports.rs` and keep `git diff --exit-code schemas apps/desktop-electron/src/generated` in gates. [VERIFIED: crates/draft_model/tests/schema_exports.rs; justfile]
**Warning signs:** New TS files under `apps/desktop-electron/src/generated` are edited by hand or not regenerated from Rust. [VERIFIED: crates/draft_model/tests/schema_exports.rs]

## Code Examples

Verified patterns from project sources:

### Strict Rust Contract Types

```rust
// Source: crates/draft_model/src/lib.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandResultEnvelope<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<CommandError>,
    pub events: Vec<CommandEvent>,
}
```

### Filesystem Boundary

```rust
// Source: crates/project_store/src/lib.rs
pub trait PlatformFileSystem {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn write_string(&self, path: &Path, contents: &str) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
}
```

### Bounded Process Execution

```rust
// Source: crates/media_runtime/src/process.rs
let mut child = Command::new(binary)
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
```

### Integer Microsecond Parsing Pattern

```rust
// Source: crates/testkit/src/lib.rs
let whole_micros = whole.parse::<u64>()?.saturating_mul(1_000_000);
let fraction_micros = padded_fraction.parse::<u64>()?;
let duration_microseconds = whole_micros.saturating_add(fraction_micros);
```

## Material Probing Strategy

Use `media_runtime` to expose a normalized `probe_material_metadata(path)` API that accepts a filesystem path or resolved URI and returns a Rust struct independent from ffprobe's raw JSON shape. [VERIFIED: 02-CONTEXT.md D-11/D-12; crates/media_runtime/src/lib.rs]

The probe should request JSON output with stream and format entries for codec type, width, height, rational frame rate, duration, sample rate, and channels; the existing testkit command already uses `-output_format json` and `-show_entries stream=...:format=duration`. [VERIFIED: crates/testkit/src/lib.rs]

Persisted `Material` should include a status such as `available`, `missing`, or `probeFailed`, but missing/probe failure should not remove the material entry. [VERIFIED: 02-CONTEXT.md D-14/D-15]

Generated test media should include:
- Video+audio MP4 from current `generate_tiny_lavfi_media`. [VERIFIED: crates/testkit/src/lib.rs]
- Still image generated by FFmpeg lavfi or another deterministic local process, but committed binary image fixtures should be avoided unless the team explicitly changes fixture policy. [VERIFIED: .planning/phases/01-foundation-and-golden-harness/01-VERIFICATION.md; ASSUMED: image generation approach]
- Audio-only generated media, probably lavfi sine into WAV/M4A for ffprobe metadata coverage. [VERIFIED: crates/testkit/src/lib.rs for sine source pattern; ASSUMED: exact container]

## Missing Material Recovery Semantics

Recommended persisted model:

```text
Material {
  materialId,
  kind,
  uri,
  displayName,
  metadata,
  status: Available | Missing | ProbeFailed,
  lastKnownResolvedPath?,
  probeError?
}
```

`uri` should preserve the user-facing original reference, while `project_store` resolves whether it is relative to the `.veproj` bundle or an external absolute path. [VERIFIED: 02-CONTEXT.md D-04/D-14]

Open/save should preserve the material entry and status even when the file is missing; only invalid JSON, unsupported schema versions, or semantic schema violations should block draft load. [VERIFIED: 02-CONTEXT.md D-09/D-15]

Relink UI, interactive recovery, and advanced compatibility reports are out of scope; Phase 2 should return enough classified diagnostics for Phase 4 UI to render later. [VERIFIED: 02-CONTEXT.md D-16; Deferred Ideas]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Duplicated editor project state and render state in one file | Single canonical semantic draft plus derived render/cache artifacts | Locked by project direction for this repo on 2026-06-17. [VERIFIED: AGENTS.md; 02-CONTEXT.md] | Reduces drift between editing semantics and FFmpeg/render execution. [VERIFIED: AGENTS.md] |
| UI-owned media import state | Rust-owned material import commands/results | Locked for Phase 2. [VERIFIED: 02-CONTEXT.md D-10] | Later timeline, preview, and export operate from the same material registry. [VERIFIED: .planning/PROJECT.md via AGENTS.md] |
| Floating-point persisted time | Integer microseconds now; frame/rational helpers later | Locked for Phase 2. [VERIFIED: 02-CONTEXT.md D-07] | Prevents draft round-trip precision drift. [VERIFIED: AGENTS.md] |
| Kdenlive/MLT runtime project format | Self-owned `.veproj/project.json` | Project initialization. [VERIFIED: AGENTS.md; .planning/REQUIREMENTS.md Out of Scope] | References inform boundaries but must not become runtime dependencies. [VERIFIED: AGENTS.md] |

**Deprecated/outdated:**
- Raw UI `Asset`/`Clip` domain language is deprecated for this project; use `Material` and `Segment` internally. [VERIFIED: 02-CONTEXT.md D-06]
- Direct renderer FFmpeg calls are forbidden; use `media_runtime`. [VERIFIED: AGENTS.md; docs/runtime-boundaries.md]
- Raw JSON byte equality is not the acceptance metric for save/open; use semantic equality. [VERIFIED: 02-CONTEXT.md D-05]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `uuid` is worth adding only if generated IDs are preferred over caller/test-supplied deterministic IDs. | Standard Stack | Planner may add a dependency unnecessarily; use human checkpoint first. |
| A2 | Image and audio generated fixtures can be produced by extending the existing lavfi pattern. | Material Probing Strategy | Testkit task may need adjustment if ffmpeg encoders/formats differ locally. |
| A3 | Exact crate age/download counts are not required for existing dependencies because crates.io API returned 403 and `cargo info` confirmed registry metadata. | Package Legitimacy Audit | Audit table has weaker legitimacy evidence than desired. |
| A4 | A grep-style terminology guard is useful for preventing `Asset`/`Clip` drift. | Common Pitfalls | The guard may need exceptions for external docs, generated comments, or third-party references. |

## Open Questions (RESOLVED)

1. **Should Phase 2 add `uuid` or require caller-supplied deterministic IDs?**
   - What we know: stable IDs are required; `uuid` 1.23.3 exists on crates.io and supports serde/v4 features. [VERIFIED: 02-CONTEXT.md D-02; cargo info uuid]
   - What's unclear: package legitimacy could not be validated with Rust-aware slopcheck in this environment. [VERIFIED: slopcheck output]
   - RESOLVED: Phase 2 will use deterministic string ID newtypes with caller/test-supplied IDs and will not add `uuid`. Runtime-generated IDs can be revisited later through a dependency review checkpoint if product needs require it. [VERIFIED: 90cf451 revised plans]

2. **Should thumbnails be implemented in Phase 2?**
   - What we know: thumbnails are allowed only as derived artifacts outside `project.json`; metadata import must not depend on thumbnails. [VERIFIED: 02-CONTEXT.md D-13]
   - What's unclear: whether Phase 2 time budget should include cache file generation and invalidation. [ASSUMED]
   - RESOLVED: Phase 2 defers thumbnail generation. It will document thumbnail/probe/waveform outputs as derived artifacts outside `project.json` and implement material metadata/status only. Preview/cache generation remains later `preview_service` work. [VERIFIED: 90cf451 revised plans]

3. **What exact `.veproj` fixture directory shape should be used?**
   - What we know: fixtures must be deterministic and classified positive/negative. [VERIFIED: 02-CONTEXT.md D-17]
   - What's unclear: whether to store nested `.veproj/project.json` directories or flat `*.project.json` fixtures for easier schema validation. [ASSUMED]
   - RESOLVED: Phase 2 will use nested `.veproj/project.json` fixtures for project-store round trips and flat positive/negative JSON fixtures for schema/model validation where useful. All fixtures must be explicitly classified in tests. [VERIFIED: 90cf451 revised plans]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | Electron build/tests | yes | v24.12.0 [VERIFIED: node --version] | none needed |
| pnpm | JS install/build/test | yes | 10.32.1 [VERIFIED: pnpm --version] | none needed |
| Rust cargo | Rust build/tests | yes | cargo 1.95.0 [VERIFIED: cargo --version] | none needed |
| rustc | Rust workspace | yes | rustc 1.95.0 [VERIFIED: rustc --version] | none needed |
| just | Public gates | yes, but not on shell PATH | 1.53.0 at `$HOME/.cargo/bin/just` [VERIFIED: command output] | Use `PATH="$HOME/.cargo/bin:$PATH" just test` or `pnpm run test`. |
| ffmpeg | Generated media tests | yes | 8.1 [VERIFIED: ffmpeg -version] | Missing ffmpeg should fail render/material smoke with remediation. [VERIFIED: crates/media_runtime/src/error.rs] |
| ffprobe | Material metadata tests | yes | 8.1 [VERIFIED: ffprobe -version] | Missing ffprobe should fail material probe tests with remediation. [VERIFIED: crates/media_runtime/src/error.rs] |
| ctx7 | Library docs lookup | no | unavailable [VERIFIED: `ctx7 not found`] | Use official docs.rs/crates.io metadata and project sources. |
| slopcheck | Package legitimacy | partially | 0.6.1 installed; `--json` unsupported and npm-oriented [VERIFIED: slopcheck output] | Treat new Rust packages as `[ASSUMED]` and add human checkpoint. |

**Missing dependencies with no fallback:**
- None for implementation if commands use `$HOME/.cargo/bin/just` or `pnpm run test`. [VERIFIED: environment commands]

**Missing dependencies with fallback:**
- `ctx7`; fallback is official docs.rs/crates.io and local source inspection. [VERIFIED: `ctx7 not found`; cargo info output]
- Rust-aware slopcheck; fallback is crates.io metadata plus human checkpoint for new external crates. [ASSUMED]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test`, `jsonschema` fixture validation, Playwright Electron smoke. [VERIFIED: justfile; crates/draft_model/Cargo.toml; apps/desktop-electron/package.json] |
| Config file | `justfile`, root `package.json`, `apps/desktop-electron/playwright.config.ts`. [VERIFIED: file inspection] |
| Quick run command | `cargo test -p draft_model && cargo test -p project_store && cargo test -p media_runtime probe && cargo test -p testkit material` [VERIFIED: existing cargo workspace; ASSUMED: future test names] |
| Full suite command | `PATH="$HOME/.cargo/bin:$PATH" just test` [VERIFIED: justfile; just location] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| DRAFT-01 | Create valid `.veproj/project.json` bundle | integration | `cargo test -p project_store create_project_bundle -- --nocapture` | no, Wave 0 [ASSUMED: test name] |
| DRAFT-02 | Save/open semantic round trip | integration | `cargo test -p project_store round_trip -- --nocapture` | no, Wave 0 [ASSUMED: test name] |
| DRAFT-03 | Jianying schema terms and strict JSON | unit/schema | `cargo test -p draft_model draft_schema -- --nocapture` | no, Wave 0 [ASSUMED: test name] |
| DRAFT-04 | Derived artifacts excluded from `project.json` | fixture/schema | `cargo test -p draft_model schema_fixtures -- --nocapture` | partial existing command fixtures only [VERIFIED: crates/draft_model/tests/schema_exports.rs] |
| DRAFT-05 | Version/migration hooks and unknown future version error | unit | `cargo test -p draft_model migration -- --nocapture` | no, Wave 0 [ASSUMED: test name] |
| MAT-01 | Import video/image/audio materials | integration | `cargo test -p project_store import_material -- --nocapture` | no, Wave 0 [ASSUMED: test name] |
| MAT-02 | Stable IDs and ffprobe metadata retained | integration | `cargo test -p media_runtime material_probe -- --nocapture` | no, Wave 0 [ASSUMED: test name] |
| MAT-03 | Metadata visible through generated contract or Electron smoke | contract/smoke | `pnpm --filter @video-editor/desktop test` | existing smoke only, needs extension [VERIFIED: apps/desktop-electron/src/renderer/App.tsx] |
| MAT-04 | Missing material opens as recoverable diagnostic | integration | `cargo test -p project_store missing_material -- --nocapture` | no, Wave 0 [ASSUMED: test name] |

### Sampling Rate

- **Per task commit:** `cargo test -p draft_model && cargo test -p project_store` once those crates have Phase 2 tests. [ASSUMED: future tests]
- **Per wave merge:** `pnpm run test:rust && cargo test -p draft_model schema -- --nocapture && git diff --exit-code schemas apps/desktop-electron/src/generated`. [VERIFIED: package.json; justfile]
- **Phase gate:** `PATH="$HOME/.cargo/bin:$PATH" just build && PATH="$HOME/.cargo/bin:$PATH" just test`. [VERIFIED: justfile; just location]

### Wave 0 Gaps

- [ ] `crates/draft_model/tests/draft_schema.rs` or extension to `schema_exports.rs` covering `Draft`, `Material`, `Track`, `Segment`, migrations, and forbidden terminology. [ASSUMED: file name]
- [ ] `crates/project_store/tests/project_bundle.rs` covering create/save/open/autosave and relative/external path resolution. [ASSUMED: file name]
- [ ] `crates/media_runtime/tests/material_probe.rs` covering video/image/audio ffprobe metadata and probe failure. [ASSUMED: file name]
- [ ] `crates/testkit` media helpers for image and audio-only generated files. [ASSUMED: file name]
- [ ] Generated artifacts: `schemas/draft.schema.json` and `apps/desktop-electron/src/generated/Draft.ts` or equivalent. [ASSUMED: artifact names]
- [ ] `just` PATH handling in local instructions or planner commands because bare `just` is not on this shell PATH. [VERIFIED: command output]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Desktop-local Phase 2 has no auth surface. [VERIFIED: phase scope in 02-CONTEXT.md] |
| V3 Session Management | no | No sessions are introduced in Phase 2. [VERIFIED: phase scope in 02-CONTEXT.md] |
| V4 Access Control | partial | Keep Electron renderer behind preload API and Rust command boundary. [VERIFIED: .planning/phases/01-foundation-and-golden-harness/01-VERIFICATION.md] |
| V5 Input Validation | yes | Strict serde `deny_unknown_fields`, JSON Schema fixture validation, path classification, and structured error types. [VERIFIED: crates/draft_model/src/lib.rs; crates/draft_model/tests/schema_exports.rs] |
| V6 Cryptography | no | No cryptography is required for local draft/material persistence in Phase 2. [VERIFIED: phase scope in 02-CONTEXT.md] |

### Known Threat Patterns for Rust/Electron Draft Import

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed `project.json` with unknown fields or mismatched command/payload data | Tampering | Use serde `deny_unknown_fields`, schema validation, and structured invalid-project errors. [VERIFIED: crates/draft_model/src/lib.rs; crates/draft_model/tests/schema_exports.rs] |
| Path traversal or ambiguous relative paths in `.veproj` material URIs | Tampering/Information Disclosure | Centralize path resolution in `project_store`; distinguish bundle-relative paths from external absolute URIs. [VERIFIED: 02-CONTEXT.md D-04] |
| Renderer invoking arbitrary native/FFmpeg operations | Elevation of Privilege | Keep renderer limited to generated typed preload API and route media operations through Rust boundaries. [VERIFIED: .planning/phases/01-foundation-and-golden-harness/01-VERIFICATION.md; AGENTS.md] |
| Hanging or noisy ffprobe processes | Denial of Service | Reuse bounded process execution and bounded stdout/stderr summaries. [VERIFIED: crates/media_runtime/src/process.rs; crates/media_runtime/src/discovery.rs] |
| Missing media corrupting draft state | Tampering | Treat missing materials as recoverable diagnostics and preserve entries during save/open. [VERIFIED: 02-CONTEXT.md D-14/D-15] |

## Sources

### Primary (HIGH confidence)
- `AGENTS.md` - project architecture, terminology, time model, derived artifact, testing, and licensing constraints. [VERIFIED: file read]
- `.planning/phases/02-draft-and-material-system/02-CONTEXT.md` - locked Phase 2 decisions, scope, non-goals, and code context. [VERIFIED: file read]
- `.planning/REQUIREMENTS.md` - DRAFT and MAT requirement definitions. [VERIFIED: file read]
- `.planning/ROADMAP.md` - Phase 2 plan skeleton, dependencies, and success criteria. [VERIFIED: file read]
- `.planning/STATE.md` - Phase 1 decisions and current position. [VERIFIED: file read]
- `docs/runtime-boundaries.md` - crate boundary rules and FFmpeg scope. [VERIFIED: file read]
- `crates/draft_model/src/lib.rs`, `crates/draft_model/tests/schema_exports.rs`, `crates/project_store/src/lib.rs`, `crates/media_runtime/src/*.rs`, `crates/testkit/src/lib.rs` - existing implementation anchors. [VERIFIED: file read]
- `cargo info` for `serde`, `serde_json`, `schemars`, `ts-rs`, `thiserror`, `tempfile`, and `uuid` - current crates.io version and repository metadata. [VERIFIED: cargo info]

### Secondary (MEDIUM confidence)
- `reference/kdenlive/dev-docs/fileformat.md` - conceptual warning about duplicated project/render state and project-bin separation; not copied into implementation. [VERIFIED: local reference read]
- `reference/pyJianYingDraft/pyJianYingDraft/*.py` grep results - conceptual vocabulary evidence for material/track/segment/timerange/keyframe terms; not copied into implementation. [VERIFIED: local reference grep]
- FFmpeg/ffprobe behavior inferred from existing testkit command and local FFmpeg 8.1 availability. [VERIFIED: crates/testkit/src/lib.rs; ffprobe --version]

### Tertiary (LOW confidence)
- Exact package age/download counts were not retrieved because crates.io API calls returned HTTP 403. [VERIFIED: curl output]
- Optional `uuid` addition remains a planning decision requiring human verification due Rust slopcheck unavailability. [ASSUMED]

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM - existing dependencies and versions are verified, but new-package legitimacy cannot be fully slopchecked for Rust crates in this environment. [VERIFIED: cargo info; slopcheck output]
- Architecture: HIGH - boundaries are locked in Phase 2 context and match Phase 1 code. [VERIFIED: 02-CONTEXT.md; docs/runtime-boundaries.md]
- Pitfalls: HIGH for locked constraints; MEDIUM for suggested terminology grep guard. [VERIFIED: 02-CONTEXT.md; ASSUMED: grep guard]

**Research date:** 2026-06-17
**Valid until:** 2026-07-17 for codebase architecture; recheck crate versions and tool availability before installing any new dependency. [ASSUMED]
