# Quick Task: 260622-sg36 Align Workspace Product Contract

## Objective

Align `workspace.spec.ts` with the production editor contract after the generic command bridge removal and session-owned UI changes.

## Production Boundary

- Tests must assert Rust project-session intents and current product UI, not low-level `addSegment`/`addTextSegment` aliases or removed duration/offset controls.
- Product mode should not require developer realtime diagnostic labels as first-class UI.
- Locator expectations must be strict and unambiguous where the UI legitimately contains buttons and status regions with related labels.

## Work Items

1. Reclassify the failing workspace tests against current product behavior and Rust-owned session semantics.
2. Update tests to assert intent names, selected segment labels, current preview/export UI, and strict locators.
3. Preserve source guards and no-fallback product preview gates.

## Verification

- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --workers=1`
