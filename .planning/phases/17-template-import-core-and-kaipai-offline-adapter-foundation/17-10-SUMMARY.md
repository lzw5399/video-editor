---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
plan: "10"
subsystem: template-import
tags: [adapter-kaipai, draft-import-plan, adaptation-report, resource-localizer, offline-mapper]

requires:
  - phase: 17-03
    provides: [provider-neutral DraftImportPlan validation and application]
  - phase: 17-05
    provides: [sanitized Kaipai fixture and expected report corpus]
  - phase: 17-07
    provides: [generic static center-anchor rotation export parity]
provides:
  - Adapter-local Kaipai bundle mapper that emits validated DraftImportPlan plus AdaptationReport
  - Offline mapper tests for main video, PIP, text sticker, BGM/audio, missing resource, native effect, static rotation, constant speed, simple transition, and visual keyframes
  - Phase 17 Rust aggregate coverage for the offline mapper gate
affects: [adapter-kaipai, draft-import-plan, project-session-import, template-import-validation]

tech-stack:
  added: [draft_model workspace dependency for adapter_kaipai mapper construction]
  patterns:
    - Resource localization runs before canonical material refs are created
    - Kaipai formula interpretation remains adapter-local and emits provider-neutral DraftImportPlan fields
    - Unsupported provider effects are report diagnostics, not canonical filter or render semantics

key-files:
  created:
    - crates/adapter_kaipai/src/mapper.rs
    - crates/adapter_kaipai/tests/mapper.rs
  modified:
    - crates/adapter_kaipai/Cargo.toml
    - Cargo.lock
    - crates/adapter_kaipai/src/error.rs
    - crates/adapter_kaipai/src/lib.rs
    - package.json

key-decisions:
  - "Kaipai offline mapper output is DraftImportPlan plus AdaptationReport; it does not mutate Draft/session state directly."
  - "Localized project-relative resource refs are required before material-backed segments enter the import plan."
  - "Kaipai level maps to provider-neutral ImportTrackPlan z-order and sorted Draft track order."
  - "Native effects and provider text effects are reported as needsNativeEffect/dropped or dropped, never hidden as supported canonical filters."

patterns-established:
  - "Mapper tests seed local source resources and disable sha256 verification only for placeholder fixture bytes; production options default to sha256 verification."
  - "Report status assertions compare status coverage while allowing snapshot-style multi-item supported reports."

requirements-completed: [COMP-01, COMP-02, PRODFX-05]

duration: 16 min
completed: 2026-06-24
status: complete
---

# Phase 17 Plan 10: Kaipai Offline Mapper Summary

**Offline Kaipai formula bundles now map into validated provider-neutral DraftImportPlan data with explicit AdaptationReport diagnostics.**

## Performance

- **Duration:** 16 min
- **Started:** 2026-06-24T09:03:28Z
- **Completed:** 2026-06-24T09:19:56Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added `map_kaipai_bundle_to_import_plan`, `KaipaiImportOptions`, and `KaipaiMappedFixture` in `adapter_kaipai`.
- Mapped sanitized fixture families into canonical canvas, material, track, segment, visual, text, audio, transition, keyframe, and report fields.
- Preserved unsupported/native/provider-specific behavior in `AdaptationReport` evidence instead of canonical draft semantics.
- Added focused offline mapper tests and wired them into `test:phase17-rust`.

## Task Commits

1. **Task 1 RED: Add offline mapper behavior tests** - `585cc01` (test)
2. **Task 1 RED refinement: Align report status assertions with snapshot-style reports** - `475ca67` (test)
3. **Task 2 GREEN: Implement Kaipai-to-DraftImportPlan mapper** - `586e4f2` (feat)
4. **Task 2 gate fix: Add offline mapper to Phase 17 Rust aggregate** - `2ab696c` (test)

## Files Created/Modified

- `crates/adapter_kaipai/src/mapper.rs` - Adapter-local formula-to-import-plan mapper with resource localization, report generation, and plan validation.
- `crates/adapter_kaipai/tests/mapper.rs` - Offline mapper behavior tests for supported and degraded fixture families.
- `crates/adapter_kaipai/src/error.rs` - Mapper/localizer/import-plan error variants.
- `crates/adapter_kaipai/src/lib.rs` - Public mapper API exports.
- `crates/adapter_kaipai/Cargo.toml` and `Cargo.lock` - `draft_model` dependency for canonical draft type construction and tests.
- `package.json` - `test:phase17-rust` now includes the offline mapper gate.

## Decisions Made

- Mapper resource options default to sha256 verification; tests disable it only for seeded placeholder bytes from sanitized fixtures.
- Provider formula paths and external IDs remain report provenance only; canonical plan fields use generic draft/material/track/segment/keyframe/filter/transition concepts.
- Static center-anchor rotation maps to `SegmentVisual` because Plan 17-07 closed generic export parity; native effects stay report-only.

## Verification

All required verification passed:

- `cargo test -p adapter_kaipai offline_mapper -- --nocapture`
- `cargo test -p draft_import draft_import_plan -- --nocapture`
- `pnpm run test:phase17-source-guards`

Additional verification passed:

- `cargo test -p adapter_kaipai -- --nocapture`
- `pnpm run test:phase17-rust`

Warnings observed:

- `pnpm` reported the existing Node engine warning (`wanted node 24.12.0`, current `24.15.0`), but commands exited successfully.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added offline mapper tests to the Phase 17 Rust aggregate**
- **Found during:** Task 2 closeout
- **Issue:** The new `offline_mapper` gate was only run by Plan 17-10 verification and would not run in the Phase 17 Rust aggregate.
- **Fix:** Added `cargo test -p adapter_kaipai offline_mapper -- --nocapture` to `test:phase17-rust`.
- **Files modified:** `package.json`
- **Verification:** `pnpm run test:phase17-rust`
- **Committed in:** `2ab696c`

**Total deviations:** 1 auto-fixed (1 missing critical aggregate gate).
**Impact on plan:** The fix keeps mapper behavior covered by the phase aggregate without expanding product scope.

## Issues Encountered

- The RED tests initially needed `draft_model` as a local test dependency so they could assert canonical field values. The dependency was promoted to a normal crate dependency when the mapper implementation needed to construct canonical draft types.
- No blockers remain.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Stub scan found no TODO/FIXME/placeholder/empty runtime data patterns in files created or modified by this plan.

## Threat Flags

None. The new mapper uses the planned adapter-to-import-plan and localized-resource trust boundaries from the plan threat model; it adds no network endpoints, auth paths, project-session mutation, file access outside the localizer, or raw provider fields in canonical plan semantics.

## TDD Gate Compliance

- RED commits present before GREEN: `585cc01`, `475ca67`.
- RED failed for the intended reason after the local dev-dependency was added: missing mapper API/types.
- GREEN commit present after RED: `586e4f2`.
- Refactor commit was not needed.

## Next Phase Readiness

Ready for project-session import application to consume the provider-neutral mapper output while preserving Rust session ownership of `.veproj/project.json` mutation.

## Self-Check: PASSED

- Files found: `crates/adapter_kaipai/src/mapper.rs`, `crates/adapter_kaipai/src/lib.rs`, `crates/adapter_kaipai/src/error.rs`, `crates/adapter_kaipai/tests/mapper.rs`, `crates/adapter_kaipai/Cargo.toml`, `Cargo.lock`, and `package.json`.
- Commits found in git history: `585cc01`, `475ca67`, `586e4f2`, `2ab696c`.
- Plan-level verification commands passed after all task commits.

---
*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Completed: 2026-06-24*
