---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# P0 Preview Export Audio Regressions

## Goal

Fix the reported product regressions with `~/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4` without introducing fallback preview behavior:

- Preview is too large.
- Preview image looks washed out.
- Dragging material to the timeline does not present a frame until playback starts.
- Export dialog appears behind the preview/native surface.
- Preview audio is intermittently silent.

## Scope

- Preserve Rust-owned preview clock/scheduler and renderGraphGpu evidence.
- Use the real P0 material when present; tests must not silently pass on fake preview evidence.
- Fix native surface/modal layering at the ownership boundary instead of CSS-only offsets.
- Keep product UI diagnostics hidden unless explicitly in developer mode.
- Add or tighten Playwright/Rust gates that fail the reported states.

## Verification

- `git diff --check`
- Focused unit/Rust tests for any runtime changes.
- Focused Playwright product tests covering:
  - P0 material first frame after drag before playback.
  - Native surface/content bounded within `.preview-canvas`.
  - Export modal topmost over preview during playback.
  - Native audio playback session starts reliably for embedded-video audio.
- Packaged app validation if native surface/AppKit behavior changes.
