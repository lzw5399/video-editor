---
status: complete
created: 2026-06-23
completed: 2026-06-23
skill: gsd-debug
review_skill: production-architecture-review
---

# Text Editing Preview Drag Stability

## Goal

Make the expanded product text editing E2E gate stable and meaningful for preview-surface text/subtitle editing. The test must prove real native render-graph GPU text evidence, not DOM overlays or artifact preview frames.

## Current Failure

`product text editing UAT exercises preview drag, multi-font captions, and staggered subtitle tracks` fails while editing the first staggered SRT cue. The native command observation count advances, but the inspector still shows the old text value.

## Production Constraints

- Keep editing semantics session-owned through Rust intents.
- Do not satisfy the gate with DOM fallback overlays or preview-frame artifacts.
- Fix the synchronization/selection issue directly; do not loosen the evidence assertions.

## Verification

- Target failing Playwright test.
- Expanded repeated-font text editing Playwright test.
- Multi-font text/subtitle native preview Playwright test.
- `build:electron`.
- `git diff --check`.
