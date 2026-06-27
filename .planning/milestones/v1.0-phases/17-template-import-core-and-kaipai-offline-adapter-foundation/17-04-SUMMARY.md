---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
plan: "04"
subsystem: template-import
tags: [adapter-kaipai, formula-bundle, schema, fixtures, source-guards]

requires:
  - phase: 17-01
    provides: [provider-neutral AdaptationReport contract, Phase 17 source guards]
provides:
  - Offline `adapter_kaipai` Rust crate registered in the workspace
  - Strict `KaipaiFormulaBundle` parser and validator for sanitized offline input
  - Sanitized positive and negative Kaipai formula fixture corpus
  - Rust-generated `schemas/kaipai-formula-bundle.schema.json`
affects: [template-import, adapter-kaipai, draft-import, resource-localization, project-session-import]

tech-stack:
  added: [adapter_kaipai Rust crate]
  patterns:
    - Strict serde/schemars adapter-local provider contracts
    - Sanitized fixture scanner for remote, signed, credential-like evidence
    - Adapter provenance bridge into provider-neutral `ExternalProvenanceRef`

key-files:
  created:
    - crates/adapter_kaipai/Cargo.toml
    - crates/adapter_kaipai/src/lib.rs
    - crates/adapter_kaipai/src/error.rs
    - crates/adapter_kaipai/src/formula_bundle.rs
    - crates/adapter_kaipai/tests/formula_bundle_contract.rs
    - crates/adapter_kaipai/tests/fixtures.rs
    - crates/adapter_kaipai/tests/schema_exports.rs
    - fixtures/kaipai/positive/sanitized-formula-bundle.json
    - fixtures/kaipai/positive/sanitized-formula-with-direct-materials.json
    - fixtures/kaipai/negative/unknown-top-level-field.json
    - fixtures/kaipai/negative/unsafe-formula-evidence.json
    - schemas/kaipai-formula-bundle.schema.json
  modified:
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Kaipai provider IDs, raw formula JSON, recognizer output, and safe-area evidence stay adapter-local and may only flow into provider-neutral provenance/report evidence."
  - "Committed fixtures are sanitized; remote URL, signed URL, and credential-like rejection is tested through in-memory mutations instead of unsafe committed data."
  - "The formula bundle schema is generated from Rust and committed without desktop TypeScript output because no UI import/report surface is added in this plan."

patterns-established:
  - "Adapter contracts use `rename_all = \"camelCase\"` plus `deny_unknown_fields` on input structs."
  - "Offline provider evidence is validated before any future mapper can emit canonical draft semantics."
  - "Adapter crates may depend on `draft_import`; core/render/session/product paths do not import `adapter_kaipai`."

requirements-completed: [COMP-01, COMP-02, NO-FALLBACK-02]

duration: 11 min
completed: 2026-06-24
status: complete
---

# Phase 17 Plan 04: Kaipai Offline Formula Bundle Summary

**Offline Kaipai formula bundle crate with strict sanitized input parsing, fixture validation, and Rust-generated schema.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-06-24T07:51:58Z
- **Completed:** 2026-06-24T08:03:06Z
- **Tasks:** 2
- **Files modified:** 14

## Accomplishments

- Added `adapter_kaipai` as a workspace crate that depends on `draft_import` and keeps Kaipai-specific formula evidence out of core/render/session semantics.
- Implemented `KaipaiFormulaBundle::from_json_str`, `from_json_value`, and `validate` with strict unknown-field rejection and unsafe evidence checks.
- Added sanitized positive/negative fixtures, fixture hygiene tests, and a committed JSON Schema generated from the Rust contract.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Kaipai formula bundle contract tests and fixtures** - `808c2ae` (test)
2. **Task 1 GREEN: Strict formula bundle parser and validation** - `cb0dac4` (feat)
3. **Task 2: Rust-generated formula bundle schema export** - `9e91b63` (feat)

**Plan metadata:** this SUMMARY commit.

## Files Created/Modified

