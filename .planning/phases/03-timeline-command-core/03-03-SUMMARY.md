---
phase: 03-timeline-command-core
plan: 03
subsystem: timeline-command-core
tags: [rust, draft_commands, history, snapping, bindings_node, command-events]

requires:
  - phase: 03-timeline-command-core
    provides: pure timeline edit commands, command state, selection state, and binding route pattern
provides:
  - Bounded session-only snapshot undo/redo for committed timeline edits
  - Rust-owned snapping and first-video-track MainTrackMagnet behavior
  - Observable snapped, mainTrackMagnetApplied, undoCommitted, redoCommitted, and historyLimitPruned events
  - Binding routes and smoke coverage for eventful undo/redo command responses
affects: [phase-03-timeline-command-core, phase-04-desktop-workspace, command-contracts]

tech-stack:
  added: []
  patterns:
    - Session-only CommandState carries bounded undo/redo snapshots through command envelopes
    - Snapping and MainTrackMagnet are computed inside draft_commands before validation and commit
    - bindings_node delegates eventful timeline commands to draft_commands without interpreting history or snap candidates

key-files:
  created:
    - crates/draft_commands/src/history.rs
    - crates/draft_commands/src/snapping.rs
    - crates/draft_commands/tests/timeline_history_snapping.rs
  modified:
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/tests/binding_smoke.rs
    - crates/draft_commands/src/lib.rs
    - crates/draft_commands/src/timeline.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/command.schema.json

key-decisions:
  - "03-03 uses bounded snapshot history with DEFAULT_HISTORY_LIMIT = 100 and keeps undo/redo out of persisted .veproj/project.json state."
  - "Rust-owned snapping uses DEFAULT_SNAP_THRESHOLD_US = 100_000 and payload-level SnappingSettings overrides; the UI receives events instead of recomputing candidates."
  - "MainTrackMagnet applies only to the first video track in Phase 3 and emits mainTrackMagnetApplied when it changes segment timing."
  - "bindings_node routes undoTimelineEdit and redoTimelineEdit through draft_commands and preserves TimelineCommandResponse.events under data.events."

patterns-established:
  - "Committed edit commands push pre-edit snapshots only after command validation and validate_draft pass."
  - "Rejected timeline edits return errors without changing Draft, TimelineSelection, undo_stack, or redo_stack."
  - "Event order for edit responses is primary edit event, semantic adjustment events, then history maintenance events."

requirements-completed: [TIME-04, TIME-05, TIME-06]

duration: 18 min
completed: 2026-06-17
---

# Phase 03 Plan 03: Timeline History And Snapping Summary

**Bounded Rust undo/redo, snapping, and main-track magnet events routed through generated command envelopes**

## Performance

- **Duration:** 18 min
- **Started:** 2026-06-17T07:16:07Z
- **Completed:** 2026-06-17T07:34:49Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- Added bounded session-only undo/redo snapshots with redo clearing, history pruning, empty-history errors, and invalid-edit atomicity tests.
- Implemented Rust-owned snapping and first-video-track MainTrackMagnet helpers using integer microseconds and deterministic command events.
- Routed undo/redo through `bindings_node::execute_command` and added JSON-envelope tests proving snapped move, undo, redo, command state, and events are preserved.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: History command tests** - `9168fd5` (test)
2. **Task 1 GREEN: Bounded timeline history** - `c2fad65` (feat)
3. **Task 2 RED: Snapping and magnet tests** - `bd1832a` (test)
4. **Task 2 GREEN: Snapping and MainTrackMagnet** - `ec08791` (feat)
5. **Task 3 RED: Binding undo/redo event tests** - `7139e4b` (test)
6. **Task 3 GREEN: Binding undo/redo routes** - `fcabca1` (feat)

## Files Created/Modified

- `crates/draft_commands/src/history.rs` - Bounded snapshot undo/redo, redo clearing, pruning, and history events.
- `crates/draft_commands/src/snapping.rs` - Snap candidate calculation, trim-boundary snapping, and first-video-track MainTrackMagnet helpers.
- `crates/draft_commands/src/timeline.rs` - History commit integration, snapped move/trim target ranges, magnet application, and extra command events.
- `crates/draft_commands/tests/timeline_history_snapping.rs` - Undo/redo, invalid edit atomicity, snapping, override threshold, trim snap, and magnet coverage.
- `crates/draft_model/src/lib.rs` - Undo/redo payloads, command state/history/session contracts, and 100 ms default snap threshold.
- `crates/draft_model/tests/schema_exports.rs`, `schemas/command.schema.json`, `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Generated contract coverage for history and undo/redo payloads.
- `crates/bindings_node/src/lib.rs` - Binding route allow-list and command dispatch for undo/redo.
- `crates/bindings_node/tests/binding_smoke.rs` - JSON-envelope test for snapped move, undo, redo, command state, and events.

## Decisions Made

- Snapshot history stays bounded by `DEFAULT_HISTORY_LIMIT = 100`; replacing it with inverse operations is deferred until there is real pressure to optimize memory or diff size.
- `SnappingSettings::DEFAULT_THRESHOLD` and `DEFAULT_SNAP_THRESHOLD_US` are both 100,000 microseconds so Rust model defaults and command implementation agree.
- Right trim may extend a segment when the resulting source range remains valid; material duration validation remains the source-of-truth guard.
- Top-level `CommandResultEnvelope.events` remains empty for timeline commands; eventful timeline semantics are preserved in `TimelineCommandResponse.events` under `data.events`, matching the existing binding envelope pattern.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Verification

- `cargo test -p draft_commands undo_redo -- --nocapture` - PASS
- `cargo test -p draft_commands snapping -- --nocapture` - PASS
- `cargo test -p draft_commands main_track_magnet -- --nocapture` - PASS
- `cargo test -p draft_commands -- --nocapture` - PASS
- `bash -lc 'set -euo pipefail; ! rg -n "durationSeconds|duration_seconds|seconds: f32|seconds: f64|\\bf32\\b|\\bf64\\b" crates/draft_commands/src crates/draft_model/src schemas/command.schema.json apps/desktop-electron/src/generated/CommandEnvelope.ts'` - PASS
- `cargo test -p bindings_node timeline -- --nocapture` - PASS
- `cargo test -p bindings_node -- --nocapture` - PASS
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - PASS
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - PASS

## Self-Check: PASSED

- Created files exist: `crates/draft_commands/src/history.rs`, `crates/draft_commands/src/snapping.rs`, and `crates/draft_commands/tests/timeline_history_snapping.rs`.
- Task commits exist: `9168fd5`, `c2fad65`, `bd1832a`, `ec08791`, `7139e4b`, `fcabca1`.
- Focused command, snapping, magnet, binding, schema export, and generated-drift verification passed.
- Unrelated untracked `reference/` was left untouched.

## Next Phase Readiness

Ready for Plan 03-04 to add MVP text and audio semantic commands on top of the same Rust-owned edit, history, event, and binding route patterns. TIME-04 currently applies to the committed core edit commands from 03-02; 03-04 should reuse the same history path for text/audio command coverage.

---
*Phase: 03-timeline-command-core*
*Completed: 2026-06-17*
