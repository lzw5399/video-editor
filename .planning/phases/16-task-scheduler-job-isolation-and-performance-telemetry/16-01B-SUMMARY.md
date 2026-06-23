---
phase: 16-task-scheduler-job-isolation-and-performance-telemetry
plan: "01B"
subsystem: runtime
tags: [rust, scheduler, task-runtime, cancellation, telemetry, backpressure]

requires:
  - phase: 16-01
    provides: task_runtime workspace crate and canonical PlaybackGeneration freshness contracts
provides:
  - typed task_runtime scheduler job contracts for domain, priority, resource, freshness, cancellation, and result classification
  - portable TaskRuntimeConfig resource budgets and queue policies without desktop-only path or worker policy fields
  - deterministic JobScheduler admission, coalescing, cancellation, completion freshness gates, resource accounting, and telemetry snapshots
  - scheduler contract and telemetry tests covering backpressure, freshness, cancellation, summaries, and diagnostic classifications
affects: [phase-16, task-runtime, realtime-preview-runtime, audio-engine, artifact-store, media-runtime]

tech-stack:
  added: []
  patterns:
    - dependency-light Rust scheduler core using std collections and local serde contracts
    - completion-gated visible-state callback after cancellation and freshness checks
    - deterministic fake-clock tests for scheduler latency and resource accounting

key-files:
  created:
    - crates/task_runtime/src/cancellation.rs
    - crates/task_runtime/src/config.rs
    - crates/task_runtime/src/job.rs
    - crates/task_runtime/src/scheduler.rs
    - crates/task_runtime/src/telemetry.rs
    - crates/task_runtime/src/testing.rs
    - crates/task_runtime/tests/scheduler_contracts.rs
    - crates/task_runtime/tests/scheduler_telemetry.rs
  modified:
    - crates/task_runtime/src/lib.rs

key-decisions:
  - "Kept task_runtime portable: scheduler config contains resource budgets and queue policies only, with no Electron paths, FFmpeg paths, renderer tokens, retry policy, fallback policy, or desktop worker names."
  - "Modeled scheduler execution as deterministic admission/start/complete/cancel transitions so downstream domain adapters can plug in without introducing an async runtime dependency."
  - "Made completion callbacks run only after cancellation and freshness gates pass, so stale or canceled jobs cannot mutate visible preview/audio/artifact state."

patterns-established:
  - "Scheduler telemetry snapshots report bounded latency/duration summaries plus cancellation, stale, fallback, unavailable, cache, frame, depth, and saturation counters."
  - "Queue overflow policy is explicit per domain: stale-sensitive preview/audio queues can coalesce obsolete work while background/export queues reject when full."

requirements-completed: [SCHED-01, SCHED-03, SCHED-04]

duration: 16 min
completed: 2026-06-23
status: complete
---

# Phase 16 Plan 01B: Scheduler Core Contracts Summary

**Typed Rust scheduler core with bounded queue admission, cancellation/freshness completion gates, portable resource config, and deterministic telemetry coverage**

## Performance

- **Duration:** 16 min
- **Started:** 2026-06-23T14:24:00Z
- **Completed:** 2026-06-23T14:40:35Z
- **Tasks:** 3
- **Files modified:** 10

## Accomplishments

- Added typed scheduler contracts: `JobId`, `JobDomain`, `JobPriority`, `ResourceClass`, `JobFreshness`, `JobEnvelope`, `JobResult`, `SchedulerRejected`, `TaskCancellationToken`, `TaskRuntimeConfig`, `ResourceBudget`, and `QueuePolicy`.
- Implemented `JobScheduler` as a deterministic state machine for bounded submission, priority/resource-aware start, obsolete preview coalescing, rejection, cancellation, completion freshness rejection, and resource accounting.
- Added `SchedulerTelemetrySnapshot` and `SchedulerTelemetrySummary` with queue latency, wait/run/job duration, cancellation, stale rejection, fallback/unavailable classification, cache, first-frame, dropped/repeated frames, queue depth, and saturation counters.
- Added deterministic contract and telemetry tests using `task_runtime::testing::FakeClock`.

## Task Commits

1. **RED: scheduler contract and telemetry gates** - `d749bc1` (test)
2. **GREEN: task runtime scheduler contracts** - `c0125de` (feat)

