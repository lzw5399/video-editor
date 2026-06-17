---
phase: 05-preview-and-export-pipeline
plan: 07
subsystem: media-runtime
tags: [rust, ffmpeg, ffprobe, export, validation]
requires:
  - phase: 05-03
    provides: structured FFmpeg argument vectors and output validation intent
provides:
  - Rust-owned FFmpeg export job runtime primitives
  - Progress parsing with integer microsecond timing and per-mille progress
  - Cancel token, timeout handling, bounded stdout/stderr summaries, and classified runtime errors
  - Rendered output validation through ffprobe metadata
affects: [media_runtime, bindings_node, export-pipeline, desktop-export]
tech-stack:
  added: []
  patterns:
    - media_runtime consumes structured args and does not depend on ffmpeg_compiler
    - export job progress uses integer microseconds and bounded log summaries
    - output validation reuses material probe normalization
key-files:
  created:
    - crates/media_runtime/src/job.rs
    - crates/media_runtime/src/validate.rs
    - crates/media_runtime/tests/export_job.rs
    - crates/media_runtime/tests/output_validation.rs
  modified:
    - crates/media_runtime/src/lib.rs
key-decisions:
  - "Export runtime accepts explicit FFmpeg binary paths and Vec<OsString> arguments, preserving the no-shell-concatenation boundary."
  - "media_runtime does not depend on ffmpeg_compiler; service/binding layers will adapt compiled jobs into runtime jobs."
  - "Output validation maps ffprobe probe errors into export-specific validation error kinds."
patterns-established:
  - "Long-running export jobs report Started, Progress, and Completed events without renderer process ownership."
  - "Cancel returns a classified Cancelled job result; timeout and FFmpeg failures return structured errors."
  - "Duration, fps, resolution, audio stream, file existence, and non-empty size validation live in media_runtime."
requirements-completed: [EXP-01, EXP-03, EXP-04]
duration: 8 min
completed: 2026-06-17
---

# Phase 05 Plan 07: Export Runtime Summary

**Rust FFmpeg export job primitives with progress, cancellation, bounded logs, classified failures, and ffprobe output validation**

## Performance

- **Duration:** 8 min
- **Started:** 2026-06-17T18:31:00Z
- **Completed:** 2026-06-17T18:39:22Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Added `FfmpegRuntimeJob`, `FfmpegJobId`, `FfmpegProgress`, `FfmpegJobEvent`, `FfmpegJobResult`, `CancelToken`, and `run_export_job`.
- Added progress parsing for `out_time_us`, `out_time_ms`, and `out_time` while keeping time math integer-based.
- Added classified runtime failures for unavailable runtime, launch failure, timeout, non-zero exit, missing encoder, missing filter, and malformed progress.
- Added `OutputValidationExpectation`, `OutputValidationReport`, `OutputValidationError`, and `validate_rendered_output` using existing ffprobe material metadata normalization.
- Added focused tests for export progress/cancel/error/log behavior and output validation success/failure behavior.

## Task Commits

Each task was committed atomically:

1. **Task 05-07-01: Add streaming FFmpeg job runtime with progress, cancel, and bounded logs** - `65ad7c5` (feat)
2. **Task 05-07-02: Validate rendered MP4 output with ffprobe metadata** - `65ad7c5` (feat)

**Plan metadata:** this summary commit

## Files Created/Modified

- `crates/media_runtime/src/job.rs` - Owns export job state, progress parsing, cancellation, timeout, bounded logs, and runtime error classification.
- `crates/media_runtime/src/validate.rs` - Validates rendered outputs for file existence, size, duration, fps, resolution, and audio stream expectations.
- `crates/media_runtime/tests/export_job.rs` - Covers progress events, cancel, timeout, malformed progress, missing encoder/filter, and bounded output summaries.
- `crates/media_runtime/tests/output_validation.rs` - Covers validation success, missing/empty outputs, mismatches, malformed ffprobe JSON, missing streams, timeout, and probe failure.
- `crates/media_runtime/src/lib.rs` - Re-exports the new export runtime and output validation API.

## Decisions Made

- Kept `media_runtime` independent from `ffmpeg_compiler`; this preserves the Phase 5 layering where compiler emits structured jobs and runtime executes platform-specific processes.
- Used `Vec<OsString>` and `Command::args` throughout export runtime execution; no renderer input or FFmpeg command is shell-concatenated.
- Used per-mille progress instead of floating-point percentages to keep persisted and crossing semantics integer-based.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The cancel test initially had a scheduling race where cancellation could happen before the script emitted progress. The test now waits for a progress-written flag before cancelling, proving that emitted progress is preserved during cancellation close-out.

## Verification

- `cargo test -p media_runtime export_job -- --nocapture` - passed, 4 tests.
- `cargo test -p media_runtime output_validation -- --nocapture` - passed, 4 tests.
- `cargo test -p media_runtime -- --nocapture` - passed, 19 tests.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for `05-08` to expose export commands and desktop UI on top of Rust-owned runtime primitives. `05-05` and `05-06` should still connect preview command/UI paths before export UI integration.

---
*Phase: 05-preview-and-export-pipeline*
*Completed: 2026-06-17*
