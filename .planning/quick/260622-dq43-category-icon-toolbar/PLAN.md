---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Category Icon Toolbar

## Goal

Replace the product workspace top category glyphs with app-local SVG icon assets sourced from `/Users/zhiwen/code/video-editor/icons`, matching the Jianying reference toolbar more closely without changing Rust-owned editing semantics.

## Scope

- Copy the needed Jianying-style SVG assets into the renderer app-local icon bundle and register them in the icon manifest.
- Move category icon selection out of semantic view-model metadata and into the workspace UI layer.
- Keep category labels, active state, keyboard/accessibility labels, and product E2E selectors stable.
- Do not change timeline command payloads, preview playback, FFmpeg runtime discovery, or project session semantics.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
