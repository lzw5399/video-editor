# Bundled FFmpeg Only

## Goal

Keep the FFmpeg/ffprobe runtime boundary simple and product-safe: the Electron app uses the bundled runtime resources only, with no Homebrew, PATH, `which`, or user-machine binary discovery in product code.

## Scope

- Confirm the existing runtime discovery path is bundled-only.
- Add a release guard that blocks future local-machine FFmpeg lookup paths.
- Do not redesign media runtime discovery or add fallback complexity.

## Verification

- `bash scripts/phase6-release-guards.sh`
- `cargo test -p media_runtime discovery -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/packaged-smoke.spec.ts --reporter=line`
