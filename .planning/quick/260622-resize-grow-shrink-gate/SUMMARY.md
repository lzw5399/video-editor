---
status: complete
completed: 2026-06-22
---

# Resize Grow/Shrink Preview Gate Summary

## Result

Made the Rust preview-service resize gate match the product UAT contract: playback resize is now tested while the surface grows and then shrinks.

## Changes

- Renamed the scheduler resize test to explicitly cover grow/shrink playback resize.
- Added a shared test helper that asserts each resize direction keeps the existing playback generation and continues presenting frames.
- Preserved the existing Playwright product gate that resizes 1120x720 -> 1500x900 -> 1120x720 during playback and captures expanded/narrow native preview evidence.

## Verification

- `cargo test -p bindings_node scheduler_surface_resize_during_playback_grow_and_shrink_keeps_generation_and_worker -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "resizing larger and smaller" --workers=1`
- `git diff --check`
