---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Inspector Draft Parameters Summary

## Result

Moved the product no-selection inspector closer to the Jianying draft-parameters reference by showing a direct `草稿参数` parameter list instead of an explanatory empty-state card.

## Changes

- The right-panel header now reads `草稿参数` when no segment is selected.
- Product mode hides `未选择片段` and the explanatory `这里显示...` copy.
- Developer diagnostics can still show the explanatory empty state.
- Product draft parameters now render as a flatter parameter table while preserving the `修改` action.
- Workspace expectations now assert the product empty-state copy is absent.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
