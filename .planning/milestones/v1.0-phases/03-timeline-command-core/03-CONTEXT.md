# Phase 3: Timeline Command Core - Context

**Gathered:** 2026-06-17
**Status:** Ready for planning
**Mode:** auto-discuss

<domain>
## Phase Boundary

Phase 3 turns the persisted Phase 2 draft schema into a Rust-owned editing system. It delivers the MVP timeline command core: one editable root timeline/sequence with video, audio, and text tracks; typed add/select/move/split/trim/delete commands; invalid-edit rejection without partial mutation; undo/redo for committed edits; Rust-computed snapping and MainTrackMagnet behavior; and semantic MVP text/audio edits. It does not build the rich desktop timeline UI, preview frames, render graph, FFmpeg export, waveform/preview caches, deterministic text rendering, advanced filters/transitions/effects, multiple sequences, or mobile/server shells.

</domain>

<decisions>
## Implementation Decisions

### Timeline Shape And Stacking
- **D-01:** The MVP has one editable sequence represented by the draft's root timeline (`Draft.tracks`). Do not introduce multi-sequence, nested sequence, or project-bin timeline semantics in Phase 3.
- **D-02:** Track order is semantic. For visual tracks, later/higher tracks stack above earlier/lower tracks; audio tracks mix in track order. Persisted `Track.muted` remains the per-track mute state, and locked tracks must reject edit commands that would mutate them.
- **D-03:** Segments on a single track must not overlap in Phase 3. Overlays and simultaneous text/audio are represented by placing segments on separate tracks. The command core, not the UI, enforces overlap and stacking rules.
- **D-04:** The planner may add helper types for resolved ordering, selection, and edit diagnostics, but persisted semantic concepts should remain Jianying-aligned `Draft`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, `Keyframe`, `Filter`, and `Transition`.

### Command API And Atomic Mutation
- **D-05:** `draft_commands` owns pure Rust timeline semantics. It may depend on `draft_model`, but it must not depend on Electron, filesystem/project_store, FFmpeg/media_runtime, preview_service, or platform traits.
- **D-06:** User-visible timeline edits are typed Rust command contracts. Electron may call them through the existing `executeCommand` envelope, but it must not directly mutate `Draft.tracks`, construct derived timeline state, or repair invalid edits in renderer code.
- **D-07:** Every edit command must be atomic. Use a clone/patch/validate/commit style or equivalent transaction so failed add/move/split/trim/delete/text/audio commands leave the input draft and undo/redo state unchanged.
- **D-08:** Caller-supplied IDs are preferred for deterministic command tests and fixtures. If the command layer generates IDs later, the generator must be injectable/testable and not tied to Electron.
- **D-09:** Command responses should include the updated draft, updated command history/selection state where relevant, and stable events such as segment added, segment moved, snapped, rejected, undo committed, or redo committed. Events are for UI synchronization and test assertions; they are not render semantics.

### Source And Target Timerange Editing
- **D-10:** Persisted time math remains integer microseconds for Phase 3. No command payload, persisted draft field, test fixture, or generated TypeScript contract should introduce naked floating-point seconds for semantic timing.
- **D-11:** `SourceTimerange` describes what part of the material or text/audio source is used. `TargetTimerange` describes where the segment lives on the timeline. Add/move/split/trim commands must update these explicitly in Rust.
- **D-12:** Moving a segment changes `TargetTimerange.start` only. Trimming left/right changes both source and target ranges as needed to preserve the visible content alignment. Splitting at a target time creates two valid segments with adjacent target ranges and adjusted source ranges.
- **D-13:** The command core must reject source ranges that exceed known material duration when the material has a duration, and reject zero-duration or overflowed target/source ranges. Missing material files stay recoverable material state; they do not by themselves make existing timeline segments invalid.

### Snapping And MainTrackMagnet
- **D-14:** Snapping is Rust-owned. Move/trim commands can accept deterministic snapping settings, but the core computes snap candidates and final target ranges; the UI only displays the result.
- **D-15:** Use a deterministic default snap threshold for tests, expressed in microseconds. The exact default can be chosen by the planner, but it must be named in code and overridable in command payloads/tests.
- **D-16:** MainTrackMagnet behavior applies to the MVP main video track, defined as the first video track unless a future schema adds an explicit main-track marker. Commands should keep main-track segment ordering deterministic and close/prevent unintended gaps only within the documented Phase 3 scope.
- **D-17:** MainTrackMagnet and snapping behavior must emit observable command events so Phase 4 UI can show snapped/magnetized edits without reimplementing the algorithm.

