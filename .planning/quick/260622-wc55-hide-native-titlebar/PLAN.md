---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Hide Native Titlebar

## Goal

Fix the double-titlebar product window shown in the user screenshot by moving the macOS window chrome contract to Electron: hide the native titlebar, use real macOS traffic lights, and keep the app product titlebar as the only visible title row.

## Scope

- Configure the main `BrowserWindow` with macOS hidden titlebar behavior and traffic light placement inside the product title row.
- Remove fake decorative traffic-light dots from the renderer titlebar.
- Preserve project title, export action, workspace layout, native preview placement, and bundled runtime behavior.
- Add or update tests so window metrics prove the native titlebar is not consuming visible top space.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- Visual review of `test-results/phase15-3/workspace-1280x800.png`, `workspace-1120x720.png`, and user-provided screenshot issue.
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `corepack pnpm -w run test:phase10-1-source-guards`
- `cargo fmt --all --check`
- `git diff --check`
