---
status: complete
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# UI Reference Playing State Gate Summary

## Result

Completed. UI reference regression now captures playing-state product workspace evidence with the native render-graph preview surface visible through macOS window-level screen capture.

## Changes

- Added a playing-state screenshot gate to `ui-reference-regression.spec.ts`.
- The gate starts playback, waits for render-graph GPU compositor evidence to advance beyond the prior target time, verifies non-black native preview pixels, captures the full workspace, captures the preview monitor crop, then pauses playback.
- Existing static 1280x800, narrow 1120x720, material, timeline, and export modal screenshots remain covered.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1 --reporter=line`
- `git diff --check`
- Screenshot inspection:
  - `test-results/phase15-3/workspace-playing-1280x800.png`
  - `test-results/phase15-3/preview-monitor-playing-1280x800.png`
  - `test-results/phase15-3/workspace-1120x720.png`
