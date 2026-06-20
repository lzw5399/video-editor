---
status: resolved
trigger: "Realtime preview background scheduler produces zero frames in Electron cadence E2E after moving playback tick out of getPresentationState."
created: 2026-06-20T16:27:31Z
updated: 2026-06-20T17:50:00Z
---

# Debug Session: preview-background-zero-frames

## Symptoms

- expected_behavior: UI only polls lightweight state snapshots while Rust-owned playback scheduler performs decode/render/present in the background and presents at least 75 frames over 3 seconds at 30fps.
- actual_behavior: E2E polling is lightweight, but background playback presents zero frames.
- error_messages: product-preview-cadence evidence reported `presentedDelta: 0`, `targetDeltaMicroseconds: 0`, `renderGraphActive: false`, `presentationDurationMs.p50: 0`.
- timeline: Started after destructively moving `present_playback_tick()` out of `presentation_state()` and into a Rust binding playback worker thread.
- reproduction: Run `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line --workers=1` against the manually assembled Electron app bundle.

## Current Focus

- hypothesis: The background worker cannot drive the real media/surface pipeline because scheduler-owned GPU/native state is thread-affine or because thread-local media pipeline state is not available on the worker thread.
- test: Inspect binding scheduler, runtime media pipeline ownership, and Electron host telemetry/error propagation; add instrumentation or redesign frame pump boundary if the worker model violates native threading constraints.
- expecting: The code will show `presentation_state()` is snapshot-only and worker failures are not surfaced as product evidence; moving/owning the pump on the correct scheduler thread should produce nonzero present evidence without blocking polling.
- next_action: gather initial evidence from current code and E2E error context
- reasoning_checkpoint:
- tdd_checkpoint:

## Evidence

- timestamp: 2026-06-20T16:27:31Z
  observation: `product-preview-cadence` initially reported `presentedDelta: 0`, `targetDeltaMicroseconds: 0`, `renderGraphActive: false`, while `getPresentationState` p50/p95 were 0 ms.
- timestamp: 2026-06-20T17:20:00Z
  observation: `MacosWgpuSurfaceAttachment::prepare_for_present` required `MainThreadMarker`, but the binding playback worker called `present_playback_tick` on a background `std::thread`.
- timestamp: 2026-06-20T17:35:00Z
  observation: Moving per-frame AppKit work out of the worker restored real product presentation but cadence was 67-73 frames/3s.
- timestamp: 2026-06-20T17:45:00Z
  observation: Reusing decoder-level `CVMetalTextureCache` and capping single-thread pull target advancement raised cadence to 79 frames/3s.

## Eliminated

- hypothesis: UI snapshot polling still performs decode/render/present.
  reason: `presentation_state()` now only reads `BindingPlaybackSnapshot`; E2E measured `getPresentationState` p50/p95 at 0 ms while frames were presented by the worker.
- hypothesis: The product path fell back to preview artifacts.
  reason: cadence E2E reported `frameRequestsBefore: 0`, `frameRequestsAfter: 0`, `renderGraphActive: true`, and visible preview pixels changed.

## Resolution

- root_cause: The initial background worker moved real macOS surface presentation onto a thread that still invoked per-frame AppKit operations guarded by `MainThreadMarker`; worker errors were only visible as empty snapshots. After that was fixed, cadence remained below the raised gate because macOS decode recreated CoreVideo/Metal interop state per frame and the synchronous pull decoder chased wall-clock target time by skipping samples.
- fix: Keep `getPresentationState` snapshot-only, run playback on the Rust binding worker, cache surface placement from main-thread attach/resize, make background `prepare_for_present` use cached placement only, release native frame leases via bounded in-flight WGPU fences, reuse the decoder-level `CVMetalTextureCache`, and cap one worker tick to one frame of target advancement until a true decode-ahead queue replaces this containment.
- verification: `cargo test -p realtime_preview_runtime media_io_handoff -- --nocapture`; `cargo test -p bindings_node realtime_preview -- --nocapture`; `corepack pnpm --dir apps/desktop-electron run build`; `product-preview-cadence` passed with 79 frames/3s, no artifact fallback, p50/p95 snapshot query 0 ms; `product-user-journey` passed 9/9; `real-workflow` dev no-mock passed.
- files_changed: `crates/bindings_node/src/realtime_preview_service.rs`, `crates/realtime_preview_runtime/src/gpu/compositor.rs`, `crates/realtime_preview_runtime/src/media_io_adapter.rs`, `crates/realtime_preview_runtime/src/platform/macos.rs`, `crates/media_runtime_desktop/src/platform/macos.rs`, Electron host/types/tests.
