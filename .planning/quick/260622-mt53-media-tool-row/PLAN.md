---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Media Tool Row

## Goal

Bring the product media panel toolbar closer to Jianying's material library: a compact import action with icon, adjacent view/filter affordances, and search immediately below, without claiming unavailable sorting/filter menus are implemented.

## Scope

- Add app-local SVG icons sourced from `/icons` for media import/list/filter toolbar controls.
- Replace the material panel header action row with a compact media library toolbar.
- Keep the real import button, search input, existing material filters, material cards, and drag payload unchanged.
- Keep unimplemented toolbar controls disabled or inertly labeled; do not add fake menus.
- Do not change Rust project/session/timeline/media semantics.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- Visual review of `test-results/phase15-3/workspace-1280x800.png`, `workspace-1120x720.png`, and Jianying material reference.
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
