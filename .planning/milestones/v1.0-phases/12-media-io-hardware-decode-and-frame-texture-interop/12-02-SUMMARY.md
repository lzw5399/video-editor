---
phase: 12-media-io-hardware-decode-and-frame-texture-interop
plan: 02
subsystem: media-runtime-capabilities
tags: [media-io, runtime-capabilities, platform-stubs, fallback-ladder, tdd]

requires:
  - phase: 12-media-io-hardware-decode-and-frame-texture-interop
    provides: Plan 12-01 shared media IO contracts, texture handles, color metadata, and fallback vocabulary
provides:
  - Additive desktop runtime capability report preserving existing FFmpeg fields
  - Shared media IO capability structs for Windows/macOS domains, codecs, pixel formats, texture interop, and fallback ladder
  - cfg-gated Windows and macOS platform capability probe stubs with explicit unsupported-platform diagnostics
  - Desktop capability tests for FFmpeg preservation, native domain reporting, H.264 acceptance target, and fallback ordering
affects: [phase-12, media-runtime-desktop, phase-12-03-ffmpeg-fallback, phase-12-04-macos, phase-12-05-windows, bindings-node]

tech-stack:
  added: []
  patterns:
    - Desktop capability reporting composes existing `probe_runtime_capabilities` rather than replacing it
    - Platform capability probes report Warning/Unavailable until native decode and texture import proof exists
    - H.264 MP4/MOV is tracked as the first native hardware-decode target without claiming current native support

key-files:
  created:
    - crates/media_runtime_desktop/src/capabilities.rs
    - crates/media_runtime_desktop/src/platform/mod.rs
    - crates/media_runtime_desktop/src/platform/windows.rs
    - crates/media_runtime_desktop/src/platform/macos.rs
    - crates/media_runtime_desktop/tests/capabilities.rs
    - .planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-02-SUMMARY.md
  modified:
    - crates/media_runtime/src/capabilities.rs
    - crates/media_runtime/src/fallback.rs
    - crates/media_runtime/src/lib.rs
    - crates/media_runtime/tests/fallback_reasons.rs
    - crates/media_runtime_desktop/src/lib.rs
    - crates/media_runtime_desktop/Cargo.toml
    - Cargo.lock

key-decisions:
  - "Desktop aggregate capability reports wrap the existing FFmpeg runtime capability report and add `mediaIo`; `probe_runtime_capabilities` remains unchanged for existing callers."
  - "Windows/macOS platform modules are cfg-gated capability stubs in 12-02; they do not import OS APIs or claim native decode readiness."
  - "`UnsupportedPlatform` is now a stable fallback reason so non-target platform domains can fail explicitly rather than panic or disappear."
  - "Texture interop remains Warning with CPU fallback required until native import compatibility is proven against a runtime device."

patterns-established:
  - "Use `probe_desktop_runtime_capabilities` for Phase 12 desktop capability aggregation."
  - "Use `RuntimeCapabilities.ffmpeg` for existing FFmpeg readiness and `RuntimeCapabilities.media_io` for native/fallback posture."
  - "Use cfg-gated platform modules for OS domain reporting while deferring dependency additions to 12-02B and implementations to 12-04/12-05."

requirements-completed: [MEDIAIO-02, MEDIAIO-05]

duration: 6min
completed: 2026-06-18
---

# Phase 12 Plan 02: Desktop Media IO Capability Reporting Summary

**Desktop runtime capability reporting now combines existing FFmpeg readiness with native media IO domains, codec posture, texture interop state, and fallback ladder diagnostics.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-06-18T18:58:37Z
- **Completed:** 2026-06-18T19:04:07Z
- **Tasks:** 1
- **Files modified:** 12

## Accomplishments

