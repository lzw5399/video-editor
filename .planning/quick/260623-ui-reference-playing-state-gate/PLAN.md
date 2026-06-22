---
status: complete
created: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# UI Reference Playing State Gate

## Goal

Make the UI reference regression produce authoritative playing-state screenshots alongside the existing static and narrow workspace evidence, so future Jianying-style UI changes are reviewed against static, playing, and narrow desktop states.

## Scope

- Capture a macOS window-level playing workspace screenshot while the native render-graph preview surface is actively presenting.
- Capture the playing preview monitor crop from the same native screen-capture path.
- Keep the gate product-safe: no DOM fallback, no artifact preview frame proof, and no developer diagnostics in product UI.
- Preserve existing 1280x800 and 1120x720 static/narrow screenshots.

## Verification

- UI reference regression Playwright test.
- Inspect refreshed static, playing, and narrow screenshots.
- `build:electron`.
- `git diff --check`.
