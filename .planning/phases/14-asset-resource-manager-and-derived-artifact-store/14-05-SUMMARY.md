---
phase: 14-asset-resource-manager-and-derived-artifact-store
plan: "05"
subsystem: artifact-store
tags: [rust, sqlite, gc, quota, sync-manifest, derived-artifacts]

requires:
  - phase: 14-03
    provides: exact artifact dependency invalidation and dirty artifact rows
  - phase: 14-04
    provides: generation job/chunk lifecycle and BlobStore-backed derived artifacts
provides:
  - DB-driven mark-and-sweep GC for unreferenced derived blobs with dry-run/apply modes
  - Tombstone records with relative blob path, fingerprint, byte count, reason, and timestamp
  - Rust-owned quota accounting and UI-safe display labels
  - Deterministic local sync manifest entries, tombstones, dependency summaries, and fingerprints
affects: [phase-14, phase-15, phase-16, bindings-node, desktop-ui, server-sync]

tech-stack:
  added: []
  patterns:
    - DB-driven maintenance APIs over artifact/dependency/job/tombstone rows
    - Derived blob deletion gated by validate_derived_relative_path and blobs/ containment
    - UI-safe aggregate labels generated in Rust
    - Deterministic local-only manifest serialization with BLAKE3 fingerprints

key-files:
  created:
    - crates/artifact_store/src/gc.rs
    - crates/artifact_store/src/quota.rs
    - crates/artifact_store/src/manifest.rs
    - crates/artifact_store/tests/gc_quota_manifest.rs
  modified:
    - crates/artifact_store/src/lib.rs
    - crates/artifact_store/src/schema.rs

key-decisions:
  - "GC candidates are selected from SQLite artifact rows and excluded by live artifact status, dependency rows, active generation jobs/chunks, and existing tombstones before any file deletion."
  - "Quota labels are Rust-owned strings for UI transport; TypeScript does not compute totals from paths or inspect SQLite/blob internals."
  - "Sync manifests are local deterministic contracts only; remote/cloud provider transport, auth, URLs, upload/download, and server rendering stay out of Phase 14."

patterns-established:
  - "Maintenance APIs preserve source media, project.json, SQLite DB/WAL/SHM, external material paths, ready artifacts, dependency-live dirty artifacts, and active job outputs."
  - "Manifest generation validates derived-relative blob refs and sorts entries/tombstones/dependencies before fingerprinting."

requirements-completed: [ASSET-05]

duration: 8 min
completed: 2026-06-19
---

# Phase 14 Plan 05: GC, Quota, And Local Sync Manifest Summary

**Safe local derived-artifact maintenance with DB-driven GC, Rust-owned quota labels, tombstones, and deterministic project-relative sync manifests.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-06-19T05:32:55Z
- **Completed:** 2026-06-19T05:41:20Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- Added `gc.rs` with dry-run/apply mark-and-sweep GC, tombstone writes, path containment validation, and temp blob sweeping.
- Added `quota.rs` with artifact-row/tombstone/GC-candidate accounting, quota severity, cleanup availability, and safe Chinese UI labels.
- Added `manifest.rs` with deterministic local manifest generation, BLAKE3 manifest fingerprints, dependency summaries, tombstones, and BlobStore-backed manifest artifact writes.
- Added `gc_quota_manifest.rs` covering live preservation, unsafe deletion failure-closed behavior, quota labels, cleanup completion, deterministic manifests, and local-only manifest constraints.

## Task Commits

1. **Task 14-05-01 RED: GC safety tests** - `ad3749c` (test)
2. **Task 14-05-01 GREEN: safe derived artifact GC** - `2322c6b` (feat)
3. **Task 14-05-02 RED: quota accounting tests** - `02334f0` (test)
4. **Task 14-05-02 GREEN: DB-driven quota state** - `7dfddf0` (feat)
5. **Task 14-05-03 RED: sync manifest tests** - `99cb0e3` (test)
6. **Task 14-05-03 GREEN: deterministic local sync manifests** - `95fb49a` (feat)

## Files Created/Modified

- `crates/artifact_store/src/gc.rs` - GC planning/apply mode, tombstones, safe derived blob deletion, and temp sweep.
- `crates/artifact_store/src/quota.rs` - Quota policy, snapshot, severity, cleanup availability, and safe display labels.
- `crates/artifact_store/src/manifest.rs` - Local sync manifest structs, deterministic generation, fingerprinting, and BlobStore write path.
- `crates/artifact_store/tests/gc_quota_manifest.rs` - Focused ASSET-05 integration coverage.
- `crates/artifact_store/src/lib.rs` - Exported `gc`, `quota`, and `manifest` modules.
- `crates/artifact_store/src/schema.rs` - Added tombstone `byte_count` support for new and existing SQLite stores.

## Decisions Made

- GC does not infer deletions from filesystem scans; SQLite rows choose candidates, and filesystem validation is only the final containment gate.
- Dirty artifacts with dependency rows remain live; unreferenced dirty/failed/tombstoned derived blobs are reclaimable.
- Quota snapshots intentionally report `sourceMediaBytes` and `untrackedBlobBytes` as zero because Phase 14 quota is DB-driven and must not scan source or untracked paths.
- Manifest output omits timestamps and transport/provider fields so equal stores serialize byte-identically.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added tombstone byte count schema support**
- **Found during:** Task 14-05-01 (DB-driven GC with tombstones)
- **Issue:** Existing `artifact_tombstone` schema had path/fingerprint/reason/timestamp but no byte count, while ASSET-05 requires tombstones to preserve byte count for quota and manifest generation.
- **Fix:** Added `byte_count` to the create-table schema and a compatibility migration helper that adds the column to existing local stores if missing.
- **Files modified:** `crates/artifact_store/src/schema.rs`
- **Verification:** `cargo test -p artifact_store gc_quota_manifest -- --nocapture`
- **Committed in:** `2322c6b`

---

**Total deviations:** 1 auto-fixed (1 missing critical)
**Impact on plan:** Required for correctness and ASSET-05 acceptance; no scope creep beyond local artifact-store schema compatibility.

## Known Stubs

None.

## Issues Encountered

None.

## Verification

- `cargo test -p artifact_store gc_quota_manifest -- --nocapture` - PASSED
- `pnpm run test:phase14-source-guards` - PASSED

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

ASSET-05 is ready for later binding/UI exposure and future server/mobile consumers. GC, quota, tombstone, and manifest semantics are Rust-owned, project-relative, local-only, and source-media safe.

## Self-Check: PASSED

- Created files exist: `gc.rs`, `quota.rs`, `manifest.rs`, `gc_quota_manifest.rs`, and this summary.
- Task commits found: `ad3749c`, `2322c6b`, `02334f0`, `7dfddf0`, `99cb0e3`, `95fb49a`.
- Required verification commands passed after implementation.
- Stub scan found no TODO/FIXME/placeholder/empty UI data stubs in files created or modified by this plan.

---
*Phase: 14-asset-resource-manager-and-derived-artifact-store*
*Completed: 2026-06-19*
