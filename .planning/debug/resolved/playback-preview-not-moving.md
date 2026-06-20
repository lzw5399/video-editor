---
status: resolved
trigger: "After importing a video, adding it to the timeline, and clicking play, the timeline advances but the visible preview video does not move."
created: "2026-06-19"
updated: "2026-06-20"
---

# Debug Session: playback-preview-not-moving

## Symptoms

- expected_behavior: A normal user can import a video, add it to the timeline, click play, and see the center preview image advance in sync with the playhead.
- actual_behavior: The playhead/timecode can advance after clicking play, but the visible preview appears static, placeholder-like, or otherwise not visibly tied to playback frames.
- error_messages: None reported in the UI; this is a user-visible behavior failure.
- timeline: Found after Phase 15.1 realtime preview routing work; previous tests proved command routing but not visible playback pixels.
- reproduction: Open the desktop app, import a video, add it to the timeline, click play, observe that the timeline advances while preview content does not visibly change.

## Current Focus

- hypothesis: Resolved. Prior passing checks were insufficient because they could observe playhead/telemetry movement while the visible preview stayed static or was covered by non-product frame imagery.
- test: Product Playwright now imports a repo-owned moving video through the normal picker, adds it to the timeline, clicks play, requires `renderGraphGpuComposited` evidence to advance, rejects fallback host calls, and waits for macOS system-screenshot pixels in the preview center to change.
- expecting: A regression must fail if playback only advances clocks, telemetry, mock frame tokens, PNG preview artifacts, FFmpeg artifact frames, decoded CPU fingerprints, or a static first frame.
- next_action: Extend the same normal-user E2E matrix in 15.2-05 for seek/scrub/split/trim/move/delete/undo-redo/text/audio/image/layer/transform flows.
- reasoning_checkpoint: Product preview success is only accepted when the render-graph GPU compositor presents visible frames in the desktop product; CPU decode, native-video bridge, and screenshot-only evidence are diagnostics, not success paths.
- tdd_checkpoint: The product journey includes both a negative no-compositor test and a positive visible-center motion test, so unsupported or fallback paths fail closed instead of passing via alternate evidence.

## Evidence

- timestamp: "2026-06-19T14:24:00Z"
  observation: "Added committed product journey fixtures under apps/desktop-electron/tests/fixtures/media: p0-moving-testsrc.mp4 and p0-tone.wav. The video uses FFmpeg testsrc2 so pixels change continuously and no external user media is required."
  source: "apps/desktop-electron/tests/fixtures/media/README.md"
- timestamp: "2026-06-19T14:31:00Z"
  observation: "Added apps/desktop-electron/tests/product-user-journey.spec.ts and helpers/userJourney.ts. The test launches product-mode UI, imports via the normal 导入素材 file-picker path, adds the repo video to the timeline, clicks play, asserts no new requestPreviewFrame calls during playback, then requires runtime presentedFrameCount and targetTimeMicroseconds to advance."
  source: "apps/desktop-electron/tests/product-user-journey.spec.ts"
- timestamp: "2026-06-19T14:34:00Z"
  observation: "The new product user journey reaches playback and fails: presentedFrameCount remains 0 after clicking play for 1.2s, even though the user-visible playhead advances. This proves the previous tests only covered routing/clock movement, not real frame presentation."
  source: "pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line"
- timestamp: "2026-06-19T15:05:00Z"
  observation: "RealtimePreviewHost now owns a playback frame loop: play starts requestRealtimePreviewFrame(...mode: playbackTick), seek presents a seek frame, pause/stop/close cancel the loop, and the renderer polls host telemetry during playback without calling requestPreviewFrame."
  source: "apps/desktop-electron/src/main/realtimePreviewHost.ts; apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
- timestamp: "2026-06-19T15:10:00Z"
  observation: "Product user journey passes: import repo fixture via 导入素材, add to timeline, click play, no additional requestPreviewFrame calls during playback, host presentedFrameCount and targetTime advance, host frame token changes, and preview region pixels change."
  source: "pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line"
- timestamp: "2026-06-19T15:14:00Z"
  observation: "Related realtime preview workspace tests pass after keeping raw cancellation counters in developer diagnostics mode."
  source: "pnpm --filter @video-editor/desktop exec playwright test tests/workspace.spec.ts -g \"预览播放按钮|native preview host|实时预览 telemetry|fallback|developer diagnostics display Rust-reported realtime cancellation counters\" --reporter=line"
