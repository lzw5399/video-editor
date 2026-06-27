---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
plan: "03"
subsystem: template-import
tags: [draft-import, import-plan, validation, schema, canonical-draft]

requires:
  - phase: 17-01
    provides: [provider-neutral AdaptationReport contract, Phase 17 source guards]
  - phase: 17-02
    provides: [bundle-local template resource refs, localizer diagnostics]
provides:
  - Provider-neutral DraftImportPlan contract using canonical draft_model canvas/material/track/segment semantics
  - Validation layer rejecting remote runtime refs, provider formula/provenance leakage, ambiguous z-order, and invalid draft shape
  - Pure application helper returning canonical Draft plus AdaptationReport without writing project bundles
  - Rust-generated draft import plan JSON Schema
affects: [adapter-kaipai, project-session-import, template-import-validation, schema-contracts, phase17-validation]

tech-stack:
  added: []
  patterns:
    - Strict serde/schemars/ts-rs import-plan contracts with deny_unknown_fields
    - Pure import-plan application into Draft before project-session mutation
    - Validation-first adapter/session boundary for canonical draft semantics

key-files:
  created:
    - crates/draft_import/src/import_plan.rs
    - crates/draft_import/src/validation.rs
    - crates/draft_import/tests/import_plan.rs
    - schemas/draft-import-plan.schema.json
  modified:
    - Cargo.lock
    - crates/draft_import/Cargo.toml
    - crates/draft_import/src/lib.rs
    - crates/draft_import/tests/schema_exports.rs
    - package.json

key-decisions:
  - "DraftImportPlan carries canonical draft semantics only; report/provenance evidence is supplied through DraftImportApplicationInput and returned as AdaptationReport."
  - "Import track z-order is provider-neutral ordering metadata validated before application, then represented by canonical Draft track order."
  - "Import application is pure and returns Draft plus AdaptationReport; project-session persistence remains a later owner."

patterns-established:
  - "Focused TDD gate: import-plan behavior tests fail before the contract exists, then pass through validation/application helpers."
  - "Schema export tests generate schemas/draft-import-plan.schema.json and prove unknown top-level provider fields are rejected."
  - "Phase 17 Rust aggregate gate now includes draft_import_plan coverage."

requirements-completed: [COMP-01, COMP-02, NO-FALLBACK-02]

duration: 10 min
completed: 2026-06-24
status: complete
---

# Phase 17 Plan 03: Draft Import Plan Summary

**Provider-neutral DraftImportPlan contract with validation that fails unsafe adapter output before project-session mutation.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-24T08:29:39Z
- **Completed:** 2026-06-24T08:39:25Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- Added `DraftImportPlan`, import material/track wrappers, application input/result types, and Rust exports from `draft_import`.
- Added `validate_import_plan` and `apply_import_plan_to_draft` to reject remote material refs, provider/raw formula leakage, ambiguous z-order, and invalid canonical draft shape before session mutation.
- Generated `schemas/draft-import-plan.schema.json` and extended schema tests to reject unknown provider-only top-level fields.
- Added focused TDD coverage for valid canvas/material/visual/text/audio/keyframe import plans plus invalid remote, formula, timerange, material, and z-order cases.

## Task Commits

1. **Task 1 RED: Specify DraftImportPlan validation behavior** - `87344df` (test)
2. **Task 2 GREEN: Implement DraftImportPlan and schema export** - `e5644e3` (feat)

## Files Created/Modified

- `crates/draft_import/src/import_plan.rs` - Provider-neutral import-plan and application result contracts.
- `crates/draft_import/src/validation.rs` - Validation and pure application helpers.
- `crates/draft_import/tests/import_plan.rs` - TDD behavior tests for valid and invalid import plans.
- `crates/draft_import/tests/schema_exports.rs` - Draft import plan schema export and unknown-field gate.
- `schemas/draft-import-plan.schema.json` - Generated JSON Schema for adapter/session contract validation.
- `crates/draft_import/src/lib.rs` - Public module and type/function re-exports.
- `crates/draft_import/Cargo.toml`, `Cargo.lock` - `draft_model` dependency for canonical draft types.
- `package.json` - Added `draft_import_plan` coverage to `test:phase17-rust`.

## Decisions Made

- Kept provenance/report evidence out of `DraftImportPlan`; `DraftImportApplicationInput` carries `report_items` separately and `apply_import_plan_to_draft` returns an `AdaptationReport`.
- Treated `z_order` as import-boundary metadata only: it validates deterministic layer ordering before canonical `Draft.tracks` are built.
- Kept project persistence out of this plan. The helper returns values for the future project-session owner and does not write `.veproj/project.json`.

## Verification

All plan verification passed:

- `cargo test -p draft_import draft_import_plan -- --nocapture`
- `cargo test -p draft_import schema_exports -- --nocapture`
- `pnpm run test:phase17-source-guards`
- `pnpm run test:contracts`

Additional aggregate gate passed:

- `pnpm run test:phase17-rust`

`pnpm` reported the existing Node engine warning (`wanted 24.12.0`, current `24.15.0`) but all commands exited successfully.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The RED test run failed as expected because `DraftImportPlan` and validation/application APIs did not exist yet.
- The first schema export run was executed with `VE_UPDATE_GENERATED_CONTRACTS=1` to create `schemas/draft-import-plan.schema.json`; the normal schema gate passed afterward.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Stub scan found no TODO/FIXME/placeholder or empty runtime data patterns in the created/modified files.

## Threat Flags

None. The new adapter-output to import-plan boundary is covered by the plan threat model, and no new network endpoint, auth path, file-write path, or session mutation surface was introduced.

## TDD Gate Compliance

- RED commit present before GREEN: `87344df` -> `e5644e3`
- RED failed for the intended reason: unresolved import-plan API symbols.
- GREEN verification passed with focused behavior tests and schema gates.

## Next Phase Readiness

Ready for later Phase 17 adapter/session plans to emit and apply validated `DraftImportPlan` values without interpreting provider formulas in core or session code.

## Self-Check: PASSED

- Files found: `crates/draft_import/src/import_plan.rs`, `crates/draft_import/src/validation.rs`, `crates/draft_import/tests/import_plan.rs`, `crates/draft_import/tests/schema_exports.rs`, `schemas/draft-import-plan.schema.json`, `crates/draft_import/src/lib.rs`, `crates/draft_import/Cargo.toml`, `package.json`.
- Commits found in git history: `87344df`, `e5644e3`.
- Plan-level verification commands passed.

---
*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Completed: 2026-06-24*
