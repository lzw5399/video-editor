# Native Surface Content Coverage Summary

## Decision

Confirmed: the left/lower or partial native preview issue was a macOS native surface placement and layer-geometry boundary bug, not a render graph fallback. The production fix is a child `NSWindow` overlay with explicit AppKit conversion and a `CAMetalLayer` frame that fills the child view.

## Changes

- Replaced parent-subview native preview attachment with a borderless child `NSWindow` overlay attached above the Electron `NSWindow`.
- Converted BrowserWindow content-local logical bounds through AppKit view/window/screen conversion instead of manual parent-height guesses.
- Fixed the `CAMetalLayer` geometry by anchoring it at the child view origin and setting its frame/bounds explicitly; this removed the half-width/half-height visible offset.
- Kept native WGPU presentation configured in physical drawable pixels while preserving logical placement telemetry.
- Added native surface placement diagnostics and Playwright pixel coverage evidence for the playing native surface.
- Kept cadence gates at production thresholds: 3 seconds of 30fps playback must account for 90 frames and advance at least 2.9 seconds.

## Verification

- `cargo fmt --all --check`
- `cargo test -p realtime_preview_runtime -- --nocapture`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "native surface aligned|composites video external audio text" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`

## Evidence

- Latest native surface coverage screenshot: `test-results/phase15-3/native-surface-playing-coverage.png`
- Cadence result: single-video and combo preview both presented 90 frames, dropped 0, advanced 2,966,637us, and kept `getPresentationState` p50/p95 at 0ms.
