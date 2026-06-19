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
- test: Add a Playwright normal-user workflow using moving fixture media, product-mode UI, and screenshot/pixel evidence before/after playback.
- expecting: The new test should fail before the implementation fix, proving the current completion standard is insufficient.
- next_action: Fix the playback chain so the realtime preview host presents advancing frames while the product playhead runs, then extend the evidence beyond telemetry to content/frame fingerprints.
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

## Eliminated

- hypothesis: "The failure is only because Playwright cannot click the product UI."
  reason: "The test successfully imports a repository fixture through the normal 导入素材 path and adds it to the timeline before failing on runtime frame presentation."
- hypothesis: "Existing requestPreviewFrame-based tests prove playback."
  reason: "The new journey explicitly verifies product playback without additional requestPreviewFrame calls, and the realtime host still reports zero presented frames."

## Resolution

- root_cause:
- fix:
- verification:
- files_changed:
