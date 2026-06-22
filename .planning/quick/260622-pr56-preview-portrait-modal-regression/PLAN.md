---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Preview Portrait Modal Regression

## Goal

Fix user-reported real-material preview regressions: portrait preview escaping the monitor, export modal being covered by the native surface, and drag-to-timeline first-frame preview coverage for `~/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4`.

## Scope

- Constrain preview canvas sizing by both available width and height so portrait material fits inside the monitor.
- Add an explicit realtime preview host detach/suspend path for modal overlays; do not rely on DOM z-index over native surfaces.
- Keep existing hidden native titlebar worktree changes intact.
- Add or update focused tests for portrait canvas bounds and export modal/native surface interaction where feasible.
- Do not change FFmpeg runtime lookup, draft semantics, or renderer-owned timeline construction.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states|auto canvas adopts the first imported portrait material without renderer-owned canvas math|预览播放按钮使用实时预览画面而不是连续请求预览帧" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product user can import a repo video, add it to the timeline, and see render-graph GPU playback frames advance|product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT uses native audio output instead of status-only or mock audio|product playback UAT plays embedded video audio through native output|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "字幕 SRT import intent path sends raw SRT once without renderer-created cue segments" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `corepack pnpm -w run test:phase10-1-source-guards`
- `cargo test -p realtime_preview_runtime playback_timeline -- --nocapture`
- `cargo test -p bindings_node scheduler_seek_presents_still_frame_without_electron_frame_pump -- --nocapture`
- `cargo fmt --all --check`
- `git diff --check`
