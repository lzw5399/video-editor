---
phase: 12-media-io-hardware-decode-and-frame-texture-interop
plan: 04
subsystem: media-runtime-desktop
tags: [media-io, macos, avfoundation, corevideo, frame-pool, texture-interop]

requires:
  - phase: 12-01
    provides: shared media IO, decoder, frame pool, frame lease, texture metadata, and fallback contracts
  - phase: 12-02
    provides: desktop runtime capability aggregation for native media IO domains
  - phase: 12-02B
    provides: approved macOS objc2 framework dependencies and generated binding-safe handle contracts
  - phase: 12-03
    provides: canonical native-to-FFmpeg fallback ladder and CPU frame fallback path
provides:
  - macOS AVFoundation/CoreVideo media reader, session, and video decoder contracts
  - H.264 MP4 first-frame decode into Rust-owned CoreVideo platform-opaque frame leases
  - session-owned native lease retention and release/close leak diagnostics
  - Metal texture interop policy and fallback diagnostics for unavailable cache or device mismatch
  - platform-opaque frame storage for native handles without exposing native pointers
affects: [phase-12, phase-11, media-runtime, media-runtime-desktop, realtime-preview, preview-service]

tech-stack:
  added:
    - objc2-core-foundation 0.3.2 as a direct macOS target dependency
    - objc2-foundation 0.3.2 as a direct macOS target dependency
  patterns:
    - Native decoded frames can be represented as platform-opaque frame leases when texture interop is not proven.
    - macOS texture selection must prove both CVMetalTextureCache availability and matching preview/native Metal device identity before returning texture storage.

key-files:
  created:
    - crates/media_runtime_desktop/tests/macos_media_io.rs
  modified:
    - Cargo.lock
    - crates/media_runtime/src/frame.rs
    - crates/media_runtime/tests/frame_pool.rs
    - crates/media_runtime_desktop/Cargo.toml
    - crates/media_runtime_desktop/src/lib.rs
    - crates/media_runtime_desktop/src/platform/macos.rs
    - crates/media_runtime_desktop/src/platform/mod.rs

key-decisions:
  - "macOS native H.264 decode returns CoreVideo-backed platform-opaque frame leases until real Metal texture interop is proven."
  - "Do not synthesize TextureHandle metadata from CVPixelBuffer without CVMetalTextureCache and device-identity proof."
  - "Native macOS leases retain platform sample buffers in Rust session state and expose only release IDs, labels, dimensions, timing, pixel format, and color diagnostics."

patterns-established:
  - "macOS native decode path: AVFoundation opens/probes the asset, AVAssetReader produces a CoreVideo frame, and FramePool owns the release lifecycle."
  - "Honest texture degradation: texture path is selected only by explicit cache/device proof; otherwise fallback diagnostics record TextureInteropUnavailable or DeviceMismatch."

requirements-completed: [MEDIAIO-02, MEDIAIO-03, MEDIAIO-04, MEDIAIO-05]

duration: 15 min
completed: 2026-06-18
---

# Phase 12 Plan 04: macOS Native Media IO Summary

**AVFoundation/CoreVideo H.264 decode returns Rust-owned native frame leases with explicit Metal texture fallback diagnostics.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-06-18T19:58:43Z
- **Completed:** 2026-06-18T20:12:49Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Added `MacosMediaReader`, `MacosMediaSession`, and `MacosVideoDecoder` behind platform-gated implementation code.
- macOS can open a local H.264 MP4 fixture through AVFoundation and decode the first frame into a `FramePool` lease backed by retained CoreVideo platform state.
- Added `FrameStorageRequest::PlatformOpaque` so native frame handles can cross the runtime boundary as metadata without exposing raw native pointers or full-frame JS byte buffers.
- Added `MacosTextureInteropPolicy` and `select_macos_texture_interop_fallback` for explicit texture unavailable/device mismatch diagnostics.
- Session frame release and session close now clear native lease state and report unreleased frame diagnostics.

