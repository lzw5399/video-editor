---
status: resolved
trigger: "普通用户播放视频预览仍然偏卡，需要对标剪映式基础剪辑体验，禁止用 artifact fallback 或妥协路径冒充实时预览。"
created: 2026-06-20
updated: 2026-06-20
resolved: 2026-06-20
---

# Debug Session: product-preview-cadence-lag

## Symptoms

- expected_behavior: Imported video material added to the timeline should play in the product preview with smooth native/GPU-backed cadence, visible motion, and no preview artifact loop.
- actual_behavior: Product journey proved renderGraphGpu playback and no requestPreviewFrame artifact loop, but cadence audit showed only 8 presented frames over 3 seconds and target time advanced only 0.98s.
- error_messages: No hard runtime error; playback silently degraded because decoded native frame leases accumulated until the frame pool limit.
- timeline: Commit 4fa2f72 moved presentation ticks to the main process and cached the scheduler media pipeline, but native frame leases still had no realtime-preview release path.
- reproduction: Launch product app, import p0-moving-testsrc.mp4, add it to timeline from the media card, play preview for 3 seconds, then compare telemetry presentedFrameCount, content evidence target time, visible center hash, and artifact fallback calls.

## Current Focus

- hypothesis: Decoded native texture leases are handed to the compositor but never released after presentation, so macOS frame pools hit the 8 outstanding lease cap and cadence collapses.
- test: Add a product cadence Playwright spec and a media IO handoff regression that presents more than 8 texture frames while releasing each lease.
- expecting: After the fix, 3s product playback should stay on renderGraphGpu, present sustained frames, advance target time near the media duration, change visible pixels, and issue no requestPreviewFrame artifact fallback calls.
- next_action: Complete commit and push to main.
- reasoning_checkpoint: External audit and code inspection confirmed DecodedVideoFrame.release was discarded in MediaIoFrameProvider and BindingSchedulerPresenter never released presented media IO frames.
- tdd_checkpoint: Added product-preview-cadence.spec.ts and media_io_handoff_releases_presented_texture_frames_to_sustain_long_preview_cadence.

## Evidence

- timestamp: 2026-06-20
  observation: Baseline product cadence failed with metrics {"sampleCount":5,"renderGraphSampleCount":1,"presentedDelta":8,"targetDeltaMicroseconds":975113,"evidenceDigestCount":2,"visibleChanged":true,"frameRequestsBefore":0,"frameRequestsAfter":0}.
- timestamp: 2026-06-20
  observation: After explicit lease release and generation propagation, product cadence passed with metrics {"renderGraphActive":true,"presentedDelta":25,"targetDeltaMicroseconds":3000000,"evidenceDigestChanged":true,"visibleChanged":true,"frameRequestsBefore":0,"frameRequestsAfter":0}.
- timestamp: 2026-06-20
  observation: media_io_handoff regression presented and released 16 texture frames, exceeding the previous 8-frame outstanding lease limit.

## Eliminated

- hypothesis: The product path was secretly using requestPreviewFrame artifact fallback.
  reason: Host call counts stayed unchanged before/after playback, and final backend stayed renderGraphGpu with renderGraphGpuComposited evidence.
- hypothesis: The packaged app import failure was the cadence root cause.
  reason: FFmpeg/ffprobe env bridge fixed macOS open-launch discovery, after which cadence assertions reached realtime playback metrics.

## Resolution

- root_cause: Realtime preview decoded native texture frames were converted to compositor inputs without preserving or releasing their DecodedVideoFrame.release lease ids. macOS/Windows frame states and FFmpeg fallback already had frame-pool release primitives, but VideoDecoder did not expose release_frame through the common trait and scheduler presentation never drained presented media IO frames.
- fix: Added VideoDecoder::release_frame, implemented it for macOS, Windows, FFmpeg CPU fallback, and mocks; MediaIoFrameProvider now tracks pending handoff leases, releases them after scheduler presentation, unregisters native texture registry handles, and best-effort drains on drop. BindingSchedulerPresenter passes active PlaybackGeneration into the compositor and drains releases after present. The compositor now caches wgpu pipeline resources by target format.
- verification: cargo test -p realtime_preview_runtime media_io_handoff -- --nocapture; cargo test -p bindings_node realtime_preview -- --nocapture; cargo test -p media_runtime_desktop macos -- --nocapture; cargo test -p media_runtime media_io_contracts -- --nocapture; corepack pnpm --dir apps/desktop-electron run package:dir; product-preview-cadence.spec.ts passed with 25 presented frames over 3s and no artifact fallback; product-user-journey + real-workflow passed 11/11.
- files_changed: crates/media_runtime/src/decoder.rs; crates/media_runtime_desktop/src/platform/macos.rs; crates/media_runtime_desktop/src/platform/windows.rs; crates/media_runtime_desktop/src/ffmpeg_fallback.rs; crates/realtime_preview_runtime/src/media_io_adapter.rs; crates/realtime_preview_runtime/src/gpu/compositor.rs; crates/realtime_preview_runtime/src/gpu/surface.rs; crates/bindings_node/src/realtime_preview_service.rs; apps/desktop-electron/tests/product-preview-cadence.spec.ts; apps/desktop-electron/src/main/index.ts; apps/desktop-electron/tests/helpers/foregroundProductApp.ts.
