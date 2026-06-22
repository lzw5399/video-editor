---
status: complete
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Product Text Export Parity Gate Summary

## Result

Added a product-level Electron E2E gate proving native preview text/subtitle evidence and exported video pixels line up for edited text overlays. The test builds a real video + external audio + text + SRT subtitle project, captures native render-graph GPU preview evidence, exports through the product dialog, extracts the matching exported frame with bundled FFmpeg, and verifies the exported frame contains the expected burned-in text pixels.

## Changes

- Added `product text/subtitle export frame matches native preview text pixels`.
- Added product-test helpers for real export completion, bundled FFmpeg runtime validation, exported-frame extraction, and export-frame text pixel checks.
- Runtime helper asserts FFmpeg and ffprobe source is `bundled` and rejects Homebrew or `/usr/local` paths.
- Reused existing native preview text evidence and session-owned command assertions.

## Verification

- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "product text/subtitle export frame matches native preview text pixels" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `git diff --check`
