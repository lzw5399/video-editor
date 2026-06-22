---
status: complete
completed: 2026-06-22
---

# Jianying Preview Alignment Summary

## Result

Added production gates for the reported native preview placement risk. The resize UAT now exercises playback while resizing larger and then smaller, and both directions must preserve native surface alignment, media-clock sync, no playback restart, and OS-level visible host content.

## Changes

- Added test-only `resizeMainWindow(width, height)` bridge for packaged app resize validation.
- Tightened P0 portrait material validation with OS-level PNG aspect ratio, foreground coverage, centered foreground bbox, and balanced margin checks.
- Extended playback resize UAT from maximize-only to explicit grow and shrink resize.
- Captured expanded and narrow playback workspace/host screenshots under `test-results/phase15-3/`.
- Added landscape host placement checks after both resize directions.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "resizing larger and smaller" --workers=1`
- `VIDEO_EDITOR_P0_USER_MATERIAL=/Users/zhiwen/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4 corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "P0 user portrait" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1`
- `git diff --check`
- `corepack pnpm -w run test:phase3-source-guards`

## Remaining

Full pixel-level Jianying replication remains active in the thread goal. The current workspace proportions and timeline density are still visibly different from the Jianying reference and need additional destructive UI work in later slices.
