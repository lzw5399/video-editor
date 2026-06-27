---
phase: 11-realtime-preview-runtime-and-gpu-render-backend
plan: 03
subsystem: realtime-preview-runtime
tags: [rust, realtime-preview, frame-provider, h264, cache, tdd]

requires:
  - phase: 11-realtime-preview-runtime-and-gpu-render-backend
    provides: Realtime preview clock/session contracts and render graph capability diagnostics from Plans 11-01 and 11-02
provides:
  - Validated CPU RGBA/static image frame provider contracts
  - Opaque texture handle descriptor contract for Phase 12 interop
  - Session-owned decoded H.264 software frame cache and provider
  - Testkit deterministic H.264 preview fixture helper
affects: [phase-11, phase-12-media-io, realtime-preview-runtime, gpu-compositor]

tech-stack:
  added: []
  patterns:
    - Integer-microsecond source position to frame-index cache lookup
    - Preview frame contracts remain renderer/platform/FFmpeg neutral
    - Generated H.264 fixtures are created before realtime requests, never inside frame_for

key-files:
  created:
    - crates/realtime_preview_runtime/src/frame_provider.rs
    - crates/realtime_preview_runtime/src/software_video_provider.rs
    - crates/realtime_preview_runtime/tests/frame_provider.rs
    - crates/realtime_preview_runtime/tests/video_frame_provider.rs
  modified:
    - Cargo.lock
    - crates/realtime_preview_runtime/Cargo.toml
    - crates/realtime_preview_runtime/src/lib.rs
    - crates/testkit/src/lib.rs

key-decisions:
  - "Frame provider contracts carry material id, integer source position, and PlaybackGeneration with validated CPU RGBA buffers."
  - "SoftwareVideoFrameProvider serves only preloaded generated H.264 frames from a session-owned cache and never runs per-request FFmpeg work."
  - "Texture handles are opaque serializable descriptors only; native pointer/texture interop remains Phase 12 scope."

patterns-established:
  - "Use DecodedVideoFrameCache for session-owned decoded CPU frames keyed by material id and frame index."
  - "Use typed PreviewFrameProviderError variants for unavailable, unsupported codec, out-of-range, and invalid-frame diagnostics."

requirements-completed: [RTPREV-02, RTPREV-03, RTPREV-05]

duration: 15min
completed: 2026-06-18
---

# Phase 11 Plan 03: Frame Provider And H.264 Software Cache Summary

**Validated CPU/static frame contracts and a cache-only H.264 software frame provider for realtime preview inputs**

## Performance

- **Duration:** 15 min
- **Started:** 2026-06-18T16:08:50Z
- **Completed:** 2026-06-18T16:23:25Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Added `PreviewFrameProvider`, `PreviewFrameInput`, `CpuVideoFrame`, `FrameColorInfo`, `PreviewFrameProviderError`, and `TextureHandleDescriptor`.
- Added `DecodedVideoFrameCache` and `SoftwareVideoFrameProvider` that map integer microsecond source positions to cached frame indices.
- Added deterministic H.264 fixture-backed tests proving `frame_for` returns preloaded frames and does not invoke per-request process work.

## Task Commits

1. **Task 11-03-01 RED:** `e397bb2` test: add failing frame provider contract tests.
2. **Task 11-03-01 GREEN:** `275c0b9` feat: implement frame provider contracts.
3. **Task 11-03-02 RED:** `fbfd2f4` test: add failing software video provider tests.
4. **Task 11-03-02 GREEN:** `f05373b` feat: implement software video frame cache.

## Files Created/Modified

- `crates/realtime_preview_runtime/src/frame_provider.rs` - Frame provider trait, CPU/static/texture frame input contracts, validation, and typed provider errors.
- `crates/realtime_preview_runtime/src/software_video_provider.rs` - Session-owned decoded H.264 cache and cache-only software provider.
- `crates/realtime_preview_runtime/src/lib.rs` - Public exports for frame/provider/cache contracts.
- `crates/realtime_preview_runtime/tests/frame_provider.rs` - Contract validation tests.
- `crates/realtime_preview_runtime/tests/video_frame_provider.rs` - H.264 fixture-backed software cache/provider tests.
- `crates/realtime_preview_runtime/Cargo.toml` - Adds `testkit` as a path dev-dependency for fixture-backed tests.
- `crates/testkit/src/lib.rs` - Adds `generate_h264_preview_fixture`.
- `Cargo.lock` - Records the path dev-dependency edge.

## Decisions Made

- Cached software video frames use `RationalFrameRate` and integer microsecond arithmetic to select frame indices.
- Cache entries are explicitly codec-labeled; non-H.264 entries return `UnsupportedCodec` rather than falling back silently.
- Missing decoded frames and out-of-range positions are distinct typed errors for clearer preview diagnostics.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed Cargo test filter coverage for frame provider tests**
- **Found during:** Task 11-03-01 verification
- **Issue:** `cargo test -p realtime_preview_runtime frame_provider -- --nocapture` initially compiled but filtered out the new test functions because their names did not include `frame_provider`.
- **Fix:** Renamed the new test functions with a `frame_provider_` prefix so the required gate runs the assertions.
- **Files modified:** `crates/realtime_preview_runtime/tests/frame_provider.rs`
- **Verification:** `cargo test -p realtime_preview_runtime frame_provider -- --nocapture` runs 5 frame-provider tests.
- **Committed in:** `275c0b9`

**Total deviations:** 1 auto-fixed Rule 1 bug.
**Impact on plan:** Verification now exercises the intended contract tests; no scope expansion.

## Known Stubs

- `crates/realtime_preview_runtime/tests/frame_provider.rs` uses the string `phase12-metal-placeholder` only to assert that `TextureHandleDescriptor` serializes opaque backend metadata without native pointers. This is intentional because native texture interop is Phase 12 scope.

## Issues Encountered

- `gsd-tools` was not on PATH in the shell, so GSD SDK calls were run through `node /Users/zhiwen/.codex/get-shit-done/bin/gsd-tools.cjs`.
- No authentication gates or external service setup were required.

## Verification

- `cargo test -p realtime_preview_runtime frame_provider -- --nocapture` - passed, 5 frame-provider tests executed.
- `cargo test -p realtime_preview_runtime video_frame_provider -- --nocapture` - passed, 2 software-video-provider tests executed.
- `cargo check -p realtime_preview_runtime --locked` - passed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 11-03B can consume validated CPU RGBA/static frame inputs and the H.264 software cache when uploading textures into the `wgpu` compositor. Phase 12 can replace `TextureHandleDescriptor` with real platform texture interop without changing the current provider boundary.

## Self-Check: PASSED

- Verified created files exist: `frame_provider.rs`, `software_video_provider.rs`, `frame_provider.rs` tests, `video_frame_provider.rs` tests, `testkit/src/lib.rs`, and this summary.
- Verified task commits exist: `e397bb2`, `275c0b9`, `fbfd2f4`, `f05373b`.

---
*Phase: 11-realtime-preview-runtime-and-gpu-render-backend*
*Completed: 2026-06-18*