### Undo/Redo Command State
- **D-18:** Undo/redo belongs to Rust command semantics. Electron may store and pass an opaque/generated Rust command state, but it must not interpret inverse operations or own undo semantics.
- **D-19:** Undo/redo state is not part of `.veproj/project.json`; the draft file remains canonical semantic project state. Command history is session state returned by commands and covered by Rust tests.
- **D-20:** For Phase 3, snapshot-based history is acceptable if it is bounded and testable. The planner may replace it with inverse operations only if that keeps atomicity and command coverage simpler.
- **D-21:** Undo and redo must work for every committed MVP edit command, including add, move, split, trim, delete, track mute, text style/content edit, and segment volume edit. Invalid rejected commands must not enter undo history.

### MVP Text And Audio Semantics
- **D-22:** Phase 3 implements semantic text/subtitle segments, not final text rendering. Text commands must cover content, font size, color, alignment, stroke, shadow, and background values required by `TEXT-01` and `TEXT-02`; deterministic layout and pinned-font rendering remain Phase 5 (`TEXT-03`).
- **D-23:** Text segments must not be faked as external media files. If validation requires a material reference, use a clearly modeled internal text material/source pattern; do not hide editable text content only in a URI string.
- **D-24:** Audio/BGM commands must support adding audio materials to audio tracks and changing segment volume plus track mute state. Audio waveform generation, preview cache invalidation, mixing/render implementation, and export behavior remain later phases.
- **D-25:** Advanced text bubbles, text effects, stickers, filters, transitions, keyframes, masks, and effect presets stay out of Phase 3 except for preserving existing placeholder schema fields without breaking validation.

### Testing And Gates
- **D-26:** Phase 3 must add command tests that cover add, select, move, split, trim, delete, snapping, MainTrackMagnet, undo, redo, text edit, and volume edit. Tests should prove invalid edits do not partially mutate either the draft or command history.
- **D-27:** Generated command schema and TypeScript contracts must be updated from Rust types and checked for drift. `just build` and `just test` remain the phase gates.
- **D-28:** Add positive and negative draft/command fixtures for timeline behavior where useful, keeping the Phase 1/2 fixture classification discipline. Avoid committing generated media unless existing testkit patterns explicitly require it.

### the agent's Discretion
- The planner may decide exact Rust module names, command payload/response structs, and whether command history is represented as snapshots or inverse operations, as long as the decisions above hold.
- The planner may decide whether timeline commands are routed directly in `bindings_node::execute_command` or through a small command service module, as long as Electron still receives generated Rust-owned contracts and cannot mutate timeline semantics directly.
- The planner may choose exact snap candidate priorities and the default threshold if they are deterministic, named, and covered by command tests.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Direction
- `.planning/PROJECT.md` - Product identity, Jianying terminology requirement, architecture constraints, and out-of-scope boundaries.
- `.planning/REQUIREMENTS.md` - Phase 3 requirements `TIME-01` through `TIME-07`, `TEXT-01`, `TEXT-02`, `AUD-01`, `AUD-02`, and `TEST-02`.
- `.planning/ROADMAP.md` - Phase 3 goal, success criteria, dependency on Phase 2, and planned work slices.
- `.planning/STATE.md` - Current phase status and accumulated decisions from Phases 1 and 2.

### Prior Phase Artifacts
- `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` - Locked decisions on Rust-owned contracts, generated schemas, pure semantic crates, and service-boundary trait placement.
- `.planning/phases/02-draft-and-material-system/02-CONTEXT.md` - Locked decisions on `.veproj/project.json`, Jianying-aligned schema vocabulary, material metadata, recoverable missing materials, and fixture gates.
- `.planning/phases/02-draft-and-material-system/02-VERIFICATION.md` - Evidence for completed draft/material behavior and gates.
- `.planning/phases/02-draft-and-material-system/02-SECURITY.md` - Threat register and mitigations around project persistence and media probing.

