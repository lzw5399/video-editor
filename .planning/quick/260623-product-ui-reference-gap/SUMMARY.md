# Product UI Reference Gap Summary

## Result

Moved the product top feature toolbar closer to the Jianying Pro reference by promoting transition and caption tools into the always-visible first-level category row.

## Changes

- Increased always-visible product categories from five to seven: `素材`, `音频`, `文本`, `贴纸`, `特效`, `转场`, `字幕`.
- Kept `智能包装`, `滤镜`, `调节`, and `数字人` in the overflow menu.
- Tightened category button sizing so the seven visible categories fit at both 1280x800 and 1120x720.
- Added UI regression checks that every always-visible category button is actually inside the navigation visible bounds, preventing clipped-but-accessible false positives.
- Preserved existing Rust-owned edit semantics and product preview/export boundaries.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "native surface aligned" --workers=1 --reporter=line`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
