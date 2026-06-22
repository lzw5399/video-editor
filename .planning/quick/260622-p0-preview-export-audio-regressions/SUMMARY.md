# P0 Preview Export Audio Regressions Summary

## Result

Fixed the product preview/export/audio regressions against the real user portrait material while preserving the renderGraphGpu/native-surface path.

## Changes

- Presented a real still frame after dragging media to the timeline by waiting for the Rust still-frame presentation evidence before caching the seek target.
- Fixed washed-out NV12 preview output by treating unknown video transfer as BT.709 linear sampling into the sRGB surface target instead of double-encoding.
- Suspended the native preview surface before opening the export dialog so the modal is not hidden behind the child native surface.
- Made playback enter the running UI state only after expected native audio accepts playback; failed audio now pauses video instead of leaving silent playback.
- Made native surface resize/maximize geometry updates coalesced and bounds-only: resize no longer advances preview generation, restarts the playback worker, or emits generic control events.
- Removed the AppKit fixed delay from native surface commits so resize updates do not throttle playback.
- Added product E2E gates for first-frame P0 material, native surface alignment, export z-order, maximize playback sync, and embedded-video audio.

## Verification

- `git diff --check`
- `corepack pnpm -w run test:phase3-source-guards`
- `cargo test -p realtime_preview_runtime update_surface_bounds_keeps_playback_generation_and_state --locked`
- `cargo test -p realtime_preview_runtime nv12_ --locked`
- `cargo test -p bindings_node scheduler_surface_resize_during_playback_keeps_generation_and_worker --locked`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`
- `VIDEO_EDITOR_P0_USER_MATERIAL=/Users/zhiwen/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4 corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "P0 user portrait|keeps the native surface aligned|keeps native preview synced while maximizing|plays embedded video audio" --workers=1`
