---
phase: 11-realtime-preview-runtime-and-gpu-render-backend
plan: 05
subsystem: realtime-preview-runtime
tags: [rust, realtime-preview, preview-service, fallback, telemetry, tdd]

requires:
  - phase: 11-realtime-preview-runtime-and-gpu-render-backend
    provides: Runtime session/clock, graph capability, H.264 software frame provider, GPU compositor, and binding session contracts from Plans 11-01 through 11-04B
provides:
  - Preview service realtime backend routing before artifact fallback
  - H.264 material preview through Rust-owned software frame provider cache
  - Rust-owned fallback ladder decisions with artifact-generation diagnostics
  - Binding response propagation for backend, fallback, cancellation, stale, diagnostics, and telemetry data
affects: [phase-11, phase-12-media-io, phase-16-scheduler, phase-17-bindings, realtime-preview-runtime]

tech-stack:
  added: []
  patterns:
    - Preview service owns realtime/fallback selection before FFmpeg artifact generation
    - Binding responses serialize diagnostics data without making fallback decisions
    - Canceled and stale realtime requests short-circuit before artifact presentation

key-files:
  created:
    - crates/preview_service/src/realtime_backend.rs
    - crates/preview_service/src/realtime_frame_provider.rs
    - crates/preview_service/tests/realtime_backend_no_ffmpeg.rs
    - crates/preview_service/tests/realtime_video_material.rs
    - crates/preview_service/tests/fallback_ladder.rs
    - crates/preview_service/tests/cancellation_telemetry.rs
  modified:
    - Cargo.lock
    - crates/preview_service/Cargo.toml
    - crates/preview_service/src/lib.rs
    - crates/preview_service/src/service.rs
    - crates/bindings_node/src/realtime_preview_service.rs
    - crates/bindings_node/tests/preview_commands.rs

key-decisions:
  - "Supported preview frame requests route through preview_service realtime backend classification before artifact fallback."
  - "Fallback decisions preserve the Rust-owned ladder reason while runtime results mark FFmpeg artifact generation explicitly as fallback."
  - "Bindings expose fallback, cancellation, diagnostics, and telemetry as serialized data without exposing GPU internals or selecting fallback paths."

patterns-established:
  - "Use RealtimePreviewServiceConfig as the preview_service-owned coordinator for runtime session, fallback classification, cancellation, and artifact fallback."
  - "Use RealtimeMaterialFrameProvider as the service-facing material frame provider alias for the Phase 11 software H.264 cache."
  - "Use separate decision and runtime fallback fields to distinguish why fallback was chosen from whether FFmpeg artifact generation ran."

requirements-completed: [RTPREV-01, RTPREV-03, RTPREV-04, RTPREV-05]

duration: 11min
completed: 2026-06-18
---

# Phase 11 Plan 05: Realtime Backend Routing And Fallback Telemetry Summary

**Preview service realtime routing for supported H.264 seek/scrub requests with Rust-owned fallback diagnostics and binding telemetry**

## Performance

- **Duration:** 11 min
- **Started:** 2026-06-18T17:16:57Z
- **Completed:** 2026-06-18T17:28:08Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments

- Added `RealtimePreviewServiceConfig`, `RealtimePreviewFrameServiceRequest`, `RealtimePreviewServiceFrameResponse`, and `request_realtime_preview_frame` in `preview_service`.
- Routed supported first-frame, seek, and scrub requests through the realtime runtime and mock/offscreen compositor before the existing preview artifact path.
- Proved generated H.264 material frames served by the Phase 11 software provider do not invoke FFmpeg per preview request.
- Added fallback ladder coverage for no adapter, unavailable surface, missing frame provider, preview artifact cache hit, and FFmpeg artifact generation.
- Extended Node binding realtime preview frame responses with fallback reason, cancellation token, diagnostics, and telemetry while keeping GPU/native internals out of serialized data.

## Task Commits

1. **Task 11-05-01 RED:** `a33dae9` test: add failing realtime backend routing tests.
2. **Task 11-05-01 GREEN:** `1490460` feat: route supported previews through realtime backend.
3. **Task 11-05-02 RED:** `6b4ae72` test: add failing fallback telemetry tests.
4. **Task 11-05-02 GREEN:** `2373f06` feat: propagate fallback telemetry diagnostics.

**Plan metadata:** pending final docs commit.

## Files Created/Modified

