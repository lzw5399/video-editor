# 260622-sg39 Remove FFmpeg Env Runtime Lookup

## Decision

Confirmed under production-architecture-review: the app should ship and use bundled FFmpeg/ffprobe. Runtime discovery must not inspect PATH, Homebrew, per-binary overrides, or a process environment override as a product/runtime resolver.

## Scope

- Remove `VE_BUNDLED_FFMPEG_DIR` from Rust runtime discovery.
- Keep Electron's explicit `configureBundledRuntimeDirectory()` as the app-shell contract.
- Keep the development default directory `apps/desktop-electron/runtime/ffmpeg/<platform>-<arch>` for local builds.
- Update tests to configure sandbox runtimes explicitly instead of relying on an env resolver.
- Add/strengthen source guards so runtime discovery cannot reintroduce PATH/Homebrew/env binary lookup.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p media_runtime discovery runtime_capability -- --nocapture`
- `cargo test -p bindings_node runtime_capabilities -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
