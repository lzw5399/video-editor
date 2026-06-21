---
status: complete
completed_at: "2026-06-21T15:01:07Z"
---

# Bundled-Only FFmpeg Runtime Summary

Simplified desktop FFmpeg/ffprobe selection to app-local bundled runtime only. Electron now always sets `VE_BUNDLED_FFMPEG_DIR` to the runtime packaged with the app or the repo-local development runtime, and product startup no longer honors external runtime directory overrides.

## Changes

- Removed the Electron test switch that injected arbitrary `VE_BUNDLED_FFMPEG_DIR`.
- Removed the dev/runtime guard that preserved externally supplied `VE_BUNDLED_FFMPEG_DIR`.
- Tightened Phase 6 release guards to require app-local runtime ownership and reject override switches.
- Updated docs to describe `VE_BUNDLED_FFMPEG_DIR` as an internal Electron-to-Rust path, not a user override.
- Updated runtime discovery remediation so it points to the bundled runtime directory instead of telling users to set env vars.
- Removed a workspace test's misleading missing-runtime env override.

## Verification

- `corepack pnpm run test:phase6-release-gates`
- `cargo fmt --all --check`
- `cargo test -p media_runtime discovery -- --nocapture`
- `cargo test -p media_runtime runtime_capability -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/packaged-smoke.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --grep "预览失败显示中文分类错误且不改草稿" --reporter=line`

Source scan confirmed no product Electron/main/runtime code matches local FFmpeg lookup or override patterns for `PATH`, Homebrew, `/opt/homebrew`, `/usr/local/bin/ffmpeg`, `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, or `--video-editor-test-ve-bundled-ffmpeg-dir`.
