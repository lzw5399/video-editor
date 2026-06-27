---
phase: 12-media-io-hardware-decode-and-frame-texture-interop
plan: 05
subsystem: media-runtime-desktop
tags: [media-io, windows, media-foundation, dxva, d3d, frame-pool, texture-interop]

requires:
  - phase: 12-01
    provides: shared media IO, decoder, frame pool, frame lease, texture metadata, and fallback contracts
  - phase: 12-02
    provides: desktop runtime capability aggregation for native media IO domains
  - phase: 12-02B
    provides: approved Windows `windows` crate dependency and binding-safe handle contracts
  - phase: 12-03
    provides: canonical native-to-FFmpeg fallback ladder and CPU frame fallback path
provides:
  - Windows Media Foundation media reader, session, and video decoder contracts
  - H.264 first-frame decode path behind `cfg(windows)` returning Rust-owned platform-opaque frame leases
  - session-owned native lease retention and release/close leak diagnostics
  - D3D texture interop policy and fallback diagnostics for unavailable interop or device mismatch
  - non-Windows unsupported-platform proof for Windows media IO APIs
affects: [phase-12, phase-11, media-runtime, media-runtime-desktop, realtime-preview, preview-service]

key-files:
  created:
    - crates/media_runtime_desktop/tests/windows_media_io.rs
  modified:
    - crates/media_runtime_desktop/src/lib.rs
    - crates/media_runtime_desktop/src/platform/mod.rs
    - crates/media_runtime_desktop/src/platform/windows.rs
    - crates/media_runtime_desktop/tests/windows_media_io.rs

key-decisions:
  - "Windows native decode is represented as session-owned Media Foundation platform-opaque frame leases until real D3D texture interop is proven."
  - "Windows D3D texture output is selected only when texture interop is available and preview/native device identities match."
  - "The runtime records `TextureInteropUnavailable` or `DeviceMismatch` diagnostics instead of synthesizing false-ready D3D texture handles."

patterns-established:
  - "Windows native decode path: Media Foundation Source Reader opens/probes the material and FramePool owns frame release lifecycle."
  - "Honest texture degradation: D3D texture readiness remains gated by explicit interop/device proof; otherwise native frame or FFmpeg fallback paths are classified."

requirements-advanced: [MEDIAIO-02, MEDIAIO-03, MEDIAIO-04, MEDIAIO-05]

duration: 22 min
completed: 2026-06-18
---

# Phase 12 Plan 05: Windows Native Media IO Summary

**Windows Media Foundation contracts now mirror the macOS native lease model, while D3D texture output remains explicitly gated.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-06-18T20:18:31Z
- **Completed:** 2026-06-18T20:29:10Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added `WindowsMediaReader`, `WindowsMediaSession`, and `WindowsVideoDecoder` with the same session-owned frame lease shape used by the macOS native path.
- Added a `cfg(windows)` Media Foundation Source Reader path that probes/configures NV12 video output and wraps decoded samples as `VideoFrameStorage::PlatformOpaque` frame leases.
- Added `WindowsTextureInteropPolicy` and `select_windows_texture_interop_fallback` so D3D texture interop is selected only when native decode, texture availability, and device identity are proven.
- Added non-Windows tests proving Windows media IO APIs fail as unsupported platform diagnostics instead of panicking.
- Added Windows-gated H.264 fixture decode, texture fallback, and native lease-close tests guarded behind `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1`.

## Task Commits

1. **Task 12-05-01/02 RED: Windows native media IO tests** - `eacd773` (test)
2. **Task 12-05-01/02 GREEN: Windows native media IO contracts** - `51fc64e` (feat)

## Files Created/Modified

- `crates/media_runtime_desktop/src/platform/windows.rs` - Windows native media reader/session/decoder, Media Foundation sample lease retention, D3D texture interop policy, and platform capability reporting.
- `crates/media_runtime_desktop/tests/windows_media_io.rs` - Windows/non-Windows API coverage, D3D fallback selector tests, and Windows-gated native decode/lease tests.
- `crates/media_runtime_desktop/src/lib.rs` and `src/platform/mod.rs` - exported Windows media IO types and fallback selector.

## Decisions Made

- Windows native decode returns Media Foundation-backed platform-opaque frame leases until a later plan proves real D3D texture import into the Phase 11 preview device.
- The D3D texture selector can choose `NativeHardwareTexture` only when texture interop is available and the preview/native D3D device identities match.
- Unsupported texture interop, missing device proof, or device mismatch is represented through structured fallback diagnostics, not hidden CPU copies advertised as texture decode.

## Deviations from Plan

### Controlled Scope Deviations

**1. Deferred actual D3D texture handle output instead of faking texture readiness**

- **Found during:** Task 12-05-02 (D3D device identity and texture handle semantics)
- **Issue:** The plan requires texture handles only when Media Foundation/DXVA output and Phase 11 preview device identity compatibility are proven. This repo does not yet have the real D3D11/D3D12 texture import and preview-device identity proof wired on Windows.
- **Fix:** Implemented explicit D3D texture policy/fallback selection and kept decoded output as `VideoFrameStorage::PlatformOpaque` with `TextureInteropUnavailable`/`DeviceMismatch` diagnostics until real interop proof exists.
- **Files modified:** `crates/media_runtime_desktop/src/platform/windows.rs`, `crates/media_runtime_desktop/tests/windows_media_io.rs`
- **Verification:** `cargo test -p media_runtime_desktop windows_texture -- --nocapture`
- **Committed in:** `51fc64e`

---

**Total deviations:** 1 controlled scope deviation.
**Impact on plan:** Correctness is preserved: Windows does not advertise texture-ready decode until a future plan implements real D3D texture/device proof. Native frame leases and fallback diagnostics are production-safe for the current phase.

## Verification Limitations

- This host cannot install or use the Windows Rust standard library target: `cargo check -p media_runtime_desktop --target x86_64-pc-windows-msvc` failed with `E0463` because `core`/`std` for `x86_64-pc-windows-msvc` are not installed.
- `rustup` is unavailable in this environment, so the target could not be added here.
- Windows-gated tests were added but not executed on a real Windows machine. They must be run with `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop windows_native -- --nocapture` on a Windows host with FFmpeg/ffprobe available.
- Actual Media Foundation decode and D3D texture interop readiness are therefore not claimed as Windows-host verified by this summary.

## Issues Encountered

- `cargo fmt --check --package media_runtime_desktop` still wants to reorder imports in existing FFmpeg fallback files outside this plan. Those unrelated files were not changed.

## Verification

- `cargo test -p media_runtime_desktop windows -- --nocapture`
- `cargo test -p media_runtime_desktop windows_texture -- --nocapture`
- `cargo test -p media_runtime frame_pool -- --nocapture`
- `cargo check --workspace --locked`
- `rustfmt --edition 2024 --check --config skip_children=true crates/media_runtime_desktop/src/lib.rs`
- `rustfmt --edition 2024 --check crates/media_runtime_desktop/src/platform/mod.rs crates/media_runtime_desktop/src/platform/windows.rs crates/media_runtime_desktop/tests/windows_media_io.rs`
- `git diff --check`

## User Setup Required

- Windows host follow-up: install the `x86_64-pc-windows-msvc` Rust target and run the Windows-gated native media tests with `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1`.

## Next Phase Readiness

Ready for `12-06`: the Phase 11 media IO handoff adapter can consume the shared frame-lease and fallback contracts. It must continue to treat D3D texture decode as gated until real Windows device compatibility is proven.

---
*Phase: 12-media-io-hardware-decode-and-frame-texture-interop*
*Completed: 2026-06-18*
