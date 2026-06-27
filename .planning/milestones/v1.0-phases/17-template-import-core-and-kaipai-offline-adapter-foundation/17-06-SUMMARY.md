---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
plan: "06"
subsystem: template-import
tags: [project-session, adapter-kaipai, draft-import-plan, resource-index, napi]

requires:
  - phase: 17-10
    provides: [Kaipai offline mapper to validated DraftImportPlan and AdaptationReport]
  - phase: 16-07
    provides: [project IO scheduler baseline and session/runtime ownership notes]
provides:
  - Rust-owned project-session API for offline Kaipai formula bundle import
  - N-API export `importKaipaiFormulaBundle`
  - Atomic revision/save/import tests for success, stale revision, missing session, invalid resource root, clean project JSON, and localized resource index rows
  - Resource-index helper for persisting localized template resource refs alongside canonical draft resources
affects: [project-session-import, adapter-kaipai, artifact-store, desktop-template-import-ui]

tech-stack:
  added: [bindings_node dependency on adapter_kaipai and draft_import]
  patterns:
    - Project session applies validated DraftImportPlan and owns save/revision behavior
    - Localized template resources are persisted through artifact_store resource_index after project save succeeds
    - Phase 17 source guards allow provider terms only at the explicit session import boundary

key-files:
  created:
    - crates/bindings_node/tests/project_session_import_kaipai.rs
    - .planning/phases/17-template-import-core-and-kaipai-offline-adapter-foundation/17-06-SUMMARY.md
  modified:
    - Cargo.lock
    - crates/artifact_store/src/resource_index.rs
    - crates/bindings_node/Cargo.toml
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/src/project_session_service.rs
    - scripts/phase17-source-guards.sh

key-decisions:
  - "Project session owns offline Kaipai import application, expected-revision checks, save, view-model response, and revision increment."
  - "Localized resource index rows are persisted through artifact_store resource_index with a transaction-backed helper, not direct SQLite writes from bindings_node."
  - "Import replaces the session draft and resets command state, selection, and playhead so stale handles cannot survive a template import."
  - "Phase 17 source guards keep provider-specific terms blocked except for the explicit importKaipaiFormulaBundle boundary."

patterns-established:
  - "Project-session import APIs return session view models plus AdaptationReport without exposing raw Draft, commandState, or selection payloads."
  - "Validated import-plan application happens before any session-visible mutation; session state changes only after save and resource indexing succeed."

requirements-completed: [COMP-01, COMP-02, NO-FALLBACK-02]

duration: 17 min
completed: 2026-06-24
status: complete
---

# Phase 17 Plan 06: Project-Session Kaipai Import Summary

**Offline Kaipai formula bundles now import through the Rust project-session boundary with revision checks, canonical `.veproj` save, localized resource indexing, and AdaptationReport response data.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-06-24T09:27:15Z
- **Completed:** 2026-06-24T09:44:19Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added focused project-session import tests covering successful import, stale revision rejection, missing session, failed localization root, canonical project JSON, and resource index persistence.
- Implemented `import_kaipai_formula_bundle` in `bindings_node` and exported it as `importKaipaiFormulaBundle`.
- Applied `DraftImportPlan` through the session-owned save/revision path and returned the updated view model plus `AdaptationReport`.
- Persisted localized material/font resource refs through `artifact_store::resource_index` after successful project save.

## Task Commits

1. **Task 1: Add project-session import API tests** - `ee786f5` (test)
2. **Task 2: Implement atomic offline import service** - `eb1fe52` (feat)

## Files Created/Modified

- `crates/bindings_node/tests/project_session_import_kaipai.rs` - Integration tests for import success, stale mutation rejection, canonical saved JSON, and no partial index rows on failure.
- `crates/bindings_node/src/project_session_service.rs` - Session import request parsing, stale checks, adapter mapping, import-plan application, save/index success path, and response model.
- `crates/bindings_node/src/lib.rs` - N-API export for `importKaipaiFormulaBundle`.
- `crates/bindings_node/Cargo.toml` and `Cargo.lock` - Added local dependencies on `adapter_kaipai` and `draft_import`.
- `crates/artifact_store/src/resource_index.rs` - Added transaction-backed helper for draft resources plus extra localized resource refs.
- `scripts/phase17-source-guards.sh` - Added a narrow allowlist for the explicit project-session import boundary.

