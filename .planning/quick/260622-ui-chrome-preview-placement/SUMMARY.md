---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# UI Chrome And Preview Placement Pass Summary

## Completed

- Reworked the top feature category order and visible/overflow split to match the Jianying-style product surface without clipping at narrow desktop widths.
- Added an enabled "更多功能" overflow menu covering 转场、字幕、智能包装、滤镜、调节、数字人.
- Replaced placeholder unavailable panels with compact showcase resource panels for non-primary feature categories.
- Updated workspace and UI reference regression tests to assert category reachability, overflow content, and absence of debug/unavailable product copy.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace|workspace panels switch|文字 panel keeps" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1 --reporter=line`
- `git diff --check`

## Notes

- This is a UI navigation/product chrome slice only. Native preview surface placement and broader timeline chrome remain separate follow-up work.
