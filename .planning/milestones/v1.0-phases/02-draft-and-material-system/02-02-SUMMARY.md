---
phase: 02-draft-and-material-system
plan: 02
subsystem: project-store
tags: [rust, project-store, veproj, draft-model, serde-json, path-resolution]
requires:
  - phase: 02-draft-and-material-system
    provides: Draft schema, validation, and migration hooks from Plan 02-01
provides:
  - `.veproj/project.json` create, open, save, and autosave APIs in `project_store`
  - Centralized project JSON path and material URI classification helpers
  - Structured project-store errors and recoverable missing-material warnings
  - Semantic round-trip integration tests for project bundles
affects: [material-import, bindings-node, generated-contracts, desktop-open-save]
tech-stack:
  added: [draft_model local dependency, serde_json, thiserror, tempfile dev-dependency]
  patterns: [platform-filesystem-boundary, semantic-round-trip-tests, centralized-material-uri-classification]
key-files:
  created:
    - crates/project_store/src/bundle.rs
    - crates/project_store/src/error.rs
    - crates/project_store/src/paths.rs
    - crates/project_store/tests/project_bundle.rs
  modified:
    - Cargo.lock
    - crates/project_store/Cargo.toml
    - crates/project_store/src/lib.rs
    - .planning/STATE.md
    - .planning/ROADMAP.md
    - .planning/REQUIREMENTS.md
key-decisions:
  - "`project_store` persists only `.veproj/project.json` and validates all drafts through `draft_model` before save/open."
  - "Material URI classification is centralized in `project_store`, with traversal rejected and missing files reported as recoverable warnings."
  - "Material import, ffprobe probing, ID assignment, and registry mutation remain out of `project_store` for later Plan 02-04 service work."
patterns-established:
  - "Bundle APIs return `ProjectBundle` / `ProjectBundleOpenResult` with warnings separate from hard load errors."
  - "Save/open tests compare `Draft` semantic equality rather than raw JSON bytes."
requirements-completed: [DRAFT-01, DRAFT-02, DRAFT-04, DRAFT-05]
duration: 10min
completed: 2026-06-17
---

# Phase 02 Plan 02: Project Bundle Persistence Summary

**Validated `.veproj/project.json` persistence with centralized material URI handling and semantic save/open/autosave round trips.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-17T02:10:15Z
- **Completed:** 2026-06-17T02:20:29Z
- **Tasks:** 2
- **Files modified:** 7 code/test files plus planning metadata

## Accomplishments

- Added `project_store` bundle APIs: `create_project_bundle`, `save_project_bundle`, `open_project_bundle`, and `autosave_project_bundle`.
- Added structured `ProjectStoreError` variants and `ProjectStoreWarning::MissingMaterial` for recoverable missing media paths.
- Added `project_json_path`, `classify_material_uri`, `resolve_material_uri`, and `material_uri_for_save` helpers with traversal rejection.
- Added integration tests for create/save/open/autosave, semantic equality, malformed JSON, unknown fields, unsupported schema versions, derived artifacts, and missing-material preservation.

## Task Commits

1. **Task 1: Implement bundle persistence and path resolution** - `b055e58` (feat)
2. **Task 2: Add save/open/autosave semantic round-trip tests** - `d41fe2d` (test)

**Plan metadata:** committed with this summary.

## Files Created/Modified

- `crates/project_store/src/bundle.rs` - Project bundle create/open/save/autosave implementation.
- `crates/project_store/src/error.rs` - Project-store errors and recoverable warning diagnostics.
- `crates/project_store/src/paths.rs` - `.veproj/project.json` and material URI classification/resolution helpers.
- `crates/project_store/tests/project_bundle.rs` - Integration tests for project bundle behavior.
- `crates/project_store/src/lib.rs` - Public API exports for bundle, error, and path modules.
- `crates/project_store/Cargo.toml` and `Cargo.lock` - Local `draft_model` dependency plus approved serialization/error/test dependencies.
- `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md` - Plan progress and completed requirement tracking.

## Decisions Made

- Kept `project_store` as a filesystem persistence boundary only; it does not call FFmpeg/ffprobe, assign material IDs, or mutate the material registry.
- Treated missing material files as warnings on open, preserving draft entries without corrupting or deleting semantics.
- Used existing approved Rust crates already present elsewhere in the workspace instead of introducing a new package family.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `gsd-tools` was not available on the shell PATH, so planning state was updated directly in the tracked markdown files and committed normally.

## Known Stubs

None.

## Authentication Gates

None.

## Threat Flags

None - this plan implemented the filesystem and project-store trust boundary already covered by the plan threat model and introduced no network endpoints, auth paths, or process execution.

## Verification

- `cargo test -p project_store create_project_bundle -- --nocapture` - passed.
- `cargo test -p project_store path_resolution -- --nocapture` - passed.
- `cargo test -p project_store round_trip -- --nocapture` - passed.
- `cargo test -p project_store -- --nocapture` - passed.
- `grep -R "FfmpegExecutor\|probe_material_metadata\|ffprobe\|ffmpeg" crates/project_store/src && exit 1 || true` - passed.

## Self-Check: PASSED

- Found all created/modified code and test files on disk.
- Found task commits `b055e58` and `d41fe2d` in git history.
- Stub scan found no placeholder/TODO/FIXME or hardcoded empty UI-data stubs in changed project-store files.
- No unexpected tracked file deletions were introduced.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 02-03 can add media-runtime probing and generated media fixtures without changing project persistence ownership. Plan 02-04 can consume the bundle APIs while keeping material import orchestration outside `project_store`.

---
*Phase: 02-draft-and-material-system*
*Completed: 2026-06-17*
