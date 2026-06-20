---
status: fixing
trigger: "User manual UAT reports Phase 15.2 product playback is not complete: preview surface is offset left/down during play, playback has no audio, video playback is stuttery/desynchronized from the timeline, and text/font editing plus on-canvas drag/rotate interactions are not available."
created: "2026-06-20T05:13:58Z"
updated: "2026-06-20T07:41:43Z"
---

# Debug Session: 15.2 Product Playback Regression

## Symptoms

- expected_behavior: "As a normal editor user, import a real video, add it to the timeline, press play, and see the video play in the preview monitor at the correct location with audio and timeline-synchronized motion. Basic text editing and on-canvas transform interactions should be usable if the UI exposes them."
- actual_behavior: "Playback preview appears offset left/down from the intended preview monitor, audio is silent, video motion stutters and keeps moving after the timeline has advanced/finished, and text/font editing plus drag/rotate interactions are not usable from the visible editor surface."
- error_messages: "No explicit error reported by the user; the failure is visible product behavior."
- timeline: "Discovered during manual UAT immediately after Phase 15.2 was marked complete and Phase 15.3 planning started."
- reproduction: "Open the desktop app, import a real driving-recorder video, add it to the timeline, press play, observe preview placement/audio/sync, then try visible text/font and on-canvas transform interactions."

## Current Focus

- hypothesis: "Phase 15.2 tests proved limited GPU compositor evidence and preview-region pixel motion, but did not assert full product playback placement, audible audio routing, timeline/video clock synchronization, or direct user-editing interactions."
- test: "Re-run/read the normal Electron Playwright product journey, inspect the playback host/main/renderer contracts, then add failing UAT-level checks before changing implementation."
- expecting: "Existing 15.2 E2E will pass despite missing one or more user-visible requirements, proving the gate is insufficient."
- next_action: "Commit the embedded video-audio GREEN fix, then run the final 15.2 product playback verification bundle before resolving the debug session."
- reasoning_checkpoint: "Do not continue Phase 15.3 until this debug session either identifies and fixes the 15.2 gap or formally reopens 15.2 with an executable remediation plan."
- tdd_checkpoint: "Add or strengthen E2E/regression tests before accepting any playback fix."

## Evidence

- timestamp: "2026-06-20T05:18:xxZ"
  observation: "`pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line` passed 3/3 in 33.1s while the user-visible playback defects remain reported."
  implication: "The current Phase 15.2 product E2E gate is insufficient; a green product-user-journey run does not prove normal-user playback quality."
- timestamp: "2026-06-20T05:20:xxZ"
  observation: "`apps/desktop-electron/tests/product-user-journey.spec.ts` playback success asserts renderGraphGpuComposited evidence, telemetry increase, timecode increase, and center-region hash change, but does not assert native surface/container alignment, audible audio output, end-of-playback sync, or direct on-canvas editing interactions."
  implication: "Existing assertions can pass when the preview is offset, silent, or clock-desynchronized."
- timestamp: "2026-06-20T05:22:xxZ"
  observation: "`apps/desktop-electron/src/renderer/App.tsx` advances the visible playhead with a renderer `requestAnimationFrame` loop, while realtime preview presentation is driven by a separate host/scheduler path."
  implication: "The UI playhead and presented video are not governed by one production TimelineClock, so timeline/video drift is structurally possible."
- timestamp: "2026-06-20T05:24:xxZ"
  observation: "`crates/bindings_node/src/audio_service.rs` updates audio preview runtime state but does not create or drive an `audio_output_desktop` CPAL sink for product playback. `audio_output_desktop::create_desktop_audio_output` still returns a mock output factory."
  implication: "Visible audio playback status is not proof of actual audible output; the no-audio report is an implementation gap, not only a test gap."
- timestamp: "2026-06-20T05:26:xxZ"
  observation: "`PreviewMonitor` publishes DOM `getBoundingClientRect()` viewport coordinates to the native realtime host; macOS native/WGPU attachment code converts those values into AppKit child view frames. No test asserts the resulting native child view frame equals the preview host screen rect."
  implication: "Surface placement can be wrong while center-pixel motion still changes enough to pass the current test."
