---
status: complete
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Preview Canvas Fill Summary

## Result

The product preview monitor now uses the available preview area instead of being compressed by unused diagnostic grid rows. The canvas is width-driven, keeps 16:9, and remains the same realtime host/native surface boundary.

## Changes

- Changed product `.preview-shell` to the actual product row structure: title, canvas, transport.
- Kept the developer diagnostics grid separate.
- Changed `.preview-canvas` from height-driven auto width to width-driven sizing.
- Added a UI reference regression assertion requiring the preview canvas to fill at least 70% of the preview panel width.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "Chinese editor workspace opens with required regions and material states" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts -g "production workspace captures five-zone hierarchy" --workers=1`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT keeps the native surface aligned with the preview monitor|product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
