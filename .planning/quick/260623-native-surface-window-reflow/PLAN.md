---
status: complete
created: 2026-06-23
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Native Surface Window Reflow

## Goal

Fix the P0 preview placement gap where the native realtime preview surface can lag or drift during BrowserWindow move/maximize/restore even when the DOM host rect has not changed in viewport-local coordinates.

## Production Decision

Partially correct current chain: renderer-owned DOM `ResizeObserver` is a valid source for content-local host bounds, but it is not sufficient for screen-space placement because BrowserWindow screen origin can change independently. Native surface placement must be owned at the Electron main/AppKit boundary using the last known content-local bounds and BrowserWindow geometry events.

## Scope

- Keep the coordinate contract as BrowserWindow content-local logical pixels from renderer to main.
- Main process must re-apply the last content-local bounds on BrowserWindow move/resize/maximize/unmaximize/fullscreen transitions.
- Do not introduce CSS offsets, fixed delays, or guessed y correction.
- Telemetry must continue exposing DOM host screen rect, raw AppKit screen rect, converted native screen rect, and max delta.
- Add a product playback regression for maximize/restore placement while playing, with screenshot evidence.

## Verification

- Product journey placement/maximize regression must prove `surfacePlacement.maxDeltaPx <= 2` after maximize and after restore.
- Captured playing screenshots for maximized and restored windows must show centered native content, not lower-left placement.
- Existing native surface alignment/resize tests must still pass.
- `build:electron`, relevant Rust tests for macOS coordinate conversion if touched, source guards, and `git diff --check`.
