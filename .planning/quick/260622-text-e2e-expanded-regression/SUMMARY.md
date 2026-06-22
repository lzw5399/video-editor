---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Text Editing E2E Expanded Regression Summary

## Completed

- Added `product text editing UAT covers repeated font switching, multiline copy, layered text, and timed subtitles`.
- Added `P0 user portrait material supports real text and subtitle native overlay editing`, skipped automatically when the local user fixture is unavailable.
- Updated product journey helpers to select top feature categories from either visible tabs or the 更多功能 overflow menu.
- Updated the workspace SRT import test to use the same overflow-aware category selection path.

## Coverage Added

- Real media fixture plus optional real user portrait material.
- Multiple bundled fonts with repeated Sans -> Serif -> Sans switching.
- Multiline CJK/Latin title and subtitle content.
- Two simultaneous text layers plus two simultaneous subtitle tracks.
- Later staggered subtitle cues with stale cue exclusion.
- Preview-canvas movement plus inspector rotation, scale, opacity, color, alignment, line-height, and letter-spacing edits.
- Native render-graph preview evidence, native host PNG pixel checks, and no artifact preview frame fallback.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "P0 user portrait material supports real text|multi-font multi-track native preview evidence|preview drag, multi-font captions|repeated font switching" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "字幕 SRT import intent path" --workers=1 --reporter=line`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
