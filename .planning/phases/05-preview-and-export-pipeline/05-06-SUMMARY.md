---
phase: 05-preview-and-export-pipeline
plan: 06
subsystem: desktop-preview-ui
tags: [electron, react, playwright, preview, source-guards]
requires:
  - phase: 05-05
    provides: Rust-owned preview command contracts, binding routes, and renderer envelope helpers
provides:
  - Command-driven preview monitor state for frame and segment preview requests
  - Simplified Chinese preview frame/segment status, artifact, and classified error UI
  - Automated 1280x800 and 1120x720 preview screenshots
  - Phase 5 renderer source guards for preview/export ownership boundaries
affects: [desktop-renderer, desktop-tests, phase5-gates]
tech-stack:
  added: []
  patterns:
    - renderer stores preview display state only and requests preview through executeCommand
    - Playwright uses a main-process test hook for preview envelopes while timeline commands keep using the real binding
    - screenshot gates preserve compact dark scrollbar and panel proportion baselines
key-files:
  created: []
  modified:
    - apps/desktop-electron/src/renderer/App.tsx
    - apps/desktop-electron/src/renderer/viewModel.ts
    - apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx
    - apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
    - apps/desktop-electron/src/renderer/workspace/preview-inspector.css
    - apps/desktop-electron/src/renderer/styles.css
    - apps/desktop-electron/src/main/index.ts
    - apps/desktop-electron/tests/workspace.spec.ts
    - apps/desktop-electron/tests/electron-smoke.spec.ts
    - scripts/phase5-source-guards.sh
key-decisions:
  - "Preview UI keeps only artifact/status/error display fields; draft, timeline, cache key, render graph, and runtime semantics remain outside the renderer."
  - "Playwright preview success assertions use a test-only Electron main hook that returns Rust-contract-shaped envelopes only when VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS=1."
  - "The obsolete global preview-shell CSS was removed so the dedicated preview monitor stylesheet owns the current preview layout."
patterns-established:
  - "Preview monitor actions call buildRequestPreviewFrameCommand and buildRequestPreviewSegmentCommand, then apply only returned artifact/status fields."
  - "Preview geometry gates assert five-region layout, preview control fit, 16:9 canvas ratio, and compact dark scrollbar CSS."
  - "Renderer source guards block preview invalidation overlap, cache fingerprint, process execution, FFmpeg, and render graph ownership."
requirements-completed: [PREV-01, PREV-02, PREV-03, PREV-04]
duration: 14 min
completed: 2026-06-18
---

# Phase 05 Plan 06: Preview UI And Screenshot Gates Summary

**Command-driven desktop preview monitor with Chinese status, artifact display, and automated geometry screenshots**

## Performance

- **Duration:** 14 min
- **Started:** 2026-06-17T18:52:00Z
- **Completed:** 2026-06-17T19:06:47Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Connected the center preview monitor to `requestPreviewFrame` and `requestPreviewSegment` through `window.videoEditorCore.executeCommand`.
- Added Simplified Chinese preview seek, frame/segment request controls, artifact metadata, and classified preview error display.
- Added Playwright coverage for preview command calls, success/error states, 1280x800 and 1120x720 screenshots, no-overlap layout, and compact scrollbar baseline.
- Strengthened Phase 5 source guards so renderer code cannot own FFmpeg, render graph, cache key, process, or preview invalidation overlap semantics.

## Task Commits

Each task was committed atomically:

1. **Task 05-06-01: Add command-driven preview state to the desktop workspace** - `fbe2e3a` (feat)
2. **Task 05-06-02: Add preview source guards and automated screenshot gates** - `4bafded` (test)

**Plan metadata:** this summary commit

## Files Created/Modified

- `apps/desktop-electron/src/renderer/viewModel.ts` - Adds preview display state and preview status labels.
- `apps/desktop-electron/src/renderer/App.tsx` - Adds preview frame and segment command handlers using generated preview helpers.
- `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx` - Passes preview state and handlers into the monitor.
- `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` - Adds Chinese seek, preview request, artifact, and error UI.
- `apps/desktop-electron/src/renderer/workspace/preview-inspector.css` - Adds compact preview monitor layout and stable preview control sizing.
- `apps/desktop-electron/src/renderer/styles.css` - Removes obsolete preview shell rules that conflicted with the current monitor layout.
- `apps/desktop-electron/src/main/index.ts` - Adds a preview-command test hook gated by `VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS`.
- `apps/desktop-electron/tests/workspace.spec.ts` - Adds preview command, error, geometry, screenshot, and scrollbar baseline tests.
- `apps/desktop-electron/tests/electron-smoke.spec.ts` - Updates smoke expectations for the connected preview shell.
- `scripts/phase5-source-guards.sh` - Adds renderer ownership guards for preview invalidation, cache fingerprints, and process execution.

## Decisions Made

- Kept preview success E2E deterministic with a test-only main process hook instead of requiring local media files and runtime availability in UI tests.
- Kept preview display paths as text metadata instead of having the renderer load or interpret artifact files.
- Removed duplicate global preview styles so `preview-inspector.css` is the single owner of the current monitor layout.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Removed stale global preview CSS conflict**
- **Found during:** Task 05-06-02 screenshot gate
- **Issue:** `styles.css` still had old `.preview-shell` and `.preview-stage` rules, causing the new preview canvas to overflow the monitor shell.
- **Fix:** Removed the obsolete global preview shell/stage/status rules and left preview monitor layout in `preview-inspector.css`.
- **Files modified:** `apps/desktop-electron/src/renderer/styles.css`
- **Verification:** `pnpm --filter @video-editor/desktop test:workspace -g "预览"` passed and screenshots were regenerated.
- **Committed in:** `fbe2e3a`

---

**Total deviations:** 1 auto-fixed (Rule 3 blocking)
**Impact on plan:** The fix was required for the planned automated screenshot gate and did not expand product scope.

## Issues Encountered

- The first preview screenshot run failed because stale global preview CSS overrode the new monitor layout. Removing the stale rules resolved the geometry failure.
- The full desktop suite initially failed on a smoke test that still expected the old preview placeholder copy. The test was updated to the connected preview shell copy.

## Verification

- `pnpm run test:phase5-source-guards` - passed.
- `pnpm --filter @video-editor/desktop test:workspace -g "预览"` - passed, 3 tests.
- `pnpm --filter @video-editor/desktop test` - passed, 14 tests.
- Screenshots written by Playwright:
  - `test-results/phase5/preview-1280x800.png`
  - `test-results/phase5/preview-1120x720.png`

## Self-Check: PASSED

- PREV-01 through PREV-03 are visible in the center monitor through preview command controls and status slots.
- Renderer code still calls only generated command helpers and `window.videoEditorCore.executeCommand`.
- 1120x720 and 1280x800 screenshots exist and verify five-region layout, no clipped preview controls, and compact scrollbar CSS.
- Phase 04.1 left-panel rule remains intact: no standalone left-side secondary primary menu is present.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for `05-08` to add export contracts, binding registry, export UI, and automated export screenshots. `05-07` is already complete, so Wave 7 is unblocked.

---
*Phase: 05-preview-and-export-pipeline*
*Completed: 2026-06-18*
