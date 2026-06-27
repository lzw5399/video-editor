---
phase: 20-long-timeline-product-uat-and-guard-baseline
plan: 01
subsystem: testing
tags: [rust, testkit, long-timeline, project-store, veproj]

requires:
  - phase: v1.0-production-core
    provides: Rust-owned draft, project_store, render graph, preview cache invalidation, and testkit foundations.
provides:
  - Phase 20 Rust fixture constants and config builders for 180, 1000, and 3000 segments per track.
  - Rust-owned Phase 20 product `.veproj` materializer CLI.
  - Canonical save/reopen, derived-artifact exclusion, and long-timeline structural pressure gates.
affects: [phase20-product-uat, phase20-source-guards, phase20-desktop-evidence]

tech-stack:
  added: []
  patterns:
    - Rust-owned fixture generation through testkit and project_store.
    - Structural long-timeline boundedness gates without wall-clock pass/fail thresholds.

key-files:
  created:
    - crates/testkit/src/bin/phase20_long_fixture.rs
    - crates/testkit/tests/long_timeline_product_fixture.rs
  modified:
    - crates/testkit/Cargo.toml
    - crates/testkit/src/large_timeline.rs
    - crates/testkit/tests/large_timeline_incremental.rs

key-decisions:
  - "Keep the product fixture contiguous at 180 segments per track with 1,000,000 microseconds per segment."
  - "Use pressure-only target stride slack for the 1000 and 3000 segments-per-track Rust gates so localized move assertions can remain overlap-free."
  - "Materialize `.veproj` bundles only through Rust testkit and project_store, then reopen and compare canonical draft equality."

patterns-established:
  - "Phase20ProductMediaUris: product media paths flow into Rust material URIs instead of TypeScript-authored segment semantics."
  - "Phase 20 materializer: write project.json through project_store, reopen it, compare canonical draft equality, and print a compact JSON summary."
  - "Phase 20 pressure tests: graph diff, dirty range, and preview cache invalidation assertions are blocking; 3000 segments per track is ignored diagnostic output."

requirements-completed: [UAT11-01, UAT11-02, LONG11-01]

duration: 12 min
completed: 2026-06-27
status: complete
---

# Phase 20 Plan 01: Rust Long-Timeline Fixture Foundation Summary

**Rust-owned Phase 20 long-timeline fixtures, `.veproj` materialization, and 1000 segments-per-track structural gates**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-27T17:46:36Z
- **Completed:** 2026-06-27T17:59:02Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Added Phase 20 fixture constants and config builders for product, blocking pressure, and diagnostic pressure scales.
- Added `build_phase20_product_timeline` with supplied video/audio material URIs and Rust-owned draft/material/track/segment semantics.
- Added `phase20_long_fixture`, a Rust CLI that writes `.veproj/project.json` through `project_store`, reopens it, compares canonical draft equality, and prints locked-count JSON.
- Added Rust tests for product fixture scale, real media URI use, canonical save/reopen, derived artifact exclusion, 1000 segments-per-track bounded graph/cache behavior, and ignored 3000 segments-per-track diagnostics.

## Task Commits

Each task was committed atomically with TDD RED/GREEN gates:

1. **Task 1: Add Phase 20 Rust fixture contracts**
   - `6c96a8b` test: failing product fixture contract tests
   - `1f44ec7` feat: Phase 20 constants, configs, media URI struct, and product fixture builder
2. **Task 2: Add the Rust `.veproj` materializer CLI**
   - `50b9702` test: failing materializer CLI and canonical bundle tests
   - `4b74cf3` feat: `phase20_long_fixture` binary and `project_store` runtime dependency
3. **Task 3: Add blocking and diagnostic long-timeline Rust gates**
   - `5a7443b` test: failing 1000 segments-per-track pressure gate and ignored 3000 diagnostic
   - `5257e6b` feat: pressure-only target stride slack for bounded localized edits

## Files Created/Modified

