---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Material Card Icon Action Summary

## Result

Product material cards no longer show a dominant `添加到时间线` text button. The media grid now treats drag-to-timeline as the primary interaction while preserving a compact icon-only add fallback with the same accessible Chinese name.

## Changes

- Moved the add action into the thumbnail corner as an icon button using the app-local `timelineAdd` SVG.
- Removed the extra material-card button row and tightened card height in both grid and narrow layouts.
- Kept the click fallback on the existing Rust-owned `addTimelineSegmentIntent` path through the same `materialId` only.
- Preserved `aria-label` and `title` so existing E2E helpers and keyboard/accessibility flows remain valid.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
