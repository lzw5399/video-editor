---
phase: 03-timeline-command-core
plan: 04
subsystem: timeline-command-core
tags: [rust, draft_model, draft_commands, text, audio, command-contracts]

requires:
  - phase: 03-timeline-command-core
    provides: timeline edit commands, bounded history, snapping events, and binding delegation
provides:
  - Semantic text/subtitle segment data with editable MVP style fields
  - Semantic audio add, segment volume, and track mute commands
  - Generated draft and command contracts for text/audio semantics
  - Binding routes and JSON smoke coverage for text/audio timeline commands
affects: [phase-03-timeline-command-core, phase-04-desktop-workspace, command-contracts]

tech-stack:
  added: []
  patterns:
    - Text content and style live on Segment.text while an internal text material preserves material/track compatibility
    - Segment volume uses integer millivolume values with unity at 1000 and max at 4000
    - Text/audio commands use clone/validate/commit/history transactions and binding delegation

key-files:
  created:
    - crates/draft_commands/src/text.rs
    - crates/draft_commands/src/audio.rs
    - crates/draft_commands/tests/text_audio_commands.rs
  modified:
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/Draft.ts
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/tests/binding_smoke.rs
    - crates/draft_commands/src/lib.rs
    - crates/draft_commands/src/timeline.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/src/timeline.rs
    - crates/draft_model/src/validation.rs
    - crates/draft_model/tests/draft_schema.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/command.schema.json
    - schemas/draft.schema.json

key-decisions:
  - "Text commands store editable content and MVP style fields on Segment.text, not only in a material URI."
  - "Text materials use an internal text:// URI only as a modeled material/source reference; segment text remains the semantic source of truth."
  - "SegmentVolume uses integer millivolume semantics: 0 silent, 1000 unity, and MAX_SEGMENT_VOLUME_MILLIS = 4000."
  - "Track mute changes go through Rust command semantics and reject locked tracks instead of direct UI mutation."

patterns-established:
  - "Text/audio command payloads include complete Draft, CommandState, and TimelineSelection inputs like earlier timeline commands."
  - "Generated Draft.ts now exposes Segment.text and Segment.volume for Phase 4 UI panels."
  - "bindings_node timeline smoke tests now cover text/audio route allow-list behavior, not just Rust enum exhaustiveness."

requirements-completed: [TIME-04, TIME-06, TEXT-01, TEXT-02, AUD-01, AUD-02]

duration: 17 min
completed: 2026-06-17
---

# Phase 03 Plan 04: Text And Audio Command Summary

**Semantic text, audio, volume, and mute commands persisted in Rust draft contracts**

## Performance

- **Duration:** 17 min
- **Started:** 2026-06-17T07:43:20Z
- **Completed:** 2026-06-17T08:00:03Z
- **Tasks:** 2
- **Files modified:** 16

## Accomplishments

- Added `TextSegment`, `TextStyle`, `TextAlignment`, stroke, shadow, and background semantic types on `Segment.text`.
- Implemented `addTextSegment` and `editTextSegment` with internal text material creation, validation, history, events, generated contracts, and binding JSON smoke coverage.
- Added `SegmentVolume` with integer millivolume semantics plus `addAudioSegment`, `setSegmentVolume`, and `setTrackMute` commands.
- Extended generated command/draft schemas and TypeScript artifacts for text/audio Phase 4 UI consumption.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Semantic text command tests** - `9eb7bb7` (test)
2. **Task 1 GREEN: Semantic text commands** - `ee49f97` (feat)
3. **Task 2 RED: Semantic audio command tests** - `ec18116` (test)
4. **Task 2 GREEN: Semantic audio commands** - `b1e5d39` (feat)

## Files Created/Modified

- `crates/draft_commands/src/text.rs` - Add/edit semantic text segment command implementation.
- `crates/draft_commands/src/audio.rs` - Add audio, set segment volume, and set track mute command implementation.
- `crates/draft_commands/tests/text_audio_commands.rs` - Text/audio command behavior, undo/redo, rejection, and compatibility tests.
- `crates/draft_model/src/timeline.rs` - `Segment.text`, `Segment.volume`, text style structs, and integer segment volume model.
- `crates/draft_model/src/validation.rs` - Text content/style validation and segment volume upper-bound validation.
- `crates/draft_model/src/lib.rs` - Text/audio command names and payload contracts.
- `crates/draft_model/tests/schema_exports.rs`, `schemas/command.schema.json`, `schemas/draft.schema.json`, `apps/desktop-electron/src/generated/CommandEnvelope.ts`, `apps/desktop-electron/src/generated/Draft.ts` - Generated contract updates.
- `crates/bindings_node/src/lib.rs`, `crates/bindings_node/tests/binding_smoke.rs` - Text/audio binding route delegation and JSON smoke coverage.

## Decisions Made

- Text rendering, deterministic layout, pinned fonts, text effects, text bubbles, preview, and export remain out of scope for Phase 3.
- Audio waveform generation, mixing, preview cache invalidation, render graph, and export remain out of scope for Phase 3.
- Volume is represented as integer millivolume instead of floats to preserve the project time/semantic no-float discipline.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added binding smoke tests for text/audio routes**
- **Found during:** Task 1 and Task 2 binding route integration
- **Issue:** The plan required binding routes but listed only binding source files. Without JSON smoke coverage, a raw allow-list omission could pass Rust enum exhaustiveness while failing renderer calls.
- **Fix:** Added `execute_command_routes_timeline_text_segment_events` and `execute_command_routes_timeline_audio_volume_and_mute_events` to `crates/bindings_node/tests/binding_smoke.rs`.
- **Files modified:** `crates/bindings_node/tests/binding_smoke.rs`
- **Verification:** `cargo test -p bindings_node timeline -- --nocapture`
- **Committed in:** `ee49f97`, `b1e5d39`

**Total deviations:** 1 auto-fixed (missing critical test coverage)
**Impact on plan:** Strengthened the planned binding delegation guarantee without changing architecture or product scope.

## Issues Encountered

- Adding optional `Segment.text` and default `Segment.volume` required updating a direct `Segment` struct literal in `crates/draft_model/tests/draft_schema.rs`; serde-compatible JSON fixtures remained backward compatible.

## User Setup Required

None - no external service configuration required.

## Verification

- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - PASS
- `cargo test -p draft_commands text_commands -- --nocapture` - PASS
- `cargo test -p draft_commands audio_commands -- --nocapture` - PASS
- `cargo test -p draft_commands undo_redo -- --nocapture` - PASS
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - PASS
- `cargo test -p bindings_node timeline -- --nocapture` - PASS
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - PASS

## Self-Check: PASSED

- Created files exist: `crates/draft_commands/src/text.rs`, `crates/draft_commands/src/audio.rs`, and `crates/draft_commands/tests/text_audio_commands.rs`.
- Task commits exist: `9eb7bb7`, `ee49f97`, `ec18116`, `b1e5d39`.
- Focused text, audio, undo, binding, schema export, and generated-drift verification passed.
- Unrelated untracked `reference/` was left untouched.

## Next Phase Readiness

Ready for Plan 03-05 to add final command fixtures, source guards, and Phase 3 gates over the accumulated timeline/text/audio command surface.

---
*Phase: 03-timeline-command-core*
*Completed: 2026-06-17*
