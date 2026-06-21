# Summary: 260622-sg32 Explicit Runtime Discovery API

## Status

Completed.

## Changes

- Added explicit Rust Node-API `probeMediaRuntime()` and wired it through nativeBinding, Electron main, and preload.
- Removed generic `executeCommand` from the renderer-facing `videoEditorCore` preload bridge while preserving main/native compatibility for legacy tests and contracts.
- Updated Electron smoke, packaged smoke, and real workflow helpers to use explicit runtime discovery APIs and test observation bridges.
- Added source guards blocking generic `executeCommand` exposure from preload and requiring explicit runtime discovery APIs.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node --test binding_smoke`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron test electron-smoke.spec.ts`
- `corepack pnpm --dir apps/desktop-electron run test:packaged-smoke`
