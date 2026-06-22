---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Preview Canvas Fill

## Goal

Fix the product preview monitor layout so the canvas uses the available preview area instead of being compressed by unused diagnostic grid rows.

## Scope

- Change product `.preview-shell` grid rows to match the actual product children: title, canvas, transport.
- Keep developer diagnostics layout separate and unchanged.
- Add a UI reference regression assertion that the preview canvas occupies a production-sized share of the preview panel.
- Do not touch realtime scheduler, native surface placement code, render graph, or playback controls.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
