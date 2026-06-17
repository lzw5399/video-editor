---
status: complete
task_id: 260618-2lz
slug: left-panel-menu-fix
completed: 2026-06-17
---

# Summary

Removed the standalone left-side secondary menu from the desktop workspace so the top feature bar is the only feature navigation.

## Changes

- Deleted `SecondaryCategoryRail` and the `媒体/文字/音频二级分类` left navigation surface.
- Adjusted workspace column proportions after removing the rail.
- Made global dark scrollbars slimmer and less visually prominent.
- Updated Playwright workspace tests to assert that left-side secondary menus do not reappear.

## Verification

- `pnpm run test:phase4-source-guards`
- `pnpm --filter @video-editor/desktop build`
- `pnpm --filter @video-editor/desktop exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace|workspace panels|layout stability"`
- Visual screenshots checked at `/tmp/video-editor-left-menu-fix-final-1120x720.png` and `/tmp/video-editor-left-menu-fix-final-1280x800.png`.
