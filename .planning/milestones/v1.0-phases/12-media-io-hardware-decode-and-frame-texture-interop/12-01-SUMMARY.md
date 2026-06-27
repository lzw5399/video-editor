---
phase: 12-media-io-hardware-decode-and-frame-texture-interop
plan: 01
subsystem: media-runtime-contracts
tags: [media-io, decoder, frame-pool, texture-handle, fallback, tdd]

requires:
  - phase: 11-realtime-preview-runtime-and-gpu-render-backend
    provides: Realtime preview runtime contracts, frame provider boundary, and runtime ownership exclusions
provides:
  - Shared `media_runtime` media reader/session/decoder trait contracts
  - Serializable color, texture handle, and fallback reason metadata
  - Explicit decoded video/audio frame contracts and frame-pool lease lifecycle
  - Contract tests for trait object use, texture opacity, fallback reason stability, and frame release diagnostics
affects: [phase-12, phase-13-cache, phase-16-scheduler, phase-18-effects, preview-service, bindings-node]

tech-stack:
  added: []
  patterns:
    - `media_runtime` owns media IO contracts while FFmpeg process execution remains a separate boundary
    - Decoded frames are represented as leases with opaque CPU or texture handles, not public byte payloads
    - Fallback decisions use stable serializable reason/path enums before platform implementations exist

key-files:
  created:
    - crates/media_runtime/src/media_io.rs
    - crates/media_runtime/src/color.rs
    - crates/media_runtime/src/decoder.rs
    - crates/media_runtime/src/texture.rs
    - crates/media_runtime/src/fallback.rs
    - crates/media_runtime/tests/media_io_contracts.rs
    - crates/media_runtime/tests/fallback_reasons.rs
    - crates/media_runtime/tests/frame_pool.rs
    - .planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-01-SUMMARY.md
  modified:
    - crates/media_runtime/src/lib.rs
    - crates/media_runtime/src/frame.rs

key-decisions:
  - "Phase 12 media IO starts with pure `media_runtime` contracts; platform API imports, FFmpeg command construction, preview scheduling, render graph dependencies, and UI-facing native pointers remain out of scope."
  - "Decoder traits return explicit decoded frame contracts, so callers receive Rust-owned frame metadata and release handles rather than an implicit side effect."
  - "Texture interop is represented by opaque IDs, owner session, generation, backend, device identity, dimensions, pixel format, and color metadata only."
  - "Frame-pool close reports leak diagnostics instead of silently dropping unreleased frame or texture leases."

patterns-established:
  - "Use `MediaReader` -> `MediaSession` -> `VideoDecoder`/`AudioDecoder` as the media IO injection path."
  - "Use `FramePool` leases and `FrameReleaseDiagnostic` for decoded frame lifecycle accounting."
  - "Use `MediaIoFallbackReason` and `SelectedDecodePath` as stable fallback vocabulary for platform and FFmpeg paths."

requirements-completed: [MEDIAIO-01, MEDIAIO-03, MEDIAIO-05]

duration: 40min
completed: 2026-06-18
---

# Phase 12 Plan 01: Shared Media IO Contract Surface Summary

**`media_runtime` now exposes the shared media reader, decoder, frame lease, texture handle, color metadata, and fallback vocabulary needed by Phase 12 platform implementations.**

## Performance

- **Duration:** 40 min
- **Started:** 2026-06-18T18:13:08Z
- **Completed:** 2026-06-18T18:53:53Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- Added object-safe media IO traits for probe service, media reader, media session, and video/audio decoders without coupling them to `FfmpegExecutor`.
- Added serializable color metadata, pixel format, texture handle, runtime device identity, fallback reason, and selected decode path contracts.
- Added decoded video/audio frame contracts plus `FramePool` lease acquisition, release, owner-session validation, outstanding lease accounting, and session-close leak diagnostics.
- Added contract tests proving trait-object use, texture handle opacity, fallback enum casing, frame lease lifecycle, texture leak diagnostics, and unknown color metadata preservation.

