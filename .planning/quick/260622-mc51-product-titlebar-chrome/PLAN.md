---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Product Titlebar Chrome

## Goal

Move the product workspace closer to Jianying's top chrome by adding a dedicated project titlebar with the real draft name and top-right export action, while keeping feature tabs as editing tools below it.

## Scope

- Add a top titlebar row with window-style visual dots, centered real draft name, and product actions.
- Move the export action into the titlebar while preserving the existing `产品操作` label and modal behavior.
- Keep feature category tabs as SVG-backed tool tabs in the second row.
- Do not show fake autosave/cloud/share state or internal paths.
- Do not change Rust project/session/timeline semantics or preview behavior.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
