---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Material Library Jianying Density

## Goal

Bring the left material library closer to the Jianying reference while preserving current Rust-owned import, drag-to-timeline, preview, and export behavior.

## Scope

- Widen the material library work area enough for a real source rail plus usable material cards.
- Restyle the source rail, import/search/filter toolbar, and material cards toward the Jianying reference screenshots.
- Keep product-mode diagnostics hidden and keep drag-to-timeline available.
- Keep this as a UI/layout slice only; do not change draft semantics, preview scheduler, FFmpeg distribution, or native surface placement.

## Verification

- Passed: `corepack pnpm --dir apps/desktop-electron run build:electron`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states|workspace panels switch categories without losing Chinese empty states" --workers=1`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product user can import a repo video, add it to the timeline, and see render-graph GPU playback frames advance" --workers=1`
- Reviewed: `test-results/phase15-3/workspace-1280x800.png`, `workspace-1120x720.png`, `material-library-1280x800.png`, and `material-library-1120x720.png` against `docs/ui-reference/jianying-pro/screenshots/04-left-material-library.png`.
- Passed: `git diff --check`
