---
phase: 02-draft-and-material-system
plan: 06
subsystem: validation-fixtures
tags: [rust, fixtures, json-schema, just, pnpm, phase-gates]
requires:
  - phase: 02-draft-and-material-system
    provides: Draft schema, project-store persistence, material probing, material service, command bindings, and Electron smoke display
provides:
  - Classified positive and negative `.veproj/project.json` fixture corpus
  - Draft fixture tests covering model migration, semantic round trips, JSON Schema validation, missing-material preservation, and negative gates
  - Named Phase 2 root test scripts for draft fixtures, project-store, material probe, material service, source guards, and contract drift
  - Final `just build`, `just test`, and generated contract drift gate coverage for Phase 2
affects: [phase-03-timeline-command-core, project-store, material-import, generated-contracts, ci-gates]
tech-stack:
  added: []
  patterns: [classified-project-fixtures, phase-gate-root-scripts, source-regression-guards]
key-files:
  created:
    - fixtures/draft/positive/minimal-draft/project.json
    - fixtures/draft/positive/materials-round-trip/project.json
    - fixtures/draft/positive/missing-material/project.json
    - fixtures/draft/negative/invalid-unknown-field/project.json
    - fixtures/draft/negative/invalid-schema-version/project.json
    - fixtures/draft/negative/derived-artifact-in-project-json/project.json
    - crates/draft_model/tests/draft_fixtures.rs
  modified:
    - crates/draft_model/tests/schema_exports.rs
    - justfile
    - package.json
    - .planning/STATE.md
    - .planning/ROADMAP.md
key-decisions:
  - "Phase 2 project fixtures live under `fixtures/draft/positive/*/project.json` and `fixtures/draft/negative/*/project.json`, while existing flat command fixtures remain separately classified."
  - "Final Phase 2 gates are exposed as named root package scripts and invoked by `just test`."
  - "Source guards enforce Jianying terminology, integer persisted time, renderer FFmpeg isolation, project-store ownership, and derived-artifact exclusion."
patterns-established:
  - "Project fixture tests classify every discovered `project.json` before validating positive and negative behavior."
  - "Root `test:*` scripts provide stable names for per-subsystem phase gates while `just test` remains the public aggregate command."
requirements-completed: [DRAFT-01, DRAFT-02, DRAFT-03, DRAFT-04, DRAFT-05, MAT-01, MAT-02, MAT-03, MAT-04]
duration: 20min
completed: 2026-06-17
---

# Phase 02 Plan 06: Draft And Material Fixtures And Gates Summary

**Classified `.veproj/project.json` fixtures and final Phase 2 gate scripts prove draft/material durability, metadata, missing-material recovery, and source-boundary constraints.**

## Performance

- **Duration:** 20 min
- **Started:** 2026-06-17T03:12:23Z
- **Completed:** 2026-06-17T03:32:23Z
- **Tasks:** 2
- **Files modified:** 10 code/test/fixture/gate files plus planning metadata

## Accomplishments

- Added positive draft fixtures for a minimal draft, video/image/audio material metadata, and recoverable missing-material preservation.
- Added negative draft fixtures for unknown fields, unsupported schema versions, and derived artifact leakage into `project.json`.
- Added `draft_fixtures.rs` to classify every discovered project fixture and validate positive fixtures through Rust migration, semantic equality, and generated JSON Schema.
- Added named Phase 2 root scripts and routed `just test` through draft fixture, project-store, material probe, material service, source guard, Electron smoke, render smoke, and contract drift gates.
- Added source guards for `Asset`/`Clip` terminology drift, persisted float seconds, renderer FFmpeg/ffprobe construction, project-store import orchestration, and derived artifact fields in generated draft semantics.

## Task Commits

1. **Task 1: Add classified draft/material project fixtures** - `f3c677b` (test)
2. **Task 2: Finalize explicit Phase 2 gates and source guards** - `035aa04` (chore)

**Plan metadata:** committed with this summary.

## Files Created/Modified

- `fixtures/draft/positive/minimal-draft/project.json` - Valid empty `.veproj` draft fixture.
- `fixtures/draft/positive/materials-round-trip/project.json` - Valid video/image/audio material metadata and track/segment fixture.
- `fixtures/draft/positive/missing-material/project.json` - Valid missing-material fixture preserving recoverable status and original URI.
- `fixtures/draft/negative/invalid-unknown-field/project.json` - Strict unknown-field rejection fixture.
- `fixtures/draft/negative/invalid-schema-version/project.json` - Unsupported schema version rejection fixture.
- `fixtures/draft/negative/derived-artifact-in-project-json/project.json` - Derived artifact leakage rejection fixture.
- `crates/draft_model/tests/draft_fixtures.rs` - Project fixture classifier and schema/model validation tests.
- `crates/draft_model/tests/schema_exports.rs` - Allows flat command fixtures and nested project fixture directories to coexist.
- `package.json` - Named Phase 2 gate scripts and source guards.
- `justfile` - Public `just test` route through root gate scripts.
- `.planning/STATE.md` and `.planning/ROADMAP.md` - Phase 2 completion state.