## Task Commits

1. **Task 12-04-01/02 RED: macOS native media IO and texture fallback tests** - `0338a1d` (test)
2. **Task 12-04-01/02 GREEN: macOS native CoreVideo frame leases** - `8a49c1e` (feat)

## Files Created/Modified

- `crates/media_runtime_desktop/src/platform/macos.rs` - macOS native media reader/session/decoder, CoreVideo frame lease retention, texture interop policy, and platform capability reporting.
- `crates/media_runtime_desktop/tests/macos_media_io.rs` - macOS/native-gated H.264 decode, texture fallback, device mismatch, unsupported-platform, and leak-close tests.
- `crates/media_runtime/src/frame.rs` - platform-opaque frame storage request and handle support.
- `crates/media_runtime/tests/frame_pool.rs` - platform-opaque frame lease coverage.
- `crates/media_runtime_desktop/src/lib.rs` and `src/platform/mod.rs` - exported macOS media IO types.
- `crates/media_runtime_desktop/Cargo.toml` and `Cargo.lock` - direct macOS framework dependency declarations used by the native path.

## Decisions Made

- macOS native H.264 decode uses AVFoundation/CoreVideo first, then exposes a Rust-owned frame lease instead of preview artifacts or renderer-owned pixels.
- Metal texture interop remains a gated capability. The helper can select the texture path only when cache/device proof is supplied, but actual decode does not claim `VideoFrameStorage::Texture` yet.
- Unknown or incomplete platform color metadata is preserved as an explicit diagnostic on the decoded frame.

## Deviations from Plan

### Controlled Scope Deviations

**1. Deferred actual CVMetalTextureCache texture creation instead of faking texture readiness**

- **Found during:** Task 12-04-02 (CoreVideo to Metal texture handle semantics)
- **Issue:** The plan allowed texture handles only when `CVMetalTextureCache` and preview/native device identity are proven. The current runtime does not yet have a complete multi-plane CoreVideo-to-Metal handoff wired to Phase 11 device identity, so returning `VideoFrameStorage::Texture` would be a false-ready signal.
- **Fix:** Implemented explicit texture policy/fallback selection and kept actual decoded output as `VideoFrameStorage::PlatformOpaque` with `TextureInteropUnavailable`/`DeviceMismatch` diagnostics until real cache/device proof exists.
- **Files modified:** `crates/media_runtime_desktop/src/platform/macos.rs`, `crates/media_runtime_desktop/tests/macos_media_io.rs`
- **Verification:** `cargo test -p media_runtime_desktop macos_texture -- --nocapture`; `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop macos_texture -- --nocapture`
- **Committed in:** `8a49c1e`

---

**Total deviations:** 1 controlled scope deviation.
**Impact on plan:** Correctness is preserved: macOS does not advertise texture-ready decode until a future plan implements real `CVMetalTextureCache` and device-identity proof. Native frame leases and fallback diagnostics are production-safe for the current phase.

## Issues Encountered

- `cargo fmt` initially formatted unrelated Rust files across the workspace; those formatting-only diffs were reverted before the GREEN commit.

## Verification

- `cargo test -p media_runtime_desktop macos -- --nocapture`
- `cargo test -p media_runtime_desktop macos_texture -- --nocapture`
- `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop macos_native -- --nocapture`
- `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop macos_texture -- --nocapture`
- `cargo test -p media_runtime frame_pool -- --nocapture`
- `cargo test -p media_runtime_desktop capabilities -- --nocapture`
- `cargo check --workspace --locked`
- `git diff --check`

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for `12-05`: the Windows native media IO path can follow the same session-owned frame lease and honest texture-degradation pattern. A later handle-based preview integration must add real macOS `CVMetalTextureCache`/device identity proof before treating macOS texture decode as ready.

---
*Phase: 12-media-io-hardware-decode-and-frame-texture-interop*
*Completed: 2026-06-18*
