---
status: investigating
trigger: "User manual UAT reports Phase 15.2 product playback is not complete: preview surface is offset left/down during play, playback has no audio, video playback is stuttery/desynchronized from the timeline, and text/font editing plus on-canvas drag/rotate interactions are not available."
created: "2026-06-20T05:13:58Z"
updated: "2026-06-20T05:28:40Z"
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
- next_action: "Inspect 15.2 verification artifacts and product E2E coverage, then reproduce with the app under Playwright."
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

## Eliminated

- hypothesis: "The issue is simply that the old product E2E was not rerun."
  evidence: "The current product-user-journey suite was rerun and passed; the suite itself is missing required assertions."

## Resolution

- root_cause:
- fix:
- verification:
- files_changed:
