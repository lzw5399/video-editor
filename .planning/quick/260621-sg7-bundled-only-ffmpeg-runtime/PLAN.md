# Bundled-Only FFmpeg Runtime

## Goal

Simplify desktop FFmpeg/ffprobe runtime selection so the Electron product app always uses the bundled app runtime directory. Product code must not honor local/Homebrew/PATH binary lookup or external runtime overrides.

## Scope

- Make Electron main unconditionally set `VE_BUNDLED_FFMPEG_DIR` to the app-local runtime path.
- Remove the test command-line hook that injects arbitrary `VE_BUNDLED_FFMPEG_DIR`.
- Tighten release guards so runtime overrides and local lookup branches cannot return.
- Keep Rust media runtime discovery bundled-only; do not add fallback search paths.

## Verification

- `corepack pnpm run test:phase6-release-gates`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/packaged-smoke.spec.ts --grep "bundled FFmpeg|external bundled runtime" --reporter=line`
