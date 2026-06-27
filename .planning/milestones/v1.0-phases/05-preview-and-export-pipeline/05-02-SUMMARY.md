---
phase: 05-preview-and-export-pipeline
plan: 02
subsystem: render-graph
tags: [rust, render_graph, renderer-neutral, preview, export, snapshots]

requires:
  - phase: 05-preview-and-export-pipeline
    provides: engine_core NormalizedDraft and RenderRangeState from Plan 05-01
provides:
  - Renderer-neutral RenderGraph built from engine_core normalized range state
  - PreviewFrame, PreviewSegment, and ExportMp4 output profiles over the same graph
  - Stable serde JSON snapshot coverage for graph/profile contracts
affects: [ffmpeg_compiler, preview_service, export, TEST-04]

tech-stack:
  added: [draft_model path dependency, engine_core path dependency, serde]
  patterns:
    - Pure render graph data over engine_core state
    - Output profiles as renderer-neutral intent metadata
    - Stable serde JSON assertions without a snapshot crate

key-files:
  created:
    - crates/render_graph/src/graph.rs
    - crates/render_graph/src/profile.rs
    - crates/render_graph/tests/render_graph_snapshots.rs
  modified:
    - Cargo.lock
    - crates/render_graph/Cargo.toml
    - crates/render_graph/src/lib.rs

key-decisions:
  - "render_graph builds only from engine_core::NormalizedDraft and RenderRangeState, rejecting foreign range-state segment references with classified RenderGraphError values."
  - "Filter and transition data are preserved as renderer-neutral degraded intents for later compiler/runtime capability handling, with no command syntax in graph snapshots."
  - "Preview frame, preview segment, and export MP4 use one RenderGraphPlan shape where only RenderOutputProfile metadata differs."

patterns-established:
  - "Render graph builders consume engine_core outputs rather than raw Draft traversal or renderer-owned layers."
  - "Output profile validation classifies unsupported dimensions, frame rates, target ranges, and preset settings before compiler/runtime work."

requirements-completed: [PREV-02, PREV-03, EXP-01, EXP-02, TEST-04]

duration: 9 min
completed: 2026-06-17
---

# Phase 05 Plan 02: Typed Renderer-Neutral Render Graph Summary

**Renderer-neutral render graph and shared preview/export output profiles built from engine_core normalized range state**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-17T17:36:28Z
- **Completed:** 2026-06-17T17:45:09Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `RenderGraph` with deterministic materials, video layers, audio mixes, text overlays, sampled frames, filter intents, transition intents, and classified `RenderGraphError` values.
- Added `build_render_graph` consuming `NormalizedDraft` and `RenderRangeState` from `engine_core`, including validation that range-state segment references belong to the normalized draft.
- Added `RenderOutputProfile` and `RenderGraphPlan` for preview still frames, preview MP4 segments, and H.264/AAC MP4 export metadata over one shared graph shape.
- Added stable serde JSON snapshot tests for graph construction, degraded filter/transition intents, output profiles, and unsupported profile setting errors.

## Task Commits

1. **Task 05-02-01 RED:** `404c122` test(05-02): add failing render graph snapshots
2. **Task 05-02-01 GREEN:** `5dbd06d` feat(05-02): build renderer neutral graph intents
3. **Task 05-02-02 RED:** `d72554d` test(05-02): add failing output profile snapshots
4. **Task 05-02-02 GREEN:** `0e64a3e` feat(05-02): add preview and export output profiles
5. **Task 05-02-02 fix:** `f54552f` fix(05-02): keep profile validation hints renderer neutral

## Files Created/Modified

- `crates/render_graph/src/graph.rs` - Typed render graph data, builder, renderer-neutral filter/transition intents, sampled frame metadata, and classified graph errors.
- `crates/render_graph/src/profile.rs` - Preview frame, preview segment, export MP4 profiles, output dimensions, MP4 preset metadata, and profile validation.
- `crates/render_graph/src/lib.rs` - Public exports for graph and profile APIs.
- `crates/render_graph/Cargo.toml` - Local `draft_model`/`engine_core` dependencies plus serde/serde_json.
- `crates/render_graph/tests/render_graph_snapshots.rs` - TEST-04 graph/profile snapshot coverage.
- `Cargo.lock` - Lockfile update for render_graph dependency edges.

