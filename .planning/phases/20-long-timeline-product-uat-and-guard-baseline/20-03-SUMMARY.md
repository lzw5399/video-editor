---
phase: 20-long-timeline-product-uat-and-guard-baseline
plan: 03
subsystem: testing
tags: [electron, playwright, packaged-uat, long-timeline, scheduler, export]

requires:
  - phase: 20-01
    provides: Rust-owned Phase 20 `.veproj` fixture generation and materializer CLI.
  - phase: 20-02
    provides: Playwright long-timeline fixture orchestration and evidence helpers.
provides:
  - Blocking packaged Electron long-session UAT for responsiveness, save/reopen, export, and scheduler pressure.
  - Two reopen cycles and two export validations using canonical summaries and media evidence.
  - Product pressure path that asserts scheduler latency, rejected/fallback counts, stale-generation behavior, and commit/cancel observations.
affects: [phase20-source-guards, phase20-aggregate, packaged-product-uat]

tech-stack:
  added: []
  patterns:
    - Packaged Electron UAT opens Rust-generated `.veproj` bundles and uses visible product UI operations.
    - Product success requires compositor, canonical persistence, ffprobe/sample-frame export, and scheduler telemetry evidence.

key-files:
  created:
    - apps/desktop-electron/tests/product-long-timeline-uat.spec.ts
  modified:
    - apps/desktop-electron/tests/product-long-timeline-uat.spec.ts

key-decisions:
  - "Keep Phase 20 UAT focused on baseline long-session operations; detailed Phase 19 parity remains deferred to Phase 23."
  - "Use packaged Electron as the blocking product proof and retain dev/list/build commands only as fast feedback."
  - "Treat export file existence as a prerequisite only; ffprobe and sampled frame evidence are required for success."

patterns-established:
  - "Phase 20 packaged UAT wraps all success and failure paths with structured evidence bundle helpers."
  - "Long-session canonical proof records summaries around two reopen cycles and two exports."
  - "Scheduler pressure proof combines export pressure, playback/preview motion, interaction commit/cancel, and product-safe telemetry."

requirements-completed: [UAT11-01, UAT11-02, LONG11-01, LONG11-02, GATE11-01]

duration: 42 min
completed: 2026-06-27
status: complete
---

# Phase 20 Plan 03: Packaged Long-Session UAT Summary

**Packaged Electron long-timeline UAT for responsiveness, canonical reopen/export cycles, and scheduler pressure evidence**

## Performance

- **Duration:** 42 min
- **Started:** 2026-06-27T18:35:00Z
- **Completed:** 2026-06-27T19:18:15Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- Added `product-long-timeline-uat.spec.ts` with packaged Electron launch, Rust-generated long `.veproj` opening, and visible product UI operations for selection, scroll/zoom, scrub/play, move, trim, split, undo/redo, and inspector visual edit.
- Added canonical save/reopen/export coverage with two reopen cycles and two validated exports using canonical draft summaries, derived-artifact checks, ffprobe metadata, and sampled frame evidence.
- Added scheduler pressure coverage that starts export pressure from the product UI, verifies playback/preview evidence, performs scrub and inspector edits, records interaction commit/cancel observations, and asserts product-safe scheduler telemetry.
- Tightened export failure diagnostics so failures include exact export progress, status, validation, and recorded native command observations.

## Task Commits

Each task was committed atomically:

1. **Task 1: Create packaged long-session responsiveness UAT**
   - `4da1b4c` test: add failing long timeline responsiveness UAT
   - `69df585` feat: implement packaged long timeline responsiveness UAT
2. **Task 2: Add two reopen cycles and two export proofs**
   - `bcc4de4` test: add failing canonical export UAT
   - `c0ee4ac` feat: add packaged canonical reopen export UAT
3. **Task 3: Add scheduler pressure, commit/cancel, and failure evidence coverage**
   - `804e364` test: add failing scheduler pressure UAT
   - `02aad4f` feat: add packaged scheduler pressure UAT
   - `8750181` fix: tighten export failure diagnostics

## Files Created/Modified

- `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts` - Packaged Phase 20 UAT covering responsiveness, canonical reopen/export cycles, and scheduler pressure evidence.

## Decisions Made

- Preserved D-07 by limiting the UAT to baseline long-session operations and not expanding into detailed Phase 19 retime/effect/filter/mask/blend parity.
- Used packaged Electron for blocking proof while retaining `--list` and build checks as faster preflight feedback.
- Kept pressure export validation asynchronous enough to create scheduler pressure without using file-exists-only export success.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Resumed after executor stall during Task 3**
- **Found during:** Task 3 (scheduler pressure coverage)
- **Issue:** The executor produced production commits and left a small uncommitted diagnostic fix without returning a completion signal or summary.
- **Fix:** Closed the stuck executor, verified the remaining diff, ran the pressure packaged UAT, committed the diagnostic fix, and wrote this summary manually.
- **Files modified:** `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts`
- **Verification:** `pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts -g "pressure" --reporter=line --workers=1`
- **Committed in:** `8750181`

---

**Total deviations:** 1 auto-fixed (Rule 3).
**Impact on plan:** No scope change. The fix strengthened failure diagnostics and preserved the required packaged pressure gate.

## Issues Encountered

- Node engine warning remains visible: repo wants Node `24.12.0`, current environment is `v24.15.0`. Commands still passed; this remains an environment warning, not a product blocker.
- Rust build warnings from pre-existing deprecated/unused items remain visible during native build. They were not introduced by this plan.

## User Setup Required

None - no external service configuration required.

## Verification

- `pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts -g "pressure" --list` - passed.
- `pnpm --filter @video-editor/desktop build` - passed.
- `pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts -g "pressure" --reporter=line --workers=1` - passed, `1 passed (2.0m)`.

## Next Phase Readiness

Plan 20-04 can now wire source guards and aggregate scripts against the concrete packaged UAT file.

## Self-Check: PASSED

- Key file exists: `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts`.
- Summary file exists: `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-03-SUMMARY.md`.
- Task commits exist for `20-03`.
- Required packaged pressure verification passed.

---
*Phase: 20-long-timeline-product-uat-and-guard-baseline*
*Completed: 2026-06-27*
