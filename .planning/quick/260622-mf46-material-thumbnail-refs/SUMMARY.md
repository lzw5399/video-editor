---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Material Thumbnail Refs Summary

## Result

Product material cards now display real ready thumbnail artifacts when Rust artifact status exposes a thumbnail `displayRef`. The renderer only resolves a bundle-local file URL for display; it does not generate thumbnails, inspect media, or call FFmpeg.

## Changes

- Threaded ready thumbnail `displayRef` data through the renderer resource display model as `thumbnailRef`.
- Updated material cards to render `<img>` only for ready thumbnail artifact refs from the current project bundle.
- Added strict project-relative path validation before converting refs to `file://` URLs.
- Kept deterministic kind-specific fallback visuals for materials without a ready thumbnail artifact.
- Added a display-model test proving only ready thumbnail artifact refs become material thumbnail refs.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "resource panel carries ready thumbnail display refs from artifact status" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