## Files Created/Modified

- `crates/task_runtime/src/cancellation.rs` - Cloneable cancellation token shared by queued, running, and completion paths.
- `crates/task_runtime/src/config.rs` - Portable resource budgets and per-domain queue overflow policies.
- `crates/task_runtime/src/job.rs` - Typed job identity, domain, priority, resource, freshness, envelope, completion freshness, result, and diagnostic classification contracts.
- `crates/task_runtime/src/scheduler.rs` - Deterministic scheduler admission, queue selection, coalescing/rejection, cancellation, freshness-gated completion, and accounting.
- `crates/task_runtime/src/telemetry.rs` - Scheduler snapshot and summary aggregation for latency, duration, cancellation, stale, fallback, unavailable, cache, frame, depth, and saturation metrics.
- `crates/task_runtime/src/testing.rs` - Fake clock and scheduler test helper.
- `crates/task_runtime/src/lib.rs` - Exports the new scheduler modules and contracts.
- `crates/task_runtime/tests/scheduler_contracts.rs` - Contract tests for domains, config portability, cancellation, freshness, priority lanes, coalescing, rejection, and stale/canceled completions.
- `crates/task_runtime/tests/scheduler_telemetry.rs` - Telemetry tests for summaries, cancel/stale/reject/cache/fallback/unavailable/depth/saturation counters.

## Decisions Made

- Scheduler config remains platform-portable and does not encode Electron, FFmpeg binary paths, renderer frame tokens, retry/fallback policy, or desktop worker labels.
- Scheduler execution is intentionally deterministic and manually stepped; downstream preview/audio/export/artifact adapters can submit and complete jobs without a new async or scheduler crate.
- Canceled running jobs release scheduler accounting immediately and terminal completion is classified without double-counting cancellation telemetry.

## Verification

- `cargo test -p task_runtime scheduler_contracts -- --nocapture` - passed, 9 tests.
- `cargo test -p task_runtime config -- --nocapture` - passed, 1 filtered config test.
- `cargo test -p task_runtime scheduler_telemetry -- --nocapture` - passed, 2 tests.
- `cargo test -p task_runtime -- --nocapture` - passed, 15 tests total plus doctests.
- `cargo check --workspace --locked` - passed with one pre-existing `media_runtime_desktop` macOS AVFoundation deprecation warning.
- `git diff --check -- . ':!reference'` - passed.

## TDD Gate Compliance

- RED gate committed in `d749bc1`; `cargo test -p task_runtime scheduler_contracts -- --nocapture` failed because the scheduler contract modules were not yet exported.
- GREEN gate committed in `c0125de`; focused and full task runtime tests passed after implementation.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Initial GREEN run exposed a serde field-name mismatch for `JobFreshness::Timeline`; fixed with explicit camelCase field serialization so envelopes expose `targetTime` and `playbackGeneration`.
- The telemetry stress test initially tried to submit a third export while the export queue still held a waiting job; corrected the test to start and complete the already queued export, which better proves resource release and queue promotion.

## Known Stubs

None. Stub scan found no TODO/FIXME/placeholder or empty UI/runtime data patterns in files created or modified by this plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 16 integration plans can now route preview, audio, export, artifact, probe, filesystem IO, and analysis adapters through a shared Rust scheduler boundary with typed budgets, cancellation, freshness gates, and telemetry.

## Self-Check

PASSED

- Created files exist: `crates/task_runtime/src/cancellation.rs`, `crates/task_runtime/src/config.rs`, `crates/task_runtime/src/job.rs`, `crates/task_runtime/src/scheduler.rs`, `crates/task_runtime/src/telemetry.rs`, `crates/task_runtime/src/testing.rs`, `crates/task_runtime/tests/scheduler_contracts.rs`, `crates/task_runtime/tests/scheduler_telemetry.rs`, and this summary.
- Task commits found in git history: `d749bc1` and `c0125de`.
- Summary frontmatter includes `status: complete` and `requirements-completed: [SCHED-01, SCHED-03, SCHED-04]`.

---
*Phase: 16-task-scheduler-job-isolation-and-performance-telemetry*
*Completed: 2026-06-23*