- timestamp: "2026-06-19T15:36:00Z"
  observation: "User reproduced a real import/playback flow with dashcam footage and saw a green/cyan flashing overlay during playback. This was caused by the Playwright-visible mock frame display being painted into the product preview surface. The prior region-pixel assertion was therefore a false-positive test proxy, not proof of decoded video playback correctness."
  source: "user screenshot and apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
- timestamp: "2026-06-19T15:18:03Z"
  observation: "The product user journey was tightened to reject mock frame tokens and require decoded/composited content evidence. The first run failed with contentEvidence.source=null, proving presentedFrameCount and playhead movement were still insufficient evidence."
  source: "apps/desktop-electron/tests/product-user-journey.spec.ts; pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line"
- timestamp: "2026-06-19T15:18:03Z"
  observation: "RealtimePreviewHost now exposes test-only native content evidence collected from Rust via requestRealtimePreviewContentEvidence. The binding resolves the active video segment at the runtime target time, resolves the material URI at the native boundary, decodes one FFmpeg CPU frame, and returns only a blake3 digest plus dimensions/time metadata. Mock frame display evidence is no longer exposed unless VIDEO_EDITOR_TEST_EXPOSE_MOCK_FRAME_DISPLAY=1."
  source: "apps/desktop-electron/src/main/realtimePreviewHost.ts; crates/bindings_node/src/realtime_preview_service.rs; crates/media_runtime_desktop/src/ffmpeg_fallback.rs"
- timestamp: "2026-06-19T15:18:03Z"
  observation: "The first content-evidence integration timed out because media_runtime::run_process_with_timeout waited for FFmpeg exit without concurrently draining stdout/stderr. Rawvideo output could fill the pipe and block FFmpeg even though the same command completed quickly in a shell."
  source: "crates/media_runtime/src/process.rs; crates/media_runtime/tests/process.rs"
- timestamp: "2026-06-19T15:18:03Z"
  observation: "After fixing process stdout/stderr draining, the product journey passes with decoded content evidence while still asserting playback does not repeatedly call requestPreviewFrame. This is a guard against fake playback, not proof that the desktop product is rendering through the final GPU compositing backend."
  source: "pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line"
- timestamp: "2026-06-19T15:31:00Z"
  observation: "The product journey was tightened again to require backend `gpu` and composited preview evidence. It fails with Expected `gpu`, Received `mock`, proving Phase 12/15.1 cannot be considered product-complete."
  source: "apps/desktop-electron/tests/product-user-journey.spec.ts; pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line"
- timestamp: "2026-06-20T00:00:00+08:00"
  observation: "User explicitly rejected fallback/degraded completion. Product realtime preview may not pass through mock output, preview PNG loops, FFmpeg artifacts, offscreen/CPU readback, or decoded CPU fingerprints. Unsupported/incomplete product paths must fail closed until true GPU-native composited output is implemented."
  source: "user direction; docs/no-product-fallback-policy.md"
- timestamp: "2026-06-20T00:00:00+08:00"
  observation: "macOS media IO now returns Metal texture leases and realtime_preview_runtime retains external texture handles, but the desktop product still lacks the final visible compositor/presentation evidence path."
  source: "commits 846e212 and a30c277"
- timestamp: "2026-06-20T00:00:00+08:00"
  observation: "Removed the product decoded/FFmpeg content-evidence N-API and Electron host calls. Product E2E now fails with Expected `composited`, Received `null`, which is the intended RED state after eliminating fallback evidence."
  source: "apps/desktop-electron/src/main/realtimePreviewHost.ts; crates/bindings_node/src/realtime_preview_service.rs; pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line"
- timestamp: "2026-06-20T10:20:00+08:00"
  observation: "The previous 04F pass was a false positive for user-visible motion: Playwright/telemetry could advance while static preview imagery or first-frame sampling kept the user-visible video from moving. The E2E now captures the actual macOS screen region for the preview center and waits for that hash to change after a normal play click."
  source: "apps/desktop-electron/tests/helpers/userJourney.ts; apps/desktop-electron/tests/product-user-journey.spec.ts"
- timestamp: "2026-06-20T10:24:00+08:00"
  observation: "macOS native decode ignored the requested source time and always returned the first AVAssetReader sample. The native decoder now sets an AVFoundation CMTimeRange at `VideoDecodeRequest.source_time_us`, with a regression proving 0us and 500000us decode to different requested positions."
  source: "crates/media_runtime_desktop/src/platform/macos.rs; crates/media_runtime_desktop/tests/macos_media_io.rs"
