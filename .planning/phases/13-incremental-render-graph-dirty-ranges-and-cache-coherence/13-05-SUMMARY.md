---
phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence
plan: 05
subsystem: preview-cache
tags: [rust, preview_service, render_graph, dirty-ranges, cache-coherence]

requires:
  - phase: 13-03
    provides: CommandDelta, DirtyRange, DirtyDomain, and consumer-domain facts
  - phase: 13-04
    provides: stable render graph node IDs, fingerprints, and snapshots
provides:
  - PreviewCacheKey v2 with graph node keys and fingerprint dimensions
  - PreviewInvalidationRequest v2 with dirty ranges, domains, graph nodes, runtime/profile facts, and full-draft fallback
  - Rust-owned conversion from CommandDelta to preview/export dirty facts
  - ExportPrepDirtyFacts classification data for downstream export preparation
affects: [preview_service, bindings_node, render_graph, phase-14-artifact-store, phase-16-scheduler]

tech-stack:
  added: []
  patterns:
    - Rust-owned graph/fingerprint cache-key derivation
    - Domain-aware preview/export dirty fact conversion
    - Legacy cache entries retained for range/material invalidation but invalidated for missing v2 graph/runtime/profile facts

key-files:
  created:
    - .planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-05-SUMMARY.md
  modified:
    - crates/preview_service/src/cache.rs
    - crates/preview_service/src/lib.rs
    - crates/preview_service/src/service.rs
    - crates/preview_service/tests/cache_invalidation.rs
    - crates/preview_service/tests/dirty_propagation.rs
    - crates/bindings_node/src/preview_export_service.rs

key-decisions:
  - "Preview cache keys are derived from RenderGraphSnapshot node fingerprints in Rust, not renderer-supplied IDs or hashes."
  - "Legacy/v1 cache entries remain targetable by range/material invalidation, but are stale-unsafe for v2 graph/runtime/output-profile requests and are invalidated."
  - "Export preparation receives dirty classification facts only; this plan adds no scheduler, SQLite artifact store, or file-persistence layer."

patterns-established:
  - "PreviewInvalidationRequest::from_command_delta expands semantic dirty domains into preview/export consumer domains."
  - "ExportPrepDirtyFacts mirrors preview invalidation facts for later export preparation without changing binding transport schemas."

requirements-completed: [INCR-01, INCR-03, INCR-04]

duration: 25min
completed: 2026-06-19T01:27:57Z
---

# Phase 13 Plan 05: Preview/Export Cache Coherence Summary

**Fingerprint-aware preview cache keys and domain-aware preview/export invalidation facts backed by Rust render graph snapshots**

## Performance

- **Duration:** 25 min
- **Started:** 2026-06-19T01:03:00Z
- **Completed:** 2026-06-19T01:27:57Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `PreviewCacheKey` v2 fields for graph node keys, semantic/input/output/runtime fingerprints, schema version, and generator version.
- Updated preview preparation to derive cache keys from Rust-built `RenderGraphSnapshot` node fingerprints and output profiles.
- Added `PreviewInvalidationRequest` v2 with dirty ranges, changed materials, changed graph node keys, changed domains, runtime/output profile fingerprints, full-draft fallback, and `CommandDelta` conversion.
- Added `ExportPrepDirtyFacts` as classification data only, mirroring preview invalidation facts without adding scheduler queues, SQLite artifact persistence, or file IO artifact state.
- Kept the existing binding invalidation payload compatible by adapting old `changedRanges` into Rust-owned `DirtyRange` values inside the binding layer.

## Task Commits

1. **Task 13-05-01/02 RED tests:** `68b2192` (`test`) added failing tests for cache key v2, invalidation v2, and export-prep dirty facts.
2. **Task 13-05-01 GREEN:** `a2a2c15` (`feat`) implemented graph/fingerprint-aware preview cache keys and Rust snapshot-based preview key derivation.
3. **Task 13-05-02 GREEN fix:** `c79ada2` (`fix`) corrected legacy stale-safety behavior for targeted invalidation with v2 facts.

## Files Created/Modified

- `crates/preview_service/src/cache.rs` - Preview cache key v2, invalidation request v2, `CommandDelta` conversion, export-prep dirty facts, and invalidation predicates.
- `crates/preview_service/src/service.rs` - Preview preparation now derives cache keys from `RenderGraphSnapshot` node fingerprints and output profile/runtime facts.
- `crates/preview_service/src/lib.rs` - Re-exported cache v2 constants and `ExportPrepDirtyFacts`.
- `crates/preview_service/tests/cache_invalidation.rs` - Added cache key v2 and invalidation v2 coverage.
- `crates/preview_service/tests/dirty_propagation.rs` - Added export-prep dirty fact and domain-aware invalidation coverage.
- `crates/bindings_node/src/preview_export_service.rs` - Adapted existing binding invalidation payloads into Rust-owned v2 invalidation requests without changing transport schema.

## Decisions Made

- `PreviewCacheKey` stores graph node stable keys as strings rather than asking transport callers to provide structured graph node objects.
- V2 key IDs are deterministic fingerprints over profile, target range, node keys, semantic/input/output/runtime fingerprints, material dependencies, artifact schema, and generator version.
- Legacy entries with no v2 facts can still be invalidated precisely by old range/material requests; they are force-invalidated when a v2 graph node, runtime, or output profile fact is needed for stale safety.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected over-broad legacy cache invalidation**
- **Found during:** Task 13-05-02 verification
- **Issue:** Treating `changedDomains` alone as requiring v2 key facts caused legacy range/material entries to be blanket-invalidated.
- **Fix:** Limited legacy stale-safety fallback to graph node, runtime capability, output profile, or full-draft v2 facts.
- **Files modified:** `crates/preview_service/src/cache.rs`
- **Verification:** `cargo test -p preview_service --test dirty_propagation -- --nocapture`; `cargo test -p preview_service --test cache_invalidation invalidation_v2 -- --nocapture`
- **Committed in:** `c79ada2`

---

**Total deviations:** 1 auto-fixed bug.
**Impact on plan:** The fix preserves correctness and avoids unnecessary cache churn while retaining stale safety for v2 facts.

## Issues Encountered

- `gsd-tools` was unavailable in this shell, so plan execution state was handled manually through commits and this summary.
- `cargo fmt` initially formatted unrelated files; those formatting-only side effects were reverted before commits. No unrelated files were staged.
- Per orchestrator instruction, `.planning/STATE.md` and `.planning/ROADMAP.md` were not updated.

## Verification

- `cargo test -p preview_service --test cache_invalidation -- --nocapture` - passed
- `cargo test -p preview_service --test dirty_propagation -- --nocapture` - passed
- `cargo test -p preview_service preview_generation -- --nocapture` - passed
- `pnpm run test:phase13-source-guards` - passed
- `git diff --check` - passed
- `cargo check --workspace --locked` - passed

## Known Stubs

None.

## Threat Flags

None. The binding touch is an internal compatibility adapter for an existing command payload and does not add a new transport schema, network endpoint, artifact persistence layer, scheduler, or FFmpeg command surface.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Preview/export dirty classification facts are now available for later binding transport expansion and artifact-store persistence. Phase 13-05B can expose binding-safe transport without renderer-owned graph/cache/fingerprint computation.

## Self-Check: PASSED

- Found summary file and all key modified files.
- Found task commits: `68b2192`, `a2a2c15`, `c79ada2`.

---
*Phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence*
*Completed: 2026-06-19*
