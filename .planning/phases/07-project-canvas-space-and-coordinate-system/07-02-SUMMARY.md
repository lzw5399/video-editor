---
phase: 07-project-canvas-space-and-coordinate-system
plan: 02
subsystem: contracts
tags: [rust, draft-model, canvas, schema, ts-rs]
requires:
  - phase: 07-project-canvas-space-and-coordinate-system
    provides: Rust-owned draft canvas model and validation from Plan 07-01
provides:
  - Canvas-aware positive and negative draft fixtures
  - Generated draft schema with required `canvasConfig`
  - Generated command schema and TypeScript contracts for `updateDraftCanvasConfig`
  - Schema export tests for canvas config dimensions, aspect ratio, and generated command drift
affects: [phase-07, phase-08, draft-commands, desktop-ui, preview, export]
tech-stack:
  added: []
  patterns: [Rust-owned generated contracts, schema post-processing for semantic numeric bounds, classified positive and negative fixtures]
key-files:
  created:
    - fixtures/draft/negative/missing-canvas-config/project.json
    - fixtures/draft/negative/invalid-canvas-background-reference/project.json
  modified:
    - fixtures/draft/positive/minimal-draft/project.json
    - fixtures/draft/positive/materials-round-trip/project.json
    - fixtures/draft/positive/missing-material/project.json
    - fixtures/draft/negative/invalid-unknown-field/project.json
    - crates/draft_model/src/canvas.rs
    - crates/draft_model/tests/draft_fixtures.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/draft.schema.json
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/Draft.ts
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
key-decisions:
  - "Generated schemas and TypeScript remain derived from Rust `draft_model` types, not hand-written renderer contracts."
  - "Canvas dimensions and custom aspect-ratio numerator/denominator are tightened to schema minimum 1 to match Rust validation semantics."
  - "`updateDraftCanvasConfig` carries a full generated `DraftCanvasConfig` payload for later Rust-owned command handling."
patterns-established:
  - "Canvas contract drift is checked through `schema_exports.rs`, generated JSON schemas, and generated Electron TypeScript files together."
  - "Cross-field canvas background material validation stays in Rust validation; JSON Schema exposes the typed background shape."
requirements-completed: [CANVAS-01, CANVAS-02, CANVAS-04]
duration: 7 min
completed: 2026-06-17
---

# Phase 07 Plan 02: Canvas Fixtures And Generated Contracts Summary

**Canvas fixtures and Rust-generated schema/TypeScript contracts now expose required `canvasConfig` and `updateDraftCanvasConfig` semantics**

## Performance

- **Duration:** 7 min
- **Started:** 2026-06-17T23:41:09Z
- **Completed:** 2026-06-17T23:47:54Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments

- Updated positive draft fixtures to include the MVP 1920 x 1080, 30/1, 16:9, black `canvasConfig`.
- Added negative coverage for missing `canvasConfig` and image background references that do not resolve to image materials.
- Exported canvas config, aspect-ratio, background, and `updateDraftCanvasConfig` command contracts through Rust-owned JSON Schema and TypeScript generation.
- Tightened generated schemas so canvas width, height, and custom aspect-ratio numerator/denominator reject zero values.

## Task Commits

1. **Task 07-02-01: Update draft fixtures for canvasConfig** - `6444bc8` (test)
2. **Task 07-02-02: Regenerate JSON schema and TypeScript canvas contracts** - `44e6daa` (test)

## Files Created/Modified

- `fixtures/draft/positive/minimal-draft/project.json` - Adds required default `canvasConfig`.
- `fixtures/draft/positive/materials-round-trip/project.json` - Adds required default `canvasConfig`.
- `fixtures/draft/positive/missing-material/project.json` - Adds required default `canvasConfig`.
- `fixtures/draft/negative/missing-canvas-config/project.json` - Proves required canvas config validation.
- `fixtures/draft/negative/invalid-canvas-background-reference/project.json` - Proves image background material validation stays Rust-owned.
- `crates/draft_model/tests/draft_fixtures.rs` - Classifies and verifies the new canvas fixtures.
- `crates/draft_model/tests/schema_exports.rs` - Exports canvas contracts and tests schema bounds.
- `schemas/draft.schema.json` - Generated schema now contains required `canvasConfig`.
- `schemas/command.schema.json` - Generated command schema now contains `updateDraftCanvasConfig`.
- `apps/desktop-electron/src/generated/Draft.ts` - Generated TypeScript now exports canvas config/background/aspect-ratio contracts.
- `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Generated TypeScript now exports canvas update command payload.

## Decisions Made

- Kept `canvasConfig` as the canonical JSON/TS field name while Rust source uses `canvas_config`.
- Reused the existing schema post-processing pattern to align generated numeric bounds with Rust validation.
- Left actual command execution, undo/redo, and binding routing to Plan 07-03.

## Deviations from Plan

None - plan executed exactly as written.

---

**Total deviations:** 0 auto-fixed.  
**Impact on plan:** No scope creep. All changes stay within fixture and generated contract surfaces.

## Issues Encountered

`schemars` emits `minimum: 0` for `u32` by default. The schema export test now patches canvas width, height, and custom aspect-ratio fields to `minimum: 1` and verifies draft/command schemas reject zero values.

## User Setup Required

None - no external service configuration required.

## Verification

- `cargo test -p draft_model draft_fixtures -- --nocapture` - passed.
- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - passed.
- `cargo test -p draft_model schema_exports -- --nocapture` - passed.
- `cargo test -p draft_model draft_schema -- --nocapture` - passed.
- `rg -n "canvasConfig|DraftCanvasConfig|CanvasAspectRatio|CanvasBackground|UpdateDraftCanvasConfig|updateDraftCanvasConfig" schemas apps/desktop-electron/src/generated` - passed.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed after task commit and regeneration.

## Self-Check: PASSED

- Required fixture and generated contract artifacts exist.
- Both task commits are present.
- Plan-level verification commands passed.

## Next Phase Readiness

Ready for Plan 07-03. The Rust command implementation and Electron binding route can now consume generated `UpdateDraftCanvasConfigCommandPayload` without renderer-owned canvas shapes.

---
*Phase: 07-project-canvas-space-and-coordinate-system*
*Completed: 2026-06-17*
