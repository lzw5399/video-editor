---
phase: 14-asset-resource-manager-and-derived-artifact-store
plan: "02"
subsystem: storage
tags: [rust, sqlite, resource-index, dependencies, artifact-store]

requires:
  - phase: 14-asset-resource-manager-and-derived-artifact-store
    provides: `artifact_store` SQLite schema, blob path helpers, and Phase 14 source guards from Plan 14-01
  - phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence
    provides: stable graph node keys, dirty domains, dirty ranges, and fingerprint contracts
provides:
  - Stable Rust-owned resource indexing for materials, fonts, filters, transitions, text effects, proxies, thumbnails, and waveforms
  - Typed artifact dependency row APIs for invalidation facts, fingerprints, checked ranges, schema versions, and generation parameters
  - Focused resource/dependency tests under `cargo test -p artifact_store resource_index -- --nocapture`
affects: [phase-14, asset-resource-manager, derived-artifact-store, preview-cache, invalidation, generation-planning]

tech-stack:
  added: []
  patterns:
    - "Resource rows derive from Rust Draft/material/timeline facts and are persisted only in `.veproj/derived/artifact-store.sqlite`."
    - "Dependency rows are validated before transaction writes so range overflow cannot leave partial invalidation facts."

key-files:
  created:
    - crates/artifact_store/src/resource_index.rs
    - crates/artifact_store/src/dependencies.rs
    - crates/artifact_store/tests/resource_index.rs
  modified:
    - Cargo.lock
    - crates/artifact_store/Cargo.toml
    - crates/artifact_store/src/error.rs
    - crates/artifact_store/src/lib.rs

key-decisions:
  - "Material resource IDs use `material:{MaterialId}` while derived roles keep parent material linkage for later generation and invalidation."
  - "Fonts, filters, transitions, and text effects are derived resource rows, not new canonical `Material` variants or draft fields."
  - "Dependency fingerprints are validity facts and graph node stable keys are identity facts."

patterns-established:
  - "Resource indexing pattern: read Draft facts, classify material refs through project_store, return an in-memory index, and persist `resource` rows."
  - "Dependency upsert pattern: validate all typed rows first, then replace an artifact's dependency rows in one SQLite transaction."

requirements-completed: [ASSET-01, ASSET-02]

duration: 7 min
completed: 2026-06-19
---

# Phase 14 Plan 02: Resource Index And Dependency Rows Summary

**Rust-owned resource identity and typed artifact dependency rows persisted in the derived SQLite store**

## Performance

- **Duration:** 7 min
- **Started:** 2026-06-19T04:58:00Z
- **Completed:** 2026-06-19T05:04:12Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added `resource_index.rs` with stable resource IDs for material, font, effect, filter, transition, proxy, thumbnail, waveform, graph snapshot, and preview artifact rows.
- Added `dependencies.rs` with typed dependency facts for material/resource IDs, graph node stable keys, dirty domains, target/source integer microsecond ranges, fingerprints, schema versions, generator versions, and generation parameters.
- Added focused TDD coverage proving resource rows stay out of canonical draft schema and dependency overflow rejects writes without partial rows.

## Task Commits

1. **Task 14-02-01 RED: resource index tests** - `c2e9841` (test)
2. **Task 14-02-01 GREEN: resource index rows** - `d1d43da` (feat)
3. **Task 14-02-02 RED: dependency row tests** - `f0d661b` (test)
4. **Task 14-02-02 GREEN: dependency row APIs** - `08c1266` (feat)

## Files Created/Modified

- `crates/artifact_store/src/resource_index.rs` - Rust-owned resource IDs, refs, status, indexing, and SQLite resource-row persistence.
- `crates/artifact_store/src/dependencies.rs` - Typed dependency rows, checked range normalization, transactional upsert, and query APIs.
- `crates/artifact_store/tests/resource_index.rs` - Resource and dependency behavior tests executed by the required `resource_index` filter.
- `crates/artifact_store/src/error.rs` - Typed resource/dependency/range errors.
- `crates/artifact_store/src/lib.rs` - Public module exports.
- `crates/artifact_store/Cargo.toml` / `Cargo.lock` - Local Rust crate dependencies for draft and project path facts.

## Verification

- `cargo test -p artifact_store resource_index -- --nocapture` - PASS, 5 tests.
- `pnpm run test:phase14-source-guards` - PASS.
- `rg -n "enum Material" crates/draft_model/src/material.rs` - PASS, only existing `MaterialKind` and `MaterialStatus` enums.
- `rg -n "font.*resource|effect.*resource|artifactStore|resourceIndex" schemas/draft.schema.json apps/desktop-electron/src/generated/Draft.ts fixtures/draft/positive` - PASS, no canonical draft leakage.

## Decisions Made

- Used `project_store::classify_material_uri` for material source refs so in-bundle material refs become project-relative and external refs remain source refs rather than derived blob paths.
- Kept artifact dependencies keyed by stable semantic/resource facts; fingerprints are stored as validity fields, not as graph node identity.
- Allowed resource deletion to leave dependency facts queryable for invalidation while artifact deletion cascades dependency rows through the existing foreign key.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added local crate dependencies for Rust-owned indexing**
- **Found during:** Task 14-02-01
- **Issue:** `artifact_store` needed direct access to `draft_model` Draft/material/timeline types and `project_store` material URI classification to implement the planned Rust-owned API.
- **Fix:** Added local path dependencies on `draft_model` and `project_store`; updated `Cargo.lock`.
- **Files modified:** `crates/artifact_store/Cargo.toml`, `Cargo.lock`
- **Verification:** `cargo test -p artifact_store resource_index -- --nocapture`
- **Committed in:** `d1d43da`

---

**Total deviations:** 1 auto-fixed (1 blocking).
**Impact on plan:** Dependency wiring was required to preserve Rust ownership and avoid duplicating project path semantics.

## Issues Encountered

- The first Task 1 GREEN run showed a test expectation mismatch around material `ResourceRef` stable keys. The test was corrected to assert the intended `material:{MaterialId}` rule directly.
- `cargo fmt --all --check` surfaced unrelated pre-existing formatting differences outside `artifact_store`; only `cargo fmt -p artifact_store` was run to avoid unrelated churn.
- The first dependency tests did not run under the required `resource_index` filter until their names were updated to include `resource_index`.

## Known Stubs

None.

## Threat Flags

None - the new SQLite resource/dependency writes are covered by the plan threat model T-14-05 and T-14-06.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 14-03 can consume resource and dependency rows for exact replacement/relink/rename/delete invalidation without moving dependency membership into Electron or `.veproj/project.json`.

## Self-Check: PASSED

- Created files exist on disk: `resource_index.rs`, `dependencies.rs`, and `tests/resource_index.rs`.
- Task commits `c2e9841`, `d1d43da`, `f0d661b`, and `08c1266` exist in git history.
- Required verification commands passed.
- Worktree has no uncommitted plan changes; only the untouched untracked `reference/` directory remains.

---
*Phase: 14-asset-resource-manager-and-derived-artifact-store*
*Completed: 2026-06-19*
