---
phase: 04-jianying-style-desktop-workspace
plan: 04
subsystem: desktop-ui-testing
tags: [electron, playwright, source-guards, chinese-workspace, jianying-workspace]

requires:
  - phase: 04-jianying-style-desktop-workspace
    provides: Chinese workspace shell, panels, inspector, and command-only timeline surface
  - phase: 03-timeline-command-core
    provides: generated timeline command contracts and TimelineCommandResponse state replacement
provides:
  - Electron smoke coverage for Chinese workspace regions and exact preload bridge shape
  - Workspace Playwright coverage for Chinese panels, material states, command-only timeline edit, and layout stability
  - Phase 4 renderer source guards for mutation, privileged APIs, FFmpeg leakage, English visible copy, and generated headers
  - Root and just test wiring for Phase 4 guard and workspace gates
affects: [phase-04-desktop-workspace, desktop-renderer, electron-e2e, public-test-gates]

tech-stack:
  added: []
  patterns:
    - Playwright observes timeline command calls at the Electron main IPC handler while delegating to the native command path
    - Source guards scope English-copy checks to visible JSX, ARIA/form attributes, and Playwright visible locators
    - Phase-specific gates are named at root and included before generated contract drift checks

key-files:
  created:
    - apps/desktop-electron/tests/workspace.spec.ts
    - scripts/phase4-source-guards.sh
  modified:
    - apps/desktop-electron/tests/electron-smoke.spec.ts
    - apps/desktop-electron/package.json
    - package.json
    - justfile

key-decisions:
  - "Kept Electron smoke focused on preload/native trust boundaries while asserting the Phase 4 Chinese workspace first screen."
  - "Used a main-process IPC wrapper in Playwright to observe `executeCommand` calls without replacing the Rust native command source."
  - "Wired Phase 4 source guards and workspace tests into root `test` and `just test` before `test:contracts`."

patterns-established:
  - "Workspace E2E tests reuse `launchWorkspaceApp`, `expectVisibleWorkspaceRegions`, geometry helpers, and main-process command spying."
  - "Layout stability checks compare visible region boxes at `1280x800` and `1120x720` after hover, selection, and playhead state changes."
  - "Phase 4 source guard permits internal TypeScript/generated English identifiers while rejecting user-facing English workspace copy."

requirements-completed: [UI-01, UI-02, UI-03, UI-04, UI-05, UI-06, TEST-06]

duration: 45min
completed: 2026-06-17
---

# Phase 04 Plan 04: Workspace Verification Gates Summary

**Executable Electron and source-guard gates for the Chinese Jianying-style workspace, command-only timeline path, and stable desktop layout.**

## Performance

- **Duration:** 45 min
- **Started:** 2026-06-17T10:25:00Z
- **Completed:** 2026-06-17T11:10:28Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- Updated Electron smoke coverage to assert `剪映风格编辑工作区`, the five Chinese workspace regions, exact preload bridge keys, no raw `ipcRenderer`, untrusted navigation isolation, and generated `executeCommand` ping.
- Added `workspace.spec.ts` covering Chinese categories, material status rows, panel switching, deferred category empty states, command-only timeline edit through Rust response, and no-overlap layout checks at `1280x800` and `1120x720`.
- Added `scripts/phase4-source-guards.sh` for renderer mutation patterns, privileged imports/globals, renderer FFmpeg/render leakage, English visible copy, generated file headers, and generated drift.
- Wired `test:workspace`, `test:phase4-source-guards`, and `test:phase4-workspace` into desktop, root, and `just test` gates.

## Task Commits

Each task was committed atomically:

1. **Task 1: Update Electron smoke for Chinese workspace and preload IPC contract** - `77ad403` (test)
2. **Task 2: Add workspace Playwright flow and layout tests** - `f8ede65` (test)
3. **Task 3: Add Phase 4 source guards and final gate wiring** - `8fd90d0` (test)

## Files Created/Modified

