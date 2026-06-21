# Quick Task: 260622-sg35 Rename Native Command Observations

## Objective

Rename the remaining test-observation bridge away from `executeCommand` terminology so tests no longer imply a generic product command path exists.

## Production Boundary

- Product renderer/preload/native surfaces must not expose generic `executeCommand`.
- Test observations may record explicit native API calls and project-session intents, but their naming must reflect observations rather than a command dispatcher.
- Source guards should block reintroducing the old test-facing `executeCommand` observation names.

## Work Items

1. Rename Electron main/preload test observation storage, IPC, and helper functions to native command observation terminology.
2. Update Playwright helpers/specs to consume the renamed observation bridge.
3. Add source guard coverage for legacy test observation names.

## Verification

- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
