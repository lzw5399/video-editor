---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Material Card Icon Action

## Goal

Reduce the product media card chrome so drag-to-timeline is the primary interaction and the click fallback is a compact icon action, closer to Jianying's media grid.

## Scope

- Replace the always-visible `添加到时间线` text button in material cards with an icon-only add action over the thumbnail.
- Keep accessible Chinese `aria-label` and `title` for the icon action.
- Keep the existing material-id-only drag payload and Rust `addTimelineSegmentIntent` path unchanged.
- Do not introduce renderer-side timeline construction, FFmpeg/media processing, or thumbnail generation.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
