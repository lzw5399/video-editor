---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# P0 User Material Preview Regression

## Goal

Add a regression gate for the user-provided portrait MP4 that previously exposed preview sizing, washed-out video, drag-to-timeline first-frame, and native playback issues.

## Scope

- Add a Playwright product-flow regression that uses `VIDEO_EDITOR_P0_USER_MATERIAL` or the known local download path when present.
- Verify import, drag-to-timeline, first composited frame before playback, renderGraphGpu playback, no fallback, and native surface alignment.
- Do not commit the 93MB local media file into the repo.
- Do not change preview scheduler, media runtime, export, or FFmpeg distribution.

## Verification

- Passed: `corepack pnpm --dir apps/desktop-electron run build:electron`
- Passed: `VIDEO_EDITOR_P0_USER_MATERIAL="$HOME/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4" corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "P0 user portrait material" --workers=1`
- Passed: `git diff --check`