- timestamp: "2026-06-20T10:29:00+08:00"
  observation: "The realtime GPU compositor requested video frames at `layer.source_timerange.start` for every target frame. It now maps graph target time to source time before asking the frame provider, with a regression expecting target 250000us over a source starting at 100000us to request source 350000us."
  source: "crates/realtime_preview_runtime/src/gpu/compositor.rs; crates/realtime_preview_runtime/tests/gpu_subset.rs"
- timestamp: "2026-06-20T10:34:00+08:00"
  observation: "Static preview PNG imagery is hidden once the native realtime surface is product-ready, preventing a stale artifact from visually covering the GPU compositor surface."
  source: "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
- timestamp: "2026-06-20T10:41:00+08:00"
  observation: "macOS product E2E passes in the packaged app: normal user import/add/play path emits renderGraphGpuComposited evidence, no fallback calls, and visible preview-center screen pixels change while playback runs."
  source: "pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line"

## Eliminated

- hypothesis: "The failure is only because Playwright cannot click the product UI."
  reason: "The test successfully imports a repository fixture through the normal 导入素材 path and adds it to the timeline before failing on runtime frame presentation."
- hypothesis: "Existing requestPreviewFrame-based tests prove playback."
  reason: "The new journey explicitly verifies product playback without additional requestPreviewFrame calls, and the realtime host still reports zero presented frames."
- hypothesis: "Mock host frame tokens or preview-region color changes prove video playback."
  reason: "The green/cyan overlay was synthetic mock evidence leaked into product UI. The product journey now requires composited content evidence and requires frameDisplay to remain null in normal product playback."
- hypothesis: "The decoded content evidence failure was caused by missing media or bad timeline target selection."
  reason: "A diagnostic run showed requestRealtimePreviewContentEvidence was called for the active segment but FFmpeg timed out; the root cause was the shared process runner not draining rawvideo stdout while waiting."
- hypothesis: "Decoded CPU content evidence is an acceptable interim P0 proof of product playback."
  reason: "User rejected fallback completion. Decoded CPU fingerprints prove only that FFmpeg can decode a frame; they do not prove the product realtime GPU compositor presented the video."
- hypothesis: "A changing compositor evidence digest proves the user sees moving video."
  reason: "Digest/telemetry can change even if the visible product surface is covered or stale. Product E2E now also asserts macOS screen-captured preview-center pixels change."
- hypothesis: "The remaining playback issue is only AppKit surface occlusion."
  reason: "After surface presentation was repaired, E2E still exposed static-visible playback risks caused by source-time decode and graph frame sampling. Both have direct regressions now."

## Resolution

- root_cause: Multiple false-success gaps were present: macOS AVFoundation decode ignored requested source time, the GPU compositor sampled each video layer at source start instead of target-relative source time, static preview image UI could remain over the native realtime surface, and product E2E did not assert actual visible screen-pixel motion.
- fix: Native decode now honors requested source time, compositor frame requests map target time to source time, product-ready realtime surface hides stale preview PNGs, macOS presenter activation/subview ordering is strengthened, and product E2E requires renderGraphGpuComposited advancement plus visible preview-center screen-pixel motion.
- verification: `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line`; `VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop macos_native_decode_honors_requested_source_time -- --nocapture`; `cargo test -p realtime_preview_runtime gpu_subset_samples_video_frame_at_target_relative_source_time -- --nocapture`; `cargo test -p realtime_preview_runtime gpu_surface_presenter -- --nocapture`; `cargo test -p render_graph -- --nocapture`; `cargo check --workspace --locked`; `pnpm run test:no-product-fallback`; `git diff --check -- . ':!reference'`
- files_changed: apps/desktop-electron/src/main/index.ts; apps/desktop-electron/src/preload/index.ts; apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx; apps/desktop-electron/tests/helpers/foregroundProductApp.ts; apps/desktop-electron/tests/helpers/userJourney.ts; apps/desktop-electron/tests/product-user-journey.spec.ts; crates/media_runtime_desktop/Cargo.toml; crates/media_runtime_desktop/src/platform/macos.rs; crates/media_runtime_desktop/tests/macos_media_io.rs; crates/realtime_preview_runtime/Cargo.toml; crates/realtime_preview_runtime/src/gpu/compositor.rs; crates/realtime_preview_runtime/src/platform/macos.rs; crates/realtime_preview_runtime/tests/gpu_subset.rs
