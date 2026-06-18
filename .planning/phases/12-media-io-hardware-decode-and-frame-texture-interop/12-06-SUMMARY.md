---
phase: 12-media-io-hardware-decode-and-frame-texture-interop
plan: 06
subsystem: realtime-preview-runtime
tags: [media-io, realtime-preview, frame-provider, fallback-diagnostics, generation-gating]

requires:
  - phase: 12-01
    provides: shared media IO, decoder, frame pool, frame lease, texture metadata, and fallback contracts
  - phase: 12-03
    provides: canonical native-to-FFmpeg fallback ladder and CPU frame fallback path
  - phase: 12-04
    provides: macOS native frame leases and texture fallback diagnostics
  - phase: 12-05
    provides: Windows native frame leases and D3D texture fallback diagnostics
provides:
  - Phase 11 preview runtime media IO handoff adapter
  - preview material source-time to `VideoDecodeRequest` conversion
  - preview-visible selected decode path, fallback reason, storage kind, and device compatibility diagnostics
  - stale playback generation rejection and handoff telemetry
  - source guard coverage proving the adapter does not own timeline, render, desktop, or process execution boundaries
affects: [phase-12, phase-11, media-runtime, realtime-preview-runtime, preview-service]

key-files:
  created:
    - crates/realtime_preview_runtime/src/media_io_adapter.rs
    - crates/realtime_preview_runtime/tests/media_io_handoff.rs
  modified:
    - Cargo.lock
    - crates/realtime_preview_runtime/Cargo.toml
    - crates/realtime_preview_runtime/src/fallback.rs
    - crates/realtime_preview_runtime/src/lib.rs
    - crates/realtime_preview_runtime/tests/media_io_handoff.rs

key-decisions:
  - "Phase 11 media IO handoff lives in `realtime_preview_runtime` and depends only on `media_runtime` traits and contracts."
  - "The adapter converts already-resolved material source-time requests into media IO decode requests; it does not normalize drafts, build render graphs, compile FFmpeg, or execute desktop platform runtimes."
  - "Selected decode path and fallback reason are preview diagnostics at the handoff boundary, while texture readiness remains gated by explicit preview/native device compatibility."

patterns-established:
  - "Adapter pattern: register material source metadata once, then decode source-time requests through a trait object `MediaReader`/`VideoDecoder` pair."
  - "Preview handoff diagnostics: selected path, fallback reason, storage kind, preview/native device IDs, texture compatibility, and stale rejection are reported as Rust-owned data."

requirements-completed: [MEDIAIO-01, MEDIAIO-02, MEDIAIO-05]

duration: 10 min
completed: 2026-06-18
---

# Phase 12 Plan 06: Media IO Preview Handoff Summary

**Phase 11 preview runtime can now request decoded material frames through Phase 12 media IO contracts without taking ownership of decode, render, or export semantics.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-18T20:35:28Z
- **Completed:** 2026-06-18T20:39:01Z
- **Tasks:** 1
- **Files modified:** 6

## Accomplishments

- Added `MediaIoFrameProvider` in `realtime_preview_runtime` as a trait-only adapter over `media_runtime::MediaReader`, `MediaSession`, and `VideoDecoder`.
- Added `PreviewMaterialDecodeRequest` and `PreviewMaterialDecodeSource` so preview handoff receives material ID, source-time microseconds, playback generation, desired storage preference, stream ID, selected decode path, and fallback metadata.
- Added `PreviewDecodeDiagnostic`, `PreviewFrameStorageKind`, `PreviewDecodeDeviceContext`, and `PreviewMediaIoTelemetry`.
- Added media IO-specific realtime fallback reasons for native CPU frame fallback, texture interop unavailable, device mismatch, FFmpeg CPU frame, preview artifact fallback, and decode unavailable.
- Added tests covering request conversion to `VideoDecodeRequest`, CPU/platform/texture storage diagnostics, proven texture device compatibility, stale generation rejection, telemetry counts, and forbidden-boundary source guards.

