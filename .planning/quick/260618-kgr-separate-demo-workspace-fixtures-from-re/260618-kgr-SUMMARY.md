---
quick_id: 260618-kgr
status: complete
date: 2026-06-18
---

# Quick Task 260618-kgr Summary

Separated demo workspace fixtures from real desktop app startup.

## Changes

- Added a blank production startup draft with empty materials and empty base tracks.
- Renamed the previous seeded workspace to a demo fixture and made tests opt into it through `VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE=demo`.
- Passed the fixture mode from Electron main to preload through a controlled renderer argument instead of exposing general environment variables.
- Updated the real no-mock workflow so it starts from the blank editor and no longer deletes fake initial segments.
- Added smoke coverage that default startup has no demo materials or demo paths, while the demo fixture remains available for tests.

## Verification

- `pnpm --filter @video-editor/desktop build`
- `pnpm --filter @video-editor/desktop exec playwright test tests/electron-smoke.spec.ts`
- `pnpm --filter @video-editor/desktop test:real-workflow`
- `pnpm --filter @video-editor/desktop exec playwright test tests/workspace.spec.ts tests/runtime-diagnostics.spec.ts`
- `pnpm --filter @video-editor/desktop test:packaged-real-workflow`
