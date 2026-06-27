---
phase: 02-draft-and-material-system
plan: 01
subsystem: draft-model
tags: [rust, serde, schemars, ts-rs, draft-schema, validation]
requires:
  - phase: 01-foundation-and-golden-harness
    provides: Rust workspace, pure semantic crate boundaries, generated contract test pattern
provides:
  - Pure `draft_model` Draft, Material, Track, Segment, ID, timerange, keyframe, filter, and transition types
  - Schema version 1 validation and migration entrypoint
  - Strict draft schema tests for Jianying-aligned terminology and derived-artifact exclusion
affects: [project_store, material-import, generated-contracts, bindings-node]
tech-stack:
  added: []
  patterns: [strict-serde-models, deterministic-string-ids, integer-microsecond-time, structured-migration-errors]
key-files:
  created:
    - crates/draft_model/src/draft.rs
    - crates/draft_model/src/ids.rs
    - crates/draft_model/src/material.rs
    - crates/draft_model/src/time.rs
    - crates/draft_model/src/timeline.rs
    - crates/draft_model/src/validation.rs
    - crates/draft_model/tests/draft_schema.rs
  modified:
    - crates/draft_model/src/lib.rs
    - .planning/STATE.md
    - .planning/ROADMAP.md
    - .planning/REQUIREMENTS.md
key-decisions:
  - "Draft schema, validation, and migration hooks live in pure `draft_model`; project persistence remains a later `project_store` concern."
  - "Phase 2 Plan 01 uses deterministic caller-supplied string ID newtypes instead of adding a UUID dependency."
  - "Persisted draft time values use integer microsecond wrappers and rational frame rates, with no floating seconds fields."
patterns-established:
  - "Strict persisted structs derive serde/schemars/ts-rs and use camelCase JSON with deny_unknown_fields."
  - "Migration prechecks schemaVersion and derived top-level fields before deserializing into Draft."
requirements-completed: [DRAFT-03, DRAFT-04, DRAFT-05]
duration: 9min
completed: 2026-06-17
---

# Phase 02 Plan 01: Draft Model Schema Summary

**Pure Rust draft/material/timeline schema with deterministic IDs, integer microsecond time, and structured versioned migration.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-17T02:01:02Z
- **Completed:** 2026-06-17T02:09:35Z
- **Tasks:** 2
- **Files modified:** 8 code/test files plus planning metadata

## Accomplishments

- Added Jianying-aligned semantic draft types: `Draft`, `Material`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, `Keyframe`, `Filter`, and `Transition`.
- Added deterministic ID newtypes and integer `Microseconds`/`RationalFrameRate` persisted time structures.
- Added `DraftValidationError`, `validate_draft`, and `migrate_draft_json` with structured errors for version, required fields, timeranges, frame rates, duplicates, and derived artifact leakage.
- Added schema tests covering valid empty drafts, material/track/segment records, strict unknown-field rejection, derived-artifact exclusion, version migration, and validation failures.

## Task Commits

1. **Task 1: Define pure draft/material/timeline schema types** - `a6cb1ca` (feat)
2. **Task 2: Add validation and migration hooks** - `0dcf46b` (feat)

**Plan metadata:** committed with this summary.

## Files Created/Modified

- `crates/draft_model/src/draft.rs` - Draft root, metadata, schema version, and `Draft::new`.
- `crates/draft_model/src/ids.rs` - Deterministic caller-supplied string ID newtypes.
- `crates/draft_model/src/time.rs` - Integer microsecond wrapper.
- `crates/draft_model/src/material.rs` - Material kind/status/metadata and rational frame rate.
- `crates/draft_model/src/timeline.rs` - Track, segment, timerange, magnet, keyframe, filter, and transition shells.
- `crates/draft_model/src/validation.rs` - Structured validation and migration API.
- `crates/draft_model/tests/draft_schema.rs` - Draft schema and migration test coverage.
- `crates/draft_model/src/lib.rs` - Public module exports.

## Decisions Made

- Used caller-supplied deterministic string IDs and did not add a new external ID dependency.
- Kept migration/validation in `draft_model`, with no filesystem, process, FFmpeg, or Electron imports.
- Treated derived artifact keys as invalid draft JSON at migration boundaries.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `ts-rs` 12 warns on unsupported `serde(transparent)` parsing. The newtype JSON shape works without that attribute, so the unsupported attributes were removed and tests rerun cleanly.
- Some GSD state helper invocations require summary-backed context or different positional arguments. STATE/ROADMAP/REQUIREMENTS were updated narrowly and verified in the metadata diff.

## Known Stubs

None.

## Authentication Gates

None.

## Verification

- `cargo test -p draft_model draft_schema -- --nocapture` - passed.
- `cargo test -p draft_model migration -- --nocapture` - passed.
- `cargo test -p draft_model -- --nocapture` - passed.
- `grep -R "FfmpegExecutor\|PlatformFileSystem\|std::fs\|std::process" crates/draft_model/src && exit 1 || true` - passed.
- `grep -R "durationSeconds\|seconds: f32\|seconds: f64\|Asset\|Clip" crates/draft_model/src crates/draft_model/tests | grep -v "must not" && exit 1 || true` - passed.

## Self-Check: PASSED

- Found all created code/test files on disk.
- Found task commits `a6cb1ca` and `0dcf46b` in git history.
- Stub scan found no blocking stubs; `ts(optional = nullable)` matches are codegen annotations, not runtime placeholders.
- No threat flags: this plan introduced no network endpoints, auth paths, filesystem access, process execution, or trust-boundary expansion beyond strict JSON migration/validation already covered by the plan threat model.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`project_store` can now consume the canonical `Draft` type plus `migrate_draft_json` and `validate_draft` for `.veproj/project.json` create/open/save behavior in Plan 02-02.

---
*Phase: 02-draft-and-material-system*
*Completed: 2026-06-17*
