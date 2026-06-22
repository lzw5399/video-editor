---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Hide Available Material Status

## Goal

Remove the default green `可用` status chip from product material cards so the media library reads like a user-facing asset browser, while keeping actionable problem states visible.

## Scope

- Hide `available` status labels in material cards.
- Keep `missing` and `probeFailed` labels and warnings visible.
- Preserve filtering, drag eligibility, import behavior, and generated material status semantics.
- Update product tests so they reject `可用` in the normal media card while still covering problem states.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states|auto canvas adopts the first imported portrait material without renderer-owned canvas math|预览播放按钮使用实时预览画面而不是连续请求预览帧" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- Visual review of `test-results/phase15-3/workspace-1280x800.png`, `workspace-1120x720.png`, and Jianying material reference.
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
