# Bundled FFmpeg Only Summary

## Result

Confirmed the production runtime boundary: FFmpeg and ffprobe come from bundled application resources only. Product code must not discover Homebrew, PATH, `which`, or per-binary local-machine FFmpeg paths.

## Changes

- Added a Phase 6 release guard blocking local/Homebrew/PATH FFmpeg lookup patterns in runtime discovery, Electron main code, desktop scripts, and packaging config.
- Added macOS `otool -L` dependency auditing to `provision:ffmpeg-runtime`, including bundled `lib/*.dylib` recursion.
- Added macOS ad-hoc signing for bundled runtime files so rewritten install names can execute in packaged builds.
- Updated runtime capability mock/product labels so diagnostics describe the bundled runtime instead of a local external FFmpeg.
- Updated project state to supersede the earlier Homebrew test-environment note.
- Rebuilt the local ignored `apps/desktop-electron/runtime/ffmpeg/darwin-arm64` runtime so the package contains app-relative dylib references instead of `/opt/homebrew` references.

## Verification

- `bash scripts/phase6-release-guards.sh`
- `cargo test -p media_runtime discovery -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/packaged-smoke.spec.ts --reporter=line`
- `otool -L` over packaged `Resources/ffmpeg/darwin-arm64` reports no `/opt/homebrew` or `/usr/local` dependencies.
