---
status: complete
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# UI Reference First Frame Evidence Summary

## Result

UI reference screenshots now capture a real native render-graph preview frame instead of a black DOM host. The healthy reference workspace plays until native preview evidence is product-ready, verifies the macOS screen capture contains non-black image detail, pauses, and then captures workspace/preview/timeline/material screenshots through window-level screen capture on macOS.

## Changes

- Added native preview readiness polling to `launchWorkspaceApp`.
- Added macOS screen-region screenshot helpers for full workspace and panel captures, so native child surfaces appear in reference screenshots.
- Added non-black/native-detail luma checks before screenshots.
- Preserved Playwright DOM screenshots for non-macOS fallback.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1 --reporter=line`
- Refreshed and inspected:
  - `test-results/phase15-3/workspace-1280x800.png`
  - `test-results/phase15-3/workspace-1120x720.png`
  - `test-results/phase15-3/preview-monitor-1280x800.png`
- `git diff --check`