## Decisions Made

- Kept the existing flat Phase 1 command fixture files under `fixtures/draft` and added project fixtures as nested `positive` / `negative` `.veproj` style folders.
- Treated unsupported schema version as a migration-gate failure; strict unknown fields and derived artifacts additionally fail generated JSON Schema.
- Used package scripts as the stable named gate surface so local `just test` and future CI can invoke the same checks.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Allowed command fixture tests to coexist with nested project fixture directories**
- **Found during:** Task 1 (classified draft/material project fixtures)
- **Issue:** Adding `fixtures/draft/positive` and `fixtures/draft/negative` directories would make the existing command fixture classifier fail because it expected only flat JSON files under `fixtures/draft`.
- **Fix:** Updated `crates/draft_model/tests/schema_exports.rs` to ignore directories when classifying the existing command-envelope fixtures.
- **Files modified:** `crates/draft_model/tests/schema_exports.rs`
- **Verification:** `cargo test -p draft_model schema_fixtures -- --nocapture` and `cargo test -p draft_model draft_fixtures -- --nocapture` passed.
- **Committed in:** `f3c677b`

---

**Total deviations:** 1 auto-fixed (1 Rule 3 blocking issue)
**Impact on plan:** The fix was necessary for Phase 1 command fixtures and Phase 2 project fixtures to share `fixtures/draft` without weakening classification.

## Issues Encountered

- Bare `gsd-tools` was not on PATH; the local Codex shim at `$HOME/.codex/get-shit-done/bin/gsd-tools.cjs` was used where possible.
- `state.update-progress` produced an inconsistent frontmatter percentage before the new summary existed. Planning metadata was normalized manually before the final docs commit.

## Known Stubs

None.

## Authentication Gates

None.

## Threat Flags

None - this plan added local JSON fixtures, test classification, root gate scripts, and grep guards only. It introduced no network endpoints, auth paths, new process execution beyond existing test commands, or unplanned persistence/runtime trust boundaries.

## Verification

- `cargo test -p draft_model schema_fixtures -- --nocapture` - passed.
- `cargo test -p draft_model draft_fixtures -- --nocapture` - passed.
- `find fixtures/draft -name project.json | sort` - listed all six classified project fixtures.
- `pnpm run test:phase2-source-guards` - passed.
- `cargo test -p draft_model draft_schema -- --nocapture` - passed.
- `cargo test -p draft_model migration -- --nocapture` - passed.
- `cargo test -p project_store create_project_bundle -- --nocapture` - passed.
- `cargo test -p project_store round_trip -- --nocapture` - passed.
- `cargo test -p project_store path_resolution -- --nocapture` - passed.
- `cargo test -p bindings_node material_service -- --nocapture` - passed.
- `cargo test -p media_runtime material_probe -- --nocapture` - passed.
- `cargo test -p testkit material -- --nocapture` - passed.
- `cargo test -p bindings_node -- --nocapture` - passed.
- `pnpm --filter @video-editor/desktop test` - passed.
- `PATH="$HOME/.cargo/bin:$PATH" just build` - passed.
- `PATH="$HOME/.cargo/bin:$PATH" just test` - passed.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed.

## Self-Check: PASSED

- Found all six project fixture files, `crates/draft_model/tests/draft_fixtures.rs`, `justfile`, and `package.json` on disk.
- Found task commits `f3c677b` and `035aa04` in git history.
- Stub scan found no placeholder/TODO/FIXME or hardcoded empty UI-data stubs in files changed by this plan.
- Threat scan found only intended local fixture parsing and negative derived-artifact test data.
- No tracked files were accidentally deleted.

## User Setup Required

None - no external service configuration required. FFmpeg and ffprobe must remain available through `VE_FFMPEG_PATH` / `VE_FFPROBE_PATH` or PATH for media-backed tests.

## Next Phase Readiness

Phase 2 is ready for verification. Phase 3 can build timeline command semantics on top of stable `.veproj/project.json` draft/material fixtures, project-store persistence, material import/probe behavior, Rust-generated contracts, and the final `just build` / `just test` gate path.

---
*Phase: 02-draft-and-material-system*
*Completed: 2026-06-17*