- `crates/preview_service/src/realtime_backend.rs` - Realtime preview service coordinator, graph preparation, capability/fallback classification, cancellation/stale short-circuiting, artifact fallback, and telemetry response assembly.
- `crates/preview_service/src/realtime_frame_provider.rs` - Service-facing alias for the Rust-owned software material frame provider.
- `crates/preview_service/src/lib.rs` - Public exports for realtime backend and material frame provider contracts.
- `crates/preview_service/src/service.rs` - Crate-visible preview error constructor reused by realtime routing.
- `crates/preview_service/Cargo.toml` and `Cargo.lock` - Existing workspace runtime dependency wiring for preview service.
- `crates/preview_service/tests/realtime_backend_no_ffmpeg.rs` - Supported graph no-FFmpeg regression.
- `crates/preview_service/tests/realtime_video_material.rs` - H.264 material multi-time realtime routing regression.
- `crates/preview_service/tests/fallback_ladder.rs` - Fallback reason/cache/FFmpeg artifact diagnostics coverage.
- `crates/preview_service/tests/cancellation_telemetry.rs` - Canceled request no-artifact presentation and telemetry coverage.
- `crates/bindings_node/src/realtime_preview_service.rs` - Binding DTO propagation for fallback/cancellation/diagnostics/telemetry.
- `crates/bindings_node/tests/preview_commands.rs` - Binding contract test for serialized fallback telemetry without GPU internals.

## Decisions Made

- Preview service, not Electron or renderer code, owns realtime-vs-fallback classification.
- Supported H.264 preview uses the Phase 11 software frame provider cache; FFmpeg artifact generation is only invoked for fallback paths and is reported as `FfmpegArtifactGenerated`.
- Canceled or stale-generation requests return runtime telemetry and diagnostics without presenting or generating preview artifacts.
- Binding responses carry backend/fallback/cache/stale/cancellation/latency/parity diagnostics as serializable data; they do not expose GPU devices, native handles, command encoders, or cache keys.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed test filter coverage for new realtime tests**
- **Found during:** Task 11-05-01 verification
- **Issue:** Required `cargo test -p preview_service realtime_backend_no_ffmpeg` and `realtime_video_material` filters initially compiled but filtered out the new assertions because test function names did not include the filter strings.
- **Fix:** Renamed test functions with `realtime_backend_no_ffmpeg_` and `realtime_video_material_` prefixes.
- **Files modified:** `crates/preview_service/tests/realtime_backend_no_ffmpeg.rs`, `crates/preview_service/tests/realtime_video_material.rs`
- **Verification:** Both filtered commands ran one assertion-bearing test each.
- **Committed in:** `1490460`

**2. [Rule 1 - Bug] Kept mock backend preference synchronized with runtime session config**
- **Found during:** Task 11-05-01 verification
- **Issue:** `with_mock_realtime_backend()` updated compositor backend selection but left the already-created runtime session configured for the default offscreen backend.
- **Fix:** Rebuilt the runtime session when changing backend preference.
- **Files modified:** `crates/preview_service/src/realtime_backend.rs`
- **Verification:** `cargo test -p preview_service realtime_video_material -- --nocapture` passed and returned `Mock` backend for H.264 realtime preview.
- **Committed in:** `1490460`

**Total deviations:** 2 auto-fixed Rule 1 bugs.
**Impact on plan:** Both fixes were required for verification correctness and backend diagnostic accuracy; no scope expansion.

## Known Stubs

None.

## Threat Flags

None - new preview service routing, artifact fallback, and binding telemetry surfaces were covered by the plan threat model.

## Issues Encountered

- `gsd-tools` was not on PATH, so GSD SDK commands used `node /Users/zhiwen/.codex/get-shit-done/bin/gsd-tools.cjs`.
- `cargo fmt --all` touched unrelated pre-existing runtime test files; those formatting-only changes were reverted before commits.
- No authentication gates or package installs were required.

## Verification

- `cargo test -p preview_service realtime_backend_no_ffmpeg -- --nocapture` - passed; 1 realtime no-FFmpeg test ran.
- `cargo test -p preview_service realtime_video_material -- --nocapture` - passed; 1 H.264 realtime material test ran.
- `cargo test -p preview_service fallback_ladder -- --nocapture` - passed; 2 fallback ladder tests ran.
- `cargo test -p preview_service cancellation_telemetry -- --nocapture` - passed; 1 cancellation telemetry test ran.
- `cargo test -p bindings_node preview_commands -- --nocapture` - passed; 4 binding preview command tests ran.
- `cargo test -p preview_service preview_generation -- --nocapture` - passed; 3 existing preview artifact generation tests ran.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 11-05B can build on a preview service path where supported interactive preview requests are realtime-first, fallback artifacts are explicitly diagnostic-rich, and bindings can display telemetry without owning fallback decisions.

## Self-Check: PASSED

- Verified created files exist: `realtime_backend.rs`, `realtime_frame_provider.rs`, `realtime_backend_no_ffmpeg.rs`, `realtime_video_material.rs`, `fallback_ladder.rs`, `cancellation_telemetry.rs`, and this summary.
- Verified task commits exist: `a33dae9`, `1490460`, `6b4ae72`, `2373f06`.
- Verified required verification commands passed.
- Verified `reference/` remains untracked and unstaged.

---
*Phase: 11-realtime-preview-runtime-and-gpu-render-backend*
*Completed: 2026-06-18*
