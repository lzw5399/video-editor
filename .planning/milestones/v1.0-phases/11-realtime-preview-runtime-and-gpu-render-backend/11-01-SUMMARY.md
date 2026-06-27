---
phase: 11-realtime-preview-runtime-and-gpu-render-backend
plan: 01
subsystem: runtime
tags: [rust, realtime-preview, timeline-clock, telemetry, cancellation]

requires:
  - phase: 10.1-usable-editor-mvp-completion
    provides: command-owned desktop MVP and accepted draft/render graph foundations
provides:
  - Rust workspace crate for realtime preview runtime contracts
  - TimelineClock and PlaybackGeneration contracts using integer microseconds and rational rates
  - Session/request/result/telemetry/diagnostic/fallback contracts for mock preview runtime behavior
  - Tests for generation advancement, stale rejection, canceled request handling, and serialization
affects: [phase-11, realtime-preview-runtime, preview-service, bindings-node, scheduler, audio-engine]

tech-stack:
  added: []
  patterns:
    - generation-gated preview presentation
    - opaque Rust-allocated preview session IDs
    - serializable fallback and diagnostic contracts

key-files:
  created:
    - crates/realtime_preview_runtime/Cargo.toml
    - crates/realtime_preview_runtime/src/clock.rs
    - crates/realtime_preview_runtime/src/session.rs
    - crates/realtime_preview_runtime/src/request.rs
    - crates/realtime_preview_runtime/src/telemetry.rs
    - crates/realtime_preview_runtime/src/diagnostics.rs
    - crates/realtime_preview_runtime/src/fallback.rs
    - crates/realtime_preview_runtime/tests/clock_generation.rs
    - crates/realtime_preview_runtime/tests/stale_frame_rejection.rs
    - crates/realtime_preview_runtime/tests/cancellation_telemetry.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/realtime_preview_runtime/src/lib.rs

key-decisions:
  - "Phase 11 Plan 01 keeps realtime preview work at the Rust contract/mock-runtime layer; GPU, FFmpeg fallback execution, audio, scheduler, and platform decode remain deferred to later plans."
  - "Preview frame presentation is generation-gated: stale request generations and canceled request tokens return non-presented results with telemetry and diagnostics."

patterns-established:
  - "TimelineClock: integer Microseconds position, RationalFrameRate, rational PlaybackRate, PlaybackState, and PlaybackGeneration."
  - "RealtimePreviewRuntime: Rust-allocated PreviewSessionId registry with mock request handling and bounded aggregate telemetry counters."

requirements-completed: [RTPREV-01, RTPREV-05]

duration: 9 min
completed: 2026-06-18
---

# Phase 11 Plan 01: Realtime Preview Runtime Contracts Summary

**Rust-owned realtime preview runtime contract crate with generation-gated mock presentation, cancellation telemetry, and serializable diagnostics.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-18T14:56:20Z
- **Completed:** 2026-06-18T15:05:34Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments

- Added `realtime_preview_runtime` as a Rust workspace crate without adding third-party dependencies beyond existing workspace crates and serde.
- Implemented `TimelineClock`, `PlaybackGeneration`, `PlaybackState`, and rational `PlaybackRate` using `Microseconds` and `RationalFrameRate`.
- Added session, request, result, telemetry, diagnostic, and fallback contracts with mock runtime behavior for stale and canceled requests.
- Added focused Rust integration tests for clock generation, serialization, stale rejection, cancellation telemetry, fallback serialization, and diagnostic fields.

## Task Commits

1. **Task 11-01-01 RED: clock generation tests** - `f4ced5a` (test)
2. **Task 11-01-01 GREEN: timeline clock implementation** - `5730e3c` (feat)
3. **Task 11-01-02 RED: session contract tests** - `bd40baa` (test)
4. **Task 11-01-02 GREEN: session/request/telemetry contracts** - `a7b9a30` (feat)
5. **Formatting follow-up: cargo fmt test cleanup** - `a319f8b` (style)

## Files Created/Modified

- `Cargo.toml` - Adds `crates/realtime_preview_runtime` to the workspace.
- `Cargo.lock` - Records the new workspace package.
- `crates/realtime_preview_runtime/Cargo.toml` - Defines the runtime crate and its workspace dependencies.
- `crates/realtime_preview_runtime/src/lib.rs` - Exports the public runtime contract surface.
- `crates/realtime_preview_runtime/src/clock.rs` - Implements clock, generation, playback state, and rational playback rate contracts.
- `crates/realtime_preview_runtime/src/session.rs` - Implements opaque sessions, configuration, mock frame request handling, and cancellation token registry.
- `crates/realtime_preview_runtime/src/request.rs` - Defines frame request/result, request modes, backend used, and cancellation token contracts.
- `crates/realtime_preview_runtime/src/telemetry.rs` - Defines deterministic aggregate preview telemetry counters and latency fields.
- `crates/realtime_preview_runtime/src/diagnostics.rs` - Defines serializable diagnostic domain/support/fallback/cancellation fields.
- `crates/realtime_preview_runtime/src/fallback.rs` - Defines serializable fallback reasons.
- `crates/realtime_preview_runtime/tests/clock_generation.rs` - Verifies generation advancement, integer microsecond serialization, and rational rates.
- `crates/realtime_preview_runtime/tests/stale_frame_rejection.rs` - Verifies current-generation presentation and stale-generation rejection.
- `crates/realtime_preview_runtime/tests/cancellation_telemetry.rs` - Verifies canceled request handling, telemetry, fallback serialization, and diagnostics.

## Decisions Made

- Kept this plan limited to the runtime crate shell and mock contract behavior. No FFmpeg execution, GPU compositor, audio output, scheduler, or platform decode was introduced.
- Used Rust-allocated opaque `PreviewSessionId` and `PreviewCancellationToken` values so renderer-created GPU/device handles are not accepted by this contract layer.
- Made stale and canceled requests return structured non-presented results instead of errors, so bindings can display diagnostics and telemetry deterministically.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Verification

- `cargo test -p realtime_preview_runtime clock_generation -- --nocapture` - passed during Task 11-01-01.
- `cargo check -p realtime_preview_runtime --locked` - passed during Task 11-01-01 and Task 11-01-02.
- `cargo test -p realtime_preview_runtime stale_frame_rejection -- --nocapture` - passed during Task 11-01-02.
- `cargo test -p realtime_preview_runtime cancellation_telemetry -- --nocapture` - passed during Task 11-01-02 and final verification.
- `cargo test -p realtime_preview_runtime -- --nocapture` - passed final verification.
- `cargo check --workspace --locked` - passed final verification.

## Known Stubs

None.

## Threat Flags

None - new trust-boundary surfaces match the plan threat model and are covered by stale generation, opaque session ID, bounded telemetry, and cancellation tests.

## Next Phase Readiness

Ready for Plan 11-02 to prepare render graph inputs, runtime capability classification, and preview/export parity diagnostics on top of the shared runtime contracts.

## Self-Check: PASSED

- Verified all created runtime crate files and tests exist.
- Verified task commits exist: `f4ced5a`, `5730e3c`, `bd40baa`, `a7b9a30`, `a319f8b`.

---
*Phase: 11-realtime-preview-runtime-and-gpu-render-backend*
*Completed: 2026-06-18*
