# Summary: 260622-sg30 Explicit Artifact Control API

## Status

Completed.

## Changes

- Added explicit Rust Node-API entry points for artifact status, refresh, retry/resume/cancel, quota, and garbage collection.
- Removed artifact command names from the public `executeCommand` allowlist; legacy artifact envelopes now return `unsupportedCommand`.
- Added typed Electron main/preload/nativeBinding wrappers for explicit artifact APIs.
- Updated renderer resource status/task/quota/cleanup handlers to call explicit APIs instead of renderer-built command envelopes.
- Removed unused renderer artifact and preview-cache command builders.
- Strengthened phase3 and phase14 source guards so artifact command-envelope builders and executeCommand allowlist entries cannot return.
- Updated workspace artifact tests to describe explicit native artifact APIs.

## Verification

- `cargo test -p bindings_node artifact_store_commands`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase14-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --grep "素材资源状态 uses explicit native artifact APIs|资源任务 and 资源维护 update"`
- `cargo fmt --all --check && git diff --check`
