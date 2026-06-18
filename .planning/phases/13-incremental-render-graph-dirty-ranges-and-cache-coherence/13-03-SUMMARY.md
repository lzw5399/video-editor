---
phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence
plan: 03
subsystem: command-delta
tags: [rust, draft-commands, preview-service, dirty-ranges, undo-redo]

requires:
  - phase: 13-02
    provides: CommandDelta core types and simple timeline command delta emission
provides:
  - domain-aware deltas for text, subtitle, audio, visual, keyframe, canvas, track mute, and undo/redo commands
  - material dependency dirty expansion that maps material IDs to dependent segment ranges or material-wide fallback scope
  - deterministic consumer-domain expansion for Phase 13 preview/export/audio/thumb/wave/proxy/snapshot/cache targets
  - restored-draft undo/redo invalidation deltas with targeted ranges when semantic segment changes can be identified
affects: [phase-13, draft_commands, preview_service, render-graph-cache-coherence]

tech-stack:
  added: []
  patterns:
    - Rust-owned semantic delta builders in draft_commands
    - Preview-service consumer-domain expansion from DirtyDomain causes
    - Conservative full-draft invalidation for canvas/output-profile changes

key-files:
  created:
    - .planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-03-SUMMARY.md
  modified:
    - crates/draft_commands/src/delta.rs
    - crates/draft_commands/src/text.rs
    - crates/draft_commands/src/audio.rs
    - crates/draft_commands/src/visual.rs
    - crates/draft_commands/src/keyframe.rs
    - crates/draft_commands/src/canvas.rs
    - crates/draft_commands/src/history.rs
    - crates/draft_commands/tests/command_delta.rs
    - crates/preview_service/src/cache.rs
    - crates/preview_service/src/lib.rs
    - crates/preview_service/tests/dirty_propagation.rs

key-decisions:
  - "Command-specific edit modules emit targeted semantic deltas after accepted validation using Rust-owned segment and track state."
  - "Canvas/output-profile edits use explicit full-draft invalidation with a draft-duration range because output profile changes affect all derived consumers."
  - "Undo/redo compares the current draft with the restored snapshot and emits targeted segment ranges when identifiable, falling back to full-draft invalidation otherwise."

patterns-established:
  - "Delta builders stay in draft_commands and return semantic facts only; no preview runtime, FFmpeg, filesystem, SQLite, scheduler, or renderer decisions enter command crates."
  - "Consumer-domain expansion uses a stable Phase 13 order: preview, export prep, audio, thumbnail, waveform, proxy, graph snapshot, preview cache."

requirements-completed: [INCR-02, INCR-03, INCR-04]

duration: 9 min
completed: 2026-06-18T21:54:40Z
---

# Phase 13 Plan 03: Text/Audio/Visual/Canvas Dirty Delta Coverage Summary

**Accepted semantic edit commands now emit domain-aware dirty facts, and undo/redo restores snapshots with deterministic invalidation ranges before any derived cache reuse is possible.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-18T21:45:58Z
- **Completed:** 2026-06-18T21:54:40Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- Added targeted deltas for text, subtitle import, audio segment, volume, track mute, visual, keyframe, and canvas/profile commands.
- Added material dependency dirty expansion for material-scoped changes, including dependent segment ranges and conservative material-wide fallback.
- Added consumer-domain expansion covering preview, export prep, audio, thumbnail, waveform, proxy, graph snapshot, and preview cache consumers.
- Updated undo/redo to compare current and restored draft snapshots, returning targeted previous/current ranges when segment-level changes are identifiable.

## Task Commits

1. **Task 13-03-01 RED:** `f22eb22` test - failing text/audio delta coverage.
2. **Task 13-03-01 GREEN:** `e5f0d25` feat - text, subtitle, audio, volume, and track mute deltas.
3. **Task 13-03-02 RED:** `5367129` test - failing visual/canvas/consumer expansion coverage.
4. **Task 13-03-02 GREEN:** `afa73c4` feat - visual, keyframe, canvas, and consumer expansion deltas.
5. **Task 13-03-03 RED:** `2bfdacf` test - failing undo/redo restored-range coverage.
6. **Task 13-03-03 GREEN:** `a35c91b` feat - deterministic undo/redo restored-draft deltas.
7. **Acceptance fix:** `049a242` fix - material dependency range helper and material-wide fallback coverage.

