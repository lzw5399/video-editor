# Phase 03: Timeline Command Core - Research

**Researched:** 2026-06-17  
**Domain:** Rust timeline command semantics, undo/redo, snapping, text/audio semantic edits  
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

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

### Deferred Ideas (OUT OF SCOPE)

## Deferred Ideas

- Rich Jianying-style desktop workspace, drag interactions, visual timeline layout, inspector controls, and Playwright visual checks belong to Phase 4.
- Preview frames, waveform/preview cache generation, render graph snapshots, FFmpeg script compilation, and MP4 export belong to Phase 5.
- Deterministic text layout with pinned fonts belongs to Phase 5 with preview/export parity.
- Multiple sequences, nested timelines, ripple editing across arbitrary linked tracks, advanced keyframes, masks, filters, transitions, sticker behavior, text bubbles/effects, and compatibility adapters are post-MVP or later-phase work unless the roadmap is explicitly changed.
</user_constraints>

## Summary

Phase 3 should implement the command core as a pure Rust semantic layer in `draft_commands`, with `draft_model` providing all persisted draft types, generated command payload/response contracts, command-history/selection session types, and schema/TypeScript exports. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md, crates/draft_commands/src/lib.rs:1-9, crates/draft_model/src/lib.rs:36-69]

The existing `Draft.tracks`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, `Track.muted`, and `Track.locked` types are the correct base, but Phase 3 must add command-only rules for non-overlap, material-duration bounds, overflow-safe timerange math, track-kind/material-kind compatibility, locked-track rejection, snapshot command history, text style/content semantics, and segment volume. [VERIFIED: crates/draft_model/src/timeline.rs:9-146, crates/draft_model/src/material.rs:7-94, crates/draft_model/src/validation.rs:89-185, .planning/phases/03-timeline-command-core/03-CONTEXT.md]

**Primary recommendation:** Use clone/patch/validate/commit transactions inside `draft_commands`, return `TimelineCommandResponse { draft, command_state, selection, events }`, keep command history session-only, and add focused Rust tests before binding routes or UI work. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md, crates/draft_model/src/lib.rs:213-251]

## Project Constraints (from AGENTS.md)

