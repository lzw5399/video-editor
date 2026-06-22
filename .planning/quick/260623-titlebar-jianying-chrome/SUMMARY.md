# Titlebar Jianying Chrome Summary

## Result

Moved the product titlebar closer to the Jianying Pro reference without changing Rust-owned editing, preview, or export semantics.

## Changes

- Replaced the single cyan-dot `本地草稿` status with macOS-style red/yellow/green window dots and `HH:mm:ss 自动保存本地` product copy.
- Kept the real draft name centered and the top-right export action unchanged.
- Adjusted titlebar columns for both 1280x800 and 1120x720 so save status, title, and export do not clip or overlap.
- Added UI regression assertions for visible titlebar save status, three window dots, and unclipped status bounds.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "native surface aligned" --workers=1 --reporter=line`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