## Files Created/Modified

- `crates/draft_commands/src/delta.rs` - Added domain-specific delta builders, material dependency expansion, consumer expansion, canvas full-draft helper, and restored-draft diff helper.
- `crates/draft_commands/src/text.rs` - Text and subtitle commands now attach targeted text deltas.
- `crates/draft_commands/src/audio.rs` - Audio add, volume, and track mute commands now attach targeted audio deltas.
- `crates/draft_commands/src/visual.rs` - Visual updates now attach segment-scoped visual deltas.
- `crates/draft_commands/src/keyframe.rs` - Keyframe set/remove now attach property-aware segment deltas.
- `crates/draft_commands/src/canvas.rs` - Canvas config updates now attach full-draft output-profile deltas.
- `crates/draft_commands/src/history.rs` - Undo/redo now emit restored snapshot invalidation deltas.
- `crates/draft_commands/tests/command_delta.rs` - Added domain coverage and undo/redo delta tests.
- `crates/preview_service/src/cache.rs` - Added semantic DirtyDomain to consumer-domain expansion.
- `crates/preview_service/src/lib.rs` - Exported the consumer-domain expansion helper.
- `crates/preview_service/tests/dirty_propagation.rs` - Added consumer-domain expansion expectations.

## Decisions Made

- Used full-draft invalidation for canvas/profile changes because output dimensions, frame rate, and background can affect all derived artifacts.
- Kept undo/redo graph snapshot reuse out of scope; correctness is handled by deterministic invalidation unless later phases prove exact fingerprint matches.
- Kept expansion helpers data-only and in Rust services; renderer remains a command/transport UI surface.

## Deviations from Plan

- The orchestrator found one missing acceptance item after subagent completion: material dependency expansion existed only as preview cache material-id invalidation, not as a draft semantic delta helper. Added `049a242` to close that gap before marking the plan complete.

## Issues Encountered

- A workspace-wide `cargo fmt` run formatted unrelated files outside the plan. Those specific unintended edits were reverted before subsequent commits; no unrelated files were committed.

## Known Stubs

None.

## Threat Flags

None.

## Verification

- `cargo test -p draft_commands --test command_delta text_audio_delta -- --nocapture` - passed
- `cargo test -p draft_commands text -- --nocapture` - passed
- `cargo test -p draft_commands audio -- --nocapture` - passed
- `cargo test -p draft_commands keyframe -- --nocapture` - passed
- `cargo test -p draft_commands canvas -- --nocapture` - passed
- `cargo test -p preview_service --test dirty_propagation consumer_domain_expansion -- --nocapture` - passed
- `cargo test -p draft_commands --test command_delta undo_redo_delta -- --nocapture` - passed
- `cargo test -p draft_commands --test command_delta material_dependency_delta -- --nocapture` - passed
- `cargo test -p draft_commands undo_redo -- --nocapture` - passed
- `pnpm run test:phase13-source-guards` - passed
- `cargo test -p draft_commands --test command_delta -- --nocapture` - passed
- `cargo test -p preview_service --test dirty_propagation -- --nocapture` - passed
- `pnpm run test:phase13` - passed
- `cargo check --workspace --locked` - passed
- `git diff --check` - passed

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 13-04 can consume the richer semantic deltas for stable render graph node identity, fingerprints, graph snapshots, and graph diff helpers.

## Self-Check: PASSED

- Summary file exists.
- Task commits exist: `f22eb22`, `e5f0d25`, `5367129`, `afa73c4`, `2bfdacf`, `a35c91b`, `049a242`.
- Required verification commands passed.

---
*Phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence*
*Completed: 2026-06-18T21:54:40Z*