- UI emits commands while Rust owns project and timeline semantics; UI must not directly construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is canonical; render graphs, FFmpeg scripts, thumbnails, waveform data, proxies, and preview caches are derived artifacts. [VERIFIED: AGENTS.md, docs/runtime-boundaries.md:76-95]
- Use Jianying-aligned terminology: draft/material/track/segment/keyframe/filter/transition-style terms. [VERIFIED: AGENTS.md, .planning/REQUIREMENTS.md:17-20]
- Core time math must use integer microseconds, frame indices, or rational frame rates, not naked persisted float time. [VERIFIED: AGENTS.md, crates/draft_model/src/time.rs:5-26]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes and reports, but does not decide editing behavior. [VERIFIED: AGENTS.md, docs/runtime-boundaries.md:35-37]
- Kdenlive and MLT are conceptual references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md, .planning/REQUIREMENTS.md:104-110]
- Each roadmap phase must define executable gates before implementation is considered complete. [VERIFIED: AGENTS.md, package.json:10-28]
- FFmpeg distribution/license review is later packaging work if redistributed binaries are shipped. [VERIFIED: AGENTS.md, docs/runtime-boundaries.md:39-57]
- GSD workflow says repo edits should happen through GSD entry points unless explicitly bypassed. [VERIFIED: AGENTS.md]

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TIME-01 | Draft supports at least one sequence with video, audio, and text tracks. | Use root `Draft.tracks` only, with `TrackKind::{Video, Audio, Text}` already present. [VERIFIED: .planning/REQUIREMENTS.md:32, crates/draft_model/src/timeline.rs:9-17] |
| TIME-02 | User can add material segments to tracks with explicit source and target time ranges. | Existing `Segment` has `material_id`, `source_timerange`, and `target_timerange`; add command must validate track/material compatibility and material duration. [VERIFIED: .planning/REQUIREMENTS.md:33, crates/draft_model/src/timeline.rs:89-121, crates/draft_model/src/material.rs:41-94] |
| TIME-03 | User can select, move, split, trim, and delete timeline segments. | Implement command payloads and session selection state in Rust contracts, with behavior in `draft_commands`. [VERIFIED: .planning/REQUIREMENTS.md:34, .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| TIME-04 | User can undo and redo every committed timeline edit. | Snapshot command history is acceptable if bounded/testable and session-only. [VERIFIED: .planning/REQUIREMENTS.md:35, .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| TIME-05 | Main-track magnet/snapping behavior is implemented in Rust core, not UI-only state. | Compute snapping/magnet in `draft_commands` and emit events. [VERIFIED: .planning/REQUIREMENTS.md:36, .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| TIME-06 | Invalid edits are rejected atomically without partially mutating the draft. | Use clone/patch/validate/commit; existing material helpers show rollback-on-validation precedent. [VERIFIED: .planning/REQUIREMENTS.md:37, crates/draft_model/src/material.rs:114-144] |
| TIME-07 | Track stacking/z-index and per-track mute state are represented in draft model. | Track order, `muted`, and `locked` already persist; command rules must enforce locked-track rejection. [VERIFIED: .planning/REQUIREMENTS.md:38, crates/draft_model/src/timeline.rs:124-146] |
| TEXT-01 | User can add text/subtitle segments to a text track. | `MaterialKind::Text` and `TrackKind::Text` exist, but text content/style fields are missing and should be modeled explicitly. [VERIFIED: .planning/REQUIREMENTS.md:42, crates/draft_model/src/material.rs:7-15, crates/draft_model/src/timeline.rs:9-17] |
| TEXT-02 | User can edit text content, font size, color, alignment, stroke, shadow, and background. | Add semantic text style/content structs to `draft_model`; do not implement deterministic rendering in this phase. [VERIFIED: .planning/REQUIREMENTS.md:43-44, .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| AUD-01 | User can add audio/BGM materials to an audio track. | `MaterialKind::Audio`, audio metadata, and `TrackKind::Audio` exist; add command should reject incompatible track/material pairs. [VERIFIED: .planning/REQUIREMENTS.md:45, crates/draft_model/src/material.rs:7-67, crates/draft_model/src/timeline.rs:9-17] |
| AUD-02 | User can adjust segment volume and track mute state. | `Track.muted` exists; per-segment volume field is missing and should be added to segment/audio semantics. [VERIFIED: .planning/REQUIREMENTS.md:46, crates/draft_model/src/timeline.rs:124-146] |
| TEST-02 | Command tests cover split, trim, move, delete, snapping, undo, redo, text edit, and volume edit. | Add direct `draft_commands` tests plus binding smoke once routes exist. [VERIFIED: .planning/REQUIREMENTS.md:72-73, package.json:16-28] |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Timeline model persistence | Rust `draft_model` | JSON schema / generated TS | Persisted semantic types and schema generation already live in `draft_model`. [VERIFIED: crates/draft_model/src/lib.rs:13-31, crates/draft_model/tests/schema_exports.rs:29-114] |
| Edit command behavior | Rust `draft_commands` | `draft_model` validation | Phase 3 decisions assign add/move/split/trim/delete/snapping/undo semantics to pure Rust commands. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md, crates/draft_commands/src/lib.rs:1-9] |
| Command routing | Node binding (`bindings_node`) | Electron preload/main | Existing `execute_command` deserializes Rust contracts and routes commands; Phase 3 should add routes after pure tests. [VERIFIED: crates/bindings_node/src/lib.rs:41-96] |
| Selection and undo/redo state | Rust command session state | Electron storage as opaque JSON | History is not persisted in `.veproj/project.json`; Electron may pass it but must not interpret it. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| Snapping and MainTrackMagnet | Rust `draft_commands` | UI display only | Core computes snap candidates/final ranges and emits events for UI synchronization. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| Text/audio semantic fields | Rust `draft_model` | `draft_commands` mutation | Text style/content and audio volume are persisted semantics; rendering, waveform, and preview are deferred. [VERIFIED: .planning/REQUIREMENTS.md:42-46, .planning/phases/03-timeline-command-core/03-CONTEXT.md] |

## Standard Stack

### Core

| Library / Crate | Version | Purpose | Why Standard |
|-----------------|---------|---------|--------------|
| Rust workspace / local crates | rustc 1.95.0; edition 2024 | Compile pure semantic crates and binding crate. | Workspace pins Rust 1.95.0 and edition 2024. [VERIFIED: Cargo.toml:17-20, `rustc --version`] |
| `draft_model` | local 0.1.0 | Persisted draft types, command contracts, validation, schema/TS exports. | Existing source of truth for Rust-owned contracts. [VERIFIED: crates/draft_model/src/lib.rs:1-31] |
| `draft_commands` | local 0.1.0 | Timeline edit semantics, snapping, command history. | Existing pure semantic boundary for Phase 3. [VERIFIED: crates/draft_commands/src/lib.rs:1-9] |
| `bindings_node` | local 0.1.0 | Route generated command envelopes from Electron to Rust services. | Existing `execute_command` envelope router. [VERIFIED: crates/bindings_node/src/lib.rs:41-96] |
| `serde` | 1.0.228 | Strict JSON IPC and persisted model serialization. | Existing dependency; Serde documents `rename_all`, `deny_unknown_fields`, and enum tagging. [VERIFIED: crates/draft_model/Cargo.toml, crates.io via `cargo info serde`, CITED: https://serde.rs/container-attrs.html, https://serde.rs/enum-representations.html] |
| `schemars` | 1.2.1 | Generate JSON Schema from Rust semantic types. | Existing generator uses `schema_for!`; docs show derive-driven schema generation. [VERIFIED: crates/draft_model/Cargo.toml, crates.io via `cargo info schemars`, CITED: https://docs.rs/schemars/1.2.1/schemars/macro.schema_for.html] |
| `ts-rs` | 12.0.1 | Generate TypeScript contracts from Rust types. | Existing tests use `Config::with_large_int("number")`; docs expose `with_large_int`. [VERIFIED: crates/draft_model/Cargo.toml, crates/draft_model/tests/schema_exports.rs:117-126, CITED: https://docs.rs/ts-rs/12.0.1/ts_rs/struct.Config.html] |

### Supporting

| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| `serde_json` | 1.0.150 | Command/draft JSON round trips and schema test values. | Use in contract tests, binding conversion, and fixture validation. [VERIFIED: crates/draft_model/Cargo.toml, crates/bindings_node/src/lib.rs:63-72, crates.io via `cargo info serde_json`] |
| `jsonschema` | 0.46.5 | Validate generated schemas against fixtures. | Extend existing schema fixture tests when command payloads grow. [VERIFIED: crates/draft_model/Cargo.toml, crates/draft_model/tests/schema_exports.rs:210-234, crates.io via `cargo info jsonschema`] |
| Rust `u64::checked_add` | std | Overflow-safe timerange end math. | Use for `start + duration`, split, trim, and material-duration bounds. [CITED: https://doc.rust-lang.org/std/primitive.u64.html#method.checked_add] |
| Rust `Vec::sort_by_key` | std | Deterministic ordering of tracks/segments/events. | Use for sorted views/events; persist track order as-is. [CITED: https://doc.rust-lang.org/std/vec/struct.Vec.html#method.sort_by_key] |
| Rust `std::mem::replace` | std | Transactional session-state replacement if needed. | Useful for bounded history update without aliasing issues. [CITED: https://doc.rust-lang.org/std/mem/fn.replace.html] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Snapshot history | Inverse operations | Inverse operations can reduce memory, but Phase 3 explicitly permits bounded snapshots and snapshots are simpler to validate for atomicity. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| New external command framework crate | Handwritten local command structs | No new crate is needed; existing contracts are generated from Rust types and command logic is project-specific. [VERIFIED: crates/draft_model/src/lib.rs:36-69, crates/draft_model/tests/schema_exports.rs:29-114] |
| UI-owned snapping | Rust-owned snapping | UI-owned snapping violates locked Phase 3 decisions and future cross-platform semantics. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |

**Installation:** No new third-party package install is recommended for Phase 3. Add only path dependencies as needed, for example `draft_commands -> draft_model` and `bindings_node -> draft_commands`. [VERIFIED: Cargo.toml:1-20, crates/draft_commands/Cargo.toml:1-10]

**Version verification:** Existing external crates were checked with `cargo search` and `cargo info` on 2026-06-17. [VERIFIED: crates.io via cargo CLI]

## Package Legitimacy Audit

Phase 3 should not install new external packages; use existing workspace crates and existing external dependencies already locked by earlier phases. [VERIFIED: Cargo.toml:1-20, Cargo.lock, crates/draft_model/Cargo.toml]

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `serde` | crates.io | Existing dependency | Not checked | https://github.com/serde-rs/serde | N/A: slopcheck is npm-oriented and produced invalid npm findings for Rust crate names | Approved as existing dependency; no new install. [VERIFIED: `cargo info serde`] |
| `serde_json` | crates.io | Existing dependency | Not checked | https://github.com/serde-rs/json | N/A: npm-oriented tool, invalid for Rust crate names | Approved as existing dependency; no new install. [VERIFIED: `cargo info serde_json`] |
| `schemars` | crates.io | Existing dependency | Not checked | https://github.com/GREsau/schemars | N/A: npm-oriented tool, invalid for Rust crate names | Approved as existing dependency; no new install. [VERIFIED: `cargo info schemars`] |
| `ts-rs` | crates.io | Existing dependency | Not checked | https://github.com/Aleph-Alpha/ts-rs | N/A: npm-oriented tool, invalid for Rust crate names | Approved as existing dependency; no new install. [VERIFIED: `cargo info ts-rs`] |
| `jsonschema` | crates.io | Existing dev dependency | Not checked | https://github.com/Stranger6667/jsonschema | N/A: npm package named `jsonschema` is unrelated to the Rust crate | Approved as existing dependency; no new install. [VERIFIED: `cargo info jsonschema`] |

**Packages removed due to slopcheck [SLOP] verdict:** none; slopcheck output was ignored because it checked npm, not crates.io. [VERIFIED: slopcheck CLI output]  
**Packages flagged as suspicious [SUS]:** none for Rust crates; npm-oriented slopcheck result is not applicable. [VERIFIED: slopcheck CLI output]

## Architecture Patterns

### System Architecture Diagram

```text
Electron renderer gesture
  -> generated CommandEnvelope JSON
  -> bindings_node::execute_command
  -> draft_commands::execute_timeline_command
  -> clone draft + clone command_state
  -> apply typed command to clone
  -> validate timeranges / material bounds / track lock / overlap / schema
  -> if valid: commit cloned draft + state, emit events
  -> if invalid: return error/rejection event with original draft/state unchanged
  -> generated CommandResultEnvelope JSON
  -> Electron stores updated draft + opaque command_state and renders events
```

[VERIFIED: crates/bindings_node/src/lib.rs:41-96, crates/draft_model/src/lib.rs:213-251, .planning/phases/03-timeline-command-core/03-CONTEXT.md]

### Recommended Project Structure

```text
crates/
  draft_model/src/
    timeline.rs          # persisted track/segment/text/audio semantic fields
    lib.rs               # command payload/response/history/session contracts
    validation.rs        # draft-level validation shared by commands/persistence
  draft_commands/src/
    lib.rs               # public command API and module exports
    error.rs             # typed invalid-edit diagnostics
    history.rs           # bounded snapshot undo/redo session state
    selection.rs         # Rust-owned selection state
    snapping.rs          # snap candidate collection and MainTrackMagnet behavior
    timeline.rs          # add/move/split/trim/delete/select/mute/volume/text commands
  bindings_node/src/
    lib.rs               # route new command variants through execute_command
```

[VERIFIED: existing crate layout via `find crates -maxdepth 3 -type f`, .planning/phases/03-timeline-command-core/03-CONTEXT.md]

### Pattern 1: Transactional Command Execution

**What:** Clone `Draft` and `CommandState`, mutate clones, validate everything, then return committed clones only on success. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]  
**When to use:** Every user-visible edit that can fail: add, move, split, trim, delete, text edit, volume edit, track mute, undo, and redo. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]

```rust
// Source: local pattern from crates/draft_model/src/material.rs:114-144 and Phase 3 D-07.
pub fn execute_edit(
    draft: &Draft,
    state: &CommandState,
    command: TimelineCommand,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let mut next_state = state.clone();
    apply_to_clones(&mut next_draft, &mut next_state, command)?;
    validate_timeline_edit(&next_draft)?;
    next_state.push_snapshot(draft.clone())?;
    Ok(TimelineCommandResponse {
        draft: next_draft,
        command_state: next_state,
        events: vec![command_event("timelineEditCommitted")],
    })
}
```

### Pattern 2: Timerange Helpers, Not Inline Arithmetic

**What:** Centralize `end = start.checked_add(duration)` and overlap/containment logic in helper types or functions. [CITED: https://doc.rust-lang.org/std/primitive.u64.html#method.checked_add]  
**When to use:** Add, split, trim, move, material-duration validation, snapping candidates, and MainTrackMagnet gap handling. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]

```rust
// Source: Rust std checked_add docs and local Microseconds wrapper.
fn checked_end(start: Microseconds, duration: Microseconds) -> Result<Microseconds, TimelineCommandError> {
    start
        .get()
        .checked_add(duration.get())
        .map(Microseconds::new)
        .ok_or(TimelineCommandError::TimerangeOverflow)
}
```

### Pattern 3: Generated Contract Expansion

**What:** Add new command payloads/responses to `draft_model`, include them in `schema_exports.rs`, regenerate schemas/TS with `VE_UPDATE_GENERATED_CONTRACTS=1`, then gate with `git diff --exit-code schemas apps/desktop-electron/src/generated`. [VERIFIED: crates/draft_model/tests/schema_exports.rs:29-165, package.json:27]  
**When to use:** Every new `CommandName`, `CommandPayload`, response, command-state, text style, or audio volume type. [VERIFIED: crates/draft_model/src/lib.rs:47-69]

### Anti-Patterns to Avoid

- **Renderer-side draft repair:** Electron must not resolve overlaps, snap targets, or infer inverse operations. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]
- **Partial in-place mutation:** Mutating the caller's draft/history before all validations pass violates atomicity. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]
- **Persisting undo/redo history in `project.json`:** History is session state only. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]
- **Text as URI-only fake media:** Text content/style must be explicit semantic data, not hidden in a URI string. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]
- **Float seconds in contracts:** Semantic time must remain integer microseconds or rational frame-rate fields. [VERIFIED: AGENTS.md, crates/draft_model/src/time.rs:5-26]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON serialization/schema/TS contracts | Parallel handwritten TypeScript/JSON contracts | `serde` + `schemars` + `ts-rs` | Existing tests generate and drift-check contracts from Rust. [VERIFIED: crates/draft_model/tests/schema_exports.rs:29-165] |
| Overflow timerange math | Raw `start + duration` | `u64::checked_add` helpers | Rust std documents checked overflow return via `Option`. [CITED: https://doc.rust-lang.org/std/primitive.u64.html#method.checked_add] |
| Undo/redo inverse algebra for MVP | Custom inverse operation graph | Bounded snapshot history | Locked decision allows snapshots if bounded/testable. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| Track stacking in UI | UI z-index as semantic truth | Persisted `Draft.tracks` order and Rust command validation | Phase 3 locks track order as semantic. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| Snapping algorithm in React | UI-only snap candidates | `draft_commands::snapping` | Snapping and MainTrackMagnet are Rust-owned and event-emitting. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |

**Key insight:** The hard part is not data storage; it is keeping every edit path atomic, typed, testable, and Rust-owned so Phase 4 UI cannot accidentally become the editor engine. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md, docs/runtime-boundaries.md:23-37]

## Common Pitfalls

### Pitfall 1: `validate_draft` Is Necessary But Not Sufficient
**What goes wrong:** A draft passes current validation while still allowing overlapping segments, incompatible material/track kinds, or source ranges beyond material duration. [VERIFIED: crates/draft_model/src/validation.rs:89-185]  
**Why it happens:** Phase 2 validation focused on draft/material integrity, not edit semantics. [VERIFIED: .planning/phases/02-draft-and-material-system/02-CONTEXT.md]  
**How to avoid:** Add command-level `validate_timeline_edit` checks and call both edit validation and `validate_draft`. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]  
**Warning signs:** Tests only assert schema validity, not exact before/after command state. [VERIFIED: .planning/REQUIREMENTS.md:72-73]

### Pitfall 2: Source/Target Timerange Drift
**What goes wrong:** Trim and split update only target ranges or only source ranges, breaking visible content alignment. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]  
**Why it happens:** `SourceTimerange` and `TargetTimerange` are separate persisted fields. [VERIFIED: crates/draft_model/src/timeline.rs:19-49]  
**How to avoid:** Test left trim, right trim, middle split, move, and material-boundary cases with exact expected `source_timerange` and `target_timerange`. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]  
**Warning signs:** Command tests assert only segment count or target start. [VERIFIED: .planning/REQUIREMENTS.md:72-73]

### Pitfall 3: Invalid Edits Enter Undo History
**What goes wrong:** Rejected commands leave history entries or clear redo stacks. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]  
**Why it happens:** History mutation occurs before command validation. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]  
**How to avoid:** Push history only after all validation passes; add tests comparing draft and history before/after rejected edits. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]  
**Warning signs:** Error paths return `ok=false` but include changed `command_state`. [VERIFIED: crates/draft_model/src/lib.rs:213-251]

