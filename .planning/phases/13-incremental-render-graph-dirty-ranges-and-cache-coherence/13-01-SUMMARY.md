---
phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence
plan: 01
subsystem: testing
tags: [rust, render-graph, dirty-ranges, cache-coherence, testkit]

requires:
  - phase: 12-media-io-hardware-decode-and-frame-texture-interop
    provides: media IO and frame/texture interop contracts for downstream incremental cache work
provides:
  - Phase 13 source guard and package gates
  - Command delta, graph identity, and dirty propagation Rust test targets
  - Deterministic large-timeline fixture helpers with stable IDs and localized edit coordinates
affects: [draft_model, draft_commands, render_graph, preview_service, testkit, phase13]

tech-stack:
  added: []
  patterns:
    - Harness-first Phase 13 gates before behavior implementation
    - Bounded deterministic large-timeline fixtures using integer microseconds

key-files:
  created:
    - scripts/phase13-source-guards.sh
    - crates/draft_commands/tests/command_delta.rs
    - crates/render_graph/tests/node_identity.rs
    - crates/preview_service/tests/dirty_propagation.rs
    - crates/testkit/src/large_timeline.rs
    - crates/testkit/tests/large_timeline_incremental.rs
  modified:
    - package.json
    - crates/draft_model/tests/schema_exports.rs
    - crates/draft_model/tests/contract.rs
    - crates/testkit/src/lib.rs

key-decisions:
  - "Phase 13 gates are harness-first: current tests pass while reserving assertion locations for later CommandDelta, node identity, and cache invalidation behavior."
  - "Large-timeline fixtures stay Rust-only and do not introduce FFmpeg, SQLite artifact store, Electron, or scheduler dependencies."

patterns-established:
  - "Phase 13 source guard blocks renderer-owned dirty range, graph diff, cache key, invalidation, and FFmpeg command decisions."
  - "testkit::large_timeline builds deterministic material-backed drafts with stable IDs, checked integer target ranges, and bounded segment counts."

requirements-completed: [INCR-01, INCR-02, INCR-03, INCR-04, INCR-05]

duration: 10min
completed: 2026-06-18
---

# Phase 13 Plan 01: Validation Harness, Source Guards, And Large-Timeline Fixtures Summary

**Executable Phase 13 harness with Rust-owned guard rails, focused test targets, and deterministic large-timeline fixture generation.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-18T21:12:16Z
- **Completed:** 2026-06-18T21:22:15Z
- **Tasks:** 3
- **Files modified:** 10

## Accomplishments

- Added `scripts/phase13-source-guards.sh` plus root `test:phase13-rust`, `test:phase13-source-guards`, and `test:phase13` scripts.
- Added focused Rust test targets for command delta anchors, graph semantic identity anchors, preview dirty propagation, and schema/contract harness assertions.
- Added `testkit::large_timeline` for bounded deterministic drafts with video/audio/text material-backed segments, stable IDs, localized edit targets, and no-overlap validation.

## Task Commits

1. **Task 13-01-01: Add Phase 13 source guards and package gates** - `4c5e67f` (chore)
2. **Task 13-01-02: Add contract and behavior test targets** - `d75bfe8` (test)
3. **Task 13-01-03: Add deterministic large-timeline fixtures** - `f510f87` (test)

## Files Created/Modified

- `package.json` - Added Phase 13 Rust, source guard, and aggregate test scripts.
- `scripts/phase13-source-guards.sh` - Added renderer/cache/dirty/FFmpeg boundary, float-time, derived-artifact, and Phase 14/16 scope guards.
- `crates/draft_model/tests/schema_exports.rs` - Added Phase 13 schema/export harness anchors.
- `crates/draft_model/tests/contract.rs` - Added timeline response serialization and half-open integer range contract anchors.
- `crates/draft_commands/tests/command_delta.rs` - Added command delta assertion target for accepted edit ranges and selection-only no-op semantics.
- `crates/render_graph/tests/node_identity.rs` - Added semantic graph identity anchor tests.
- `crates/preview_service/tests/dirty_propagation.rs` - Added dirty propagation and retention target tests.
- `crates/testkit/src/lib.rs` - Exported the `large_timeline` module.
- `crates/testkit/src/large_timeline.rs` - Added deterministic large-timeline fixture builder.
- `crates/testkit/tests/large_timeline_incremental.rs` - Added fixture determinism, stable ID, track mix, and bounds tests.

## Verification

- `bash -n scripts/phase13-source-guards.sh` - PASS
- package script presence check for `test:phase13-rust`, `test:phase13-source-guards`, `test:phase13` - PASS
- `bash scripts/phase13-source-guards.sh` - PASS
- `cargo test -p draft_model contract -- --nocapture` - PASS
- `cargo test -p draft_commands --test command_delta -- --nocapture` - PASS
- `cargo test -p render_graph --test node_identity -- --nocapture` - PASS
- `cargo test -p preview_service --test dirty_propagation -- --nocapture` - PASS
- `cargo test -p testkit large_timeline -- --nocapture` - PASS
- `cargo test -p draft_model draft_fixtures -- --nocapture` - PASS

## Decisions Made

- Kept Phase 13 tests passing against current contracts while making the target files the required landing zones for downstream behavior assertions.
- Kept source guards executable now by checking current ownership boundaries and test target presence, without requiring future `CommandDelta` types before Plan 13-02.
- Used bounded fixture construction instead of runtime media generation so large-timeline tests do not depend on FFmpeg, Electron, SQLite, or scheduler work.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected contract test JSON expectation**
- **Found during:** Task 13-01-02
- **Issue:** The new timeline response contract test expected an outdated canvas aspect-ratio JSON shape.
- **Fix:** Updated the expected JSON to the current `{ kind: "preset", preset: "ratio16x9" }` contract.
- **Files modified:** `crates/draft_model/tests/contract.rs`
- **Verification:** `cargo test -p draft_model contract -- --nocapture`
- **Committed in:** `d75bfe8`

**2. [Rule 1 - Bug] Removed undeclared serde_json usage from draft_commands test**
- **Found during:** Task 13-01-02
- **Issue:** `crates/draft_commands` does not depend on `serde_json`, and package installs/lockfile edits are disallowed.
- **Fix:** Reworked the test to assert Rust-owned response fields directly.
- **Files modified:** `crates/draft_commands/tests/command_delta.rs`
- **Verification:** `cargo test -p draft_commands --test command_delta -- --nocapture`
- **Committed in:** `d75bfe8`

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both fixes kept the harness within existing dependency and contract boundaries. No scope expansion.

## Issues Encountered

- `cargo fmt --all` touched unrelated Phase 12 files with rustfmt-only changes. Those unstaged formatting changes were reverted file-by-file before continuing; no unrelated files were committed.

## Known Stubs

None - this plan intentionally adds harness targets and fixture helpers, not placeholder production behavior.

## Threat Flags

None - no new network endpoints, auth paths, filesystem persistence surfaces, or trust-boundary schema changes were introduced.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 13-02 can add concrete `CommandDelta` types and range helpers into the established `draft_model`/`draft_commands` test targets. Plan 13-04 and 13-05 have graph identity and dirty propagation targets ready for stronger behavior assertions.

## Self-Check: PASSED

- Created files exist on disk.
- Task commits `4c5e67f`, `d75bfe8`, and `f510f87` exist in git history.
- Worktree contains only this summary plus the pre-existing untracked `reference/` item before the summary commit.

---
*Phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence*
*Completed: 2026-06-18*
