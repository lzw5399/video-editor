---
status: complete
created: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Text Reopen Preview Parity

## Goal

Add a production product E2E gate proving multi-font text and subtitle edits survive save/close/reopen and still render through the native render-graph preview path without DOM/artifact fallback.

## Scope

- Add an opened-project product journey launcher for tests that need `打开项目` instead of always creating a new project.
- Build a real video + external audio + layered text/subtitle project with edited fonts, layout, preview-drag movement, rotation, scale, and opacity.
- Close and reopen the same `.veproj` bundle, then verify the restored timeline/inspector/native preview evidence for the edited text and subtitle state.
- Assert session-owned commands and no artifact preview frame fallback.

## Verification

- Focused product text reopen parity Playwright test.
- `build:electron`.
- `git diff --check`.
