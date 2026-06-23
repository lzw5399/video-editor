---
phase: 16-task-scheduler-job-isolation-and-performance-telemetry
plan: "02"
subsystem: runtime-scheduler
tags: [rust, task-runtime, realtime-preview, audio, telemetry, starvation-tests]

requires:
  - phase: 16-01B
    provides: "task_runtime scheduler contracts, freshness gates, resource budgets, and telemetry snapshots"
provides:
  - "Realtime preview first-frame, seek, and playback tick admission through task_runtime"
  - "Audio refill and decode-window admission through task_runtime lanes"
  - "Preview/audio scheduler telemetry propagation"
  - "Deterministic starvation tests for preview, scrub, analysis, decode, and audio under background pressure"
affects: [bindings_node, realtime_preview_runtime, audio_engine, task_runtime, phase16-source-guards]

tech-stack:
  added: []
  patterns:
    - "Binding services may own native handles and drivers, but task_runtime owns queue admission, freshness, cancellation, and scheduler telemetry."
    - "Visible preview/audio state commits are guarded by CompletionFreshness::playback_generation."

key-files:
  created:
    - crates/task_runtime/tests/starvation.rs
  modified:
    - crates/bindings_node/src/realtime_preview_service.rs
    - crates/bindings_node/src/audio_service.rs
    - crates/bindings_node/tests/scheduler_preview_audio.rs
    - crates/realtime_preview_runtime/src/session.rs
    - crates/realtime_preview_runtime/src/telemetry.rs
    - crates/audio_engine/src/session.rs
    - crates/audio_engine/src/telemetry.rs

key-decisions:
  - "Kept RenderGraphGpuComposited as the only product preview success evidence; scheduler admission wraps the compositor path instead of replacing it."
  - "Used task_runtime JobScheduler inside preview/audio binding adapters while leaving draft, compositor, and audio output semantics in Rust domain crates."
  - "Stopped audio output on seek/cancel so stale refill jobs cannot continue enqueueing audio after generation changes."

patterns-established:
  - "Scheduler adapter pattern: submit JobEnvelope, start_next, execute domain work, then complete_with_commit with CompletionFreshness before mutating visible state."
  - "Source guard pattern: bindings_node tests fail on legacy binding-owned worker/poll/direct-executor strings and require task_runtime domain/resource evidence."

requirements-completed: [SCHED-01, SCHED-02, SCHED-04]

duration: "not captured by local executor"
completed: 2026-06-23
status: complete
---

# Phase 16 Plan 02: Preview and Audio Scheduler Isolation Summary

**Preview and audio work now enter task_runtime lanes with generation-aware commits and starvation tests under background pressure.**

## Performance

- **Duration:** Not captured by local executor
- **Started:** Not captured
- **Completed:** 2026-06-23T15:15:23Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- Replaced preview binding-owned still-frame/playback worker policy with task_runtime admission for first-frame, seek, and playback tick work.
- Routed native audio refill and FFmpeg decode-window work through task_runtime Audio and Decode lanes.
- Added preview/audio telemetry fields for scheduler queue latency, queue depth, saturation, rejection, cancellation, and stale rejection.
- Added source guards for preview/audio bypass patterns and deterministic task_runtime starvation tests.

## Task Commits

1. **Task 16-02-01 RED: Preview scheduler admission guard** - `cfdf8cc` (test)
2. **Task 16-02-01 GREEN: Preview admission through task_runtime** - `a427471` (feat)
3. **Task 16-02-02 RED: Audio scheduler admission guard** - `537963f` (test)
4. **Task 16-02-02 GREEN: Audio refill/decode scheduler lanes** - `4a15dda` (feat)
5. **Task 16-02-03: Starvation resistance tests** - `3696f88` (test)

## Files Created/Modified

- `crates/bindings_node/src/realtime_preview_service.rs` - submits preview work through `task_runtime::JobScheduler`, gates visible preview commits on freshness, and removes legacy worker maps/thread labels.
- `crates/realtime_preview_runtime/src/session.rs` - records scheduler telemetry snapshots into preview sessions.
- `crates/realtime_preview_runtime/src/telemetry.rs` - adds scheduler aggregate telemetry fields.
- `crates/bindings_node/src/audio_service.rs` - submits audio refill and decode-window work through scheduler lanes and removes legacy refill loop/direct decode guard patterns.
- `crates/audio_engine/src/session.rs` - records scheduler telemetry snapshots into audio sessions.
- `crates/audio_engine/src/telemetry.rs` - adds scheduler aggregate telemetry fields.
- `crates/bindings_node/tests/scheduler_preview_audio.rs` - guards preview/audio bindings against legacy bypass patterns and requires task_runtime evidence.
- `crates/task_runtime/tests/starvation.rs` - proves interactive preview/audio/decode/analysis starts under export/artifact/probe/filesystem pressure and stale/canceled completions do not mutate visible state.

## Decisions Made

- Scheduler admission wraps the existing product compositor path instead of adding fallback preview success. The only visible preview evidence remains `RenderGraphGpuComposited`.
- Audio output still uses the native CPAL output boundary, but refill and decode-window work now pass through shared scheduler domains/resources before samples are enqueued.
- Seek/cancel stops active native audio output so old-generation refill jobs cannot continue after playback generation changes.

## Deviations from Plan

None - plan scope was executed as written.

## Issues Encountered

- The first starvation test run used an exact `2_000us` p95 queue-latency assertion, but jobs submitted at `1us` correctly produced `1_999us`. The assertion was changed to a `<= 2_000us` budget before the test was committed.

## Verification

- `cargo test -p task_runtime starvation -- --nocapture` - passed, 2 starvation tests.
- `cargo test -p bindings_node scheduler_preview_audio -- --nocapture` - passed, 2 binding source guard tests.
- `cargo test -p realtime_preview_runtime realtime_playback -- --nocapture` - passed, filtered runtime playback targets.
- `cargo test -p audio_engine -- --nocapture` - passed, 6 audio engine tests.
- Preview source guard for binding-owned preview worker patterns - passed.
- Audio source guard for binding-owned refill/direct FFmpeg patterns - passed.
- `git diff --check -- . ':!reference'` - passed.

## Known Stubs

None.

## Threat Flags

None - the new scheduler admission and completion gates implement the plan threat mitigations and add no unplanned trust boundary.

## Self-Check: PASSED

- Summary file created at `.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-02-SUMMARY.md`.
- Task commits found: `cfdf8cc`, `a427471`, `537963f`, `4a15dda`, `3696f88`.
- No `.planning/STATE.md` or `.planning/ROADMAP.md` updates were made.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 16 follow-up plans can route export, artifact, probe, and filesystem work through the same scheduler contracts. Preview/audio source guards are in place to prevent regression to binding-owned worker maps, thread labels, refill loops, or direct decode bypasses.

---
*Phase: 16-task-scheduler-job-isolation-and-performance-telemetry*
*Completed: 2026-06-23*
