# Product E2E Acceptance Policy

This policy defines what counts as completion for user-visible editor behavior.
It is a required review gate for playback, preview, timeline editing, export,
and any default UI control that claims to edit video content.

## Rule

Code-level contracts are not enough. A product-facing editing feature is not
complete until a Playwright/Electron test performs the same action a normal user
would perform and verifies the visible or exported result.

The test must start from the UI whenever possible:

1. Create a new project or open an existing project first when the product
   entry shell exists.
2. Import repository-owned fixture media through the product import path.
3. Add or drag the material to a timeline track.
4. Perform the edit through visible controls, mouse/keyboard interaction, or the
   same command bridge the UI uses.
5. Verify the preview, timeline, inspector, save/reopen state, or export output
   that a user would judge.
6. Assert that fallback, mock, debug, first-frame, artifact, or CPU-probe paths
   did not satisfy the success condition.
7. Assert that replaced legacy implementations are not still reachable as a
   product substitute for the new path.

## Required Case Families

Feature work should add or extend cases in the smallest useful matrix. The
matrix should grow over time and must cover these families before a feature is
called production-ready:

- material import: video, image, audio, missing/unavailable media
- project entry: create new project, open existing project, enter editor before
  material import
- timeline: add, drag/move, edge trim, split, delete, undo, redo, snapping
- playback: play, pause, seek, scrub, playhead drag, previous/next frame
- composition: video layer, image overlay, text overlay, track visibility,
  stacking order, fit/fill/stretch, transform, opacity, crop when exposed
- text: bundled font, content edit, size, color, position, preview/export parity
- audio: playback, mute, volume, multiple-track mix state where exposed
- persistence: save, close/reopen, semantic equality for edited drafts
- export: output file, duration, resolution, fps, audio stream, preview/export
  parity for the supported subset, launched from the top-right modal flow once
  the production UI convergence phase lands
- production UI: default visible controls must either work in the E2E matrix or
  be hidden/gated until implemented

## Evidence Requirements

Tests must verify actual product evidence, not implementation-adjacent signals.

- Preview success requires visible compositor output that changes when timeline
  time or edit state changes.
- Playback success requires both timeline time advancement and rendered content
  advancement.
- Export success requires output-file validation and semantic parity with the
  supported preview subset.
- Save/reopen success requires persisted draft semantics, not only UI state.

## Fixtures

Use repository-owned generated fixtures or committed deterministic media so the
case can run without selecting local files manually. Do not rely on user
downloads, absolute machine-local paths, or fake placeholder media for product
acceptance.

## Review Checklist

Every review touching user-visible behavior must ask:

- What exact user workflow proves this feature?
- Which Playwright/Electron test performs that workflow?
- Does the assertion inspect the visible preview, timeline state, saved project,
  or exported video rather than only a Rust/unit return value?
- Does the test fail if fallback/mock/artifact/CPU evidence is used?
- Are unsupported visible controls hidden or explicitly unavailable instead of
  appearing functional?
- Did the change extend the matrix when it added a new visible editing behavior?
- Did the review apply `docs/refactor-and-legacy-cleanup-policy.md` when the
  feature replaces an older implementation?
