---
phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence
plan: 04
subsystem: render-graph
tags: [rust, render_graph, incremental, fingerprints, dirty-ranges]

requires:
  - phase: 13-02
    provides: DirtyRange and DirtyDomain command delta facts for downstream graph/cache consumers
  - phase: 13-03
    provides: Domain-aware command deltas and undo/redo invalidation facts
provides:
  - Stable semantic RenderGraphNodeId and RenderGraphNodeRole contracts
  - Separate per-node fingerprints for semantic, input, output profile, runtime, schema, and generator facts
  - In-memory RenderGraphSnapshot contracts for session diffing
  - Deterministic RenderGraphDiff helpers that compare node ID before fingerprint
affects: [render_graph, preview_service, artifact-store, scheduler, phase14, phase16]

tech-stack:
  added: []
  patterns:
    - Semantic node identity separated from output validity fingerprints
    - In-memory graph snapshots sorted by stable node key
    - Identity-first snapshot diff with Rust-owned dirty range/domain carry-through

key-files:
  created:
    - crates/render_graph/src/incremental.rs
    - crates/render_graph/src/fingerprint.rs
  modified:
    - crates/render_graph/src/lib.rs
    - crates/render_graph/src/graph.rs
    - crates/render_graph/Cargo.toml
    - crates/render_graph/tests/node_identity.rs
    - crates/render_graph/tests/render_graph_snapshots.rs
    - crates/render_graph/tests/canvas_background.rs

key-decisions:
  - "Render graph node IDs are semantic stable keys derived from draft, role, track, segment, material, and deterministic local frame/filter IDs."
  - "Fingerprints are separate from identity and include semantic, input, output profile, runtime capability, schema, and generator dimensions."
  - "Graph snapshots remain in-memory render_graph contracts and are not written into .veproj/project.json."
  - "Graph diffs compare stable node IDs first, then fingerprints, and only carry DirtyRange/DirtyDomain facts supplied by Rust command deltas."

patterns-established:
  - "Node identity answers what semantic graph node this is; fingerprints answer whether the exact output remains valid."
  - "Snapshot node fingerprints are sorted by stable_key for deterministic comparison and downstream cache decisions."
  - "Diffs expose added, removed, changed, and unchanged buckets without renderer, scheduler, filesystem, FFmpeg, SQLite, or UI state dependencies."

requirements-completed: [INCR-01, INCR-05]

duration: 18min
completed: 2026-06-18T22:15:22Z
---

# Phase 13 Plan 04: Stable Render Graph Identity Summary

**Stable semantic render graph node IDs with separate fingerprints, in-memory snapshots, and deterministic identity-first graph diffs**

## Performance

- **Duration:** 18 min
- **Started:** 2026-06-18T21:57:00Z
- **Completed:** 2026-06-18T22:15:22Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- Added `RenderGraphNodeId` and `RenderGraphNodeRole` with stable semantic keys for canvas, materials, video/audio/text segment nodes, filters, transitions, sampled frames, and reserved output/composite roles.
- Added `RenderGraphNodeFingerprint` and `RenderGraphSnapshot` with independent semantic, input, output profile, runtime capability, schema, and generator fingerprint dimensions.
- Added `RenderGraphDiff::between` to classify added, removed, changed, and unchanged nodes deterministically while carrying supplied `DirtyRange` and `DirtyDomain` facts.
- Updated render graph snapshots to treat `nodeId` as part of the renderer-neutral derived graph contract.

## Task Commits

1. **Task 13-04-01 RED:** `b16a237` - failing stable node identity coverage
2. **Task 13-04-01 GREEN:** `74acf46` - stable render graph node IDs
3. **Task 13-04-02 RED:** `c08713f` - failing graph fingerprint coverage
4. **Task 13-04-02 GREEN:** `b83f8b5` - graph fingerprints and snapshots
5. **Task 13-04-03 RED:** `d066c5f` - failing graph diff coverage
6. **Task 13-04-03 GREEN:** `df8f035` - deterministic graph diff helpers
7. **Verification fix:** `0143a9e` - canvas snapshots updated for node IDs

## Files Created/Modified

- `crates/render_graph/src/incremental.rs` - stable node identity roles/keys and deterministic graph diff types.
- `crates/render_graph/src/fingerprint.rs` - deterministic fingerprint helpers and in-memory graph snapshot contracts.
- `crates/render_graph/src/graph.rs` - attaches semantic node IDs during graph construction.
- `crates/render_graph/src/lib.rs` - exports incremental identity, fingerprint, snapshot, and diff contracts.
- `crates/render_graph/Cargo.toml` - makes existing `serde_json` dependency available to library fingerprint code.
- `crates/render_graph/tests/node_identity.rs` - TDD coverage for stable IDs, fingerprints, and graph diffs.
- `crates/render_graph/tests/render_graph_snapshots.rs` - snapshot coverage for serialized node IDs and in-memory fingerprints.
- `crates/render_graph/tests/canvas_background.rs` - existing canvas serialization expectations updated for `nodeId`.

## Decisions Made

- Stable keys intentionally exclude content hashes, fingerprints, output paths, SQLite rows, scheduler IDs, FFmpeg/runtime process details, and UI state.
- Runtime capability facts enter fingerprints through a caller-supplied runtime capability fingerprint string; they do not alter node identity.
- The snapshot API lives entirely in `render_graph` and is session/in-memory only, staging Phase 14 artifact persistence without implementing it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated existing canvas snapshot tests for new node identity field**
- **Found during:** Overall verification
- **Issue:** Full `cargo test -p render_graph -- --nocapture` failed because older canvas background snapshots expected the pre-identity canvas serialization.
- **Fix:** Added explicit canvas `nodeId` expectations to `crates/render_graph/tests/canvas_background.rs`.
- **Verification:** `cargo test -p render_graph -- --nocapture`
- **Committed in:** `0143a9e`

---

**Total deviations:** 1 auto-fixed bug
**Impact on plan:** Required to keep existing render_graph tests aligned with the planned public graph identity contract. No scope expansion.

## Issues Encountered

- `cargo fmt` touched unrelated crates during task 1. Those unrelated format-only changes were reverted by specific path before any commit; only render_graph task files were committed.
- `gsd-tools` was not available on PATH, so execution used normal git commits and did not update planning state.

## Known Stubs

None.

## Threat Flags

None.

## User Setup Required

None - no external service configuration required.

## Verification

- `cargo test -p render_graph --test node_identity -- --nocapture`
- `cargo test -p render_graph --test render_graph_snapshots -- --nocapture`
- `cargo test -p render_graph -- --nocapture`
- `pnpm run test:phase13-source-guards`
- `git diff --check`
- `cargo check --workspace --locked`

## Next Phase Readiness

Phase 14 can consume stable node IDs and fingerprint dimensions for artifact-store metadata. Phase 16 can consume deterministic diff buckets plus Rust-owned dirty ranges/domains for scheduling without moving graph diff ownership into the renderer.

## Self-Check: PASSED

- Summary file exists.
- Task commits exist in git history.
- No `reference/` changes were staged or committed.
- `.planning/STATE.md` and `.planning/ROADMAP.md` were not modified.

---
*Phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence*
*Completed: 2026-06-18*