### Pitfall 4: Contract Drift After Adding Command Variants
**What goes wrong:** Rust accepts a command that generated TypeScript/schema do not know about, or schema accepts stale variants. [VERIFIED: crates/draft_model/tests/schema_exports.rs:29-165]  
**Why it happens:** New payload/result types are not added to `schema_exports.rs`. [VERIFIED: crates/draft_model/tests/schema_exports.rs:44-80]  
**How to avoid:** Every contract type must be exported in the schema export test, then committed generated artifacts must pass drift checks. [VERIFIED: crates/draft_model/tests/schema_exports.rs:143-165, package.json:27]  
**Warning signs:** `git diff --exit-code schemas apps/desktop-electron/src/generated` fails. [VERIFIED: package.json:27]

### Pitfall 5: Platform Leakage Into Pure Crates
**What goes wrong:** `draft_commands` imports project-store, FFmpeg, preview, Electron, or filesystem abstractions. [VERIFIED: docs/runtime-boundaries.md:23-37]  
**Why it happens:** Command implementation tries to probe media or save project bundles. [VERIFIED: docs/runtime-boundaries.md:76-88]  
**How to avoid:** Commands only inspect the `Draft` and material metadata already present in it; binding services handle persistence. [VERIFIED: docs/runtime-boundaries.md:83-88]  
**Warning signs:** `rg "FfmpegExecutor|PlatformFileSystem|PreviewRenderer|ffmpeg|ffprobe" crates/draft_commands crates/draft_model` returns matches. [VERIFIED: docs/runtime-boundaries.md:23-37]