- `apps/desktop-electron/tests/electron-smoke.spec.ts` - Chinese workspace smoke assertions, exact preload bridge checks, generated ping command, and untrusted navigation coverage.
- `apps/desktop-electron/tests/workspace.spec.ts` - Focused Playwright suite for workspace regions, panels, command-only timeline edit, and layout stability.
- `apps/desktop-electron/package.json` - Adds `test:workspace`.
- `scripts/phase4-source-guards.sh` - Adds Phase 4 source guard and generated drift checks.
- `package.json` - Adds root `test:phase4-source-guards` and `test:phase4-workspace`, included in `test`.
- `justfile` - Adds Phase 4 guard and workspace gates to public `just test`.

## Decisions Made

- Observed command-only timeline edits from Electron main IPC rather than monkey-patching the context-isolated renderer bridge; the original handler still executes the native Rust command.
- Used `addSegment` as the visible accepted timeline edit in Playwright because it produces a stable UI change from `TimelineCommandResponse`.
- Kept English test names allowed while guarding English user-facing locator strings and known old smoke copy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Tightened source guard visible-locator matching**
- **Found during:** Task 3
- **Issue:** The first guard pattern treated the Playwright method name `toContainText` as the forbidden visible label `Text`, causing false positives on Chinese material row assertions.
- **Fix:** Scoped the Playwright guard to `getByRole(... name: ...)` and direct `getByText`/`getByLabel`/`getByPlaceholder`/`getByTitle` arguments only.
- **Files modified:** `scripts/phase4-source-guards.sh`
- **Verification:** `pnpm run test:phase4-source-guards` passed.
- **Committed in:** `8fd90d0`

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** The fix preserved the requested English-copy guard scope without weakening renderer architecture checks.

## Issues Encountered

- The plan's direct `pnpm --filter @video-editor/desktop playwright test ...` form is not available as a pnpm package script in this workspace, so the equivalent `pnpm --filter @video-editor/desktop exec playwright test ...` was used for the focused smoke run. The committed public gates use package scripts.
- The renderer context-bridge object was not a reliable place to monkey-patch command observation, so the workspace test observes the Electron main IPC handler and delegates to the original native handler.
- `reference/` and `KAIPAI_FORMULA_ADAPTER_RESEARCH.md` remained untracked external files and were left unstaged.

## Known Stubs

None blocking. The existing Phase 4 preview placeholder copy remains intentional and is verified as `预览将在下一阶段接入`; deterministic preview frames and export remain Phase 5/6 scope.

## Authentication Gates

None.

## Threat Flags

None. This plan added test and guard surfaces only; it introduced no renderer network endpoint, filesystem access, Electron/Node import, FFmpeg construction, render graph generation, preview cache, waveform path, auth path, or schema boundary.

## Verification

- `pnpm --filter @video-editor/desktop exec playwright test tests/electron-smoke.spec.ts` - PASS.
- `pnpm --filter @video-editor/desktop test:workspace` - PASS.
- `pnpm run test:phase4-source-guards` - PASS.
- `pnpm run test:phase4-workspace` - PASS.
- `pnpm run test:desktop` - PASS.
- `pnpm run test:contracts` - PASS.
- `pnpm run test` - PASS.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - PASS.

## Self-Check: PASSED

- Key files exist on disk: `electron-smoke.spec.ts`, `workspace.spec.ts`, `phase4-source-guards.sh`, root `package.json`, desktop `package.json`, and `justfile`.
- Task commits `77ad403`, `f8ede65`, and `8fd90d0` exist in git history.
- Generated contract drift check passed for `schemas` and `apps/desktop-electron/src/generated`.
- Stub scan found no blocking test or gate stubs. The shell guard's literal `placeholder` term is part of the required visible-copy scan.
- `reference/` and `KAIPAI_FORMULA_ADAPTER_RESEARCH.md` remain untracked and unstaged.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 4 workspace verification is complete. The app now has executable gates for the Chinese desktop workspace subset of TEST-06 while leaving deterministic preview rendering, waveform/cache behavior, export, and packaged release smoke to Phase 5/6.

---
*Phase: 04-jianying-style-desktop-workspace*
*Completed: 2026-06-17*
