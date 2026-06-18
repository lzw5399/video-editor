---
phase: 11-realtime-preview-runtime-and-gpu-render-backend
plan: 05B
subsystem: desktop-realtime-preview
tags: [electron, react, playwright, telemetry, fallback, realtime-preview, tdd]

requires:
  - phase: 11-realtime-preview-runtime-and-gpu-render-backend
    provides: Plan 11-04B Electron native preview host bridge and Plan 11-05 Rust-owned realtime/fallback telemetry binding data
provides:
  - Desktop realtime preview display model for backend, latency, pacing, stale, cancellation, cache, and fallback fields
  - Preview monitor Chinese telemetry and fallback artifact display driven by main/Rust response data
  - Playwright coverage for supported realtime, FFmpeg fallback artifact display, cancellation telemetry, and renderer display-only source guards
affects: [phase-11, phase-12-media-io, phase-16-scheduler, phase-17-bindings, desktop-preview]

tech-stack:
  added: []
  patterns:
    - Renderer formats realtime preview telemetry through view-model helpers only
    - Electron main carries Rust binding backend/fallback/cancellation fields into a safe display state
    - FFmpeg appears in the normal UI only as a fallback artifact label when Rust reports FFmpeg artifact fallback

key-files:
  created:
    - .planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-05B-SUMMARY.md
  modified:
    - apps/desktop-electron/src/renderer/viewModel.ts
    - apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
    - apps/desktop-electron/tests/workspace.spec.ts
    - apps/desktop-electron/src/main/nativeBinding.ts
    - apps/desktop-electron/src/main/realtimePreviewHost.ts
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/src/realtime_preview_service.rs

key-decisions:
  - "Renderer display code formats realtime preview telemetry only; backend, fallback, cancellation, and artifact visibility are carried from main/Rust state."
  - "Supported realtime responses display Mock/GPU/offscreen backend labels and never show FFmpeg as the active realtime backend."
  - "FFmpeg is shown only as a fallback artifact label when Rust reports `ffmpegArtifactGenerated`."

patterns-established:
  - "Use `RealtimePreviewDisplayModel` and `summarizeRealtimePreviewDisplay` for compact user-facing telemetry copy."
  - "Use `fallbackArtifactVisible` from main state to decide whether the renderer shows the fallback artifact row."
  - "Use narrow cancellation-token binding calls when Electron tests need Rust-reported cancellation telemetry."

requirements-completed: [RTPREV-03, RTPREV-05]

duration: 8min
completed: 2026-06-19
---

# Phase 11 Plan 05B: Realtime Preview Telemetry Display Summary

**Desktop preview monitor shows Rust/main-provided realtime backend, latency, pacing, cancellation, cache, and fallback diagnostics without renderer-owned fallback decisions**

## Performance

- **Duration:** 8 min
- **Started:** 2026-06-18T17:33:57Z
- **Completed:** 2026-06-18T17:41:22Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added `RealtimePreviewDisplayModel` plus Chinese formatters for backend, fallback reason, latency, pacing, stale rejection, cancellation, cache, and fallback counters.
- Updated the desktop preview monitor to render compact telemetry and fallback artifact details from main/Rust state.
- Added Playwright coverage for supported realtime display without active FFmpeg labeling, FFmpeg artifact fallback display, cancellation telemetry, and renderer display-only source guards.
- Added narrow Rust binding cancellation-token methods so main-process tests can surface runtime-reported cancellation telemetry.

## Task Commits

1. **Task 11-05B-01 RED:** `12148ee` test: add failing realtime display model test.
2. **Task 11-05B-01 GREEN:** `f90b627` feat: add realtime telemetry display model.
3. **Task 11-05B-02 RED:** `1e9c641` test: add failing realtime telemetry UI tests.
4. **Task 11-05B-02 GREEN:** `a943d92` feat: render realtime telemetry and fallback display.

**Plan metadata:** pending final docs commit.

## Files Created/Modified

- `apps/desktop-electron/src/renderer/viewModel.ts` - Realtime preview display model and Chinese backend/fallback/telemetry formatting helpers.
- `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` - Preview monitor telemetry readout and fallback artifact row driven by main-provided state.
- `apps/desktop-electron/tests/workspace.spec.ts` - RED/GREEN Playwright and source-guard coverage for telemetry, fallback, cancellation, and renderer boundary behavior.
- `apps/desktop-electron/src/main/nativeBinding.ts` - TypeScript binding shape for backend/fallback/diagnostics/cancellation telemetry fields and narrow cancellation calls.
- `apps/desktop-electron/src/main/realtimePreviewHost.ts` - Main-process display state assembly from Rust binding frame/telemetry responses.
- `crates/bindings_node/src/lib.rs` - Node-API exports for cancellation-token allocation and request cancellation.
- `crates/bindings_node/src/realtime_preview_service.rs` - Binding registry methods for Rust runtime cancellation telemetry.

