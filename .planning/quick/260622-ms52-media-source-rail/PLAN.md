---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Media Source Rail

## Goal

Bring the product media panel closer to Jianying's material library hierarchy by adding a source rail and denser media tool row while keeping real import/search/filter/drag behavior unchanged.

## Scope

- Add a media-source rail with `导入` active and unavailable sources disabled.
- Keep the real import button, search input, material filters, material cards, and material drag payload unchanged.
- Do not claim cloud/AI/source libraries are implemented.
- Do not generate fake thumbnails or expose artifact/cache diagnostics in product mode.
- Do not change Rust project/session/timeline semantics.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
