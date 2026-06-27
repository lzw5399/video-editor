---
phase: 14-asset-resource-manager-and-derived-artifact-store
plan: "03"
subsystem: storage
tags: [rust, sqlite, invalidation, artifact-store, command-delta]

requires:
  - phase: 14-asset-resource-manager-and-derived-artifact-store
    provides: SQLite artifact/resource/dependency rows from Plans 14-01 and 14-02
  - phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence
    provides: Rust-owned CommandDelta, DirtyRange, DirtyDomain, graph node stable keys, and fingerprint facts
provides:
  - Dependency-driven artifact dirty marking for source replacement, relink, rename, and delete
  - Rust-owned artifact invalidation from Phase 13 CommandDelta dirty facts
  - Explicit full-draft fallback records for unknown dependency and checked integer range overflow cases
affects: [phase-14, artifact-store, resource-manager, preview-cache, generation-planning, scheduler-planning]

tech-stack:
  added: []
  patterns:
    - "Artifact invalidation queries SQLite dependency rows before marking derived rows dirty."
    - "Full-draft invalidation is an explicit fallback result, not the normal path for known dependencies."
    - "CommandDelta dirty facts are consumed in Rust without renderer-side invalidation decisions."

key-files:
  created:
    - crates/artifact_store/src/invalidation.rs
    - crates/artifact_store/tests/invalidation.rs
  modified:
    - crates/artifact_store/src/lib.rs
    - crates/artifact_store/src/schema.rs

key-decisions:
  - "Dirty artifact rows record audit-safe dirty reasons and source change kind fields on the artifact row."
  - "Source delete tombstones dependent artifact rows for audit/status instead of deleting source media or derived blobs."
  - "Dirty domain rows filter localized range invalidation and do not independently dirty every artifact in the same domain when a tighter range/material/graph key exists."

patterns-established:
  - "Source-change invalidation pattern: update resource refs when needed, collect dependency-matched artifact IDs, then mark rows dirty/tombstoned in one transaction."
  - "CommandDelta invalidation pattern: convert Rust delta facts into ArtifactInvalidationRequest and match material/resource/graph/domain/range dependency rows."
  - "Fingerprint mismatch pattern: compare new fingerprint facts against recorded dependency fingerprints and emit audit-safe reason strings without raw old fingerprint disclosure."

requirements-completed: [ASSET-03]

duration: 8 min
completed: 2026-06-19
---

# Phase 14 Plan 03: Exact Artifact Invalidation Summary

**Dependency-row artifact invalidation for source changes, CommandDelta dirty facts, fingerprints, and fail-closed fallback cases**

## Performance

- **Duration:** 8 min
- **Started:** 2026-06-19T05:07:48Z
- **Completed:** 2026-06-19T05:15:17Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added `artifact_store::invalidation` with `SourceChange`, `ArtifactInvalidationRequest`, dirty result rows, fallback records, and exact invalidation entry points.
- Source replacement/relink/rename/delete now dirty or tombstone only dependency-matched artifact rows unless stable dependency facts are missing.
- Phase 13 `CommandDelta` facts now drive artifact dirty state through Rust-owned material IDs, graph node keys, dirty domains, integer ranges, and fingerprint mismatch checks.
- Checked integer range overflow records a `RangeOverflow` full-draft fallback instead of wrapping or retaining stale artifacts.

## Task Commits

1. **Task 14-03-01 RED: source invalidation tests** - `d328736` (test)
2. **Task 14-03-01 GREEN: source invalidation implementation** - `3f84235` (feat)
3. **Task 14-03-02 RED: dirty fact invalidation tests** - `70fef66` (test)
4. **Task 14-03-02 GREEN: CommandDelta/fingerprint invalidation** - `42673a5` (feat)

## Files Created/Modified

- `crates/artifact_store/src/invalidation.rs` - Rust-owned source, dependency, CommandDelta, fingerprint, and fallback invalidation APIs.
- `crates/artifact_store/tests/invalidation.rs` - Focused ASSET-03 TDD coverage for exact dirtying, tombstones, fingerprint mismatch, and overflow fallback.
- `crates/artifact_store/src/schema.rs` - Added dirty reason/source-change audit fields to artifact rows.
- `crates/artifact_store/src/lib.rs` - Exported the invalidation module.

## Verification

- `cargo test -p artifact_store invalidation -- --nocapture` - PASS, 7 invalidation tests plus dependency-filtered resource tests.
- `pnpm run test:phase14-source-guards` - PASS.
- `rg -n "remove_file|remove_dir|DELETE FROM artifact|DELETE FROM resource|unlink|std::fs::remove" crates/artifact_store/src/invalidation.rs crates/artifact_store/tests/invalidation.rs` - PASS, no source/blob deletion logic in invalidation.
- `rg -n "TODO|FIXME|placeholder|coming soon|not available|= \\[\\]|= \\{\\}|= null|= \\\"\\\"" crates/artifact_store/src/invalidation.rs crates/artifact_store/tests/invalidation.rs crates/artifact_store/src/schema.rs crates/artifact_store/src/lib.rs` - PASS, no known stubs.

## Decisions Made

- Dirty reasons are stored as audit-safe strings such as `sourceChange:replaced:...`, `dependencyMatch:...`, and `fingerprintMismatch:...`; old raw fingerprints are not exposed in result reasons by default.
- Source rename/relink updates `resource` row refs and source fingerprint facts before dirty marking, while canonical `.veproj/project.json` remains untouched.
- Domain-only dependency matching is used only when no tighter material/resource/graph/range target exists; localized dirty ranges must retain unrelated artifacts in the same consumer domain.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Prevented dirty-domain over-invalidation for localized ranges**
- **Found during:** Task 14-03-02
- **Issue:** The first GREEN implementation treated `DirtyDomain` dependency rows as independent global matches, so an unrelated artifact in the same consumer domain was marked dirty even when its integer target range did not overlap.
- **Fix:** Changed dependency matching so dirty domains filter localized range matches when tighter facts exist; direct domain-only dirtying is kept for pure domain requests.
- **Files modified:** `crates/artifact_store/src/invalidation.rs`
- **Verification:** `cargo test -p artifact_store invalidation -- --nocapture`
- **Committed in:** `42673a5`

---

**Total deviations:** 1 auto-fixed (1 bug).
**Impact on plan:** The fix tightened invalidation precision and is required for ASSET-03 correctness.

## Issues Encountered

- TDD RED phases failed as expected on missing invalidation APIs before implementation.
- No authentication gates or package installation were encountered.

## TDD Gate Compliance

- RED commits exist before implementation: `d328736`, `70fef66`.
- GREEN commits exist after RED: `3f84235`, `42673a5`.
- No separate REFACTOR commit was needed.

## Known Stubs

None.

## Threat Flags

None - the new invalidation surface is covered by the plan threat model T-14-08, T-14-09, and T-14-10.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 14-04 can build generation job/chunk state on top of persistent dirty artifact rows and explicit fallback records without moving invalidation, graph, cache, or SQLite decisions into Electron.

## Self-Check: PASSED

- Created files exist on disk: `crates/artifact_store/src/invalidation.rs` and `crates/artifact_store/tests/invalidation.rs`.
- Task commits `d328736`, `3f84235`, `70fef66`, and `42673a5` exist in git history.
- Required verification commands passed.
- Worktree has no uncommitted production plan changes; only the untouched untracked `reference/` directory remains.

---
*Phase: 14-asset-resource-manager-and-derived-artifact-store*
*Completed: 2026-06-19*
