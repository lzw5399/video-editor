---
status: complete
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Text Reopen Preview Parity Summary

## Result

Completed. Product E2E now proves edited multi-font text/subtitle state survives `.veproj` save, close, and reopen, then renders again through the native render-graph preview path.

## Changes

- Added `launchOpenedProductJourneyApp` and `openProjectFromProductEntry` test helpers for product journey tests that must click `打开项目` instead of always creating a new project.
- Added a real video + external audio + text + subtitle reopen parity E2E.
- The new gate edits title text and two subtitle cues, switches bundled Sans/Serif fonts, drags title/subtitle in the preview, applies rotation/scale/opacity, waits for final strings/font refs in `.veproj/project.json`, closes the app, reopens the same bundle, plays through realtime preview, and verifies restored native text overlay evidence.
- The reopened proof rejects DOM text overlays, artifact preview frame requests, and realtime fallback calls.

## Verification

- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product text/subtitle edits survive reopen" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
- Screenshot evidence:
  - `test-results/phase15-3/text-reopen-same-time-host.png`
  - `test-results/phase15-3/text-reopen-later-host.png`
