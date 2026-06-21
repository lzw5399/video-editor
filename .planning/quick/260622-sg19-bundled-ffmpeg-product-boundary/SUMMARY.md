# 260622-sg19 Bundled FFmpeg Product Boundary Summary

## Completed

- Replaced product startup FFmpeg runtime env injection with an explicit native binding call: Electron computes the app-local bundled `resources/ffmpeg/<platform>-<arch>` path and passes it to Rust through `configureBundledRuntimeDirectory`.
- Added `media_runtime::configure_bundled_runtime_directory` as the app-shell boundary for release/product runtime discovery.
- Kept `VE_BUNDLED_FFMPEG_DIR` only as a debug/test helper branch so Rust release discovery no longer treats process env as the product resolver.
- Removed user-facing runtime capability copy that told users to set `VE_BUNDLED_FFMPEG_DIR`.
- Updated release docs and release guards to describe native binding configuration, forbid product docs from referencing the env resolver, and ensure any Rust env read stays behind `#[cfg(debug_assertions)]`.
- Corrected stale capability test posture so bundled runtime is not treated as `externalRuntime`.
- Updated packaged smoke bridge expectations for the current session preview API surface.

## Architecture Review

- Subagent review returned `partially correct`: the existing chain already rejected PATH/Homebrew/per-binary lookup, but using process-level `VE_BUNDLED_FFMPEG_DIR` as the resolver input was still a weak production boundary.
- Implemented the recommended boundary simplification without adding a complex runtime provider layer: product uses one explicit app-local bundled directory; debug/test can still inject sandbox runtime directories.

## Verification

- `cargo fmt --all --check` passed.
- `git diff --check` passed.
- `cargo test -p media_runtime discovery -- --nocapture` passed: 5/5.
- `cargo test -p media_runtime runtime_capability -- --nocapture` passed: 4/4.
- `cargo test -p media_runtime_desktop capabilities -- --nocapture` passed: 4/4.
- `cargo test -p bindings_node runtime_capabilities -- --nocapture` passed: 3/3.
- `corepack pnpm run test:phase6-release-gates` passed.
- `corepack pnpm run test:phase3-source-guards` passed.
- `corepack pnpm --dir apps/desktop-electron run build:native` passed.
- `corepack pnpm --dir apps/desktop-electron run build:electron` passed.
- `corepack pnpm --dir apps/desktop-electron run test:packaged-smoke` passed: 2/2, including the external bundled-runtime env override poison case.
