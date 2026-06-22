---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Hide Timeline Material Quick Add

## Goal

Remove the default product timeline toolbar's material select/add shortcut so the main material-to-timeline path is drag from the media grid, matching the production editor boundary and Jianying-style toolbar density.

## Scope

- Hide the `素材` select and adjacent quick add button in product mode.
- Keep the existing material-id-only quick add path available only under developer diagnostics.
- Keep material drag-to-timeline and media-card icon fallback unchanged.
- Do not add renderer-owned track, segment, sourceTimerange, or targetTimerange construction.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
