---
status: complete
created: 2026-06-23
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Drag Primary Material Cards

## Goal

Make product material cards communicate drag-to-timeline as the primary workflow instead of showing a persistent add button on every thumbnail.

## Production Decision

Confirmed narrow UI fix: material-to-timeline semantics are already Rust-owned through `addTimelineSegmentIntent` and drag payloads carry only material IDs. This task adjusts product affordance only; it must not create renderer-owned timeline structures or remove the existing add command path used for accessibility and tests.

## Scope

- Hide material card add buttons from the default visual state.
- Reveal add buttons on hover/focus for accessibility and fallback command entry.
- Keep cards draggable when material status is available.
- Add UI regression evidence that default product screenshots are drag-primary and the hover affordance still exists.
- Preserve missing/probe-failed material display in tests that intentionally use those states.

## Verification

- `build:electron`
- UI reference regression and refreshed 1280/1120 screenshots.
- Product drag-to-timeline journey/playback smoke.
- `test:phase3-source-guards`
- `git diff --check`
