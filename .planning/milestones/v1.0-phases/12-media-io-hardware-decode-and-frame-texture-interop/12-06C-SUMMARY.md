---
phase: 12-media-io-hardware-decode-and-frame-texture-interop
plan: 06C
subsystem: media-runtime
tags: [media-io, frame-pool, texture-handles, leak-diagnostics, source-guards, phase-gates]

requires:
  - phase: 12-06B
    provides: binding-safe preview decode and frame release handle contracts
provides:
  - release/session-close leak diagnostics tests for CPU, platform-opaque, and texture frame leases
  - texture leak diagnostic metadata for backend, runtime device ID, and compatibility state
  - final Phase 12 source guard coverage for media IO ownership boundaries
  - root `test:phase12` script covering Rust, binding, source guard, and generated contract gates
  - platform verification notes for macOS and Windows native decode acceptance
affects: [phase-12, media-runtime, media-runtime-desktop, realtime-preview-runtime, bindings-node]

key-files:
  created:
    - crates/media_runtime_desktop/tests/session_leaks.rs
    - .planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-06C-SUMMARY.md
  modified:
    - crates/media_runtime/src/frame.rs
    - scripts/phase12-source-guards.sh
    - package.json

key-decisions:
  - "FrameReleaseDiagnostic records texture backend, runtime device ID, and compatibility for texture leak reports."
  - "`pnpm run test:phase12` is the final focused Phase 12 gate across media runtime contracts, desktop fallback/native tests, bindings, generated contracts, and source guards."
  - "Native platform proof is explicit: macOS was run with VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 on this host, while Windows remains a required Windows-host verification item."

patterns-established:
  - "Session-close leak diagnostics preserve owner session, playback generation, storage kind, and texture device context before releasing retained leases."
  - "Phase source guards use comment-filtered rg checks for forbidden ownership patterns instead of brittle raw grep counts."

requirements-completed: [MEDIAIO-03, MEDIAIO-04, MEDIAIO-05]

duration: 20 min
completed: 2026-06-19
---

# Phase 12 Plan 06C: Final Media IO Lifetime And Gate Summary

**Phase 12 now has focused lifetime tests, texture leak diagnostics, source-boundary guards, a root phase gate, and recorded platform verification status.**

## Performance

- **Duration:** 20 min
- **Started:** 2026-06-19T05:00:41+08:00
- **Completed:** 2026-06-19T05:06:18+08:00
- **Tasks:** 2
- **Files modified:** 4 code/config files plus this summary

## Accomplishments

- Added desktop session leak tests for unreleased CPU frame leases, platform-opaque leases, and texture leases.
- Extended `FrameReleaseDiagnostic` so texture leak reports include backend, `RuntimeDeviceId`, and compatibility state.
- Tightened `scripts/phase12-source-guards.sh` to reject renderer-owned native media/FFmpeg ownership, raw pointer/full-frame payload exposure, and Phase 12 runtime paths owning render graph/timeline semantics.
- Added root `pnpm run test:phase12` covering the focused Phase 12 Rust, binding, generated contract, and source guard gates.
- Ran macOS native media proof with `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1`; Windows hardware proof remains pending on a Windows host.

## Task Commits

1. **Task 12-06C-01 RED: session leak diagnostics tests** - `2de9f66` (test)
2. **Task 12-06C-01 GREEN: texture leak diagnostics** - `4a0032a` (feat)
3. **Task 12-06C-02 RED: final Phase 12 gate requirement** - `ce0fe25` (test)
4. **Task 12-06C-02 GREEN: final Phase 12 gate wiring** - `d5ca1ae` (feat)
5. **Formatting follow-up: session leak test rustfmt** - `98a405a` (style)

## Files Created/Modified

- `crates/media_runtime_desktop/tests/session_leaks.rs` - release/session-close leak diagnostics tests for CPU, platform-opaque, and texture storage.
- `crates/media_runtime/src/frame.rs` - texture backend/device/compatibility metadata on `FrameReleaseDiagnostic`.
- `scripts/phase12-source-guards.sh` - final media IO source-boundary guards and `test:phase12` script assertions.
- `package.json` - root `test:phase12` script and Phase 12 source guard script wiring.

