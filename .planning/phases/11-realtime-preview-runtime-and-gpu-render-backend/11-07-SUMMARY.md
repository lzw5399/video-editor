---
phase: 11-realtime-preview-runtime-and-gpu-render-backend
plan: 07
subsystem: realtime-preview-closeout
tags: [source-guards, playwright, runtime-boundaries, realtime-preview, tdd]

requires:
  - phase: 11-realtime-preview-runtime-and-gpu-render-backend
    provides: Plans 11-05B and 11-06 realtime telemetry, fallback, text parity, and preview/export parity contracts
provides:
  - Phase 11 renderer/runtime ownership source guard
  - Root Phase 11 Rust, source guard, workspace, contract, and aggregate test scripts
  - Runtime boundary documentation for Phase 11 and downstream Phase 12/15/16/18 ownership
  - Final desktop Playwright smoke gates for native host rect, telemetry, fallback artifact display, viewport layout, and platform smoke docs
affects: [phase-11, phase-12-media-io, phase-15-audio, phase-16-scheduler, phase-18-effects, desktop-preview]

tech-stack:
  added: []
  patterns:
    - Comment-filtered `rg` source guards with local negative checks for renderer ownership boundaries
    - Root phase gates compose Rust runtime, source guard, workspace smoke, and contract drift checks
    - Manual D3D12/Metal WGPU adapter smoke remains documented and opt-in

key-files:
  created:
    - scripts/phase11-source-guards.sh
    - .planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-07-SUMMARY.md
  modified:
    - package.json
    - docs/runtime-boundaries.md
    - apps/desktop-electron/tests/workspace.spec.ts
    - apps/desktop-electron/src/main/realtimePreviewHost.ts

key-decisions:
  - "Phase 11 closeout source guards block renderer-owned FFmpeg, render graph, GPU command, cache key, dirty range, fallback, timeline mutation, keyframe evaluation, and persisted floating-point timeline request fields."
  - "Runtime boundary documentation explicitly keeps Phase 12 media IO, Phase 15 audio, Phase 16 scheduling, and Phase 18 effects outside Phase 11 ownership."
  - "Windows D3D12 and macOS Metal WGPU smoke commands are documented as manual platform gates, not default CI steps."

patterns-established:
  - "Use `scripts/phase11-source-guards.sh` for comment-filtered renderer/runtime ownership checks."
  - "Use `pnpm run test:phase11` as the Phase 11 aggregate closeout gate."
  - "Use docs/runtime-boundaries.md as the handoff contract for Phase 12/15/16/18."

requirements-completed: [RTPREV-01, RTPREV-02, RTPREV-03, RTPREV-04, RTPREV-05]

duration: 12min
completed: 2026-06-18
---

# Phase 11 Plan 07: Source Guards And Runtime Boundary Closeout Summary

**Phase 11 now has executable renderer/runtime ownership guards, root closeout gates, and downstream runtime-boundary documentation for realtime preview.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-18T17:53:00Z
- **Completed:** 2026-06-18T18:05:20Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Added `scripts/phase11-source-guards.sh` with comment-filtered `rg` checks and local negative tests for renderer-owned semantic tokens.
- Added root scripts `test:phase11-rust`, `test:phase11-source-guards`, `test:phase11-workspace`, and `test:phase11`; root `test` now includes Phase 11.
- Documented the Phase 11 realtime preview boundary, fallback rules, H.264 software frame cache role, and Phase 12/15/16/18 exclusions.
- Extended Playwright closeout coverage for supported seek telemetry and static platform smoke documentation.

## Task Commits

1. **Task 11-07-01 RED:** `b0fad08` test: add failing phase 11 guard wiring test.
2. **Task 11-07-01 GREEN:** `9fe8bd3` feat: add phase 11 source guards and scripts.
3. **Task 11-07-02 RED:** `9c76699` test: add failing phase 11 closeout smoke tests.
4. **Task 11-07-02 GREEN:** `0efc2a4` feat: document runtime boundary and closeout smoke gates.

**Plan metadata:** pending final docs commit.

## Files Created/Modified

- `scripts/phase11-source-guards.sh` - Comment-filtered source guard blocking renderer-owned FFmpeg, render graph, GPU command, cache key, dirty range, fallback, timeline mutation, keyframe evaluation, and persisted floating-point timeline request fields.
- `package.json` - Added Phase 11 root scripts and included the aggregate gate in root `test`.
- `docs/runtime-boundaries.md` - Added Phase 11 runtime ownership map, fallback rules, downstream exclusions, and manual Windows/macOS platform smoke commands.
- `apps/desktop-electron/tests/workspace.spec.ts` - Added static guard/script/doc tests plus supported seek telemetry smoke.
- `apps/desktop-electron/src/main/realtimePreviewHost.ts` - Added a test-only supported seek frame path so Playwright can observe Rust-reported seek latency without renderer fallback logic.

## Decisions Made

