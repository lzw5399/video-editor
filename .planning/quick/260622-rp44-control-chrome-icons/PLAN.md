---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Control Chrome Icons

## Goal

Replace product-visible preview and timeline control glyph/text chrome with app-local SVG icons from `/Users/zhiwen/code/video-editor/icons`, matching the Jianying reference control surfaces more closely without changing edit or playback semantics.

## Scope

- Add missing preview/timeline/track-header icon assets to the renderer icon bundle and manifest.
- Replace preview transport raw symbols for stop, previous frame, next frame, and fit with SVG masks; keep canvas ratio as text; remove the disabled fullscreen fake control.
- Replace timeline add/track/snap controls and track-header state text glyphs with SVG icon buttons where commands already exist.
- Hide product-mode timeline idle diagnostic status copy while retaining developer diagnostics elsewhere.
- Keep command payloads, project session boundaries, preview playback, and FFmpeg runtime untouched.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
