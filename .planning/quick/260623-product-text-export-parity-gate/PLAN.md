---
status: complete
created: 2026-06-23
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Product Text Export Parity Gate

## Goal

Add a product-level Electron E2E gate proving that edited text/subtitle overlays visible in the native render-graph GPU preview also burn into the exported video, using only the bundled FFmpeg runtime.

## Production Decision

Partially correct current state: Rust preview/export parity tests exist, and Electron product text preview gates are strong, but Electron export workflow currently proves only completion and ffprobe metadata. Product acceptance needs at least one end-to-end text/subtitle export pixel gate.

## Scope

- Reuse existing product journey fixtures and text/subtitle editing helpers.
- Start a real export from the product dialog.
- Read bundled FFmpeg/ffprobe paths from app runtime telemetry and reject Homebrew/PATH-sourced binaries.
- Extract an exported frame at the same timeline time as native preview evidence.
- Verify the exported frame contains the same edited text/subtitle color pixels in the expected transformed text boxes.
- Keep session-owned command and no-fallback assertions.

## Verification

- Target product text export parity Playwright test.
- `build:electron`.
- `git diff --check`.
