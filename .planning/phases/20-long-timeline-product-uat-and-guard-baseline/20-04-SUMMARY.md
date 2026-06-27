---
phase: 20-long-timeline-product-uat-and-guard-baseline
plan: 04
subsystem: testing
tags: [shell, guard, package-scripts, playwright, rust, no-fallback]

requires:
  - phase: 20-01
    provides: Rust-owned Phase 20 fixture constants, `.veproj` materializer, and long-timeline Rust pressure gates.
  - phase: 20-02
    provides: Playwright long-timeline fixture orchestration and canonical/preview/export evidence helpers.
  - phase: 20-03
    provides: Packaged long-session UAT with responsiveness, two reopen/export cycles, and scheduler pressure evidence.
provides:
  - Dedicated Phase 20 source guard requiring Rust materialization, packaged UAT, canonical reopen/export, preview compositor, and export media proof.
  - Shared no-product-fallback guard coverage for the Phase 20 long UAT and evidence helper.
  - Root `test:phase20*` scripts with packaged Electron as the blocking product gate and the 3000 segments-per-track diagnostic excluded from the aggregate.
affects: [phase20-closeout, phase21-readiness, no-product-fallback, packaged-product-uat]

tech-stack:
  added: []
  patterns:
    - Source guards use comment-filtered injected negative self-tests.
    - Aggregate gates compose Rust/testkit, source/no-fallback guards, packaged Playwright UAT, cargo check, and contract diff checks.

key-files:
  created:
    - scripts/phase20-source-guards.sh
  modified:
    - scripts/no-product-fallback-guards.sh
    - package.json

key-decisions:
  - "Keep `scripts/phase20-source-guards.sh` independent from package-script assertions so it can pass before Task 3 wiring and avoid recursion with the shared no-fallback guard."
  - "Keep `test:phase20-diagnostic` available for the 3000 segments-per-track ignored diagnostic but outside the blocking `test:phase20` aggregate."
  - "Make the blocking Phase 20 aggregate require packaged Electron UAT, Rust/testkit fixture proof, no-fallback/source guards, cargo check, and generated-contract consistency."

patterns-established:
  - "Phase 20 source guard: require concrete Rust, helper, and packaged UAT evidence strings and reject TypeScript-owned draft construction, dev-only closeout, mock/fallback evidence, and source-only export success."
  - "Phase 20 shared no-fallback extension: conditionally checks the long UAT spec and evidence helper when present without calling the dedicated Phase 20 source guard."
  - "Phase 20 root aggregate: `test:phase20` packages Electron before Playwright and keeps diagnostics separate from blocking closeout."

requirements-completed: [UAT11-01, UAT11-02, LONG11-01, LONG11-02, GATE11-01]

duration: 16 min
completed: 2026-06-27
status: complete
---

# Phase 20 Plan 04: Source Guards And Aggregate Gate Summary

**Phase 20 closeout guard baseline requiring Rust-owned fixtures, packaged UAT, production preview/export evidence, and no-fallback aggregate wiring**

## Performance

- **Duration:** 16 min
- **Started:** 2026-06-27T19:22:07Z
- **Completed:** 2026-06-27T19:37:29Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- Added `scripts/phase20-source-guards.sh`, a dedicated Phase 20 guard with comment-filtered negative self-tests and required checks for Plans 20-01 through 20-03 artifacts.
- Extended `scripts/no-product-fallback-guards.sh` so the shared product fallback gate checks Phase 20 preview, scheduler, export, evidence bundle, and two-cycle markers when the long UAT spec exists.
- Added root `test:phase20-rust`, `test:phase20-source-guards`, `test:phase20-desktop`, `test:phase20-diagnostic`, and `test:phase20` scripts.
- Verified the full blocking aggregate, including packaged Electron long UAT, cargo check, and contract consistency.

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Phase 20 source guard script** - `5d093a8`
2. **Task 2: Extend the shared no-product-fallback guard for Phase 20** - `7b631d7`
3. **Task 3: Wire Phase 20 aggregate scripts** - `1c2b702`

## Files Created/Modified

- `scripts/phase20-source-guards.sh` - Dedicated Phase 20 source guard for Rust fixture, packaged UAT, canonical, preview, export, and fallback/source-only evidence requirements.
- `scripts/no-product-fallback-guards.sh` - Shared no-product-fallback guard extended with conditional Phase 20 long UAT and evidence helper checks.
- `package.json` - Root Phase 20 script aggregate and non-blocking diagnostic command wiring.

## Decisions Made

- The dedicated Phase 20 source guard does not check root package scripts; Task 3 owns aggregate wiring.
- The shared no-product-fallback guard does not call `scripts/phase20-source-guards.sh`; `test:phase20` runs both guards explicitly.
- `test:phase20-desktop` packages Electron before Playwright, and `test:phase20` excludes the 3000 segments-per-track diagnostic by construction.

## Verification

- `bash scripts/phase20-source-guards.sh` - passed.
- Missing-artifact temp-tree check for `scripts/phase20-source-guards.sh` - passed, exited non-zero on missing Phase 20 artifact.
- `pnpm run test:no-product-fallback` - passed.
- Temp-tree negative check for the shared guard - passed, exited non-zero when the Phase 20 spec lacked `renderGraphGpuComposited`.
- Package script assertion command - passed; verified exact Rust command, packaged desktop order, aggregate composition, and diagnostic exclusion.
- `pnpm run test:phase20-source-guards && pnpm run test:no-product-fallback` - passed.
- `pnpm run test:phase20-rust` - passed, 6 product fixture tests and 1 blocking 1000 segments-per-track boundedness test.
- `pnpm run test:phase20` - passed; packaged long UAT reported `3 passed (6.4m)`, then `cargo check --workspace --locked` and `pnpm run test:contracts` passed.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `pnpm` continued to warn that the current Node version is `v24.15.0` while the repo engine asks for `24.12.0`; all Phase 20 commands passed.
- Rust continued to emit pre-existing warnings in `media_runtime_desktop` and `bindings_node`; no warning was introduced by this plan.
- Electron Builder continued to warn that desktop package description/author/icon metadata is missing; packaging still completed and the packaged UAT passed.

## Known Stubs

None. Stub scan found no placeholder, TODO/FIXME, hardcoded empty UI data, or disconnected mock data in the files changed by this plan.

## Authentication Gates

None.

## Threat Flags

None. This plan added guard scripts and script wiring for the threat surfaces already modeled as `source files -> product success`, `root scripts -> packaged UAT`, and packaged runtime discovery evidence.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 20 now has a blocking closeout command: `pnpm run test:phase20`. Phase 21 can depend on this baseline to prove long-session product truth before adding high-frequency interaction and shortcut hardening.

## Self-Check: PASSED

- Found all key files created or modified by the plan.
- Found all three task commits in git history: `5d093a8`, `7b631d7`, `1c2b702`.
- Verified `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-04-SUMMARY.md` exists on disk.
