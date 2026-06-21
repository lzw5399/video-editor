# Summary: 260622-sg39 Remove FFmpeg Env Runtime Lookup

## Status

Completed.

## Changes

- Removed the debug/test `VE_BUNDLED_FFMPEG_DIR` resolver from `media_runtime` discovery.
- Kept runtime resolution limited to Electron/native explicit bundled-directory configuration, with the app-local development runtime as the only default.
- Reworked Rust tests to use a scoped bundled-runtime directory guard instead of process environment variables.
- Removed the packaged smoke test for the obsolete external runtime override and strengthened PATH poisoning to include Windows `.exe` names.
- Strengthened Phase 6 release guards so FFmpeg env resolver reintroduction fails, and added `test:phase6-release` to run packaging plus release guards together.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p media_runtime discovery -- --nocapture`
- `cargo test -p media_runtime runtime_capability -- --nocapture`
- `cargo test -p bindings_node runtime_capabilities -- --nocapture`
- `cargo test -p bindings_node --test binding_smoke -- --nocapture`
- `cargo test -p bindings_node --test export_commands -- --nocapture --test-threads=1`
- `cargo test -p bindings_node --test project_session project_session_export_starts_from_session_snapshot_without_renderer_draft -- --nocapture --test-threads=1`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase6-release-gates`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm run test:phase6-release`