### Research
- `.planning/research/SUMMARY.md` - MVP shape, terminology, semantic pipeline, and test strategy.
- `.planning/research/ARCHITECTURE.md` - Layer responsibilities, Kdenlive/MLT/Jianying lessons, and semantic spine.
- `.planning/research/STACK.md` - Rust/Electron stack, crate responsibilities, and project format recommendation.
- `.planning/research/PITFALLS.md` - Known traps around UI-owned semantics, duplicate state, time bugs, command drift, and preview/export drift.

### Local Source Boundaries
- `docs/runtime-boundaries.md` - Pure semantic crate boundaries and service boundary rules.
- `crates/draft_model/src/timeline.rs` - Current `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, and placeholder effect structs.
- `crates/draft_model/src/validation.rs` - Current draft validation, duplicate ID checks, material reference checks, and timerange duration checks.
- `crates/draft_model/src/lib.rs` - Existing `CommandEnvelope`, command payload generation pattern, response envelope, and error kinds.
- `crates/draft_commands/src/lib.rs` - Empty pure semantic command crate boundary to implement in Phase 3.
- `crates/bindings_node/src/lib.rs` - Existing `execute_command` routing pattern and current material command integration.
- `crates/draft_model/tests/schema_exports.rs` - Generated JSON Schema and TypeScript contract drift pattern to extend.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/draft_model/src/timeline.rs` already contains Jianying-aligned `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, `Keyframe`, `Filter`, and `Transition` types.
- `crates/draft_model/src/validation.rs` already validates schema version, duplicate track/segment/material IDs, material references, zero-duration timeranges, frame rates, and derived artifact leakage.
- `crates/draft_model/src/lib.rs` already owns generated command contracts and the standardized `ok/error/events` envelope consumed by Electron.
- `crates/draft_commands` exists as the pure semantic command crate and is intentionally empty except for its boundary marker.
- `crates/bindings_node/src/lib.rs` already routes `execute_command` through Rust-owned `CommandEnvelope` variants and maps command errors into binding-safe envelopes.

### Established Patterns
- Rust semantic types are the source of truth; generated schemas and TypeScript files are checked through Rust tests and `just test`.
- Pure semantic crates do not import platform traits, FFmpeg, filesystem, Electron, preview services, or Node bindings.
- Service-boundary crates coordinate platform work only after semantic crates define the model and contracts.
- Existing material commands use generated Rust payload/response types and return complete updated `Draft` data through the command envelope.

### Integration Points
- Phase 3 should extend `draft_model` with command payload/response/history types and any missing text/audio semantic structs.
- Phase 3 should implement edit behavior in `draft_commands`, with direct Rust unit tests before binding integration.
- Phase 3 should route timeline commands through `bindings_node::execute_command` only after pure command tests and generated contract updates exist.
- Phase 3 should update `schemas/command.schema.json`, `schemas/draft.schema.json` if the draft model evolves, and generated TypeScript files in `apps/desktop-electron/src/generated/`.
- Phase 3 should add command fixtures and/or tests under existing `fixtures/draft` and Rust crate test directories, following the positive/negative fixture discipline from Phases 1 and 2.

</code_context>

<specifics>
## Specific Ideas

- The user explicitly wants internal and external terminology to use Jianying-style concepts, not a split vocabulary such as internal `Asset`/`Clip` for public `Material`/`Segment`.
- Kdenlive and MLT remain conceptual references for track/timeline discipline and media-engine separation, but Phase 3 must not copy their code, XML formats, presets, or runtime model.
- The editor UI later should feel like Jianying/CapCut, but Phase 3 should not build that UI. This phase should make Phase 4 possible by giving the UI a complete command-only surface.
- Tests should focus on exact draft state before/after commands, command events, undo/redo history, and rejection behavior.

</specifics>

<deferred>
## Deferred Ideas

- Rich Jianying-style desktop workspace, drag interactions, visual timeline layout, inspector controls, and Playwright visual checks belong to Phase 4.
- Preview frames, waveform/preview cache generation, render graph snapshots, FFmpeg script compilation, and MP4 export belong to Phase 5.
- Deterministic text layout with pinned fonts belongs to Phase 5 with preview/export parity.
- Multiple sequences, nested timelines, ripple editing across arbitrary linked tracks, advanced keyframes, masks, filters, transitions, sticker behavior, text bubbles/effects, and compatibility adapters are post-MVP or later-phase work unless the roadmap is explicitly changed.

</deferred>

---

*Phase: 3-Timeline Command Core*
*Context gathered: 2026-06-17*
