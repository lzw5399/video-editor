# Summary: 260622-sg20 Product UI Diagnostic Copy Boundary

## Status

Completed.

## Changes

- Product export modal now shows sanitized export status by default and only exposes raw export errors/diagnostics when developer diagnostics are enabled.
- Export status aria copy changed from "导出日志" to "导出状态".
- Realtime preview native host aria copy changed from "实时预览宿主" to "实时预览画面".
- Product-mode realtime preview host failures now surface as "预览画面暂不可用" instead of internal bridge/host messages.
- Export diagnostic labels avoid render-graph/runtime wording in user-facing formatters.
- UI reference regression now checks aria-label/title attributes in addition to visible text.
- Phase 15.3 source guard now blocks old host/log labels and direct product export diagnostic surfacing.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm run test:no-product-fallback`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/runtime-diagnostics.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/export-modal.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
