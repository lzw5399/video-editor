# 260622-sg19 Bundled FFmpeg Product Boundary

## Decision

Confirmed under `production-architecture-review`: product runtime discovery must stay bundled-only. Electron owns the app-local runtime directory configuration; Rust media runtime resolves only that bundled directory and must never fall back to PATH, Homebrew, or per-binary override variables.

## Scope

- Keep FFmpeg/ffprobe packaged as Electron `extraResources` and resolved through an explicit native binding runtime-directory configuration.
- Remove user-facing guidance that tells product users to set `VE_BUNDLED_FFMPEG_DIR`.
- Correct stale capability tests so bundled runtime posture is not treated as external runtime.
- Verify release/source guards still reject Homebrew/PATH/local-machine lookup.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p media_runtime discovery runtime_capability -- --nocapture`
- `cargo test -p media_runtime_desktop capabilities -- --nocapture`
- `cargo test -p bindings_node runtime_capabilities -- --nocapture`
- `corepack pnpm run test:phase6-release-gates`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run test:packaged-smoke`
