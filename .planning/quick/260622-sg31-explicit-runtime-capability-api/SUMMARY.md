# Summary: 260622-sg31 Explicit Runtime Capability API

## Status

Completed.

## Changes

- Added explicit Rust Node-API `probeRuntimeCapabilities()` and routed Electron main/preload/nativeBinding through that API.
- Moved renderer runtime diagnostics off generic `executeCommand` and removed the runtime capability command builder from renderer helpers.
- Changed legacy `executeCommand(probeRuntimeCapabilities)` to return `unsupportedCommand`, with Rust tests covering the compatibility boundary.
- Added source guards blocking renderer runtime capability command envelopes and generic runtime capability `executeCommand` calls.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node runtime_capabilities`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
