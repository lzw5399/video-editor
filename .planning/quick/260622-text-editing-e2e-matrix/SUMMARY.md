---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Text Editing E2E Matrix Expansion Summary

## Completed

- Added a packaged product E2E gate covering real video/audio fixture media, one manual text overlay, three SRT subtitle tracks, same-time multi-subtitle overlays, staggered subtitle cues, later subtitle cues, two bundled CJK fonts, color/layout/line-height/letter-spacing edits, preview-canvas drag movement, and inspector rotation/scale/opacity edits.
- Tightened native text overlay evidence waiting with exact active-overlay counts and forbidden stale contents so old subtitle cues cannot satisfy later timeline checks.
- Added preview-drag verification that waits for Rust session visual-update commands, then proves the updated position appears in native render-graph evidence and changes the native host PNG.
- Fixed realtime preview refresh after preview-affecting project session intents (`editSelectedText`, `importSubtitleSrtIntent`, `updateSelectedSegmentVisual`, and track visibility changes) so text/visual edits update the native preview immediately instead of waiting for playback.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "preview drag, multi-font captions" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "multi-font multi-track native preview evidence" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "direct canvas drag" --workers=1 --reporter=line`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
