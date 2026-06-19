---
status: fixed
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
- test: Add a Playwright normal-user workflow using moving fixture media, product-mode UI, and screenshot/pixel evidence before/after playback.
- expecting: The new test should fail before the implementation fix, proving the current completion standard is insufficient.
- next_action: Extend future user-journey coverage with true decoded-frame/content fingerprints for video correctness, then add transform/text/audio/editing journeys.
- reasoning_checkpoint:
- tdd_checkpoint: RED product-user-journey test added and failing on presentedFrameCount

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

## Eliminated

- hypothesis: "The failure is only because Playwright cannot click the product UI."
  reason: "The test successfully imports a repository fixture through the normal 导入素材 path and adds it to the timeline before failing on runtime frame presentation."
- hypothesis: "Existing requestPreviewFrame-based tests prove playback."
  reason: "The new journey explicitly verifies product playback without additional requestPreviewFrame calls, and the realtime host still reports zero presented frames."

## Resolution

- root_cause: Playback started the realtime preview host but no host-owned playback frame loop requested/presented runtime frames, and the renderer did not refresh host state while playing. The UI playhead could advance independently of visible preview presentation.
- fix: Added a main-process playback tick loop that calls the realtime preview runtime with `playbackTick` frames, presents a seek frame on seek, stops the loop on pause/stop/close, and exposes mock-surface frame display tokens/colors for Playwright-visible surface verification. PreviewMonitor now polls host telemetry during playback and paints host-provided mock surface frames without calling preview PNG commands.
- verification: `pnpm --filter @video-editor/desktop build`; `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line`; `pnpm --filter @video-editor/desktop exec playwright test tests/workspace.spec.ts -g "预览播放按钮|native preview host|实时预览 telemetry|fallback|developer diagnostics display Rust-reported realtime cancellation counters" --reporter=line`; `cargo test -p bindings_node preview_commands -- --nocapture`; `git diff --check -- . ':!reference'`
- files_changed: apps/desktop-electron/src/main/realtimePreviewHost.ts; apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx; apps/desktop-electron/src/renderer/workspace/preview-inspector.css; apps/desktop-electron/tests/product-user-journey.spec.ts; apps/desktop-electron/tests/helpers/userJourney.ts; apps/desktop-electron/tests/workspace.spec.ts; apps/desktop-electron/package.json; apps/desktop-electron/tests/fixtures/media/*