## Platform Verification Notes

### macOS

- **Command:** `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop macos -- --nocapture`
- **Result:** Passed.
- **Native capability/decode:** `macos_native_decodes_h264_fixture_into_corevideo_frame_lease_when_enabled` generated an H.264 MP4 fixture, opened it through AVFoundation, decoded the first frame through AVFoundation/CoreVideo, and returned a platform-opaque CoreVideo frame lease.
- **Fallback diagnostics:** `macos_texture_decode_degrades_when_texture_interop_is_disabled` recorded `TextureInteropUnavailable` while still returning a native frame lease.
- **Texture compatibility:** selection tests covered disabled texture interop, device mismatch, and compatible Metal device routing. Real decode still returns platform-opaque storage unless texture interop/device compatibility is proven by policy.
- **Session-close leak diagnostics:** `macos_native_close_reports_unreleased_corevideo_leases` passed and reported unreleased CoreVideo leases on session close.

### Windows

- **Command to run on Windows:** `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop windows -- --nocapture`
- **Result in this run:** Not run on a Windows host.
- **Required verification:** Media Foundation capability report, H.264 MP4/MOV first-frame decode, texture fallback/device-compatibility diagnostics, and session-close leak diagnostics must be recorded on Windows before claiming Windows native hardware acceptance.
- **Current automated coverage on non-Windows:** `pnpm run test:phase12` runs the Windows test target filters and non-Windows unsupported-platform checks, but that is not a substitute for Windows hardware proof.

## Decisions Made

- Texture leak diagnostics belong in `media_runtime::FrameReleaseDiagnostic`, not in desktop tests, because every future runtime needs the same close/release evidence shape.
- The final public gate is `pnpm run test:phase12`; it intentionally composes focused test filters instead of requiring the entire workspace for this phase-specific proof.
- Platform native tests remain env-gated with `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1` so CI can prove contracts without silently pretending to run hardware decode.

## Deviations from Plan

### Auto-fixed Issues

**1. Added `crates/media_runtime/src/frame.rs` to the task scope**

- **Found during:** Task 12-06C-01 RED test execution
- **Issue:** Texture close diagnostics did not expose backend, runtime device ID, or compatibility state, so the planned texture leak test could not assert required device context.
- **Fix:** Added optional texture metadata fields to `FrameReleaseDiagnostic` and populated them from `VideoFrameStorage::Texture`.
- **Files modified:** `crates/media_runtime/src/frame.rs`
- **Verification:** `cargo test -p media_runtime_desktop session_leaks -- --nocapture`; `cargo test -p media_runtime frame_pool -- --nocapture`
- **Committed in:** `4a0032a`

---

**Total deviations:** 1 auto-fixed scope addition.
**Impact on plan:** Required for MEDIAIO-03 correctness. It keeps diagnostics in the shared media runtime boundary rather than hardcoding assumptions in desktop tests.

## Issues Encountered

- The RED source guard task correctly failed until `package.json` contained the final `test:phase12` script. This was fixed in `d5ca1ae`.
- `rustfmt --check` found one wrapping-only formatting issue in `session_leaks.rs`; it was fixed in `98a405a` with no semantic changes.

## Verification

- `cargo test -p media_runtime_desktop session_leaks -- --nocapture`
- `cargo test -p media_runtime frame_pool -- --nocapture`
- `pnpm run test:phase12-source-guards`
- `pnpm run test:phase12`
- `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop macos -- --nocapture`

## User Setup Required

- Windows native acceptance still requires a Windows host with FFmpeg/ffprobe available and `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1`.

## Next Phase Readiness

Phase 12 is ready to hand off to Phase 13. Media IO now exposes binding-safe decode/release contracts, shared frame/texture lifetime diagnostics, native platform capability/decode posture, and a focused regression gate. Phase 13 can build incremental graph/cache invalidation on top of these contracts without moving media decode, FFmpeg execution, or render graph semantics into the renderer.

---
*Phase: 12-media-io-hardware-decode-and-frame-texture-interop*
*Completed: 2026-06-19*
