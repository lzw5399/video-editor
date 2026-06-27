---
phase: "16-task-scheduler-job-isolation-and-performance-telemetry"
plan: "04"
subsystem: runtime
tags:
  - task-runtime
  - scheduler
  - artifact-store
  - media-probe
  - project-store
  - electron-bindings

# Dependency graph
requires:
  - phase: "16-01B"
    provides: "Task scheduler admission, cancellation, queue status, and telemetry foundation"
provides:
  - "Artifact refresh and generation submitted through scheduler admission with cancellation/backpressure/status telemetry"
  - "Queued/pending media probe responses that no longer block product import paths on ffprobe completion"
  - "Scheduler-controlled project filesystem IO with stale revision commit gates"
affects:
  - "16-06"
  - "artifact refresh"
  - "material import"
  - "project session IO"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Binding product paths submit heavy artifact/probe/IO work to task_runtime instead of running it inline"
    - "Artifact-visible, material-visible, and project-session-visible mutations require scheduler completion plus freshness evidence"
    - "Media probe command responses expose queued probe status and job identity before ffprobe completes"

key-files:
  created:
    - "crates/bindings_node/tests/scheduler_artifact_probe.rs"
    - ".planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-04-SUMMARY.md"
  modified:
    - "apps/desktop-electron/src/main/nativeBinding.ts"
    - "crates/artifact_store/src/generation.rs"
    - "crates/artifact_store/src/jobs.rs"
    - "crates/artifact_store/tests/artifact_generation.rs"
    - "crates/artifact_store/tests/invalidation.rs"
    - "crates/bindings_node/src/artifact_store_service.rs"
    - "crates/bindings_node/src/material_service.rs"
    - "crates/bindings_node/src/project_session_service.rs"
    - "crates/media_runtime/src/lib.rs"
    - "crates/media_runtime/src/probe.rs"
    - "crates/project_store/src/lib.rs"

key-decisions:
  - "Artifact refresh creates durable artifact job rows and schedules ArtifactGeneration work instead of generating thumbnails from the binding path."
  - "Material import returns queued probe status and probeJobId after scheduler admission; metadata commits happen asynchronously behind stale revision and cancellation gates."
  - "Project create/open/save paths use FilesystemIo scheduler jobs and re-check expected revision before session-visible mutation."

patterns-established:
  - "Scheduler-owned heavy work: product bindings submit jobs with typed domains/resource classes and return status instead of blocking on runtime executors."
  - "Freshness-gated completion: generated artifacts, probed metadata, and project IO results must pass expected revision/generation and cancellation checks before visibility."
  - "Derived artifact isolation: thumbnails, waveforms, probe diagnostics, and generated outputs remain derived state outside `.veproj/project.json`."

requirements-completed:
  - SCHED-01
  - SCHED-02
  - SCHED-04

# Metrics
duration: "20min from first task commit to summary handoff completion"
completed: "2026-06-23"
status: complete
---

# Phase 16 Plan 04: Artifact, Probe, and Filesystem IO Scheduler Boundary Summary

**Artifact generation, media probing, and project filesystem IO now enter task_runtime with bounded admission, cancellation, telemetry, and stale commit gates.**

## Performance

- **Duration:** 20 min from first task commit to summary handoff completion
- **Started:** 2026-06-23T15:54:29Z (first task commit)
- **Completed:** 2026-06-23T16:13:48Z
- **Tasks:** 3 completed
- **Files modified:** 13

## Accomplishments

- Replaced binding-inline artifact refresh generation with scheduler-admitted artifact jobs, durable artifact store status, cancellation gates, and freshness checks before artifact visibility.
- Changed material import/probe behavior to return queued/pending probe state after scheduler admission instead of waiting in bindings, main, or renderer paths for ffprobe completion.
- Added scheduler-controlled project filesystem IO wrappers and expected-revision gates before project-session-visible mutation while keeping `.veproj/project.json` canonical.

## Task Commits

The three plan tasks were covered by a shared TDD RED/GREEN sequence because artifact refresh, media probe, and project IO all cross the same binding scheduler boundary:

1. **RED gate for Tasks 16-04-01 through 16-04-03** - `23ec771` (`test(16-04): add scheduler artifact probe gates`)
2. **GREEN implementation for Tasks 16-04-01 through 16-04-03** - `87c8fac` (`feat(16-04): route artifact probe io through scheduler`)

**Plan metadata:** committed separately after this summary is created.

## Files Created/Modified

