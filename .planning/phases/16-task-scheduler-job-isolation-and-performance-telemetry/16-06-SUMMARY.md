---
phase: 16-task-scheduler-job-isolation-and-performance-telemetry
plan: "06"
subsystem: testing
tags: [scheduler, telemetry, product-e2e, electron, rust-bindings]

requires:
  - phase: 16-02
    provides: preview and audio scheduler lanes
  - phase: 16-03
    provides: export scheduler admission and telemetry
  - phase: 16-04
    provides: artifact, probe, and project IO scheduler routing
  - phase: 16-05
    provides: product-safe scheduler status and telemetry APIs
provides:
  - Phase 16 scheduler source guards and no-product-fallback checks
  - Product scheduler stress E2E using repository fixtures and packaged desktop flow
  - Product-safe scheduler telemetry aggregation across preview, export, artifact, probe, IO, and audio domains
  - Project-session read sync before renderer mutations so background probes cannot stale product commands
  - Root package scripts for Phase 16 focused and aggregate gates
affects: [phase-16-verification, phase-17-template-import, desktop-product-e2e]

tech-stack:
  added: []
  patterns:
    - Product tests wait for successful Rust command results, not request dispatch alone
    - Project-session read APIs can sync current state while mutation APIs keep stale revision gates
    - Scheduler telemetry is aggregated at product-safe native boundaries

key-files:
  created:
    - apps/desktop-electron/tests/product-scheduler-stress.spec.ts
  modified:
    - apps/desktop-electron/src/main/index.ts
    - apps/desktop-electron/src/main/nativeBinding.ts
    - apps/desktop-electron/src/main/realtimePreviewHost.ts
    - apps/desktop-electron/src/renderer/App.tsx
    - apps/desktop-electron/src/renderer/viewModel.ts
    - apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx
    - apps/desktop-electron/tests/helpers/userJourney.ts
    - crates/bindings_node/src/project_session_service.rs
    - crates/bindings_node/src/task_runtime_service.rs
    - package.json

key-decisions:
  - "Product UI synchronizes the current project-session revision through the read model before mutations; Rust mutation stale-revision gates remain strict."
  - "Scheduler stress success requires renderGraphGpuComposited product evidence, no fallbackActive state, native scheduler telemetry, and visible preview motion."
  - "Test observations may record command results for diagnostics, but they do not satisfy product preview or scheduler success."

patterns-established:
  - "Current-state project-session reads are safe sync points for UI-owned session revision."
  - "Background material probing may advance session revision independently of the last user mutation."
  - "Product stress helpers assert native command success before checking UI state."

requirements-completed:
  - SCHED-01
  - SCHED-02
  - SCHED-03
  - SCHED-04

duration: resumed
completed: 2026-06-24
status: complete
---

# Phase 16 Plan 06: Scheduler Guards And Product Stress Summary

**Scheduler ownership guards, product stress E2E, package gates, and project-session revision sync for real desktop pressure workflows**

## Performance

- **Duration:** resumed
- **Started:** earlier session
- **Completed:** 2026-06-24T05:23:41Z
- **Tasks:** 3
- **Files modified:** 19

## Accomplishments

- Added Phase 16 source and no-product-fallback gates that reject scheduler bypasses, renderer-owned scheduler policy, mock/fallback product success, and duplicate freshness definitions.
- Added a packaged desktop product stress test that imports real repository media, adds timeline content, starts realtime playback, starts export, triggers import/probe pressure, edits inspector properties, and asserts scheduler telemetry plus visible compositor evidence.
- Aggregated scheduler telemetry from realtime preview, export, artifact/probe/project IO, and audio paths through product-safe native APIs.
- Fixed a real product state race: background media probe completion advanced the Rust project-session revision after import, while renderer timeline mutations still used the pre-probe revision. The renderer now syncs through current-state project-session reads before mutation commands.
- Added package scripts for `test:phase16-rust`, `test:phase16-source-guards`, `test:phase16-desktop`, and `test:phase16`.

## Task Commits

1. **Task 16-06-01: Add Phase 16 scheduler source guards** - `141f02f`, `1c9f839`, `2e1bebc`
2. **Task 16-06-02: Add product scheduler stress E2E** - `533e38d`
3. **Task 16-06-03: Add Phase 16 aggregate package scripts** - `533e38d`

**Plan metadata:** pending this SUMMARY commit

