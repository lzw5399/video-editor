---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Material Thumbnail Refs

## Goal

Let product material cards show real thumbnail artifacts when Rust/artifact-store status exposes a ready thumbnail display reference, without renderer-side media processing or fake thumbnails.

## Scope

- Preserve Rust/session/artifact-store ownership of derived thumbnail generation.
- Carry ready thumbnail `displayRef` data through the renderer display model.
- Resolve project-relative display refs against the current `.veproj` bundle path for image display only.
- Keep deterministic kind-specific fallback visuals when no ready thumbnail artifact exists.
- Do not add PATH/Homebrew FFmpeg discovery, renderer-side ffmpeg calls, or semantic draft construction.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