- `crates/testkit/src/bin/phase20_long_fixture.rs` - Rust materializer CLI for Phase 20 `.veproj` bundles.
- `crates/testkit/tests/long_timeline_product_fixture.rs` - Product fixture scale, URI, validation, materializer, canonical reopen, and derived-artifact tests.
- `crates/testkit/tests/large_timeline_incremental.rs` - Phase 20 1000-segment blocking gate and ignored 3000-segment diagnostic path.
- `crates/testkit/src/large_timeline.rs` - Phase 20 constants, configs, product media URI builder, and pressure stride behavior.
- `crates/testkit/Cargo.toml` - Moved local `project_store` dependency to runtime dependencies for the binary.

## Decisions Made

- The product UAT fixture remains exactly `180 x 3` with `1_000_000` microseconds per segment and no product-only TypeScript segment construction.
- The 1000/3000 pressure configs use `1_250_000` microsecond target stride with `1_000_000` microsecond segment duration, leaving room for localized move assertions without changing the product fixture scale.
- The materializer treats `project_store` save/open as the canonical boundary and does not write render graphs, preview caches, exports, runtime handles, or temp output paths into `project.json`.

## Verification

- `cargo test -p testkit --test long_timeline_product_fixture -- --nocapture` - passed, 6 tests.
- `cargo test -p testkit --test large_timeline_incremental phase20_blocking_1000_segments_per_track_keeps_localized_diff_bounded -- --nocapture` - passed, 1 test.
- `cargo run -p testkit --bin phase20_long_fixture -- --bundle "$tmp_dir/phase20-long.veproj" --video "$PWD/apps/desktop-electron/tests/fixtures/media/p0-long-av-tone-testsrc.mp4" --audio "$PWD/apps/desktop-electron/tests/fixtures/media/p0-long-tone.wav"` - passed and wrote `project.json`; summary reported 3 tracks, 180 segments per track, 540 total segments, and 180000000 duration microseconds.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected Phase 20 draft title assignment**
- **Found during:** Task 1 GREEN implementation
- **Issue:** The first implementation assigned `fixture.draft.title`, but `Draft` stores the display name in `metadata.name`.
- **Fix:** Updated the product fixture builder to set `fixture.draft.metadata.name`.
- **Files modified:** `crates/testkit/src/large_timeline.rs`
- **Verification:** `cargo test -p testkit --test long_timeline_product_fixture -- --nocapture`
- **Committed in:** `1f44ec7`

---

**Total deviations:** 1 auto-fixed (Rule 1 bug)
**Impact on plan:** No scope change. The fix aligned the implementation with the existing draft model schema.

## Issues Encountered

- A local command invocation attempted `cargo fmt -- crates/testkit/Cargo.toml`; `cargo fmt` treats explicit inputs as Rust files, so this failed without modifying files. The command was rerun against Rust files only.
- Rust emitted an existing deprecation warning in `media_runtime_desktop` for `AVAsset::tracksWithMediaType`; this is pre-existing and outside this plan.

## Known Stubs

- `crates/testkit/tests/long_timeline_product_fixture.rs:171` and `:173` write byte strings named `phase20 video placeholder` and `phase20 audio placeholder` into temporary files. These are intentional test-only file contents used to make absolute material paths exist for `project_store`; they do not flow to UI rendering or product evidence.

## Authentication Gates

None.

## Threat Flags

None. The new local CLI path-input and generated `.veproj/project.json` surfaces were already covered by T20-01 and T20-02 in the plan threat model.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 20-02 can consume the Rust materializer to generate product long-timeline bundles for Playwright helpers and evidence collection. The ignored 3000 segments-per-track diagnostic remains available via:

`cargo test -p testkit --test large_timeline_incremental phase20_diagnostic_3000_segments_per_track_reports_structural_stats -- --ignored --nocapture`

## Self-Check

PASSED.

- Found all key files created or modified by the plan.
- Found all six task commits in git history: `6c96a8b`, `1f44ec7`, `50b9702`, `4b74cf3`, `5a7443b`, `5257e6b`.

---
*Phase: 20-long-timeline-product-uat-and-guard-baseline*
*Completed: 2026-06-27*
