# Phase 2: Draft And Material System - Context

**Gathered:** 2026-06-17
**Status:** Ready for planning
**Mode:** auto-discuss

<domain>
## Phase Boundary

Phase 2 establishes the first real editor data layer: a `.veproj` draft bundle with canonical `project.json`, Jianying-aligned draft/material/track/segment schema concepts, material import/probing with FFmpeg metadata, project open/save/autosave round trips, and recoverable missing-material detection. It does not implement timeline edit commands, undo/redo, rich desktop UI panels, preview playback, render graph compilation, or MP4 export; those start in later phases.

</domain>

<decisions>
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Direction
- `.planning/PROJECT.md` - Product identity, Jianying terminology requirement, architecture constraints, and out-of-scope boundaries.
- `.planning/REQUIREMENTS.md` - Phase 2 requirements `DRAFT-01` through `DRAFT-05` and `MAT-01` through `MAT-04`.
- `.planning/ROADMAP.md` - Phase 2 goal, success criteria, and planned work slices.
- `.planning/STATE.md` - Current phase status and accumulated decisions from Phase 1.

### Prior Phase Artifacts
- `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` - Locked Phase 1 decisions on Rust-owned contracts, pure semantic crates, service boundaries, FFmpeg discovery, and fixture gates.
- `.planning/phases/01-foundation-and-golden-harness/01-VERIFICATION.md` - Evidence that Phase 1 gates and infrastructure are present.
- `.planning/phases/01-foundation-and-golden-harness/01-SECURITY.md` - Threat register and mitigations for the current binding/runtime/process boundaries.

### Research
- `.planning/research/SUMMARY.md` - MVP shape, terminology, semantic pipeline, and test strategy.
- `.planning/research/ARCHITECTURE.md` - `.veproj` bundle shape, layer responsibilities, Kdenlive/MLT/Jianying lessons.
- `.planning/research/STACK.md` - Rust/Electron stack, crate responsibilities, and project format recommendation.
- `.planning/research/PITFALLS.md` - Known traps around duplicate state, terminology drift, time bugs, FFmpeg leakage, and preview/export drift.

### Local Source Boundaries
- `docs/runtime-boundaries.md` - Trait placement, pure semantic crates, project store runtime, and FFmpeg scope.
- `crates/draft_model/src/lib.rs` - Current Rust-owned command/schema generation pattern to extend.
- `crates/project_store/src/lib.rs` - Existing `PlatformFileSystem` boundary for `.veproj` persistence.
- `crates/media_runtime/src/discovery.rs` - Existing FFmpeg/ffprobe discovery and probe pattern.
- `crates/testkit/src/lib.rs` - Existing tiny media generation and ffprobe metadata helpers.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/draft_model` already owns serde/schemars/ts-rs generation and command-envelope validation. Phase 2 should extend this crate with persisted draft/material schema, not create a parallel model crate.
- `crates/project_store::PlatformFileSystem` and `StdPlatformFileSystem` already provide the filesystem abstraction needed for `.veproj/project.json` read/write.
- `media_runtime::discover_runtime_config` and `run_process_with_timeout` already provide bounded ffprobe discovery/execution boundaries.
- `crates/testkit` already generates tiny FFmpeg media and parses ffprobe metadata. Phase 2 can reuse or extend this for material import tests.
- Electron already consumes generated TypeScript contracts through `apps/desktop-electron/src/generated`, and the smoke workbench has material/draft/timeline labels to evolve later.

### Established Patterns
- Rust types are the source of truth; generated JSON Schema and TypeScript are compared in tests rather than hand-maintained.
- `just build` and `just test` are the public gates. New Phase 2 tests should flow through these existing commands.
- Pure semantic crates must not import runtime/platform traits. `draft_model`, `draft_commands`, and `engine_core` stay free of filesystem, FFmpeg process, Electron, and preview runtime dependencies.
- Service-boundary crates own platform abstractions: `project_store` for filesystem persistence, `media_runtime` for ffprobe/ffmpeg, and later `preview_service` for preview caches.

### Integration Points
- `draft_model` should define `Draft`, `Material`, `Track`, `Segment`, timeranges, schema version, IDs, and validation/migration primitives.
- `project_store` should provide create/open/save/autosave-style functions that accept a filesystem implementation and operate on `.veproj/project.json`.
- `bindings_node` should expose Phase 2 command variants/results only after Rust schema and tests exist.
- `schemas/` and `apps/desktop-electron/src/generated/` should receive generated draft/material and command contracts through the existing export/test pattern.
- `fixtures/draft` and/or a new project fixture directory should contain deterministic `.veproj`/`project.json` fixtures classified by tests.

</code_context>

<specifics>
## Specific Ideas

- Treat Phase 2 as the durability layer for the whole editor. If a draft cannot round-trip safely here, later timeline/UI/rendering work will build on unstable ground.
- Maintain internal/external Jianying terminology consistently. The user explicitly rejected inventing separate internal terms such as `Asset`/`Clip` when Jianying-aligned terms already exist.
- Do not implement rich UI yet. A minimal Electron smoke can prove binding/contracts, but Phase 2 acceptance is schema, project store, and material import behavior.

</specifics>

<deferred>
## Deferred Ideas

- Timeline edit commands, segment overlap rules, snapping/main-track magnet behavior, undo/redo, and invalid edit rejection belong to Phase 3.
- Full Jianying-style material bin UI, inspector, timeline interactions, and visual layout checks belong to Phase 4.
- Preview frames, waveform/preview cache generation, render graph snapshots, and export jobs belong to Phase 5.
- Material relink UI and advanced compatibility import/export adapters are later work; Phase 2 only needs recoverable missing-material state and API evidence.

</deferred>

---

*Phase: 2-Draft And Material System*
*Context gathered: 2026-06-17*
