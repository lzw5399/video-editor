# Quick Task: 260622-sg20 Product UI Diagnostic Copy Boundary

## Objective

Remove product-visible engineering diagnostics from the default desktop editor UI while preserving developer diagnostics behind the existing diagnostics switch.

## Production Boundary

- Product UI may show user-level preview/export status, unavailable states, retry guidance, and progress.
- Product UI must not show runtime, fallback, telemetry, cache, artifact, diagnostic, debug, FFmpeg/ffprobe, host/log wording, render graph wording, raw paths, or raw Rust diagnostic messages.
- Developer diagnostics may still display internal telemetry and raw runtime details.

## Work Items

1. Gate export modal diagnostic/error display behind `showDeveloperDiagnostics`.
2. Replace product export "log" copy with sanitized status copy.
3. Replace preview native host user-facing aria/copy with preview screen terminology.
4. Sanitize realtime preview host command errors in product mode.
5. Strengthen source and screenshot gates to inspect visible text plus aria/title attributes.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm run test:no-product-fallback`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/runtime-diagnostics.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --reporter=line`