## Task Commits

1. **Task 12-01-01 RED:** `b8f0342` test: add failing media IO contract tests.
2. **Task 12-01-01 GREEN:** `12ec743` feat: implement media IO contract surface.
3. **Task 12-01-02 RED:** `fec3a7d` test: add failing frame pool lease tests.
4. **Task 12-01-02 GREEN:** `5884f44` feat: implement frame pool lease lifecycle.

**Plan metadata:** pending final docs commit.

## Files Created/Modified

- `crates/media_runtime/src/media_io.rs` - Reader/session/probe contracts and stream metadata.
- `crates/media_runtime/src/color.rs` - Pixel format and diagnostic-bearing color metadata.
- `crates/media_runtime/src/decoder.rs` - Video/audio decoder request, result, and error contracts.
- `crates/media_runtime/src/texture.rs` - Opaque texture handle and runtime device identity metadata.
- `crates/media_runtime/src/fallback.rs` - Stable fallback reason and selected decode path enums.
- `crates/media_runtime/src/frame.rs` - Frame dimensions, decoded frame contracts, frame pool leases, release diagnostics, and lifecycle implementation.
- `crates/media_runtime/src/lib.rs` - Public re-exports for the new media IO contract surface.
- `crates/media_runtime/tests/media_io_contracts.rs` - Trait object and texture-handle opacity coverage.
- `crates/media_runtime/tests/fallback_reasons.rs` - Stable serde casing coverage for fallback vocabulary.
- `crates/media_runtime/tests/frame_pool.rs` - Lease acquire/release, close leak diagnostics, owner mismatch, and unknown color metadata coverage.

## Decisions Made

- Decoder success paths return `DecodedVideoFrame` and `DecodedAudioFrame` so frame lifecycle is explicit at the contract boundary.
- CPU frames expose an opaque `CpuFrameHandle` with bounded metadata and estimated size, not raw decoded bytes.
- Texture frames carry `TextureHandle` metadata only; no native pointer, raw handle, GPU object, or platform API type crosses the contract.
- `FramePool` validates owner session on texture acquisition and explicit release, then reports unreleased leases during session close.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Execution resumed from an interrupted 12-01 state where Task 12-01-01 RED/GREEN and Task 12-01-02 RED commits already existed but the SUMMARY was missing.
- The frame-pool GREEN implementation changed decoder trait success results from `Result<()>` to explicit decoded frame types; the Task 12-01-01 fake decoder test implementations were updated to match the strengthened contract.

## Verification

- `cargo test -p media_runtime media_io_contracts -- --nocapture` - passed.
- `cargo test -p media_runtime frame_pool -- --nocapture` - passed.
- `cargo test -p media_runtime fallback_reasons -- --nocapture` - passed.
- `cargo test -p media_runtime -- --nocapture` - passed.
- `cargo check --workspace --locked` - passed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 12-02 can build hardware/media capability reporting on top of stable stream, decoder, texture, color, and fallback vocabulary. Later platform plans can implement native sessions and texture interop without changing renderer, preview scheduling, render graph, or FFmpeg process ownership.

## Self-Check: PASSED

- Verified created summary exists: `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-01-SUMMARY.md`.
- Verified task commits exist: `b8f0342`, `12ec743`, `fec3a7d`, `5884f44`.
- Verified required commands passed: `cargo test -p media_runtime media_io_contracts -- --nocapture`, `cargo test -p media_runtime frame_pool -- --nocapture`, and `cargo test -p media_runtime fallback_reasons -- --nocapture`.
- Verified public API compile surface with `cargo check --workspace --locked`.
- Verified `reference/` remains untracked and unstaged.

---
*Phase: 12-media-io-hardware-decode-and-frame-texture-interop*
*Completed: 2026-06-18*