## Task Commits

1. **Task 12-06-01 RED: media IO handoff tests** - `a95942d` (test)
2. **Task 12-06-01 GREEN: media IO preview handoff adapter** - `6b9bc6b` (feat)

## Files Created/Modified

- `crates/realtime_preview_runtime/src/media_io_adapter.rs` - preview-to-media-IO adapter, request/output/diagnostic structs, storage classification, stale rejection, and telemetry.
- `crates/realtime_preview_runtime/tests/media_io_handoff.rs` - adapter conversion, fallback, texture compatibility, stale generation, and boundary tests.
- `crates/realtime_preview_runtime/src/fallback.rs` - media IO fallback variants and mapping from `media_runtime` selected path/reason.
- `crates/realtime_preview_runtime/src/lib.rs` - exported handoff adapter contracts.
- `crates/realtime_preview_runtime/Cargo.toml` and `Cargo.lock` - added the `media_runtime` dependency for the adapter contract.

## Decisions Made

- The adapter accepts selected path/fallback metadata at material registration instead of downcasting concrete desktop sessions. Current `MediaReader`/`VideoDecoder` traits return frames but do not expose selected fallback path.
- Stale handoff results are rejected after decode metadata is available and counted in `PreviewMediaIoTelemetry`, matching the Phase 11 playback generation model.
- Texture handoff is marked compatible only when the request's preview device matches the decoded texture's native device. CPU/platform-opaque frames remain valid with texture fallback diagnostics.

## Deviations from Plan

### Controlled Interface Mapping

**1. Selected fallback path is adapter metadata until the media IO trait exposes it directly**

- **Found during:** Task 12-06-01 implementation
- **Issue:** `media_runtime::VideoDecoder` returns `DecodedVideoFrame`, but the trait does not include selected ladder path or fallback reason. Concrete desktop implementations may keep more detail, but using those concrete types here would violate the Phase 11/12 boundary.
- **Fix:** `PreviewMaterialDecodeSource` carries `selected_path` and optional `MediaIoFallbackSelection`, allowing preview diagnostics to remain explicit without depending on `media_runtime_desktop`.
- **Files modified:** `crates/realtime_preview_runtime/src/media_io_adapter.rs`, `crates/realtime_preview_runtime/tests/media_io_handoff.rs`
- **Verification:** `cargo test -p realtime_preview_runtime media_io_handoff -- --nocapture`
- **Committed in:** `6b9bc6b`

---

**Total deviations:** 1 controlled interface mapping.
**Impact on plan:** Boundary correctness is preserved. A later plan can promote selected fallback reporting into the shared media IO trait if needed, but 12-06 does not depend on desktop concrete implementations.

## Issues Encountered

- `cargo fmt --check --package realtime_preview_runtime` still wants to reorder imports and assertions in existing `frame_provider.rs` and `video_frame_provider.rs` tests outside this plan. Those unrelated files were not changed.

## Verification

- `cargo test -p realtime_preview_runtime media_io_handoff -- --nocapture`
- `cargo test -p realtime_preview_runtime stale_generation -- --nocapture`
- `cargo test -p realtime_preview_runtime -- --nocapture`
- `cargo check --workspace --locked`
- `rustfmt --edition 2024 --check --config skip_children=true crates/realtime_preview_runtime/src/lib.rs`
- `rustfmt --edition 2024 --check crates/realtime_preview_runtime/src/fallback.rs crates/realtime_preview_runtime/src/media_io_adapter.rs crates/realtime_preview_runtime/tests/media_io_handoff.rs`
- `git diff --check`

## User Setup Required

None.

## Next Phase Readiness

Ready for `12-06B`: the binding/release plan can expose handle-based preview decode and release contracts without making the renderer own native handles, decode selection, or fallback routing.

---
*Phase: 12-media-io-hardware-decode-and-frame-texture-interop*
*Completed: 2026-06-18*
