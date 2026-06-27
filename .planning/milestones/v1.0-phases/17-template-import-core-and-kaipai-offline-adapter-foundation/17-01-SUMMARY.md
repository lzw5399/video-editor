---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
plan: "01"
subsystem: template-import
tags: [draft-import, adaptation-report, contracts, source-guards, no-fallback]

requires:
  - phase: 16
    provides: [comment-filtered source guard patterns, no-product-fallback guard integration]
provides:
  - Provider-neutral AdaptationReport Rust contract
  - Rust-generated adaptation report JSON Schema and desktop TypeScript contract
  - Phase 17 source guards for provider leakage, remote runtime dependencies, Android worker dependencies, raw formula semantics, and fallback success evidence
affects: [template-import, adapter-kaipai, project-session-import, desktop-report-ui, phase17-validation]

tech-stack:
  added: [draft_import Rust crate]
  patterns:
    - Strict serde/schemars/ts-rs provider-neutral report contracts
    - Comment-filtered shell guards with injected negative checks
    - Package-level Phase 17 Wave 0 gates

key-files:
  created:
    - crates/draft_import/Cargo.toml
    - crates/draft_import/src/lib.rs
    - crates/draft_import/src/adaptation_report.rs
    - crates/draft_import/tests/adaptation_report.rs
    - crates/draft_import/tests/schema_exports.rs
    - schemas/adaptation-report.schema.json
    - apps/desktop-electron/src/generated/TemplateImport.ts
    - scripts/phase17-source-guards.sh
  modified:
    - Cargo.toml
    - Cargo.lock
    - package.json

key-decisions:
  - "AdaptationReport is provider-neutral and keeps external references only in provenance evidence."
  - "Desktop report types are generated from Rust with ts-rs instead of hand-written TypeScript."
  - "Phase 17 source guards scan core/render/session/export paths while reserving provider-specific parsing for adapter crates."

patterns-established:
  - "Report contracts use rename_all camelCase plus deny_unknown_fields on structs."
  - "Source guards include injected negative checks and comment filtering before scanning production paths."
  - "Package gates expose test:phase17-rust, test:phase17-source-guards, and test:phase17."

requirements-completed: [COMP-02, NO-FALLBACK-01, NO-FALLBACK-02]

duration: 14 min
completed: 2026-06-24
status: complete
---

# Phase 17 Plan 01: Template Import Core And Kaipai Offline Adapter Foundation Summary

**Provider-neutral adaptation report contract with Rust-generated schema/TypeScript outputs and Phase 17 boundary guards.**

## Performance

- **Duration:** 14 min
- **Started:** 2026-06-24T07:10:37Z
- **Completed:** 2026-06-24T07:25:33Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- Added the `draft_import` workspace crate with `AdaptationReport`, summary, item, status, severity, category, target, and provenance types.
- Generated `schemas/adaptation-report.schema.json` and `apps/desktop-electron/src/generated/TemplateImport.ts` from Rust contract types.
- Added `scripts/phase17-source-guards.sh` plus package scripts for Wave 0 report and boundary validation.

## Task Commits

1. **Task 1 RED: AdaptationReport tests** - `685a4f1` (test)
2. **Task 1 GREEN: AdaptationReport implementation** - `19df25b` (feat)
3. **Task 2 RED: Schema/TypeScript export tests** - `a19c17f` (test)
4. **Task 2 GREEN: Generated contract artifacts** - `93b9955` (feat)
5. **Task 3: Phase 17 source guards and package gates** - `8c5af87` (feat)

## Files Created/Modified

- `crates/draft_import/src/adaptation_report.rs` - Provider-neutral report contract and summary counting.
- `crates/draft_import/tests/adaptation_report.rs` - Status/category/provenance behavior tests.
- `crates/draft_import/tests/schema_exports.rs` - Rust-derived schema and TypeScript generation gate.
- `schemas/adaptation-report.schema.json` - Generated JSON Schema with required statuses and unknown-field rejection.
- `apps/desktop-electron/src/generated/TemplateImport.ts` - Generated desktop TypeScript report contract.
- `scripts/phase17-source-guards.sh` - Phase 17 boundary and no-fallback source guard.
- `Cargo.toml`, `Cargo.lock`, `package.json` - Workspace and package gate wiring.

