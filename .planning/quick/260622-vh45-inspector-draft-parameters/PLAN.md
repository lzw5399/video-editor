---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Inspector Draft Parameters

## Goal

Move the product right inspector closer to the Jianying draft-parameters reference by replacing the explanatory empty state with a direct draft parameter list when no segment is selected.

## Scope

- Show `草稿参数` as the right-panel header when no timeline segment is selected.
- Hide product-mode explanatory empty-state copy such as `未选择片段` and `这里显示...`.
- Keep developer diagnostics able to show the explanatory empty state.
- Restyle the no-selection draft parameter list as a flat parameter table instead of nested card-like diagnostic rows.
- Do not invent unsupported parameters or change Rust/session project semantics.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
