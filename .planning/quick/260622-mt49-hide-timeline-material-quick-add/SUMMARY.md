---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Hide Timeline Material Quick Add Summary

## Result

The default product timeline toolbar no longer exposes the material select and quick add shortcut. Material-to-timeline remains centered on dragging from the media grid, with the existing compact media-card add fallback still available.

## Changes

- Hid the timeline `素材` select and adjacent `添加片段` quick-add button outside developer diagnostics.
- Preserved the existing developer diagnostic quick-add path without changing its material-id-only Rust intent boundary.
- Added a workspace assertion that product mode does not show the timeline material quick-add controls.
- Left material drag/drop, media-card icon add fallback, and Rust session timeline semantics unchanged.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
