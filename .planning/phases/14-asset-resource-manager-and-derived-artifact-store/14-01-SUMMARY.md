---
phase: 14-asset-resource-manager-and-derived-artifact-store
plan: "01"
subsystem: storage
tags: [rust, sqlite, rusqlite, blake3, artifact-store, blob-store]

requires:
  - phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence
    provides: Rust-owned dirty ranges, graph node keys, and fingerprint facts for downstream artifact persistence
provides:
  - Rust-owned `artifact_store` crate with `.veproj/derived/artifact-store.sqlite` schema foundation
  - Project-contained derived blob path validation, BLAKE3 fingerprints, atomic blob writes, and repair behavior
  - Initial Phase 14 source guards and package scripts
affects: [phase-14, asset-resource-manager, derived-artifact-store, preview-cache, resource-generation]

tech-stack:
  added: [rusqlite 0.40.1, blake3 1.8.5, fs2 0.4.3]
  patterns:
    - "SQLite store lives under `.veproj/derived` and never extends canonical `project.json` semantics"
    - "Blob rows become ready only after contained file writes and BLAKE3 verification succeed"

key-files:
  created:
    - crates/artifact_store/Cargo.toml
    - crates/artifact_store/src/blob_store.rs
    - crates/artifact_store/src/error.rs
    - crates/artifact_store/src/fingerprint.rs
    - crates/artifact_store/src/lib.rs
    - crates/artifact_store/src/paths.rs
    - crates/artifact_store/src/schema.rs
    - crates/artifact_store/tests/blob_store.rs
    - crates/artifact_store/tests/sqlite_schema.rs
    - scripts/phase14-source-guards.sh
  modified:
    - Cargo.toml
    - Cargo.lock
    - package.json

key-decisions:
  - "Phase 14 artifact validity starts in a dedicated Rust `artifact_store` crate backed by `.veproj/derived/artifact-store.sqlite`; `.veproj/project.json` remains semantic-only."
  - "Blob references are stored relative to `.veproj/derived` and content-addressed with versioned BLAKE3 fingerprints."
  - "Initial Phase 14 guards allow existing preview artifact display fields but reject renderer-owned artifact roots, SQLite, fingerprints, cache keys, dirty ranges, graph IDs, invalidation, and FFmpeg command construction."

patterns-established:
  - "Connection opener pattern: create `.veproj/derived`, open SQLite, apply foreign keys/WAL/busy timeout, then run deterministic migrations."
  - "Blob write pattern: temp write under `derived/blobs/tmp`, sync, atomic rename, verify fingerprint/byte count, then commit ready row."
  - "Repair pattern: DB rows drive ready-blob validation and demotion; temp cleanup does not delete live ready blobs."

requirements-completed: [ASSET-02]

duration: 7 min
completed: 2026-06-19
---

# Phase 14 Plan 01: Artifact Store Foundation Summary

**Rust-owned SQLite artifact index and project-contained BLAKE3 blob store under `.veproj/derived`**

## Performance

- **Duration:** 7 min
- **Started:** 2026-06-19T04:46:54Z
- **Completed:** 2026-06-19T04:54:10Z
- **Tasks:** 3
- **Files modified:** 13

## Accomplishments

- Added the `artifact_store` workspace crate with SQLite schema/migrations for `store_metadata`, `resource`, `artifact`, dependencies, generation jobs/chunks, quota, tombstones, and sync manifest entries.
- Implemented derived path containment, symlink escape rejection, versioned BLAKE3 byte/file fingerprints, atomic content-addressed blob writes, ready-row commits, and repair demotion for missing blobs.
- Added `test:phase14-rust`, `test:phase14-source-guards`, and `test:phase14` plus an initial renderer/source ownership guard for Phase 14.

## Task Commits

1. **Task 14-01-01 RED: schema behavior tests** - `2740320` (test)
2. **Task 14-01-01 GREEN: SQLite schema boundary** - `b6923e4` (feat)
3. **Task 14-01-02 RED: blob store behavior tests** - `486dd99` (test)
4. **Task 14-01-02 GREEN: blob store foundation** - `aa32d02` (feat)
5. **Task 14-01-03: Phase 14 guards/scripts** - `2cac61f` (chore)

## Files Created/Modified

- `crates/artifact_store/src/schema.rs` - SQLite opener, PRAGMAs, schema version, and initial ASSET-02 tables.
- `crates/artifact_store/src/paths.rs` - `.veproj/derived` path helpers and relative-path containment validation.
- `crates/artifact_store/src/fingerprint.rs` - versioned BLAKE3 byte/file fingerprint helpers.
- `crates/artifact_store/src/blob_store.rs` - atomic blob writes, ready row commits, verification, and repair.
- `crates/artifact_store/tests/sqlite_schema.rs` - PRAGMA, migration, FK, and project JSON separation tests.
- `crates/artifact_store/tests/blob_store.rs` - traversal, symlink, atomic write, mismatch, temp cleanup, and repair tests.
- `scripts/phase14-source-guards.sh` - initial Phase 14 renderer/source ownership guard.
- `package.json` - Phase 14 focused test scripts.
- `Cargo.toml` / `Cargo.lock` - workspace membership and approved dependency lock entries.

## Verification

- `cargo test -p artifact_store sqlite_schema -- --nocapture` - PASS
- `cargo test -p artifact_store blob_store -- --nocapture` - PASS
- `pnpm run test:phase14-source-guards` - PASS
- `pnpm run test:phase14-rust` - PASS
- `pnpm run test:phase14` - PASS

## Decisions Made

- Used a single new `artifact_store` crate for the storage foundation, matching the Phase 14 context decision to defer a separate asset-resource-manager crate.
- Kept artifact/blob paths relative to `.veproj/derived`, so binding/UI layers can display refs without owning roots or absolute filesystem semantics.
- Kept `pnpm run test:phase14` limited to Plan 14-01 tests and guards; generated contract checks remain deferred until binding-visible contracts are introduced.

## Deviations from Plan

None - plan executed exactly as written.

**Total deviations:** 0 auto-fixed.
**Impact on plan:** No scope expansion.

## Issues Encountered

- The first version of `scripts/phase14-source-guards.sh` overmatched existing renderer display field `frameArtifactPath`. The guard was narrowed to root/SQLite/blob-root ownership terms while still rejecting injected `.veproj/derived` artifact-root computation.

## Known Stubs

None.

## Threat Flags

None - new SQLite/file/blob surfaces were covered by the plan threat model T-14-01 through T-14-04.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 14-02 can build resource indexing on top of the `artifact_store` schema, derived path helpers, fingerprint helpers, and BlobStore commit/repair behavior.

## Self-Check: PASSED

- Created files exist on disk.
- Task commits `2740320`, `b6923e4`, `486dd99`, `aa32d02`, and `2cac61f` exist in git history.
- Final verification commands passed.

---
*Phase: 14-asset-resource-manager-and-derived-artifact-store*
*Completed: 2026-06-19*
