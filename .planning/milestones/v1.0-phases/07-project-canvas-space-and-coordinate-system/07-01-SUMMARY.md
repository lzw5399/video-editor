---
phase: 07-project-canvas-space-and-coordinate-system
plan: 01
subsystem: model
tags: [rust, draft-model, canvas, validation, coordinates]
requires:
  - phase: 06-mvp-hardening-and-packaging
    provides: MVP baseline and release gates before canvas expansion
provides:
  - Rust-owned draft canvas config model
  - Canvas validation for dimensions, fps, aspect ratio, color, and image background references
  - Normalized canvas coordinate conversion helpers and documentation
affects: [phase-07, phase-08, transform, text, sticker, keyframe, preview, export]
tech-stack:
  added: []
  patterns: [serde/schemars/ts-rs draft contracts, Rust-owned canvas validation, center-origin normalized coordinates]
key-files:
  created:
    - crates/draft_model/src/canvas.rs
    - crates/draft_model/tests/canvas_config.rs
    - docs/canvas-coordinate-system.md
  modified:
    - crates/draft_model/src/draft.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/src/validation.rs
key-decisions:
  - "Persist draft canvas semantics as Rust `canvas_config` serialized to JSON `canvasConfig`."
  - "Use explicit preset/custom aspect-ratio state and validate it against reduced canvas dimensions."
  - "Use normalized canvas coordinates with origin at canvas center, +X right, +Y up, and 1.0 as half canvas width/height."
patterns-established:
  - "Draft canvas validation lives in `draft_model::validation` and rejects malformed semantic state before commands/rendering consume it."
  - "UI pixel coordinates are derived display state; normalized canvas coordinates are the shared semantic contract."
requirements-completed: [CANVAS-01, CANVAS-02, CANVAS-03, CANVAS-04]
duration: 18 min
completed: 2026-06-17
---

# Phase 07 Plan 01: Draft Canvas Model Summary

**Rust-owned `canvasConfig` model with validated canvas dimensions, background semantics, and center-origin normalized coordinate documentation**

## Performance

- **Duration:** 18 min
- **Started:** 2026-06-17T23:18:00Z
- **Completed:** 2026-06-17T23:35:59Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `DraftCanvasConfig` as the canonical draft-level canvas/profile source with width, height, rational frame rate, explicit aspect ratio, and semantic background modes.
- Extended draft validation to reject invalid canvas dimensions, frame rates, aspect ratio mismatches, invalid colors, and invalid image background material references.
- Added normalized canvas coordinate helpers and documentation locking center-origin, `+X` right, `+Y` up semantics for later transform/text/sticker/keyframe phases.

## Task Commits

1. **Task 07-01-01: Add draft canvas config model and validation** - `ff6deb1` (feat)
2. **Task 07-01-02: Document normalized canvas coordinate system** - `03d02c4` (docs)

## Files Created/Modified

- `crates/draft_model/src/canvas.rs` - Canvas config, aspect ratio, background capability, and coordinate helper types/functions.
- `crates/draft_model/tests/canvas_config.rs` - Focused tests for defaults, validation, background capabilities, and coordinate conversions.
- `docs/canvas-coordinate-system.md` - Shared coordinate contract for future visual editing phases.
- `crates/draft_model/src/draft.rs` - Adds required `Draft.canvas_config` with MVP defaults.
- `crates/draft_model/src/lib.rs` - Exports canvas types and adds source command payload names for later contract generation.
- `crates/draft_model/src/validation.rs` - Validates `canvasConfig` during migration and draft validation.

## Decisions Made

- Used explicit `CanvasAspectRatio::Preset` / `CanvasAspectRatio::Custom` state instead of deriving a display string from dimensions.
- Allowed `CanvasBackground::Image` as semantic state while validating any provided material reference as an image material.
- Classified black and solid color backgrounds as supported, blur fill as degraded, and image background as unsupported until later render/material-picker work connects it.

## Deviations from Plan

None - plan executed exactly as written.

---

**Total deviations:** 0 auto-fixed.  
**Impact on plan:** No scope creep. Generated schema/TypeScript artifacts were intentionally left untouched for Plan 07-02.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Verification

- `cargo test -p draft_model canvas -- --nocapture` - passed.
- `cargo test -p draft_model draft_schema -- --nocapture` - passed.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed.
- `rg -n "origin at canvas center|\\+X right|\\+Y up|half canvas width|half canvas height|top-left" docs/canvas-coordinate-system.md` - passed.

## Next Phase Readiness

Ready for Plan 07-02. Fixtures, schema export wiring, and generated JSON/TypeScript contracts can now consume the Rust-owned canvas model.

---
*Phase: 07-project-canvas-space-and-coordinate-system*
*Completed: 2026-06-17*
