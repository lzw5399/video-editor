---
phase: 03-timeline-command-core
plan: 01
subsystem: timeline-command-core
tags: [rust, draft_commands, draft_model, timeline, command-contracts]

requires:
  - phase: 02-draft-and-material-system
    provides: draft/material/track/segment schema, validation, and generated contract pattern
provides:
  - Pure draft_commands crate wiring with draft_model dependency only
  - Session-only timeline command contracts and generated schema/TypeScript exports
  - Timeline validation helpers for checked timeranges, track/material compatibility, overlap, locked tracks, and material-duration bounds
affects: [phase-03-timeline-command-core, phase-04-desktop-workspace, command-contracts]

tech-stack:
  added: []
  patterns:
    - Rust-owned command session contracts generated through schema_exports.rs
    - Pure draft_commands helpers layered on draft_model validate_draft
    - Checked integer microsecond timerange helpers

key-files:
  created:
    - crates/draft_commands/src/error.rs
    - crates/draft_commands/src/selection.rs
    - crates/draft_commands/src/timeline.rs
    - crates/draft_commands/tests/timeline_model.rs
  modified:
    - Cargo.lock
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
    - crates/draft_commands/Cargo.toml
    - crates/draft_commands/src/lib.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/command.schema.json

key-decisions:
  - "CommandState, CommandHistorySnapshot, TimelineSelection, SnappingSettings, and TimelineCommandResponse are session contracts in draft_model, not persisted Draft fields."
  - "draft_commands depends only on draft_model and re-exports TimelineSelection instead of adding serialization/schema dependencies."
  - "validate_timeline_rules runs command-specific timerange, compatibility, material-bound, and overlap checks before delegating to validate_draft."

patterns-established:
  - "Standalone command response/session schemas are merged into command.schema.json from Rust schema_for outputs."
  - "Track order helpers return semantic TrackId order for visual stacking, audio mixing, and first-video main track selection."
  - "TimelineCommandError carries exact TimelineCommandErrorKind values for invalid edit rejection tests."

requirements-completed: [TIME-01, TIME-06, TIME-07]

duration: 15 min
completed: 2026-06-17
---

# Phase 03 Plan 01: Timeline Command Foundation Summary

**Rust-owned timeline command contracts plus pure validation helpers for root tracks, checked timeranges, overlap rejection, locked tracks, and material compatibility**

## Performance

- **Duration:** 15 min
- **Started:** 2026-06-17T06:14:28Z
- **Completed:** 2026-06-17T06:30:02Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments

- Added session-only command contracts for selection, snapping settings, bounded history snapshots, command state, and timeline command responses.
- Generated updated command schema and TypeScript contracts from Rust, including the new command/session types.
- Implemented pure `draft_commands` timeline validation helpers for root `Draft.tracks`, track order, checked integer microsecond range ends, same-track overlap rejection, locked-track rejection, track/material compatibility, and material-duration bounds.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Base contract export test** - `cb1e33a` (test)
2. **Task 1 GREEN: Timeline command base contracts** - `8580a72` (feat)
3. **Task 2 RED: Timeline validation tests** - `a97aacd` (test)
4. **Task 2 GREEN: Timeline validation rules** - `796db1f` (feat)

## Files Created/Modified

- `crates/draft_commands/src/error.rs` - Structured `TimelineCommandError` and `TimelineCommandErrorKind`.
- `crates/draft_commands/src/selection.rs` - Pure re-export of the Rust-owned `TimelineSelection` contract.
- `crates/draft_commands/src/timeline.rs` - Checked timerange, compatibility, overlap, locked-track, material-bound, stacking, and main-video helpers.
- `crates/draft_commands/tests/timeline_model.rs` - `timeline_tracks`, `track_rules`, and `timerange_rules` coverage.
- `crates/draft_model/src/lib.rs` - Session-only command state, history, selection, snapping, and response contracts.
- `crates/draft_model/tests/schema_exports.rs` - Generated contract export coverage for new command/session types.
- `schemas/command.schema.json` - Rust-generated command contract schema with standalone command/session definitions.
- `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Rust-generated command/session request-side TypeScript contracts.
- `apps/desktop-electron/src/generated/CommandResultEnvelope.ts` - Rust-generated timeline command response TypeScript contract.
- `crates/draft_commands/Cargo.toml`, `crates/draft_commands/src/lib.rs`, `Cargo.lock` - Pure local dependency and module wiring.

## Decisions Made

- Kept command history/session state out of `Draft` and `.veproj/project.json`.
- Kept `draft_commands` dependency scope to `draft_model` only.
- Modeled overlap and material-duration rejection as command-level validation, while still delegating persisted draft invariants to `validate_draft`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Preserved frame-rate schema constraints after adding standalone command contract schemas**
- **Found during:** Task 1 (Base contract generation)
- **Issue:** Merging standalone schemas for `TimelineCommandResponse` and session types initially reintroduced unconstrained nested `RationalFrameRate` definitions after the existing zero-frame-rate patch.
- **Fix:** Moved draft schema-version and rational-frame-rate constraint patches after all standalone command contract schemas are merged.
- **Files modified:** `crates/draft_model/tests/schema_exports.rs`
- **Verification:** `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture`; `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture`
- **Committed in:** `8580a72`

**2. [Rule 1 - Bug] Isolated same-track overlap test fixture from material-duration overrun**
- **Found during:** Task 2 (Timeline validation rules)
- **Issue:** The RED overlap fixture also exceeded the source material duration, so the material-bound guard fired before overlap validation.
- **Fix:** Adjusted the second segment's source range to stay within the known material duration while preserving target overlap.
- **Files modified:** `crates/draft_commands/tests/timeline_model.rs`
- **Verification:** `cargo test -p draft_commands track_rules -- --nocapture`
- **Committed in:** `796db1f`

**Total deviations:** 2 auto-fixed (2 bug fixes)
**Impact on plan:** Both fixes preserved planned behavior and test precision. No scope expansion.

## Issues Encountered

- `git diff --exit-code schemas apps/desktop-electron/src/generated` was run after staging generated artifacts, because the plan also required committing Rust-generated schema/TypeScript updates. The staged worktree check passed and the post-commit plan-level drift check passed.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Stub scan only matched existing `#[ts(optional = nullable)]` annotations.

## Verification

- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - PASS
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - PASS
- `cargo check -p draft_commands --locked` - PASS
- `cargo test -p draft_commands timeline_tracks -- --nocapture` - PASS
- `cargo test -p draft_commands track_rules -- --nocapture` - PASS
- `cargo test -p draft_commands timerange_rules -- --nocapture` - PASS
- `cargo test -p draft_commands -- --nocapture` - PASS
- `bash -lc 'set -euo pipefail; ! rg -n "FfmpegExecutor|PlatformFileSystem|PreviewRenderer|ffmpeg|ffprobe|std::fs|std::process|napi|electron" crates/draft_commands/src crates/draft_commands/tests'` - PASS
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - PASS

## Self-Check: PASSED

- Created files exist: `crates/draft_commands/src/error.rs`, `crates/draft_commands/src/selection.rs`, `crates/draft_commands/src/timeline.rs`, `crates/draft_commands/tests/timeline_model.rs`.
- Task commits exist: `cb1e33a`, `8580a72`, `a97aacd`, `796db1f`.
- Plan-level verification passed after all task commits.
- Unrelated untracked `reference/` was left untouched.

## Next Phase Readiness

Ready for Plan 03-02 to implement add, move, split, trim, delete, select, and invalid-edit rejection commands on top of these validation helpers.

---
*Phase: 03-timeline-command-core*
*Completed: 2026-06-17*