- `Cargo.toml`, `Cargo.lock` - Registered `adapter_kaipai` in the Rust workspace.
- `crates/adapter_kaipai/Cargo.toml` - Adapter crate manifest using workspace package policy and the `draft_import` boundary.
- `crates/adapter_kaipai/src/lib.rs` - Public adapter exports.
- `crates/adapter_kaipai/src/error.rs` - Adapter error taxonomy for invalid JSON, invalid evidence, and unsafe evidence.
- `crates/adapter_kaipai/src/formula_bundle.rs` - Strict offline input contract and validation logic.
- `crates/adapter_kaipai/tests/formula_bundle_contract.rs` - Positive/negative parser behavior tests.
- `crates/adapter_kaipai/tests/fixtures.rs` - Explicit fixture classification and hygiene scanner.
- `crates/adapter_kaipai/tests/schema_exports.rs` - Rust-derived schema generation and fixture/schema validation gate.
- `fixtures/kaipai/positive/*.json` - Sanitized offline bundle examples.
- `fixtures/kaipai/negative/*.json` - Unknown-field and unsafe-evidence rejection examples.
- `schemas/kaipai-formula-bundle.schema.json` - Generated formula bundle JSON Schema.

## Decisions Made

- Kept Kaipai `templateId`, `recipeId`, formula task/request IDs, raw formula JSON, recognizer output, and `safeArea` evidence inside the adapter input/provenance boundary only.
- Used `ExternalProvenanceRef` from `draft_import` for future report evidence instead of creating canonical render semantics from provider IDs.
- Did not generate desktop TypeScript for the formula bundle yet; this plan adds no UI/API surface that should expose raw adapter input to the renderer.

## Verification

All verification passed:

- `cargo test -p adapter_kaipai formula_bundle -- --nocapture`
- `cargo test -p adapter_kaipai schema_exports -- --nocapture`
- `pnpm run test:phase17-source-guards`

`pnpm` emitted the existing Node engine warning (`wanted node 24.12.0`, current `24.15.0`) but exited successfully.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Kept committed unsafe fixture sanitized**
- **Found during:** Task 1 (Create current-main adapter crate and fixture tests)
- **Issue:** The first unsafe negative fixture used a literal remote URL, which correctly made the fixture hygiene scanner fail.
- **Fix:** Changed the committed negative fixture to unsafe safe-area provenance that is still sanitized, and kept remote URL, signed URL, and credential-like rejection as in-memory parser mutations.
- **Files modified:** `crates/adapter_kaipai/tests/formula_bundle_contract.rs`, `fixtures/kaipai/negative/unsafe-formula-evidence.json`
- **Verification:** `cargo test -p adapter_kaipai formula_bundle -- --nocapture`
- **Committed in:** `cb0dac4`

**Total deviations:** 1 auto-fixed (1 bug fix).
**Impact on plan:** The fix tightened the fixture safety requirement without changing the adapter scope.

## Issues Encountered

- The command runner PATH became inconsistent for ad hoc post-checks after a `pnpm` child command; using absolute `/usr/bin/*` paths for nonessential shell checks resolved it. Project verification commands themselves passed.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. The stub scan found no placeholder/TODO values in files created or modified by this plan.

## Threat Flags

None. The new untrusted offline JSON input surface is covered by the plan threat model and mitigated with strict serde/schema tests plus unsafe evidence rejection.

## TDD Gate Compliance

- RED commit present before Task 1 GREEN: `808c2ae` -> `cb0dac4`
- Task 2 was a standard auto task and produced `9e91b63`.

## Next Phase Readiness

Ready for downstream Phase 17 plans to build resource localization, import-plan mapping, and project-session application on top of the strict offline adapter input boundary.

## Self-Check: PASSED

- All 14 created/modified files exist on disk.
- Task commits `808c2ae`, `cb0dac4`, and `9e91b63` exist in git history.
- Plan-level verification commands passed.
- Source guards confirm core/render/session/product paths do not import `adapter_kaipai`.

---
*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Completed: 2026-06-24*
