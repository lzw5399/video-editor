---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Text Editing E2E Expanded Regression

## Goal

Broaden product end-to-end validation for text and subtitle editing beyond the existing matrix: real fixture media, multiple bundled fonts, repeated font/content edits, simultaneous subtitles, staggered subtitles, preview-canvas movement, inspector rotation/scale/opacity, and native render-graph preview evidence.

## Scope

- Add high-value Playwright coverage that exercises user-visible text editing flows through the product UI.
- Use real fixture media and native realtime preview evidence; DOM text overlays or artifact preview frames must not satisfy the tests.
- Cover multiple fonts and multi-line real example copy, same-time and different-time subtitles, preview drag, and inspector visual edits.
- Keep changes focused on tests/helpers unless the new gate exposes a real product defect.

## Verification

- Targeted product Playwright text-editing tests.
- `build:electron`.
- `test:phase3-source-guards`.
- `git diff --check`.

## Result

- Added expanded product text/subtitle E2E coverage for repeated font switching, multiline CJK/Latin copy, layered text segments, same-time subtitle tracks, staggered subtitle cues, preview drag, inspector transform edits, and native render-graph evidence.
- Added a local P0 regression for the user-provided portrait material when present at `~/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4`.
- Updated product journey test helpers so overflow top feature categories such as 字幕 can be selected through the current product navigation.
