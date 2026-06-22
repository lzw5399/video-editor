---
status: complete
completed: 2026-06-22
---

# Jianying Material Bin Density Summary

## Result

Tightened the product media bin toward the Jianying reference without changing draft semantics, import commands, or drag-to-timeline behavior.

## Changes

- Removed the extra visible `素材` title row in the media library pane.
- Merged import, search, and view/filter controls into a single compact toolbar row.
- Reduced media source rail button height, spacing, and padding.
- Changed material cards to fixed dense tiles so narrow windows no longer stretch one card across the library pane.
- Added UI reference geometry assertions for the compact source rail, missing title row, same-row import/search controls, and dense material tile width.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `VIDEO_EDITOR_P0_USER_MATERIAL=/Users/zhiwen/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4 corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "P0 user portrait" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "resizing larger and smaller" --workers=1`
- `git diff --check`
- `corepack pnpm -w run test:phase3-source-guards`

## Notes

Do not run packaged foreground app tests in parallel. A parallel P0/resize run produced transient `placement: null` because two real macOS app instances competed for foreground/native surface state; the same resize gate passed when run sequentially.
