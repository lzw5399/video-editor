# Summary: 260622-sg29 Explicit Audio Preview API

## Status

Completed.

## Changes

- Added explicit Rust Node-API entry points for audio preview create/play/pause/stop/seek/cancel/status/device/waveform actions.
- Removed audio preview command names from the public `executeCommand` allowlist; legacy audio envelopes now return `unsupportedCommand`.
- Added typed Electron main/preload/nativeBinding wrappers for explicit audio preview APIs.
- Updated renderer audio preview/device/waveform controls to call explicit APIs instead of renderer-built command envelopes.
- Removed renderer audio preview command builders.
- Strengthened phase3/phase15 source guards so audio preview command-envelope builders and executeCommand allowlist entries cannot return.
- Fixed a renderer `useMemo` import that caused the workspace to render blank during focused Electron verification.
- Reworked the product status-copy forbidden-word regex literal to avoid source-guard self-matches while preserving runtime filtering.

## Verification

- `cargo test -p bindings_node audio_preview_commands_use_project_session_snapshot_without_renderer_draft`
- `cargo test -p bindings_node audio_service`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase15-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --grep "音频预览 controls call explicit native APIs"`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "product playback UAT uses native audio output"`
- `cargo fmt --all --check && git diff --check`
