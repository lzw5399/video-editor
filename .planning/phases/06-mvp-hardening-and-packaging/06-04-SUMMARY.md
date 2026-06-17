---
phase: 06-mvp-hardening-and-packaging
plan: 04
subsystem: e2e
tags: [electron, playwright, packaging, preview, export]
requires:
  - phase: 06-mvp-hardening-and-packaging
    provides: Packaged app launch helper and runtime diagnostics
provides:
  - Deterministic Phase 06 media fixtures for real workflow tests
  - Shared UI-driven import-preview-export workflow helper
  - Dev and packaged no-mock Playwright gates
affects: [desktop-tests, packaging, preview, export, phase-06]
tech-stack:
  added: []
  patterns:
    - Playwright helpers may generate deterministic media with FFmpeg, while product renderer/main code still routes preview/export through Rust commands.
    - The same visible Chinese UI workflow is reused for dev and packaged launches.
key-files:
  created:
    - apps/desktop-electron/tests/helpers/mediaFixtures.ts
    - apps/desktop-electron/tests/helpers/realWorkflow.ts
    - apps/desktop-electron/tests/real-workflow.spec.ts
  modified:
    - apps/desktop-electron/package.json
key-decisions:
  - "Real workflow tests explicitly disable preview, export, and runtime capability mocks through VIDEO_EDITOR_TEST_MOCK_* = 0."
  - "Fixtures import absolute media paths outside the .veproj bundle so current preview/export runtime path resolution receives executable file paths."
  - "The workflow deletes initial placeholder segments before adding generated media, preventing stale relative demo URIs from entering real preview/export jobs."
patterns-established:
  - "Command bridge logs are asserted for importMaterial, deleteSegment, addSegment, addAudioSegment, requestPreviewFrame, requestPreviewSegment, startExport, and getExportJobStatus."
  - "Packaged workflow tests launch through the packaged executable helper from Plan 06-01."
requirements-completed: [TEST-06, TEST-07]
duration: 20 min
completed: 2026-06-18
---

# Phase 06 Plan 04: Real Workflow Gates Summary

**The desktop app now has dev and packaged no-mock workflow gates for import, timeline edit, preview, export, status polling, and output validation.**

## Performance

- **Duration:** 20 min
- **Completed:** 2026-06-18
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added deterministic generated video/audio fixtures under ignored `test-results/phase6`.
- Added `runRealImportPreviewExportWorkflow`, a shared helper that drives visible Chinese UI controls rather than renderer shortcuts.
- The workflow imports video and audio materials, deletes placeholder segments, adds generated video/audio segments, requests a real preview frame and preview segment, exports an MP4, polls export status, validates `1920x1080` plus `含音频`, and checks artifact paths exist.
- Added dev and packaged tests with preview/export/runtime capability mocks disabled.
- Added desktop scripts: `test:real-workflow`, `test:packaged-real-workflow`, and `test:packaged`.

## Task Commits

1. **Task 1: Add deterministic media fixtures and shared real workflow helper** - `a622505` (feat)
2. **Task 2: Add dev and packaged no-mock workflow specs and scripts** - `50091d6` (test)

## Files Created/Modified

- `apps/desktop-electron/tests/helpers/mediaFixtures.ts` - generates fresh deterministic video/audio fixture media and output paths.
- `apps/desktop-electron/tests/helpers/realWorkflow.ts` - shared UI-driven import-preview-export workflow plus command bridge assertions.
- `apps/desktop-electron/tests/real-workflow.spec.ts` - dev and packaged no-mock workflow specs.
- `apps/desktop-electron/package.json` - adds named workflow and packaged gates.

## Decisions Made

- Test-only fixture generation may call FFmpeg directly; product code remains within the Rust-owned command path.
- The helper imports external absolute media paths instead of media inside the `.veproj` bundle because the current runtime consumes material URIs directly.
- Placeholder demo video/audio segments are removed before real preview/export to avoid stale relative demo material paths.

## Deviations from Plan

None - plan executed as specified.

## Issues Encountered

- Non-plan exploratory command `pnpm --filter @video-editor/desktop exec tsc --noEmit` fails because the desktop package currently lacks the `@types/node` type package expected by its tsconfig. This is not part of the Plan 04 gate; `pnpm --filter @video-editor/desktop build` and both Playwright workflow gates pass.

## Verification

- `rg -n "runRealImportPreviewExportWorkflow|generatePhase6MediaFixtures|requestPreviewFrame|requestPreviewSegment|startExport|getExportJobStatus" apps/desktop-electron/tests/helpers apps/desktop-electron/tests/real-workflow.spec.ts`
- `rg -n "VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS.*0|VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS.*0|VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES.*0|requestPreviewFrame|requestPreviewSegment|startExport|getExportJobStatus" apps/desktop-electron/tests/real-workflow.spec.ts apps/desktop-electron/tests/helpers/realWorkflow.ts`
- `pnpm --filter @video-editor/desktop test:real-workflow`
- `pnpm --filter @video-editor/desktop test:packaged-real-workflow`
- `pnpm run test:phase5-source-guards`

## User Setup Required

Local `ffmpeg` and `ffprobe` must be available through `PATH` or the existing `VE_FFMPEG_PATH` / `VE_FFPROBE_PATH` environment variables for no-mock workflow gates.

## Next Phase Readiness

Ready for 06-05: the MVP workflow is now executable in both dev and packaged app contexts, so the remaining Phase 06 work can document known limits, license posture, and post-MVP backlog.

## Self-Check: PASSED

All Plan 04 tasks are implemented, committed, and covered by the required no-mock workflow and source-guard gates.

---
*Phase: 06-mvp-hardening-and-packaging*
*Completed: 2026-06-18*
