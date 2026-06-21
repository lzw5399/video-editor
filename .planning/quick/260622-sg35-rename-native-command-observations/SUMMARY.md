# Summary: 260622-sg35 Rename Native Command Observations

## Status

Completed.

## Changes

- Renamed the Electron test observation bridge from `executeCommand` call terminology to native command observation terminology.
- Updated Playwright helpers/specs to read `getNativeCommandObservations` and `__videoEditorTestNativeCommandObservations`.
- Added a phase3 source guard that rejects the old test observation names.
- Removed stale product helper usage of deleted text/audio/subtitle duration/offset UI controls; helper verification now checks Rust-owned default segment duration through the timeline view.

## Verification

- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/electron-smoke.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/inspector-modal.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/export-modal.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`

## Known Follow-Up

- `corepack pnpm --dir apps/desktop-electron exec tsc --noEmit` is still blocked by the existing missing `@types/node` dependency.
- Full `workspace.spec.ts` still has historical product expectation failures unrelated to the observation bridge rename, including stale segment labels, strict locator collisions, and removed realtime diagnostic labels.