## Code Examples

### New Command Contract Shape

```rust
// Source: existing CommandEnvelope pattern in crates/draft_model/src/lib.rs:47-69.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveSegmentCommandPayload {
    pub draft: Draft,
    pub command_state: CommandState,
    pub segment_id: SegmentId,
    pub target_track_id: TrackId,
    pub target_start: Microseconds,
    pub snapping: SnappingSettings,
}
```

### Command-State Response Shape

```rust
// Source: existing CommandResultEnvelope pattern in crates/draft_model/src/lib.rs:213-251.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineCommandResponse {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
}
```

### Overlap Predicate

```rust
// Source: Phase 3 D-03 plus Rust checked arithmetic docs.
fn overlaps(a: &TargetTimerange, b: &TargetTimerange) -> Result<bool, TimelineCommandError> {
    let a_end = checked_end(a.start, a.duration)?;
    let b_end = checked_end(b.start, b.duration)?;
    Ok(a.start < b_end && b.start < a_end)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| UI mutates timeline arrays directly | UI emits generated command envelopes to Rust-owned command core | Locked before Phase 3 planning | Preserves cross-platform semantics and atomic edits. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| Persisted undo stack | Session-only command history | Locked before Phase 3 planning | Keeps `.veproj/project.json` canonical semantic draft state. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| Float seconds in command payloads | Integer microseconds in persisted/IPC semantics | Project constraint from initialization | Avoids drift and locale/timecode bugs. [VERIFIED: AGENTS.md, crates/draft_model/src/time.rs:5-26] |
| Render/runtime commands decide editing behavior | `draft_commands` decides edits; render/FFmpeg layers derive later | Project architecture | Prevents preview/export drift and FFmpeg leakage. [VERIFIED: docs/runtime-boundaries.md:23-37] |

**Deprecated/outdated:**
- UI-only snapping is out of scope for this architecture; Rust must compute snapping and emit events. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]
- External draft compatibility semantics are deferred and must not shape Phase 3 internals beyond Jianying vocabulary. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | No new external crates are needed for Phase 3 if the planner accepts handwritten local command/history modules. [ASSUMED] | Standard Stack | Planner may need a human-verify checkpoint before adding any extra crate. |

## Open Questions (RESOLVED)

1. **What exact default snap threshold should Phase 3 use? RESOLVED**
   - What we know: The threshold must be deterministic, named, microsecond-based, overridable, and tested. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]
   - Resolution: Phase 3 plans use `DEFAULT_SNAP_THRESHOLD_US = 100_000`. The value is named in code, payload-overridable, and covered by snapping tests. [RESOLVED: 03-03-PLAN.md]

2. **What bounded history limit should snapshot undo use? RESOLVED**
   - What we know: Snapshot history is acceptable if bounded and testable. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]
   - Resolution: Phase 3 plans use `DEFAULT_HISTORY_LIMIT = 100`. The value is named in code, test-overridable where needed, and covered by undo/redo history-pruning tests. [RESOLVED: 03-03-PLAN.md]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| `cargo` | Rust checks/tests | yes | 1.95.0 | none needed. [VERIFIED: `cargo --version`] |
| `rustc` | Rust compile | yes | 1.95.0 | none needed. [VERIFIED: `rustc --version`] |
| `node` | Electron/generated contract tooling | yes | 24.12.0 | none needed. [VERIFIED: `node --version`, package.json:5-8] |
| `pnpm` | Root build/test scripts | yes | 10.32.1 | none needed. [VERIFIED: `pnpm --version`, package.json:5-8] |
| `just` | Published root gates | no | unavailable | Use `pnpm run build` / `pnpm run test` or install `just` before final phase gates. [VERIFIED: `command -v just`, justfile:1-18] |
| `ffmpeg` | Root `just test` includes render smoke from prior phases | yes | 8.1 | Phase 3 direct command tests do not need it. [VERIFIED: `ffmpeg -version`, package.json:25] |
| `ffprobe` | Runtime/material tests in root gate | yes | 8.1 | Phase 3 direct command tests do not need it. [VERIFIED: `ffprobe -version`, package.json:20-21] |
| `ctx7` | Preferred docs lookup | no | unavailable | Used official docs and crates.io metadata instead. [VERIFIED: `command -v ctx7`] |

**Missing dependencies with no fallback:**
- `just` is missing for the exact `just build` / `just test` commands, but equivalent `pnpm run build` / `pnpm run test` scripts exist. [VERIFIED: `command -v just`, package.json:15-28, justfile:17-18]

**Missing dependencies with fallback:**
- `ctx7` is unavailable; official docs and crates.io metadata were used. [VERIFIED: `command -v ctx7`, CITED: https://serde.rs/container-attrs.html, https://docs.rs/schemars/1.2.1/schemars/macro.schema_for.html, https://docs.rs/ts-rs/12.0.1/ts_rs/struct.Config.html]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness via `cargo test`; Electron tests remain in pnpm workspace. [VERIFIED: package.json:16-28] |
| Config file | `Cargo.toml`, `package.json`, `justfile`. [VERIFIED: Cargo.toml:1-20, package.json:10-28, justfile:1-18] |
| Quick run command | `cargo test -p draft_commands -- --nocapture` after implementation. [VERIFIED: crates/draft_commands/Cargo.toml:1-10] |
| Full suite command | `pnpm run test` or `just test` after `just` is installed. [VERIFIED: package.json:28, justfile:17-18] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| TIME-01 | Root timeline has video/audio/text tracks | unit/fixture | `cargo test -p draft_commands timeline_tracks -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:32] |
| TIME-02 | Add segment with explicit source/target ranges | unit | `cargo test -p draft_commands add_segment -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:33] |
| TIME-03 | Select/move/split/trim/delete | unit | `cargo test -p draft_commands timeline_edits -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:34] |
| TIME-04 | Undo/redo committed edits | unit | `cargo test -p draft_commands undo_redo -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:35] |
| TIME-05 | Snapping and MainTrackMagnet | unit | `cargo test -p draft_commands snapping -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:36] |
| TIME-06 | Invalid edits atomic rejection | unit | `cargo test -p draft_commands invalid_edits_are_atomic -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:37] |
| TIME-07 | Track stacking and mute | unit/fixture | `cargo test -p draft_commands track_rules -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:38] |
| TEXT-01 | Add text segment | unit | `cargo test -p draft_commands text_commands -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:42] |
| TEXT-02 | Edit text content/style | unit | `cargo test -p draft_commands text_commands -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:43] |
| AUD-01 | Add audio/BGM segment | unit | `cargo test -p draft_commands audio_commands -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:45] |
| AUD-02 | Segment volume and track mute | unit | `cargo test -p draft_commands audio_commands -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:46] |
| TEST-02 | Required command coverage | meta/source gate | `cargo test -p draft_commands -- --nocapture` | No, Wave 0. [VERIFIED: .planning/REQUIREMENTS.md:72-73] |

