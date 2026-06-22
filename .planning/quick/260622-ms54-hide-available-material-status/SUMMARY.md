---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Hide Available Material Status Summary

## Completed

- Removed the default `可用` status chip from product material cards.
- Kept missing/probe-failed status chips and warnings visible for actionable problem states.
- Updated workspace, smoke, product journey, and real workflow helpers so they wait for material article visibility instead of the removed `可用` copy.
- Converted old workspace material-add coverage to drag a material into the timeline instead of relying on the hidden product quick-add path.

## Verification

- Passed: `corepack pnpm --dir apps/desktop-electron run build:electron`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states|auto canvas adopts the first imported portrait material without renderer-owned canvas math|预览播放按钮使用实时预览画面而不是连续请求预览帧" --workers=1`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- Passed: visual review of `test-results/phase15-3/workspace-1280x800.png`, `workspace-1120x720.png`, and Jianying reference `docs/ui-reference/jianying-pro/screenshots/04-left-material-library.png`
- Passed: `corepack pnpm --dir apps/desktop-electron run package:dir`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/electron-smoke.spec.ts -g "test fixture opt-in loads demo workspace materials" --workers=1`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/real-workflow.spec.ts -g "dev no-mock import-preview-export workflow" --workers=1`
- Passed: `corepack pnpm -w run test:phase3-source-guards`
- Passed: `git diff --check`
