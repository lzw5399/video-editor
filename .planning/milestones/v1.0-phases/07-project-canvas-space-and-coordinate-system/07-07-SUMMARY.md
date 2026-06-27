---
phase: 07-project-canvas-space-and-coordinate-system
plan: 07
subsystem: testing
tags: [source-guards, phase-gates, canvas, playwright, contracts]
requires:
  - phase: 07-project-canvas-space-and-coordinate-system
    provides: Draft canvas schema, commands, preview/export propagation, and desktop canvas UI from Plans 07-01 through 07-06
provides:
  - Phase 07 source guards for canvas ownership, terminology, generated contracts, coordinate docs, Chinese UI copy, and renderer boundary rules
  - Root `pnpm run test:phase7` and `just test` wiring for the Phase 07 executable gate
  - Cross-phase guard alignment for Phase 7 canvas coordinates and command fixtures
affects: [phase-07, phase-08, testing, desktop-ui]
tech-stack:
  added: []
  patterns: [phase-specific source guard script, root phase gate chaining, generated contract drift gate]
key-files:
  created:
    - scripts/phase7-source-guards.sh
  modified:
    - package.json
    - justfile
    - scripts/phase5-source-guards.sh
    - apps/desktop-electron/tests/workspace.spec.ts
    - fixtures/draft/minimal-timeline-command.json
    - fixtures/draft/invalid-timeline-command.json
key-decisions:
  - "Phase 07 verification is exposed as `pnpm run test:phase7` and included in root `pnpm run test` plus `/Users/zhiwen/.cargo/bin/just test`."
  - "Phase 07 guards block renderer-owned canvas mutation, hard-coded production canvas profiles, missing generated canvas contracts, missing coordinate docs, missing Chinese canvas copy, and forbidden alternate terms."
  - "Existing command fixtures must include required draft `canvasConfig` so schema tests fail for the intended command-contract reason instead of obsolete draft shape."
patterns-established:
  - "Phase guards should filter or scope historical invariants when later phases intentionally add non-time semantic types such as canvas coordinate floats."
  - "Playwright assertions for repeated editor status text should scope to a stable region or accessible label to avoid strict-locator ambiguity."
requirements-completed: [CANVAS-01, CANVAS-02, CANVAS-03, CANVAS-04]
duration: 12 min
completed: 2026-06-18
---

# Phase 07 Plan 07: Source Guards And Root Gates Summary

**Phase 07 now has executable root gates that lock Rust-owned canvas semantics, Chinese desktop copy, and generated contract drift**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-18T00:51:32Z
- **Completed:** 2026-06-18T01:03:51Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added `scripts/phase7-source-guards.sh` for generated canvas contracts, coordinate docs, Jianying terminology, renderer ownership boundaries, production preview/export canvas profile ownership, Chinese UI copy, and no package churn.
- Added root Phase 07 scripts and chained `pnpm run test:phase7` into both root `pnpm run test` and `/Users/zhiwen/.cargo/bin/just test`.
- Fixed gate fallout uncovered by the final test loop: command fixtures now include required `canvasConfig`, the canvas Playwright readout assertion is region-scoped, and the Phase 5 float-time guard no longer rejects Phase 7 canvas coordinate helpers.

## Task Commits

Each task was committed atomically:

1. **Task 07-07-01: Add Phase 07 source guards** - `37176df` (test)
2. **Task 07-07-02: Wire Phase 07 root pnpm and just gates** - `c648bdd` (test)
3. **Gate repair: Canvas gate regressions** - `68e9990` (test)

## Files Created/Modified

- `scripts/phase7-source-guards.sh` - Enforces Phase 07 canvas contract, UI copy, renderer-boundary, coordinate-doc, terminology, and no-package invariants.
- `package.json` - Adds `test:phase7-rust`, `test:phase7-source-guards`, `test:phase7-workspace`, and `test:phase7`, then includes Phase 07 in root `test`.
- `justfile` - Adds `pnpm run test:phase7` to the public `just test` gate.
- `scripts/phase5-source-guards.sh` - Narrows the naked-float-time guard so it still catches time floats while allowing Phase 7 canvas coordinate conversion in `draft_model/src/canvas.rs`.
- `fixtures/draft/minimal-timeline-command.json` - Adds required default `canvasConfig` to the embedded draft command fixture.
- `fixtures/draft/invalid-timeline-command.json` - Adds required default `canvasConfig` so the negative fixture remains negative for command payload shape, not missing draft fields.
- `apps/desktop-electron/tests/workspace.spec.ts` - Scopes the canvas readout assertion to `预览窗口` to avoid strict-locator collisions with duplicate status text.

