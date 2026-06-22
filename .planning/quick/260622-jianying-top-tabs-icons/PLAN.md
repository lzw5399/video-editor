---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Jianying Top Tabs And Icon Polish

## Goal

Move the product workspace chrome and top feature tabs closer to the Jianying Pro reference screenshots while preserving the current Rust-owned preview/playback/export behavior.

## Scope

- Remove the oversized left product mark from the feature-tab row.
- Add a Jianying-like titlebar left status area and right action cluster.
- Use repository SVG icon assets for top-level actions where matching assets exist.
- Keep product diagnostics hidden in normal product mode.
- Do not change preview frame pump, native surface placement, draft semantics, or FFmpeg discovery.

## Verification

- Passed: `corepack pnpm --dir apps/desktop-electron run build:electron`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- Reviewed: `test-results/phase15-3/workspace-1280x800.png` and `test-results/phase15-3/workspace-1120x720.png` against `docs/ui-reference/jianying-pro/screenshots/02-workspace-media-window.png` and `03-top-feature-tabs.png`.
- Passed: `corepack pnpm --dir apps/desktop-electron run package:dir`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts -g "video external audio text and two-cue SRT" --workers=1`
- Passed: `corepack pnpm -w run test:phase3-source-guards`
- Passed: `corepack pnpm -w run test:phase10-1-source-guards`
- Passed: `git diff --check`
