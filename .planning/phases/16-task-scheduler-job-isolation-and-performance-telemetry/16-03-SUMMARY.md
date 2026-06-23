---
phase: 16-task-scheduler-job-isolation-and-performance-telemetry
plan: "03"
subsystem: runtime
tags: [rust, scheduler, export, cancellation, telemetry, bindings-node]

# Dependency graph
requires:
  - phase: 16-01B
    provides: task_runtime scheduler lanes, resource classes, bounded admission, and telemetry snapshot contracts
provides:
  - Scheduler-backed export admission/status/cancel facade in bindings_node
  - Export validation tracked through task_runtime telemetry
  - Export scheduler contract tests for status, cancellation, saturation, and interactive lane non-starvation
affects: [task_runtime, media_runtime, bindings_node, export, scheduler, telemetry]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Binding API submits heavy export and validation work to task_runtime instead of owning worker policy
    - media_runtime remains the FFmpeg process executor behind scheduler-managed admission
    - Product status exposes scheduler telemetry without moving render semantics into Electron or bindings

key-files:
  created:
    - crates/bindings_node/tests/scheduler_export.rs
  modified:
    - crates/bindings_node/src/preview_export_service.rs
    - crates/task_runtime/src/scheduler.rs
    - crates/task_runtime/src/telemetry.rs

key-decisions:
  - "Keep media_runtime::run_export_job as the FFmpeg executor while moving export admission, validation, status, and cancellation ownership to task_runtime."
  - "Expose scheduler job identity and telemetry in export status so queue/run/cancel/saturation state is visible at the binding boundary."
  - "Use timestamped scheduler cancellation paths so queued and running export cancellation records wait/run/duration samples."

patterns-established:
  - "SchedulerExportService: binding-facing export facade that delegates heavy work and validation to task_runtime."
  - "Export validation telemetry: validation runs as scheduler-visible work and cannot overwrite a terminal cancelled export status."
  - "Scheduler cancellation telemetry: cancellation records wait/run/job duration based on whether work was queued or running."

requirements-completed: [SCHED-01, SCHED-02, SCHED-04]

# Metrics
duration: 30min
completed: 2026-06-23
status: complete
---

# Phase 16 Plan 03: Export Scheduler Boundary Summary

**Scheduler-managed export admission, cancellation, validation, and telemetry while preserving media_runtime as the FFmpeg process executor**

## Performance

- **Duration:** 30 min
- **Started:** 2026-06-23T15:13:00Z
- **Completed:** 2026-06-23T15:42:55Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Replaced binding-owned export worker registry behavior with a scheduler-backed export service.
- Routed export execution through `task_runtime` using typed export domain/resource policy while keeping `media_runtime::run_export_job` responsible for FFmpeg process execution.
- Added deterministic queued/running export cancellation with scheduler-visible cancellation, wait/run/duration telemetry, and terminal-status protection.
- Represented export validation as scheduler-visible work so validation telemetry cannot hide starvation or overwrite cancellation.
- Added binding-level export scheduler tests covering status telemetry, cancellation, queue rejection, source guard behavior, and interactive lane non-starvation.

## Task Commits

The TDD plan was committed as RED and GREEN gates:

1. **RED gate: failing export scheduler contracts** - `e3318b4` (`test`)
2. **GREEN gate: scheduler-backed export implementation** - `5f07bb8` (`feat`)
3. **Plan metadata: summary** - committed after this file was written

## Files Created/Modified

- `crates/bindings_node/tests/scheduler_export.rs` - New scheduler export contract tests for status, cancellation, telemetry, queue rejection, and source guard coverage.
- `crates/bindings_node/src/preview_export_service.rs` - Replaced the old export registry worker policy with scheduler-backed export admission/status/cancel/validation handling.
- `crates/task_runtime/src/scheduler.rs` - Added timestamped cancellation entry point used by export status/telemetry accounting.
- `crates/task_runtime/src/telemetry.rs` - Added cancellation telemetry recording for queued/running scheduler jobs.
- `.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-03-SUMMARY.md` - Plan execution summary.

## Decisions Made

- Export work now enters `task_runtime` as `JobDomain::Export` with export/validation resource classes instead of a binding-owned unbounded export thread registry.
- `media_runtime::run_export_job` remains the sole FFmpeg execution boundary; bindings prepare and submit export work but do not construct FFmpeg commands.
- Export status includes scheduler job metadata and telemetry alongside product-safe export status/progress.
- Cancellation is scheduler-owned first, then propagated to the `media_runtime` cancel token, preventing late FFmpeg or validation events from rewriting terminal cancelled status.

## Deviations from Plan

None - plan executed as written. The timestamped scheduler cancellation API was an implementation detail required to satisfy Task 16-03-02 telemetry acceptance criteria.

## Issues Encountered

- The TDD RED run failed as expected against the old `ExportJobRegistry` behavior: queued exports immediately ran outside scheduler accounting and status did not expose scheduler telemetry.
- Cargo verification emitted an existing macOS AVFoundation deprecation warning from `crates/media_runtime_desktop/src/platform/macos.rs`; it is unrelated to this plan.

## Verification

- `cargo test -p bindings_node scheduler_export -- --nocapture --test-threads=1` - passed, 6 tests.
- `cargo test -p media_runtime export_job -- --nocapture` - passed, 4 export job tests.
- `cargo test -p task_runtime scheduler_telemetry -- --nocapture` - passed, 2 scheduler telemetry tests.
- `rg -n "ExportJobRegistry|thread::spawn\\(move \\|\\|" crates/bindings_node/src/preview_export_service.rs | awk 'END {exit (NR == 0 ? 0 : 1)}'` - passed, no legacy registry or direct unbounded spawn pattern found.
- `git diff --check -- . ':!reference'` - passed.
- Extra regression: `cargo test -p bindings_node export_commands -- --nocapture --test-threads=1` - passed, 7 export command tests.

## Known Stubs

None. The stub scan only found `last=""` shell variables inside fake FFmpeg scripts in `crates/bindings_node/tests/scheduler_export.rs`; those are intentional local script state for tests, not product stubs.

## Threat Flags

None. The implementation stayed within the plan's declared trust boundaries: binding export API to scheduler, scheduler to `media_runtime`, and export completion to product status.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Export is now a scheduler-managed heavy lane with bounded admission, deterministic cancellation, and visible scheduler telemetry. Later phases can build additional export UI or orchestration on top of the scheduler status surface without reintroducing binding-owned worker policy.

## Self-Check: PASSED

- Summary file exists at `.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-03-SUMMARY.md`.
- Implementation commits `e3318b4` and `5f07bb8` are reachable in git history.
- Source guard passed for `ExportJobRegistry|thread::spawn(move ||)` in `crates/bindings_node/src/preview_export_service.rs`.
- `git diff --check -- . ':!reference'` passed.

---
*Phase: 16-task-scheduler-job-isolation-and-performance-telemetry*
*Completed: 2026-06-23*