- Source guards allow legitimate renderer DOM measurement and Chinese telemetry display while blocking semantic ownership patterns.
- Source guards allow generated command route/type vocabulary in `commandHelpers.ts`, but block cache-key and dirty-range ownership elsewhere in renderer code.
- Manual platform smoke is documented but not added to default scripts because it requires real Windows D3D12 or macOS Metal GPU/native surface hosts.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added supported seek telemetry test hook**
- **Found during:** Task 11-07-02 (final Playwright smoke gates)
- **Issue:** Existing realtime preview smoke covered first-frame telemetry and fallback/cancellation paths, but did not expose a supported seek frame with non-null seek latency for the final gate.
- **Fix:** Added `VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_SEEK_FRAME` in Electron main to request a supported Rust realtime frame in seek mode, then asserted `寻帧 7 ms` and no fallback artifact in Playwright.
- **Files modified:** `apps/desktop-electron/src/main/realtimePreviewHost.ts`, `apps/desktop-electron/tests/workspace.spec.ts`
- **Verification:** `pnpm --filter @video-editor/desktop test:workspace -g "supported seek latency|runtime boundary docs"` and `pnpm run test:phase11` passed.
- **Committed in:** `0efc2a4`

---

**Total deviations:** 1 auto-fixed (Rule 2).
**Impact on plan:** The hook is test-only and keeps the renderer UI-only; it was required to prove supported seek telemetry through the existing main/Rust binding path.

## Known Stubs

None introduced by this plan. The closeout scan found an existing `audio-waveform-placeholder` Playwright assertion from earlier phases; it is unrelated to Phase 11 closeout and does not block this plan.

## Threat Flags

None - the touched guard script, docs, existing realtime-preview IPC/main path, and Playwright tests are covered by the plan threat model. No new network endpoint, auth path, file access boundary, or schema trust boundary was introduced.

## Issues Encountered

- `gsd-tools` was not on PATH through the bare shell command, but the repository-local workflow shim was available at `/Users/zhiwen/.codex/get-shit-done/bin/gsd-tools.cjs`.
- Exact static doc assertions initially failed on Markdown line wrapping; the Phase 11 ownership and exclusion phrases were made explicit and grep-friendly.

## Verification

- `bash scripts/phase11-source-guards.sh` - passed.
- `pnpm run test:phase11-rust` - passed; includes `realtime_preview_runtime`, preview service fallback/H.264/no-FFmpeg tests, testkit parity, and bindings realtime tests.
- `pnpm run test:phase11-source-guards` - passed.
- `pnpm --filter @video-editor/desktop test:workspace -g "实时预览|fallback|telemetry|五大区域"` - passed; 10 Playwright tests.
- `pnpm run test:phase11` - passed; Rust, source guards, workspace smoke, and contract drift checks.

## Manual Platform Smoke Notes

Documented but not run by default:

- Windows D3D12: `VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime real_wgpu_adapter -- --ignored --nocapture`, then `pnpm --filter @video-editor/desktop test:workspace -g "实时预览 native preview host rectangle reports integer bounds and telemetry"`.
- macOS Metal: `VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime real_wgpu_adapter -- --ignored --nocapture`, then `pnpm --filter @video-editor/desktop test:workspace -g "实时预览 native preview host rectangle reports integer bounds and telemetry"`.

## Guard Coverage

`scripts/phase11-source-guards.sh` blocks:

- Renderer WebGPU/`wgpu` ownership, GPU device/context/command encoder/render pass/draw command tokens.
- Renderer render graph and FFmpeg ownership: `build_render_graph`, `RenderGraph`, `compile_ffmpeg_job`, `FfmpegExecutor`, FFmpeg script/job/process tokens.
- Renderer preview cache key, semantic fingerprint, material dependency, dirty range, and changed material/range propagation tokens outside generated command helper routing.
- Renderer fallback ladder/selection/classification and fallback reason assignment tokens.
- Direct renderer draft track/segment/timerange/keyframe/visual/text/audio/undo/redo mutation.
- Renderer keyframe evaluation/interpolation/easing/frame-time animation ownership.
- Persisted floating-point seconds timeline request fields.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 12 can consume the documented texture/frame handle boundary and H.264 software cache handoff without moving decode ownership into Phase 11. Phase 15, Phase 16, and Phase 18 have explicit exclusions and can build on the shared clock/generation, telemetry, and fallback contracts.

## Self-Check: PASSED

- Verified created summary exists: `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-07-SUMMARY.md`.
- Verified task commits exist: `b0fad08`, `9fe8bd3`, `9c76699`, `0efc2a4`.
- Verified required commands passed: `bash scripts/phase11-source-guards.sh`, `pnpm run test:phase11-rust`, `pnpm run test:phase11-source-guards`, `pnpm --filter @video-editor/desktop test:workspace -g "实时预览|fallback|telemetry|五大区域"`, and `pnpm run test:phase11`.
- Verified `reference/` remains untracked and unstaged.

---
*Phase: 11-realtime-preview-runtime-and-gpu-render-backend*
*Completed: 2026-06-18*