- timestamp: "2026-06-20T05:27:xxZ"
  observation: "The editing matrix updates transforms through inspector form fields and command calls; it does not simulate normal on-canvas text selection, drag, resize, rotate, or crop handles."
  implication: "Visible editor interactions that a normal user expects are not covered and should not have been counted as complete."
- timestamp: "2026-06-20T05:46:xxZ"
  observation: "Added and ran RED product playback UAT tests. Surface placement fails because host state exposes no native placement evidence; audio fails because output status remains `系统默认`; sync fails because timeline time and presented video target time differ by 1,366,683us at sequence end."
  implication: "The reported offset/audio/desync issues are now executable product E2E failures instead of informal observations."
- timestamp: "2026-06-20T05:49:xxZ"
  observation: "Added and ran RED direct text/transform UAT. Dragging visible preview text produces no `updateSegmentVisual` command and no canvas movement."
  implication: "Visible text overlay interaction is not implemented as a normal user canvas operation."
- timestamp: "2026-06-20T05:52:00Z"
  observation: "Added native/WGPU surface placement evidence and verified `pnpm --filter @video-editor/desktop package:dir` plus `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line -g \"native surface aligned\"` passes against the packaged app."
  implication: "The preview host can now prove native surface placement aligns with the preview monitor; remaining 15.2-07 failures are audio output, playback clock sync, and direct canvas editing."
- timestamp: "2026-06-20T06:25:00Z"
  observation: "Replaced renderer-owned playback head advancement with realtime host telemetry and added a binding scheduler wall-clock playback anchor that presents the current target time instead of one frame per poll. Verified `pnpm --filter @video-editor/desktop package:dir` plus `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line -g \"synchronized with timeline\"` passes."
  implication: "The timeline no longer falsely runs ahead of presented video; the compositor may drop/skip to the current playback target rather than playing late frames after the UI has finished."
- timestamp: "2026-06-20T06:38:00Z"
  observation: "Added direct preview overlay pointer drag that commits through `updateSegmentVisual` and applies segment visual transform to text overlay display. Verified `pnpm --filter @video-editor/desktop package:dir` plus `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line -g \"direct canvas drag\"` passes."
  implication: "Visible text/segment overlays now support a normal-user canvas drag path backed by the existing timeline command model."
- timestamp: "2026-06-20T07:23:51Z"
  observation: "Replaced status-only audio preview with a native CPAL PCM output path for timeline audio materials, removed the renderer label masking that reported default devices as `系统默认`, and normalized native surface placement evidence across Electron/AppKit coordinate conventions. Verified `cargo check -p audio_output_desktop -p bindings_node --locked`, `cargo test -p bindings_node audio_service -- --nocapture`, `pnpm --filter @video-editor/desktop package:dir`, `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line -g \"product playback UAT|product text and transform interaction UAT\"`, and the full `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line` pass."
  implication: "The reopened 15.2 product UAT checks for native surface placement, separate timeline-audio native output, timeline/video sync, and direct canvas drag now pass against the packaged desktop app. This still does not prove embedded audio from a video material plays, so the dashcam-video no-sound case remains open until covered by its own product UAT."
- timestamp: "2026-06-20T07:41:43Z"
  observation: "Added `p0-av-tone-testsrc.mp4` as a repository fixture with video plus AAC audio, committed a RED product UAT proving a video-only timeline segment with embedded audio stayed paused, then extended the Rust audio preview service to decode non-WAV media audio through the desktop FFmpeg runtime and mix it into the native CPAL output queue. Verified `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line -g \"embedded video audio\"`, `cargo test -p bindings_node audio_service -- --nocapture`, `cargo check -p audio_output_desktop -p bindings_node --locked`, and full `pnpm --filter @video-editor/desktop exec playwright test tests/product-user-journey.spec.ts --reporter=line` pass."
  implication: "The dashcam-style case is now covered by an end-to-end packaged-app journey: importing a video material that contains an audio stream, adding it to the timeline, and pressing play must produce native audio output evidence instead of a status-only paused state."

## Eliminated

- hypothesis: "The issue is simply that the old product E2E was not rerun."
  evidence: "The current product-user-journey suite was rerun and passed; the suite itself is missing required assertions."

## Resolution

- root_cause:
- fix:
- verification:
- files_changed:
