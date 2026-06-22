---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Product Titlebar Chrome Summary

## Result

The product workspace now has a dedicated Jianying-style project titlebar with visual window dots, the real draft name centered, and the export action in the top-right product action area. Feature category tabs remain below as editing tools.

## Changes

- Added `product-titlebar` with decorative macOS-style window dots and a centered `项目标题` sourced from `workspace.viewModel.project.draftName`.
- Moved the existing export button into the titlebar while keeping `产品操作` and export modal behavior unchanged.
- Split workspace rows into titlebar, feature tabs, editor body, and timeline, with explicit grid row/column placement for all workspace regions.
- Updated narrow-window media query rows so 1120px layouts do not create a stretched feature-tab area.
- Added static UI assertions for the titlebar and real draft title.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
