---
status: complete
completed: 2026-06-22
---

# Summary: 260622-sg42 Remove Project-Session Preview Artifacts

## Status

Completed.

## Changes

- Removed Electron-visible `requestProjectSessionPreviewFrame` and `requestProjectSessionPreviewSegment` APIs from renderer, preload, main IPC, native binding wrapper, and Rust N-API exports.
- Removed developer-diagnostics PNG/MP4 preview artifact controls, artifact image display state, and the obsolete `platform:pathToFileUrl` bridge.
- Kept product preview on the Rust-owned realtime preview host: project-session snapshot sync, seek/play/pause/stop controls, telemetry subscription, and GPU frame evidence.
- Updated source guards so Electron-facing code and `bindings_node` cannot reintroduce project-session preview artifact APIs.
- Updated workspace/electron smoke tests away from artifact preview generation and kept negative gates that prove playback does not use artifact fallback.

## Verification

- `cargo fmt --all --check`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `cargo test -p bindings_node --test preview_commands -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase5-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/electron-smoke.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1`
- `git diff --check`

## Evidence

- Product cadence stayed at 90 accounted frames / 3 seconds for both single-video and video + external audio + text + two-cue SRT playback.
- `requestProjectSessionPreviewFrame` fallback count remained 0 in cadence gates after the API removal.