- Added `RuntimeCapabilities` and `RuntimeMediaIoCapabilities` with Windows/macOS domains, codec capabilities, pixel-format capabilities, texture interop posture, and fallback ladder details.
- Added `probe_desktop_runtime_capabilities` in `media_runtime_desktop` to preserve the existing FFmpeg report while adding `mediaIo`.
- Added cfg-gated Windows and macOS platform capability probe stubs that report unsupported-platform diagnostics off target platforms and pending/warning status on target platforms.
- Added `UnsupportedPlatform` fallback reason with stable serde coverage.
- Added desktop capability tests covering FFmpeg field preservation, media IO serialization shape, non-Windows unsupported diagnostics, H.264 MP4/MOV first-target status, degraded HEVC/ProRes/AV1 posture, and fallback ladder ordering.

## Task Commits

1. **Task 12-02-01 RED:** `c1b3af8` test: add failing desktop media IO capability tests.
2. **Task 12-02-01 GREEN:** `8732755` feat: add desktop media IO capability reports.

**Plan metadata:** pending final docs commit.

## Files Created/Modified

- `crates/media_runtime/src/capabilities.rs` - Added shared desktop/media IO capability report structs.
- `crates/media_runtime/src/fallback.rs` - Added `UnsupportedPlatform` fallback reason.
- `crates/media_runtime/src/lib.rs` - Re-exported the new capability types.
- `crates/media_runtime/tests/fallback_reasons.rs` - Added serde stability coverage for `unsupportedPlatform`.
- `crates/media_runtime_desktop/src/capabilities.rs` - Added desktop aggregate report construction.
- `crates/media_runtime_desktop/src/platform/windows.rs` - Added cfg-gated Windows Media Foundation/DXVA/D3D capability domain stub.
- `crates/media_runtime_desktop/src/platform/macos.rs` - Added cfg-gated macOS AVFoundation/VideoToolbox/CoreVideo/Metal capability domain stub.
- `crates/media_runtime_desktop/src/platform/mod.rs` - Added platform module exports.
- `crates/media_runtime_desktop/src/lib.rs` - Exposed `probe_desktop_runtime_capabilities`.
- `crates/media_runtime_desktop/tests/capabilities.rs` - Added desktop aggregate capability tests.
- `crates/media_runtime_desktop/Cargo.toml`, `Cargo.lock` - Added existing `serde_json` as a desktop crate dev-dependency for serialization assertions.

## Decisions Made

- The desktop capability report is additive. Existing FFmpeg capability behavior remains available through `RuntimeCapabilities.ffmpeg`.
- Native platform domains use Warning/Unavailable until platform probes and real decode implementations exist.
- H.264 MP4/MOV is the first acceptance target but is not reported as Ready by default.
- FFmpeg CPU frame and FFmpeg preview artifact remain Ready fallback paths when the existing FFmpeg report is available.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The initial RED test names used singular `capability`, so the planned `cargo test -p media_runtime_desktop capabilities -- --nocapture` filter skipped them. The GREEN commit renamed the tests to `desktop_capabilities_*` so the documented gate executes the intended tests.
- `serde_json` was already present in the workspace but not declared by `media_runtime_desktop`; it was added as a dev-dependency only for the integration test serialization assertion.

## Verification

- `cargo test -p media_runtime runtime_capability -- --nocapture` - passed.
- `cargo test -p media_runtime_desktop capabilities -- --nocapture` - passed; 4 applicable tests ran on this platform.
- `cargo test -p media_runtime fallback_reasons -- --nocapture` - passed.
- `cargo check --workspace --locked` - passed.
- `git diff --check` - passed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 12-02B can verify package/dependency posture before real platform dependency additions. Plan 12-03 can use the fallback ladder vocabulary for FFmpeg CPU frame fallback without changing the existing FFmpeg probe/export/transcode boundary.

## Self-Check: PASSED

- Verified created summary exists: `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-02-SUMMARY.md`.
- Verified task commits exist: `c1b3af8`, `8732755`.
- Verified required commands passed: `cargo test -p media_runtime runtime_capability -- --nocapture` and `cargo test -p media_runtime_desktop capabilities -- --nocapture`.
- Verified additional fallback enum coverage with `cargo test -p media_runtime fallback_reasons -- --nocapture`.
- Verified public API compile surface with `cargo check --workspace --locked`.
- Verified `reference/` remains untracked and unstaged.

---
*Phase: 12-media-io-hardware-decode-and-frame-texture-interop*
*Completed: 2026-06-18*
