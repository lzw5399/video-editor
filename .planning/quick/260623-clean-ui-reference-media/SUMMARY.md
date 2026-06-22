# Clean UI Reference Media Summary

## Result

Product UI reference screenshots now use a healthy project with real local fixture media instead of the demo fixture that intentionally contains missing and probe-failed materials.

## Changes

- Changed `ui-reference-regression.spec.ts` workspace launch to create a fresh project named `未命名草稿`.
- Imported real local video, audio, and image fixtures through the product import flow.
- Added video and audio to the timeline, then selected the video track so screenshots retain filmstrip/waveform timeline evidence while the inspector returns to draft parameters.
- Added screenshot gate coverage that the default product material panel does not contain missing/probe-failed copy.
- Left the Rust demo fixture and tests that intentionally cover missing/failed material states unchanged.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "native surface aligned" --workers=1 --reporter=line`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
