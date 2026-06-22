---
status: complete
created: 2026-06-23
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# UI Reference First Frame Evidence

## Goal

Make product UI reference screenshots show a real native preview frame for the healthy fixture project instead of a black preview monitor. Static workspace screenshots should be useful for Jianying visual comparison and must not rely on artifact or DOM fallback preview.

## Scope

- Update UI reference regression setup to request/wait for product-ready render-graph GPU preview evidence before capturing workspace and preview-monitor screenshots.
- Keep product mode free of diagnostics and fallback copy.
- Preserve the existing 1280x800 and 1120x720 screenshot outputs.

## Verification

- UI reference regression Playwright test.
- Inspect refreshed workspace and preview-monitor screenshots.
- `build:electron`.
- `git diff --check`.
