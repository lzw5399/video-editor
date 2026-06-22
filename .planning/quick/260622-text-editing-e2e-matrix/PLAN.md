---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Text Editing E2E Matrix Expansion

## Goal

Add broader product end-to-end validation for text/subtitle editing using the real packaged preview path: real fixture media, multiple bundled fonts, same-time text/subtitle overlays, different subtitle tracks and cue times, preview-canvas drag movement, inspector rotation/scale/opacity, and content/style edits.

## Scope

- Extend Playwright product UAT coverage around native render-graph text evidence.
- Verify edits through native host PNG pixels and realtime preview evidence, not DOM overlays or artifact preview requests.
- Cover same-time multi-subtitle overlays and later cue transitions across separate text/subtitle tracks.
- Include direct preview drag plus inspector visual edits for rotation, scaling, opacity, and position.

## Verification

- Targeted product Playwright test for the new text-editing matrix.
- Existing text/subtitle product gate remains compatible.
- `build:electron` and `git diff --check`.