## Decisions Made

- The import API requires `sessionId` and `expectedRevision`; stale revisions fail before reading/mapping resources.
- Resource index persistence remains owned by `artifact_store::resource_index`; `bindings_node` converts localized manifest refs into generic `ResourceRef` inputs.
- Provider-specific terms remain blocked in core/render/export/session paths except the explicit import function and associated request/diagnostic strings.

## Verification

All required verification passed:

- `cargo test -p bindings_node project_session_import_kaipai -- --nocapture` - passed, 5/5 tests.
- `pnpm run test:phase17-source-guards` - passed.
- `cargo check --workspace --locked` - passed.

Additional focused verification passed:

- `cargo test -p artifact_store resource_index -- --nocapture` - passed, 5/5 tests.

Warnings observed:

- `pnpm` reported the existing Node engine warning (`wanted node 24.12.0`, current `v24.15.0`), but the source guard exited successfully.
- `cargo` reported the existing macOS AVFoundation deprecation warning in `media_runtime_desktop`; the workspace check and tests passed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added resource-index support for localized import refs**
- **Found during:** Task 2
- **Issue:** The planned service had to persist localized material/font refs after save, but the existing resource-index API only indexed refs discoverable from canonical draft semantics.
- **Fix:** Added `index_draft_resources_with_extra_refs` and made resource persistence transaction-backed.
- **Files modified:** `crates/artifact_store/src/resource_index.rs`
- **Verification:** `cargo test -p artifact_store resource_index -- --nocapture`; `cargo test -p bindings_node project_session_import_kaipai -- --nocapture`
- **Committed in:** `eb1fe52`

**2. [Rule 3 - Blocking] Narrowed Phase 17 source guard for the explicit import boundary**
- **Found during:** Task 2 verification
- **Issue:** `pnpm run test:phase17-source-guards` failed on the plan-required `importKaipaiFormulaBundle` session API because the guard blocked all provider terms under `bindings_node/src`.
- **Fix:** Added an allowlist only for `project_session_service.rs` and `lib.rs` lines that name the explicit import/formula boundary; all other provider-term matches still fail.
- **Files modified:** `scripts/phase17-source-guards.sh`
- **Verification:** `pnpm run test:phase17-source-guards`
- **Committed in:** `eb1fe52`

**Total deviations:** 2 auto-fixed (1 missing critical, 1 blocking).
**Impact on plan:** Both fixes were required to satisfy the planned save/index and source-guard gates without broadening provider semantics into core/render/export paths.

## Issues Encountered

- The plan's Task 1 `read_first` referenced validation row `17-W0-06`, but the validation file maps Plan 17-06 to `17-W0-08`. Execution followed `17-W0-08`, the project-session import gate.
- RED failed for the intended reason: `bindings_node::import_kaipai_formula_bundle` did not exist before Task 2.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Stub scan found no runtime TODO/FIXME/placeholder/empty-data stubs in files created or modified by this plan. The only `rawFormula` match is the intentional source-guard negative self-test string in `scripts/phase17-source-guards.sh`.

## Threat Flags

None. The new trust-boundary surface is the planned Electron/native caller to Rust project-session import API and the planned adapter/import-plan to `.veproj/project.json` boundary from the plan threat model.

## TDD Gate Compliance

- RED commit present before GREEN: `ee786f5`.
- RED failed for the intended reason: missing `import_kaipai_formula_bundle` binding/service API.
- GREEN commit present after RED: `eb1fe52`.
- Refactor commit was not needed.

## Next Phase Readiness

Ready for desktop UI/report-panel work to call the Rust-owned import API. Downstream UI must pass `sessionId` and `expectedRevision`, surface the returned `AdaptationReport`, and must not write provider formula data into project JSON or renderer-held draft state.

## Self-Check: PASSED

- Files found: `crates/bindings_node/tests/project_session_import_kaipai.rs`, `crates/bindings_node/src/project_session_service.rs`, `crates/bindings_node/src/lib.rs`, `crates/bindings_node/Cargo.toml`, `crates/artifact_store/src/resource_index.rs`, `scripts/phase17-source-guards.sh`, and `Cargo.lock`.
- Commits found in git history: `ee786f5`, `eb1fe52`.
- Plan-level verification commands passed after all task commits.

---
*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Completed: 2026-06-24*
