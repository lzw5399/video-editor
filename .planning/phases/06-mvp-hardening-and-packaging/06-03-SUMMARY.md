---
phase: 06-mvp-hardening-and-packaging
plan: 03
subsystem: ui
tags: [electron, react, runtime-diagnostics, playwright]
requires:
  - phase: 06-mvp-hardening-and-packaging
    provides: Rust-owned runtime capability command from Plan 06-02
provides:
  - Runtime diagnostics display state built from probeRuntimeCapabilities results
  - Compact Chinese diagnostics panel inside the preview/export shell
  - Playwright Electron diagnostics coverage at 1280x800 and 1120x720
affects: [desktop-ui, preview, export, phase-06]
tech-stack:
  added: []
  patterns:
    - Renderer displays runtime capability data returned by Rust-owned command envelopes.
    - Preview/export actions are disabled from read-only diagnostics state without renderer-owned runtime probing.
key-files:
  created:
    - apps/desktop-electron/tests/runtime-diagnostics.spec.ts
  modified:
    - apps/desktop-electron/src/renderer/commandHelpers.ts
    - apps/desktop-electron/src/renderer/viewModel.ts
    - apps/desktop-electron/src/renderer/App.tsx
    - apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx
    - apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
    - apps/desktop-electron/src/renderer/workspace/preview-inspector.css
    - apps/desktop-electron/src/main/index.ts
    - apps/desktop-electron/package.json
key-decisions:
  - "Runtime diagnostics remain display-only renderer state derived from the Rust probeRuntimeCapabilities command."
  - "Diagnostics live inside the existing preview shell as a compact panel, not a new route, modal, or duplicate navigation surface."
patterns-established:
  - "Runtime capability UI rows are generated from commandHelpers display data so renderer components do not hardcode runtime probing logic."
  - "Test runtime capability mocks are opt-out with VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES=0 for future no-mock gates."
requirements-completed: [TEST-06]
duration: 20 min
completed: 2026-06-17
---

# Phase 06 Plan 03: Runtime Diagnostics Preview Shell Summary

**Rust-owned runtime capability reports now appear as compact Chinese diagnostics inside the desktop preview/export shell.**

## Performance

- **Duration:** 20 min
- **Started:** 2026-06-17T21:22:00Z
- **Completed:** 2026-06-17T21:42:23Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- Added runtime diagnostics display state and the generated `probeRuntimeCapabilities` command helper.
- Wired startup and manual re-detection through `window.videoEditorCore.executeCommand`.
- Rendered `运行环境诊断`, `运行环境状态`, `运行能力列表`, and capability rows inside the preview shell.
- Disabled affected preview/export actions with Chinese unavailable labels when runtime diagnostics fail.
- Added a dedicated `test:runtime-diagnostics` Playwright gate for 1280x800 and 1120x720.

## Task Commits

1. **Task 1: Add runtime diagnostics state and command wiring** - `529103b` (feat)
2. **Task 2: Add runtime diagnostics Playwright coverage** - `94941af` (test)

## Files Created/Modified

- `apps/desktop-electron/tests/runtime-diagnostics.spec.ts` - diagnostics layout, ready/error, and command-boundary coverage.
- `apps/desktop-electron/src/renderer/commandHelpers.ts` - runtime diagnostics command helper and report-to-display mapping.
- `apps/desktop-electron/src/renderer/viewModel.ts` - runtime diagnostics display state types and initial/checking states.
- `apps/desktop-electron/src/renderer/App.tsx` - startup probe, manual recheck, and preview/export unavailable guards.
- `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx` - passes diagnostics state and recheck callback into the preview monitor.
- `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` - compact diagnostics panel and disabled preview/export CTA states.
- `apps/desktop-electron/src/renderer/workspace/preview-inspector.css` - fixed-height dense diagnostics styling inside the preview shell.
- `apps/desktop-electron/src/main/index.ts` - Electron test mock for runtime capability reports/errors.
- `apps/desktop-electron/package.json` - adds `test:runtime-diagnostics`.

## Decisions Made

- Diagnostics are shown in the existing preview monitor because Phase 06 hardens runtime readiness rather than adding a settings workflow.
- The runtime capability test mock defaults on only for test command recording and can be disabled with `VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES=0`.
- Runtime labels containing FFmpeg/ffprobe are produced by display-state helpers in `commandHelpers.ts`, keeping renderer components clear of source-guarded runtime ownership strings.

## Deviations from Plan

None - plan executed as specified.

## Issues Encountered

None.

## Verification

- `pnpm --filter @video-editor/desktop build`
- `pnpm run test:phase5-source-guards`
- `pnpm --filter @video-editor/desktop test:runtime-diagnostics`
- `pnpm --filter @video-editor/desktop test:workspace`
- `rg -n "buildProbeRuntimeCapabilitiesCommand|probeRuntimeCapabilities" apps/desktop-electron/src/renderer`
- `rg -n "运行环境诊断|运行环境状态|运行能力列表|重新检测运行环境" apps/desktop-electron/src/renderer apps/desktop-electron/tests/runtime-diagnostics.spec.ts`

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for 06-04: the desktop UI now exposes runtime readiness before preview/export actions, and the test mock can be disabled for no-mock import-preview-export gates.

## Self-Check: PASSED

All Plan 03 tasks are implemented, committed, and covered by the required diagnostics and source-guard gates.

---
*Phase: 06-mvp-hardening-and-packaging*
*Completed: 2026-06-17*
