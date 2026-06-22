---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Preview Empty Monitor

## Goal

Make the product preview empty state read as a quiet monitor surface instead of a bordered instruction card, closer to the Jianying preview reference while preserving native surface geometry.

## Scope

- Keep `.preview-canvas` dimensions, aspect ratio, and realtime host attachment unchanged.
- Reduce empty placeholder chrome and remove card-like visual emphasis.
- Preserve the product placeholder text and accessibility labels.
- Do not touch realtime playback, native surface placement, or render graph behavior.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
