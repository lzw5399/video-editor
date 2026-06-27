---
phase: 12-media-io-hardware-decode-and-frame-texture-interop
plan: 03
subsystem: media-runtime-desktop
tags: [media-io, ffmpeg-fallback, frame-pool, fallback-ladder, preview-regression]

requires:
  - phase: 12-01
    provides: shared media IO, decoder, frame pool, frame lease, and fallback contracts
  - phase: 12-02
    provides: desktop runtime capability aggregation for FFmpeg and native media IO domains
  - phase: 12-02B
    provides: generated binding-safe media IO capability and handle metadata contracts
provides:
  - FFmpeg-backed CPU frame fallback reader, session, and video decoder
  - CPU decoded frame leases through FramePool with dimensions, source time, pixel format, and color diagnostics
  - Serializable media IO fallback candidates, diagnostics, and selected-path reports
  - Canonical native-to-FFmpeg fallback ladder reused by desktop capability reporting
affects: [phase-12, media-runtime, media-runtime-desktop, preview-service, realtime-preview]

tech-stack:
  added:
    - serde 1.0.228 as a direct media_runtime_desktop dependency
    - serde_json 1.0.150 as a direct media_runtime_desktop dependency
  patterns:
    - FFmpeg CPU decode implements MediaReader/MediaSession/VideoDecoder instead of bypassing media IO contracts.
    - Fallback ordering is defined once in media_runtime and reused by desktop capability reporting.

key-files:
  created:
    - crates/media_runtime_desktop/src/ffmpeg_fallback.rs
    - crates/media_runtime_desktop/tests/ffmpeg_fallback.rs
    - crates/media_runtime_desktop/tests/fallback_ladder.rs
  modified:
    - Cargo.lock
    - crates/media_runtime_desktop/Cargo.toml
    - crates/media_runtime_desktop/src/lib.rs
    - crates/media_runtime/src/fallback.rs
    - crates/media_runtime/src/lib.rs
    - crates/media_runtime_desktop/src/capabilities.rs

key-decisions:
  - "FFmpeg CPU frame fallback is exposed as a desktop MediaReader/MediaSession/VideoDecoder implementation and returns FramePool CPU leases, not renderer-owned pixels."
  - "FFmpeg fallback probing parses stream metadata inside media_runtime_desktop without changing the existing probe_material_metadata contract."
  - "media_runtime owns the canonical fallback ladder and serializable selected-path diagnostics so desktop capability reports cannot drift from decode selection semantics."

patterns-established:
  - "Process fallback decode: use FfmpegExecutor::run with explicit argv arrays, bounded stdout/stderr summaries, and classified runtime/decode errors."
  - "Canonical fallback selection: native hardware texture -> native hardware CPU copy -> native software CPU frame -> FFmpeg CPU frame -> FFmpeg preview artifact."

requirements-completed: [MEDIAIO-01, MEDIAIO-03, MEDIAIO-05]

duration: 19 min
completed: 2026-06-18
---

# Phase 12 Plan 03: FFmpeg CPU Fallback And Structured Ladder Summary

**FFmpeg CPU frame decode through media IO traits with canonical fallback selection diagnostics.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-06-18T19:31:00Z
- **Completed:** 2026-06-18T19:49:39Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- Added `FfmpegFallbackMediaReader`, `FfmpegFallbackMediaSession`, and `FfmpegCpuVideoDecoder` in `media_runtime_desktop`.
- FFmpeg fallback now opens media through ffprobe stream metadata and decodes a requested video frame through `FfmpegExecutor::run` into a `FramePool` CPU frame lease.
- Added classified missing-runtime and failed-process diagnostics with bounded stdout/stderr summaries.
- Added `MediaIoFallbackCandidate`, `MediaIoFallbackDiagnostic`, `MediaIoFallbackSelection`, `media_io_fallback_ladder`, and `select_media_io_fallback` to `media_runtime`.
- Desktop capability fallback ladder now uses the canonical media runtime ordering.
- Preview artifact and export job regression tests still pass unchanged.

## Task Commits

1. **Task 12-03-01 RED: FFmpeg CPU fallback tests** - `eee1088` (test)
2. **Task 12-03-01 GREEN: FFmpeg CPU fallback decoder** - `53a62b4` (feat)
3. **Task 12-03-02 RED: media IO fallback ladder tests** - `88a24bb` (test)
4. **Task 12-03-02 GREEN: structured fallback ladder** - `4b5c049` (feat)

## Files Created/Modified

- `crates/media_runtime_desktop/src/ffmpeg_fallback.rs` - FFmpeg fallback reader/session/video decoder and decode diagnostics.
- `crates/media_runtime_desktop/tests/ffmpeg_fallback.rs` - H.264 fixture decode, missing runtime, argv-array, and bounded-output tests.
- `crates/media_runtime/src/fallback.rs` - Canonical fallback ladder, candidates, diagnostics, and selection result.
- `crates/media_runtime_desktop/src/capabilities.rs` - Desktop capability ladder generated from the canonical media runtime order.
- `crates/media_runtime_desktop/tests/fallback_ladder.rs` - Canonical ladder and selected-path reason tests.
- `crates/media_runtime_desktop/Cargo.toml` and `Cargo.lock` - Direct production serde/serde_json declarations for fallback ffprobe JSON parsing.

## Decisions Made

- FFmpeg fallback decode is a media IO implementation, not a preview/export owner. It returns `DecodedVideoFrame` CPU storage through `FramePool`.
- FFmpeg fallback stream probing stays local to `media_runtime_desktop` so existing `probe_material_metadata` behavior and tests remain stable.
- Fallback selection reason records why the native path degraded, while the selected path records where the request will execute.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Declared production serde/serde_json dependencies for desktop fallback probing**

- **Found during:** Task 12-03-01 (FFmpeg fallback implementation)
- **Issue:** Production `ffmpeg_fallback.rs` needs structured ffprobe JSON parsing, but `serde_json` was only a dev-dependency of `media_runtime_desktop` and `serde` was not a direct dependency.
- **Fix:** Promoted existing locked `serde_json` and added direct `serde` dependency for `media_runtime_desktop`; no new package versions were introduced.
- **Files modified:** `crates/media_runtime_desktop/Cargo.toml`, `Cargo.lock`
- **Verification:** `cargo test -p media_runtime_desktop ffmpeg_fallback -- --nocapture`; `cargo check --workspace --locked`
- **Committed in:** `53a62b4`

---

**Total deviations:** 1 auto-fixed blocking issue.
**Impact on plan:** Required for structured parsing in production code. It does not change FFmpeg/probe/export semantics and does not introduce unreviewed dependency versions.

## Issues Encountered

- `rustfmt` invoked through crate roots recursively formatted unrelated media runtime modules; those formatting-only diffs were reverted before commits.

## Verification

- `cargo test -p media_runtime_desktop ffmpeg_fallback -- --nocapture`
- `cargo test -p media_runtime_desktop fallback_ladder -- --nocapture`
- `cargo test -p media_runtime material_probe -- --nocapture`
- `cargo test -p preview_service preview -- --nocapture`
- `cargo test -p media_runtime export_job -- --nocapture`
- `cargo check --workspace --locked`
- `git diff --check`

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for `12-04`: native Windows Media Foundation / D3D media IO implementation can use the shared frame lease contracts and canonical fallback diagnostics established here.

---
*Phase: 12-media-io-hardware-decode-and-frame-texture-interop*
*Completed: 2026-06-18*