## Decisions Made

- Kept Phase 07 gate wiring dependency-free; no packages or lockfile changes were introduced.
- Kept renderer canvas ownership checks in Phase 07 source guards while allowing generated command helper construction.
- Treated the Phase 5 guard false positive as a cross-phase invariant update rather than changing Phase 7 canvas coordinate semantics.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated command fixtures for required `canvasConfig`**
- **Found during:** Final `pnpm run test`
- **Issue:** `schema_fixtures_validate_command_contracts` failed because the positive timeline command fixture embedded a draft without the new required `canvasConfig`.
- **Fix:** Added the default 16:9 black canvas config to both positive and negative timeline command fixtures so each fixture exercises its intended command-contract path.
- **Files modified:** `fixtures/draft/minimal-timeline-command.json`, `fixtures/draft/invalid-timeline-command.json`
- **Verification:** `cargo test -p draft_model schema_fixtures_validate_command_contracts -- --nocapture`, `pnpm run test:phase7`, `pnpm run test`, `/Users/zhiwen/.cargo/bin/just test`
- **Committed in:** `68e9990`

**2. [Rule 3 - Blocking] Scoped duplicated canvas readout assertion**
- **Found during:** `pnpm run test:phase7`
- **Issue:** The expected readout text appeared in the preview status, preview chip, and inspector readout; Playwright strict mode rejected the broad locator.
- **Fix:** Scoped the assertion to `预览窗口` and exact text so the test verifies the preview readout without depending on global text uniqueness.
- **Files modified:** `apps/desktop-electron/tests/workspace.spec.ts`
- **Verification:** `pnpm run test:phase7`, `pnpm run test`, `/Users/zhiwen/.cargo/bin/just test`
- **Committed in:** `68e9990`

**3. [Rule 3 - Blocking] Narrowed Phase 5 float-time guard for canvas coordinates**
- **Found during:** Final `pnpm run test`
- **Issue:** Phase 5's historical naked-floating-time guard matched Phase 7 canvas coordinate conversion `f64` values in `draft_model/src/canvas.rs`.
- **Fix:** Filtered `canvas.rs` out of that specific time guard while leaving generated contracts and all other draft model paths covered.
- **Files modified:** `scripts/phase5-source-guards.sh`
- **Verification:** `pnpm run test:phase5-source-guards`, `pnpm run test`, `/Users/zhiwen/.cargo/bin/just test`
- **Committed in:** `68e9990`

---

**Total deviations:** 3 auto-fixed blocking gate issues.  
**Impact on plan:** All fixes support the planned gates and do not add product scope or dependencies.

## Issues Encountered

- Root `pnpm run test` initially failed on stale command fixtures and the Phase 5 source guard's broad `f64` check. Both were fixed and verified with the full root and just gates.
- The Phase 07 workspace test initially failed due to a strict locator collision after the same readout appeared in multiple editor regions. The assertion now targets the preview region explicitly.

## User Setup Required

None - no external service configuration required.

## Verification

- `cargo test -p draft_model schema_fixtures_validate_command_contracts -- --nocapture` - passed.
- `pnpm run test:phase7` - passed.
- `pnpm run test:phase5-source-guards` - passed.
- `pnpm run test` - passed.
- `/Users/zhiwen/.cargo/bin/just test` - passed.
- `/Users/zhiwen/.cargo/bin/just build` - passed.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed.

## Self-Check: PASSED

- Phase 07 source guards exist and pass.
- Root `pnpm run test:phase7`, `pnpm run test`, and `/Users/zhiwen/.cargo/bin/just test` include the Phase 07 checks and pass.
- Generated schema and TypeScript contracts have no drift.
- Renderer canvas settings remain command-owned and no package additions were introduced.

## Next Phase Readiness

Phase 07 can move to phase-level verification. Phase 08 can build segment 画面/基础/变换 and compositing semantics on top of the canonical draft canvas profile and coordinate system.

---
*Phase: 07-project-canvas-space-and-coordinate-system*
*Completed: 2026-06-18*