### Sampling Rate
- **Per task commit:** `cargo test -p draft_commands -- --nocapture` plus affected `draft_model` tests. [VERIFIED: package.json:16-18]
- **Per wave merge:** `pnpm run test:rust && pnpm run test:contracts`. [VERIFIED: package.json:16, package.json:27]
- **Phase gate:** `just build && just test` if `just` is installed, otherwise `pnpm run build && pnpm run test` with a note that `just` is missing locally. [VERIFIED: justfile:17-18, package.json:15-28, `command -v just`]

### Wave 0 Gaps
- [ ] `crates/draft_commands/Cargo.toml` needs `draft_model = { path = "../draft_model" }`. [VERIFIED: crates/draft_commands/Cargo.toml:1-10]
- [ ] `crates/draft_commands/src/*.rs` command modules and tests do not exist yet. [VERIFIED: crates/draft_commands/src/lib.rs:1-9]
- [ ] `draft_model` command payload/response/history/selection/text/audio contract types need schema export wiring. [VERIFIED: crates/draft_model/src/lib.rs:47-69, crates/draft_model/tests/schema_exports.rs:44-80]
- [ ] Phase 3 source guard should extend Phase 2 guard for no platform leakage in `draft_commands` and no float seconds in new contracts. [VERIFIED: package.json:26, docs/runtime-boundaries.md:23-37]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Desktop local command semantics do not introduce authentication. [VERIFIED: phase scope in .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| V3 Session Management | yes | Undo/redo history is session state; keep it opaque, bounded, and non-persisted in `project.json`. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| V4 Access Control | yes | Locked tracks must reject mutating commands. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md, crates/draft_model/src/timeline.rs:124-146] |
| V5 Input Validation | yes | Strict Serde contracts, schema validation, command semantic validation, checked timerange arithmetic. [VERIFIED: crates/draft_model/src/lib.rs:36-69, crates/draft_model/src/validation.rs:89-185, CITED: https://serde.rs/container-attrs.html] |
| V6 Cryptography | no | No cryptographic feature in Phase 3. [VERIFIED: phase scope in .planning/phases/03-timeline-command-core/03-CONTEXT.md] |

### Known Threat Patterns for Rust Timeline Commands

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed command payload changes draft unexpectedly | Tampering | Strict `deny_unknown_fields`, command/payload matching, typed command variants. [VERIFIED: crates/draft_model/src/lib.rs:36-111, CITED: https://serde.rs/container-attrs.html] |
| Overflowed time ranges bypass validation | Tampering/DoS | Use `checked_add` and reject overflowed source/target ranges. [CITED: https://doc.rust-lang.org/std/primitive.u64.html#method.checked_add] |
| Locked track mutation | Tampering | Reject mutating commands when `Track.locked` is true. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md, crates/draft_model/src/timeline.rs:124-146] |
| Rejected command mutates undo history | Repudiation/Tampering | Clone/validate/commit and push history only after success. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md] |
| Renderer bypasses Rust semantics | Elevation of Privilege/Tampering | Add binding routes only through generated `execute_command`; renderer must not mutate `Draft.tracks`. [VERIFIED: crates/bindings_node/src/lib.rs:41-96, .planning/phases/03-timeline-command-core/03-CONTEXT.md] |

## Sources

### Primary (HIGH Confidence)
- `.planning/phases/03-timeline-command-core/03-CONTEXT.md` - locked Phase 3 decisions, discretion, deferred scope. [VERIFIED: local file]
- `.planning/REQUIREMENTS.md` - TIME/TEXT/AUD/TEST requirements. [VERIFIED: local file]
- `AGENTS.md` - architecture, terminology, time, rendering, testing, licensing constraints. [VERIFIED: local file]
- `docs/runtime-boundaries.md` - pure semantic crate and runtime boundary rules. [VERIFIED: local file]
- `crates/draft_model/src/*.rs` - existing draft, material, timeline, validation, command contract types. [VERIFIED: codebase grep/read]
- `crates/draft_model/tests/schema_exports.rs` - generated schema/TypeScript drift pattern. [VERIFIED: codebase grep/read]
- `crates/bindings_node/src/lib.rs` - existing `execute_command` route pattern. [VERIFIED: codebase grep/read]
- Rust standard library docs for `checked_add`, `mem::replace`, and `Vec::sort_by_key`. [CITED: https://doc.rust-lang.org/std/primitive.u64.html#method.checked_add, https://doc.rust-lang.org/std/mem/fn.replace.html, https://doc.rust-lang.org/std/vec/struct.Vec.html#method.sort_by_key]
- Serde official docs for attributes and enum representation. [CITED: https://serde.rs/container-attrs.html, https://serde.rs/enum-representations.html]
- docs.rs for `schemars::schema_for!` and `ts-rs::Config`. [CITED: https://docs.rs/schemars/1.2.1/schemars/macro.schema_for.html, https://docs.rs/ts-rs/12.0.1/ts_rs/struct.Config.html]

### Secondary (MEDIUM Confidence)
- crates.io metadata via `cargo info` / `cargo search` for `serde`, `serde_json`, `schemars`, `ts-rs`, and `jsonschema`. [VERIFIED: crates.io via cargo CLI]
- `.planning/phases/02-draft-and-material-system/02-VERIFICATION.md` and `02-SECURITY.md` - prior phase gates and security precedent. [VERIFIED: local file]

### Tertiary (LOW Confidence)
- None used as authoritative guidance. [VERIFIED: research log]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Phase 3 can use existing workspace crates and already verified external dependencies. [VERIFIED: Cargo.toml, crates/draft_model/Cargo.toml, crates.io via cargo CLI]
- Architecture: HIGH - Locked decisions and current crate boundaries align. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md, docs/runtime-boundaries.md]
- Pitfalls: HIGH - Pitfalls come from explicit Phase 3 decisions, current validation gaps, and Phase 2 verification/security precedent. [VERIFIED: local files and codebase grep]

**Research date:** 2026-06-17  
**Valid until:** 2026-07-17 for local architecture decisions; re-check crate/tool versions before adding any new package. [ASSUMED]
