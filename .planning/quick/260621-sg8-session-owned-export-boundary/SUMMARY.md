# Session-Owned Export Boundary Summary

## Completed

- Added `startProjectSessionExport` to the Rust N-API boundary.
- Desktop product export now sends `sessionId`, `expectedRevision`, `outputPath`, and `preset`; Rust reads the canonical project-session draft before starting the existing export registry.
- Removed the renderer helper that built full-draft `startExport` payloads for the product start-export path.
- Updated Electron main/preload/native binding bridge and test observation mapping for session-owned export.
- Added project-session Rust tests for successful export start, stale/unknown session rejection, and renderer draft payload rejection.
- Strengthened renderer/source guards against reintroducing legacy full-draft start export payloads.
- Strengthened Phase 6 release guards so local/Homebrew/PATH FFmpeg lookup is forbidden in `bindings_node`, `media_runtime`, `media_runtime_desktop`, `preview_service`, Electron main, scripts, and packaging config.
- Rebuilt the directory package so packaged smoke validates the current preload bridge and bundled FFmpeg runtime.

## Verification

- `cargo fmt --all --check`
- `cargo test -p bindings_node --test project_session project_session_export -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase11-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm run test:phase6-release-gates`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/export-modal.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --grep "导出" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/electron-smoke.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/packaged-smoke.spec.ts --reporter=line`

## Notes

- Product runtime policy remains bundle-only: product startup sets the app-local `VE_BUNDLED_FFMPEG_DIR`; release guards reject PATH/Homebrew/local discovery patterns.
- Existing cancel/status export commands still use the job-id command path for this slice.