- `apps/desktop-electron/src/main/nativeBinding.ts` - Updated desktop binding types to expose queued material probe status and probe job identity.
- `crates/artifact_store/src/generation.rs` - Added cancellation-aware artifact generation wrappers so scheduler cancellation can stop before blob writes and ready rows.
- `crates/artifact_store/src/jobs.rs` - Preserved durable artifact job/chunk state while integrating scheduler-visible status behavior.
- `crates/artifact_store/tests/artifact_generation.rs` - Covered cancellation/freshness behavior and updated stale test enum references.
- `crates/artifact_store/tests/invalidation.rs` - Updated stale command delta test references.
- `crates/bindings_node/src/artifact_store_service.rs` - Routed artifact refresh through scheduler admission instead of direct thumbnail generation or direct FFmpeg executor construction.
- `crates/bindings_node/src/material_service.rs` - Returned queued material probe state after scheduler admission without blocking product import on ffprobe completion.
- `crates/bindings_node/src/project_session_service.rs` - Added scheduler completion and expected-revision gates before material-visible and project-session-visible mutation.
- `crates/bindings_node/tests/scheduler_artifact_probe.rs` - Added binding-level scheduler gates for artifact, probe, and filesystem IO behavior.
- `crates/media_runtime/src/lib.rs` - Exported scheduled material probe support.
- `crates/media_runtime/src/probe.rs` - Added scheduled material probe entry point while preserving the existing ffprobe argument-array implementation.
- `crates/project_store/src/lib.rs` - Added scheduler-controlled project bundle filesystem IO wrappers and revision freshness evidence.
- `.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-04-SUMMARY.md` - Captures execution outcome and verification evidence.

## Decisions Made

- Artifact refresh now creates durable artifact jobs and uses `JobDomain::ArtifactGeneration` with background CPU scheduling; binding code no longer invokes thumbnail generation or desktop FFmpeg execution directly.
- Media probe uses scheduler admission with queued/running status in product responses; the old synchronous import/probe behavior was intentionally not preserved through a wait.
- Project filesystem IO uses `JobDomain::FilesystemIo` and `ResourceClass::DiskIo`, with scheduler completion checked before session-visible state changes.
- Probe diagnostics, thumbnails, and waveform artifacts remain derived evidence only and are not treated as product preview success.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated stale artifact_store test enum references**
- **Found during:** Task 16-04-01 verification (`cargo test -p artifact_store artifact_jobs -- --nocapture`)
- **Issue:** Existing artifact_store tests imported `CommandName::{ImportMaterial, MoveSegment}`, but the current codebase uses `CommandDeltaName` for those invalidation test cases.
- **Fix:** Updated the tests to import and use `CommandDeltaName::{ImportMaterial, MoveSegment}`.
- **Files modified:** `crates/artifact_store/tests/artifact_generation.rs`, `crates/artifact_store/tests/invalidation.rs`
- **Verification:** `cargo test -p artifact_store artifact_jobs -- --nocapture` and `cargo test -p artifact_store artifact_generation -- --nocapture` passed.
- **Committed in:** `87c8fac` (part of GREEN implementation commit)

---

**Total deviations:** 1 auto-fixed blocking issue.
**Impact on plan:** Required to run the planned artifact_store verification against the current repository state. No scope expansion.

## Issues Encountered

- Rust tests emitted a pre-existing macOS deprecation warning for `objc2_av_foundation::AVAsset::tracksWithMediaType` in `media_runtime_desktop`; this plan did not modify that path.

## User Setup Required

None - no external service configuration required.

## Verification

- `cargo test -p bindings_node scheduler_artifact_probe -- --nocapture` - passed, 5 tests
- `cargo test -p artifact_store artifact_jobs -- --nocapture` - passed, 6 tests
- `cargo test -p artifact_store artifact_generation -- --nocapture` - passed, 7 tests
- `cargo test -p media_runtime material_probe -- --nocapture` - passed, 6 tests
- `cargo test -p project_store save_project_bundle -- --nocapture` - passed, 2 tests
- `cargo test -p project_store open_project_bundle -- --nocapture` - passed, 5 tests
- `rg -n "generate_thumbnail_artifact\\(|DesktopFfmpegExecutor::with_timeout|executor\\.run\\(&self\\.runtime\\.ffmpeg\\.path" crates/bindings_node/src/artifact_store_service.rs | awk 'END {exit (NR == 0 ? 0 : 1)}'` - passed
- `rg -n "probe_material_metadata\\(|submit.*wait|wait.*probe|block.*probe|probe.*wait" crates/bindings_node/src/material_service.rs crates/bindings_node/src/project_session_service.rs | awk 'END {exit (NR == 0 ? 0 : 1)}'` - passed
- `git diff --check -- . ':!reference'` - passed

## Known Stubs

None found in files created or modified by this plan.

## Threat Flags

None. The plan intentionally touched artifact/probe/project IO scheduler boundaries; no unplanned network, auth, file trust boundary, or schema surface was introduced.

## Next Phase Readiness

Plan 16-06 can now exercise slow and saturated artifact/probe/IO pressure through normal desktop workflows with scheduler status, cancellation, and stale-result rejection already available at the binding boundary.

## Self-Check: PASSED

- Summary file exists at `.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-04-SUMMARY.md`.
- Task commits `23ec771` and `87c8fac` exist in git history.
- Required verification commands are recorded above and passed before summary creation.

---
*Phase: 16-task-scheduler-job-isolation-and-performance-telemetry*
*Completed: 2026-06-23*