## Decisions Made

- Kept `templateId`, raw formula, and provider-specific identifiers out of report item fields; external references live under `ExternalProvenanceRef`.
- Used `approximated` and `dropped` status names for the new provider-neutral contract instead of old compatibility wording.
- Added an aggregate `test:phase17` package script so later plans can extend a single Phase 17 gate.

## Verification

All verification passed:

- `cargo test -p draft_import adaptation_report -- --nocapture`
- `cargo test -p draft_import schema_exports -- --nocapture`
- `bash scripts/phase17-source-guards.sh`
- `pnpm run test:phase17-source-guards`
- `pnpm run test:contracts`
- `pnpm run test:phase17`

`pnpm` emitted the existing engine warning (`wanted node 24.12.0`, current `24.15.0`) but all commands exited successfully.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed focused test filter coverage**
- **Found during:** Task 1
- **Issue:** The exact command `cargo test -p draft_import adaptation_report -- --nocapture` initially selected only one test because three test names did not include `adaptation_report`.
- **Fix:** Renamed those tests so the plan's focused command exercises all report contract cases.
- **Files modified:** `crates/draft_import/tests/adaptation_report.rs`
- **Verification:** `cargo test -p draft_import adaptation_report -- --nocapture`
- **Committed in:** `19df25b`

**2. [Rule 1 - Bug] Fixed source guard comment filtering for URLs and single-file rg output**
- **Found during:** Task 3
- **Issue:** The initial guard filter treated `https://` inside strings as comments and did not filter comment-only matches when `rg` omitted filenames for single-file input.
- **Fix:** Anchored comment filtering to `file:line:` or `line:` prefixes so injected negative tests catch real code while comments are ignored.
- **Files modified:** `scripts/phase17-source-guards.sh`
- **Verification:** `bash scripts/phase17-source-guards.sh`
- **Committed in:** `8c5af87`

**3. [Rule 1 - Bug] Avoided false positive on fallback diagnostics**
- **Found during:** Task 3
- **Issue:** The first fallback-success pattern flagged existing diagnostic state `fallbackActive: true`, which is not a product success claim.
- **Fix:** Narrowed the pattern to explicit success-evidence names and kept broader fallback policy enforcement delegated to `scripts/no-product-fallback-guards.sh`.
- **Files modified:** `scripts/phase17-source-guards.sh`
- **Verification:** `bash scripts/phase17-source-guards.sh && pnpm run test:phase17-source-guards`
- **Committed in:** `8c5af87`

**Total deviations:** 3 auto-fixed (3 bug fixes).
**Impact on plan:** All fixes tightened the intended gates and did not expand product scope.

## Issues Encountered

- `pnpm` reported the existing Node engine mismatch warning (`wanted 24.12.0`, current `24.15.0`). It did not block any verification command.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Stub scan hits were intentional guard negative-check strings, ts-rs optional annotations, or pre-existing workspace metadata.

## Threat Flags

None. The new report contract and source guard surfaces are covered by the plan threat model.

## TDD Gate Compliance

- RED commit present before Task 1 GREEN: `685a4f1` -> `19df25b`
- RED commit present before Task 2 GREEN: `a19c17f` -> `93b9955`

## Next Phase Readiness

Ready for Plan 17-02 to build resource localization on top of the provider-neutral report and source guard foundation.

## Self-Check: PASSED

- All 11 created/modified files exist.
- Task commits `685a4f1`, `19df25b`, `a19c17f`, `93b9955`, and `8c5af87` exist in git history.
- Plan-level verification commands passed.

---
*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Completed: 2026-06-24*
