---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Material Drag To Timeline

## Goal

Support the production workflow where a user drags an imported material from the media panel into the timeline to add a segment, without the renderer constructing timeline segment structure.

## Scope

- Add drag source semantics to material items in the product media panel.
- Add a timeline drop target that accepts material drops and invokes the existing Rust-owned `addTimelineSegmentIntent` path.
- Keep the current click add button only as a secondary accessible command if already present; the main product gate must use drag and drop.
- Add/adjust Playwright coverage so the main user journey imports media, drags it to the timeline, and verifies the Rust project-session intent.

## Verification

- Passed: `corepack pnpm --dir apps/desktop-electron run build:electron`
- Passed: `corepack pnpm --dir apps/desktop-electron run package:dir`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback rejects missing render-graph GPU compositor evidence" --workers=1`
- Passed: `corepack pnpm -w run test:phase3-source-guards`
- Passed: `git diff --check`
