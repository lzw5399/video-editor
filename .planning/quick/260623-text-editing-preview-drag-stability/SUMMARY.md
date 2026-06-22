---
status: complete
completed: 2026-06-23
skill: gsd-debug
review_skill: production-architecture-review
---

# Text Editing Preview Drag Stability Summary

## Result

The preview-drag text/subtitle product gate is stable again. The test helper now waits for the inspector to show the selected text or subtitle segment's current content before applying edits, so selection-state races fail at the correct boundary instead of filling a stale inspector form.

## Changes

- Added `expectedCurrentContent` support to `editSelectedTextThroughInspector`.
- Added current-content assertions to subtitle cue edit steps across the native text/subtitle E2E matrix.
- Kept native GPU evidence assertions intact: no DOM text overlays, no artifact preview frame requests, real native host pixel checks, session-owned intents, and no realtime fallback calls.

## Verification

- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "product text editing UAT exercises preview drag" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "product text editing UAT covers repeated font switching" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "product text and subtitle editing UAT covers multi-font" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
