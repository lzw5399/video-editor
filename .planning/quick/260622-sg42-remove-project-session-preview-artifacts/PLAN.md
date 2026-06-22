---
status: in_progress
created: 2026-06-22
skill: gsd-quick
---

# Remove Project-Session Preview Artifact APIs

## Goal

Delete Electron-visible project-session preview frame/segment artifact commands so product preview can only flow through the Rust-owned realtime preview scheduler.

## Scope

- Remove renderer/preload/main/native-binding exposure for `requestProjectSessionPreviewFrame` and `requestProjectSessionPreviewSegment`.
- Remove developer-diagnostics UI controls and state that present PNG/MP4 preview artifacts as a preview path.
- Invert source guards so renderer/preload/main cannot reintroduce those APIs.
- Update E2E tests away from artifact preview assertions and keep realtime playback gates as the product proof.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`
- `git diff --check`