## Decisions Made

- Renderer-owned logic remains display-only: it formats backend/fallback values and counters already returned by main/Rust.
- Fallback artifact UI uses `fallbackArtifactVisible` from main state; React does not infer artifact visibility from draft contents, materials, or fallback ladders.
- Cancellation telemetry is obtained through Rust runtime cancellation-token methods instead of being synthesized in the renderer.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Exposed narrow cancellation-token binding calls for desktop telemetry**
- **Found during:** Task 11-05B-02 (telemetry/fallback UI wiring)
- **Issue:** The runtime had cancellation telemetry, but the Node-API/Electron main path did not expose a way to allocate and cancel a request token, so the desktop UI could not verify Rust-reported cancellation counters.
- **Fix:** Added `nextRealtimePreviewCancellationToken` and `cancelRealtimePreviewRequest` as narrow binding calls, then used them only in the test-mode main-process telemetry path.
- **Files modified:** `crates/bindings_node/src/lib.rs`, `crates/bindings_node/src/realtime_preview_service.rs`, `apps/desktop-electron/src/main/nativeBinding.ts`, `apps/desktop-electron/src/main/realtimePreviewHost.ts`
- **Verification:** `pnpm --filter @video-editor/desktop test:workspace -g "实时预览|fallback|telemetry"` passed with cancellation telemetry assertions.
- **Committed in:** `a943d92`

---

**Total deviations:** 1 auto-fixed (Rule 2).
**Impact on plan:** The binding addition was required to display and test real runtime cancellation telemetry. It did not add external packages or move fallback decisions into the renderer.

## Known Stubs

None introduced by this plan. The scan found existing unrelated `audio-waveform-placeholder` test copy from earlier phases and ordinary nullable display state fields; neither blocks this plan.

## Threat Flags

None - the binding/main to renderer telemetry surface and renderer-to-user display surface were covered by the plan threat model.

## Issues Encountered

- The first UI RED run exposed the TypeScript main binding request shape was stale after Plan 11-05; updating it fixed the existing first-frame mock path and enabled backend/fallback/cancellation display.
- A broad source-guard regex initially matched legitimate fallback vocabulary in `viewModel.ts`; it was tightened before the RED commit so it guards renderer fallback assignment/inference instead of display labels.

## Verification

- `pnpm --filter @video-editor/desktop test:workspace -g "telemetry display model"` - passed during Task 11-05B-01 GREEN; 1 test ran.
- `pnpm --filter @video-editor/desktop build` - passed during Task 11-05B-01 GREEN.
- `pnpm --filter @video-editor/desktop test:workspace -g "实时预览|fallback|telemetry"` - passed after Task 11-05B-02 GREEN and final verification; 8 Playwright tests ran.
- `pnpm --filter @video-editor/desktop build` - passed after Task 11-05B-02 GREEN and final verification.

## Boundary Notes

- Renderer does not construct FFmpeg commands, render graphs, cache keys, native handles, GPU objects, support classification, or fallback ladders.
- Supported realtime responses display `实时后端：Mock/GPU/离屏` and do not show FFmpeg as the active realtime backend.
- `备用产物：FFmpeg` appears only when main/Rust reports `ffmpegArtifactGenerated` with fallback artifact visibility enabled.
- The preload bridge remains unchanged and exposes only `updateHostRect` and `getTelemetry`.
- `reference/` remained untracked and untouched.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Desktop UI can now surface RTPREV-05 telemetry and RTPREV-03 fallback distinctions from the runtime path. Later Phase 11/12 work can add real GPU/platform frame data while preserving the renderer display-only boundary.

## Self-Check: PASSED

- Verified created summary exists: `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-05B-SUMMARY.md`.
- Verified task commits exist: `12148ee`, `f90b627`, `1e9c641`, `a943d92`.
- Verified required commands passed: `pnpm --filter @video-editor/desktop test:workspace -g "实时预览|fallback|telemetry"` and `pnpm --filter @video-editor/desktop build`.
- Verified `reference/` remains untracked and unstaged.

---
*Phase: 11-realtime-preview-runtime-and-gpu-render-backend*
*Completed: 2026-06-19*
