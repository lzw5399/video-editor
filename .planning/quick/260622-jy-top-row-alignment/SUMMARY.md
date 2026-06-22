---
status: complete
completed: 2026-06-22
---

# Jianying Top Row Alignment Summary

## Result

Aligned the product workbench top row with the Jianying reference: the feature tabs now occupy only the left material column, while the preview monitor and inspector start on the same row.

## Changes

- Moved `.top-feature-bar` to the left column instead of spanning the full workspace.
- Let `.preview-monitor` and `.inspector-panel` span the feature-tab row plus editor body row.
- Added UI reference assertions that preview/inspector headers align with the feature tabs and the material library starts below them.
- Kept selected-segment edit chrome visible over the native preview surface, while leaving text DOM overlay disabled when the real native surface is active.

## Verification

- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1`
- `VIDEO_EDITOR_P0_USER_MATERIAL=/Users/zhiwen/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4 corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "P0 user portrait" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "resizing larger and smaller" --workers=1`
- `git diff --check`
- `corepack pnpm -w run test:phase3-source-guards`

## Remaining

The subagent UI audit recommends the next isolated slice: material-bin density. Collapse/tighten the media library title, import/search/filter chrome, source rail, and material card sizing against `02-workspace-media-window.png`.
