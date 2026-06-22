---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Preview Monitor Jianying Chrome

## Goal

Move the center preview monitor chrome closer to the Jianying reference while preserving the existing native realtime preview surface and renderGraphGpu playback behavior.

## Scope

- Replace the draft-name/canvas-readout monitor title with a Jianying-style player title bar.
- Rework product-mode transport layout into left timecode, centered playback, and right monitor options.
- Keep developer diagnostics gated behind developer mode.
- Add screenshot/geometry assertions for the monitor chrome at 1280x800 and 1120x720.
- Do not change Rust preview scheduling, native surface placement, render graph, export, or draft semantics.

## Verification

- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor" --workers=1`
- Screenshot review of `test-results/phase15-3/workspace-1280x800.png`, `workspace-1120x720.png`, and new monitor crops against `docs/ui-reference/jianying-pro/screenshots/05-center-preview-monitor.png`.
- `git diff --check`

## Result

- Center monitor chrome now uses the Jianying-style `播放器-时间线01` title bar.
- Product transport is left timecode, centered playback, and right monitor controls.
- The shell spans the full preview panel while leaving the native surface path untouched.
- Packaged app playback screenshot was regenerated after `package:dir` to avoid stale `out/mac-arm64` evidence.
