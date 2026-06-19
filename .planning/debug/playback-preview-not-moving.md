---
status: investigating
trigger: "After importing a video, adding it to the timeline, and clicking play, the timeline advances but the visible preview video does not move."
created: "2026-06-19"
updated: "2026-06-19"
---

# Debug Session: playback-preview-not-moving

## Symptoms

- expected_behavior: A normal user can import a video, add it to the timeline, click play, and see the center preview image advance in sync with the playhead.
- actual_behavior: The playhead/timecode can advance after clicking play, but the visible preview appears static, placeholder-like, or otherwise not visibly tied to playback frames.
- error_messages: None reported in the UI; this is a user-visible behavior failure.
- timeline: Found after Phase 15.1 realtime preview routing work; previous tests proved command routing but not visible playback pixels.
- reproduction: Open the desktop app, import a video, add it to the timeline, click play, observe that the timeline advances while preview content does not visibly change.

## Current Focus

- hypothesis: Existing tests validate realtime-preview host routing and playhead clock movement, but the product UI lacks an end-to-end assertion that visible preview pixels advance during playback.
- test: Keep the Playwright normal-user workflow on moving fixture media, but require native decoded/composited content evidence instead of mock frame tokens or screenshot color proxies.
- expecting: Product playback must fail if the realtime host only advances clocks, telemetry, mock frame tokens, or PNG preview artifacts.
- next_action: Replace the remaining Mock realtime backend/product presentation path with true GPU/native texture decode-to-compositor presentation. Offscreen/CPU decoded probes are not an acceptable P0 completion path.
- reasoning_checkpoint: User rejected treating Phase 12 contract/platform-opaque decode work as implementation-complete; Phase 12 must be corrected because it did not connect native texture interop and visible GPU preview into the desktop product.
- tdd_checkpoint: Product journey now requires backend `gpu` and composited output evidence; current product fails because it still reports backend `mock`.

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

## Eliminated

- hypothesis: "The failure is only because Playwright cannot click the product UI."
  reason: "The test successfully imports a repository fixture through the normal 导入素材 path and adds it to the timeline before failing on runtime frame presentation."
- hypothesis: "Existing requestPreviewFrame-based tests prove playback."
  reason: "The new journey explicitly verifies product playback without additional requestPreviewFrame calls, and the realtime host still reports zero presented frames."
- hypothesis: "Mock host frame tokens or preview-region color changes prove video playback."
  reason: "The green/cyan overlay was synthetic mock evidence leaked into product UI. The product journey now requires decoded/composited content evidence and requires frameDisplay to remain null in normal product playback."
- hypothesis: "The decoded content evidence failure was caused by missing media or bad timeline target selection."
  reason: "A diagnostic run showed requestRealtimePreviewContentEvidence was called for the active segment but FFmpeg timed out; the root cause was the shared process runner not draining rawvideo stdout while waiting."

## Resolution

- root_cause: Playback had three distinct gaps. First, the product path originally advanced only UI clocks/telemetry and later leaked synthetic mock surface colors into the preview. Second, the native content evidence path initially deadlocked on rawvideo stdout because the shared FFmpeg process runner did not drain pipes while waiting. Third, Phase 12 completed contracts, capability probes, platform-opaque native decode, and handle metadata, but did not implement production native texture interop into the realtime GPU compositor and visible desktop preview surface. The desktop realtime session still uses the Mock backend.
- fix: Added a product E2E gate that rejects requestPreviewFrame loops, mock frame tokens, and synthetic preview pixels as playback proof. Added native decoded-frame content evidence through the realtime host for test/recording mode, returning only a digest and metadata. Fixed the shared FFmpeg process runner to drain stdout/stderr concurrently so raw frame extraction cannot deadlock on pipe buffers.
- verification: `cargo test -p media_runtime process -- --nocapture`; `cargo test -p media_runtime_desktop ffmpeg_fallback_frame_fingerprint -- --nocapture`; `cargo test -p bindings_node realtime_preview -- --nocapture`; `pnpm --filter @video-editor/desktop build`; `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line`; `pnpm --filter @video-editor/desktop exec playwright test tests/workspace.spec.ts -g "预览播放按钮|native preview host|实时预览 telemetry|fallback|developer diagnostics display Rust-reported realtime cancellation counters" --reporter=line`; `git diff --check -- . ':!reference'`
- files_changed: crates/media_runtime/src/process.rs; crates/media_runtime/tests/process.rs; crates/media_runtime_desktop/src/ffmpeg_fallback.rs; crates/media_runtime_desktop/tests/ffmpeg_fallback.rs; crates/bindings_node/src/realtime_preview_service.rs; crates/bindings_node/src/lib.rs; apps/desktop-electron/src/main/nativeBinding.ts; apps/desktop-electron/src/main/realtimePreviewHost.ts; apps/desktop-electron/src/preload/index.ts; apps/desktop-electron/src/renderer/App.tsx; apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx; apps/desktop-electron/tests/product-user-journey.spec.ts; apps/desktop-electron/tests/helpers/userJourney.ts
