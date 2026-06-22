# Native Surface Window Reflow Summary

## Result

Closed the P0 native preview placement drift during BrowserWindow geometry changes by re-applying the last renderer-owned content-local host bounds from the Electron main/native boundary.

## Changes

- Added throttled BrowserWindow geometry listeners for move, resize, maximize, restore, fullscreen, and show events in `RealtimePreviewHost`.
- Reflow now calls the native surface bounds update path from main process using the last known content-local logical rect, records `reflowSurfaceBounds` telemetry, refreshes cached state, and does not restart playback.
- Added a packaged product test bridge for moving the main BrowserWindow.
- Added product playback regression coverage for window move, maximize, and restore while playing, with native host/workspace screenshots and no-restart assertions.
- Kept DOM overlay as editing aid only; product preview evidence still requires native render-graph/GPU composited output.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "maximizing and restoring" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "native surface aligned|resizing larger and smaller|maximizing and restoring" --reporter=line`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`

## Follow-Up

This is a correct containment at the Electron/AppKit boundary. The final production shape should split placement-only reflow from size/scale reconfigure in Rust/native so pure BrowserWindow moves do not pass through the full surface bounds update path.