## Files Created/Modified

- `scripts/phase16-source-guards.sh` - Scheduler ownership and source-boundary guard.
- `scripts/no-product-fallback-guards.sh` - Product-success guard for fallback/mock/artifact/CPU/DOM substitutes.
- `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` - Packaged product scheduler stress workflow.
- `apps/desktop-electron/tests/helpers/userJourney.ts` - Product helper support for telemetry, command-result waiting, and stress workflow actions.
- `apps/desktop-electron/src/renderer/App.tsx` - Material readiness polling, current-state project-session revision sync before mutations, and preview snapshot semantic keying.
- `apps/desktop-electron/src/renderer/viewModel.ts` - Material readiness semantics and user-facing analysis-in-progress state.
- `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx` - Material add/drag enablement follows readiness semantics.
- `crates/bindings_node/src/task_runtime_service.rs` - Aggregated scheduler telemetry model.
- `crates/bindings_node/src/project_session_service.rs` - Current-state material reads and scheduler telemetry snapshot recording.
- `package.json` - Phase 16 package-level gates.

## Decisions Made

- Keep Rust stale-revision rejection strict for mutation commands. Product UI must refresh its session revision before issuing a mutation when background scheduler work can advance canonical state.
- Treat material list reads as current-state synchronization reads. They return the latest session revision and material state even when the caller's known revision is behind.
- Use diagnostic command-result observations only to make product tests fail with actionable Rust errors. Product success remains tied to compositor, timeline, command, and scheduler evidence.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Product stress exposed stale project-session revisions**

- **Found during:** Task 16-06-02 product stress verification
- **Issue:** `addTimelineSegmentIntent` was dispatched with `expectedRevision=3` after background audio probe advanced the session to `4`, so Rust correctly rejected the mutation and no timeline segment appeared.
- **Fix:** Renderer now syncs project-session reads before mutation commands, while Rust mutation stale gates remain unchanged. Product helpers also wait for successful Rust command results before asserting UI state.
- **Files modified:** `apps/desktop-electron/src/renderer/App.tsx`, `apps/desktop-electron/tests/helpers/userJourney.ts`, `apps/desktop-electron/src/main/index.ts`
- **Verification:** `pnpm run test:phase16-desktop`
- **Committed in:** `533e38d`

**2. [Rule 2 - Missing Critical] Product stress needed cross-domain telemetry evidence**

- **Found during:** Task 16-06-02 product scheduler stress implementation
- **Issue:** The stress gate needed product-safe telemetry that included preview host, artifact/probe/project IO, export, and audio scheduler domains, not only the binding-local runtime service.
- **Fix:** Added snapshot recording and aggregation through the native telemetry boundary.
- **Files modified:** `apps/desktop-electron/src/main/index.ts`, `apps/desktop-electron/src/main/realtimePreviewHost.ts`, `crates/bindings_node/src/task_runtime_service.rs`, `crates/bindings_node/src/project_session_service.rs`, scheduler-adjacent binding services
- **Verification:** `pnpm run test:phase16-rust`, `pnpm run test:phase16-desktop`
- **Committed in:** `533e38d`

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 missing critical)
**Impact on plan:** The deviations were required to make the planned product stress gate meaningful. They keep the Rust ownership boundary intact and do not introduce fallback success paths.

## Issues Encountered

- The packaged product stress test initially failed after material import because the add-to-timeline command was rejected as stale. This was a real product state synchronization issue, not a test timing issue.
- Node engine warnings appeared during pnpm commands because the local Node version is `v24.15.0` while package metadata asks for `24.12.0`; commands completed successfully.

## Verification

- `pnpm run test:phase16-source-guards` - passed
- `pnpm run test:no-product-fallback` - passed
- `cargo test -p bindings_node project_session_material_reads_use_canonical_session_draft -- --nocapture` - passed
- `pnpm --filter @video-editor/desktop build` - passed
- `pnpm run test:phase16-rust` - passed
- `pnpm run test:phase16-desktop` - passed
- `git diff --check -- . ':!reference'` - passed

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 16 can now proceed to aggregate verification in Plan 16-07. Phase 17 can rely on scheduler telemetry and current-state project-session reads when template import triggers background material localization/probing before timeline mutations.

---
*Phase: 16-task-scheduler-job-isolation-and-performance-telemetry*
*Completed: 2026-06-24*