## Decisions Made

- Kept filter and transition support as `Degraded` intent data because compiler/runtime capability mapping belongs to later Phase 5 plans.
- Used `RenderGraphPlan { graph, output_profile }` so preview and export consumers share one graph payload and differ only by profile metadata.
- Kept profile validation hints generic and renderer-neutral; runtime-specific probe behavior remains outside `render_graph`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Test Bug] Corrected the graph snapshot range to include text overlays**
- **Found during:** Task 05-02-01
- **Issue:** The initial RED snapshot sampled `0..100000`, before the fixture text segment starts at `500000`, while the task required text overlay coverage.
- **Fix:** Moved the snapshot render range to `600000..700000` and asserted the resolved text overlay data from `engine_core`.
- **Files modified:** `crates/render_graph/tests/render_graph_snapshots.rs`
- **Verification:** `cargo test -p render_graph render_graph -- --nocapture`
- **Committed in:** `5dbd06d`

**2. [Rule 1 - Implementation Bug] Fixed output profile enum field serialization**
- **Found during:** Task 05-02-02
- **Issue:** Enum variant fields initially serialized as snake_case, which would drift from the repo's camelCase serde contract pattern.
- **Fix:** Added `rename_all_fields = "camelCase"` to `RenderOutputProfile`.
- **Files modified:** `crates/render_graph/src/profile.rs`
- **Verification:** `cargo test -p render_graph output_profiles -- --nocapture`
- **Committed in:** `0e64a3e`

**3. [Rule 2 - Missing Critical Boundary Detail] Removed runtime-specific probe wording from profile hints**
- **Found during:** Plan-level boundary scan
- **Issue:** Export profile hints mentioned runtime probe details directly, which was more specific than a renderer-neutral profile contract needs.
- **Fix:** Reworded validation hints to generic runtime metadata validation.
- **Files modified:** `crates/render_graph/src/profile.rs`, `crates/render_graph/tests/render_graph_snapshots.rs`
- **Verification:** `cargo test -p render_graph -- --nocapture`
- **Committed in:** `f54552f`

---

**Total deviations:** 3 auto-fixed (2 Rule 1 bugs, 1 Rule 2 boundary detail)
**Impact on plan:** All fixes tightened the planned renderer-neutral contract. No architecture change or scope expansion.

## Issues Encountered

- `gsd-tools` was not on PATH in the shell, so close-out used the documented `node $HOME/.codex/get-shit-done/bin/gsd-tools.cjs` fallback.
- The main working tree contained unrelated changes before and during execution, including the protected user paths named in the prompt. These were not staged or modified by this plan.

## Known Stubs

None. Stub scan found no TODO/FIXME/placeholder text or hardcoded empty UI data sources in the modified render_graph files.

## Threat Flags

None. The new trust-boundary surface is the planned `engine_core` to `render_graph` data boundary, and the crate contains no process execution, filesystem cache, Electron, or project persistence code.

## Verification

- `cargo test -p render_graph render_graph -- --nocapture` - PASS
- `cargo test -p render_graph output_profiles -- --nocapture` - PASS
- `cargo test -p render_graph -- --nocapture` - PASS
- Boundary scan for process/filesystem/cache/project persistence APIs in `crates/render_graph` - PASS; remaining matches are negative snapshot assertions for command syntax absence.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Plan 05-03. `ffmpeg_compiler` can consume `RenderGraphPlan` and map renderer-neutral materials, layers, audio mixes, text overlays, filters, transitions, and output profiles into structured compiler jobs without reading draft semantics directly.

## Self-Check: PASSED

- Found key created files: `graph.rs`, `profile.rs`, `render_graph_snapshots.rs`.
- Found task commits: `404c122`, `5dbd06d`, `d72554d`, `0e64a3e`, `f54552f`.
- Plan requirements listed in frontmatter: `PREV-02`, `PREV-03`, `EXP-01`, `EXP-02`, `TEST-04`.
- No unexpected tracked file deletions found in task commits.

---
*Phase: 05-preview-and-export-pipeline*
*Completed: 2026-06-17*
