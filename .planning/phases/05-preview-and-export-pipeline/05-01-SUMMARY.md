---
phase: 05-preview-and-export-pipeline
plan: 01
subsystem: engine-core
tags: [rust, engine_core, normalization, frame-state, text-layout]

requires:
  - phase: 04.1-professional-jianying-workspace-ui-refinement
    provides: Command-only Jianying workspace shell ready for preview/export integration
provides:
  - NormalizedDraft semantic API for preview/export callers
  - FrameState and RenderRangeState evaluation over normalized draft semantics
  - Deterministic TextLayoutProfile and resolved text overlay inputs
affects: [render_graph, ffmpeg_compiler, preview_service, export]

tech-stack:
  added: [draft_model path dependency, serde, serde_json dev dependency]
  patterns:
    - Pure engine_core semantic transforms over Draft input
    - Stable serde JSON snapshots for frame-state verification

key-files:
  created:
    - crates/engine_core/src/normalize.rs
    - crates/engine_core/src/frame_state.rs
    - crates/engine_core/src/text_layout.rs
    - crates/engine_core/tests/normalization.rs
    - crates/engine_core/tests/frame_state_snapshots.rs
  modified:
    - Cargo.lock
    - crates/engine_core/Cargo.toml
    - crates/engine_core/src/lib.rs

key-decisions:
  - "engine_core normalizes Draft into render-ready tracks, segments, material refs, and diagnostics before preview/export callers read timeline semantics."
  - "Frame-state and render-range APIs use integer microseconds, frame indices, and RationalFrameRate sampling without floating-point persisted fields."
  - "Text layout uses a pinned MVP profile with explicit font candidate identities; engine_core validates the candidate identity but does not probe the filesystem."

patterns-established:
  - "Normalize first: preview/export code should consume NormalizedDraft rather than raw Draft track traversal."
  - "Frame state is deterministic serde data sorted by track stack, track ID, and segment ID for stable snapshots."
  - "Text overlays resolve through TextLayoutProfile so preview and export share font, safe-area, wrapping, and integer layout dimensions."

requirements-completed: [TEXT-03, EXP-02, TEST-03]

duration: 12 min
completed: 2026-06-17
---

# Phase 05 Plan 01: Engine Core Normalization, Frame State, And Text Layout Summary

**Rust-owned normalized draft, deterministic frame-state sampling, and pinned MVP text layout inputs for shared preview/export semantics**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-17T17:16:10Z
- **Completed:** 2026-06-17T17:28:53Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- Added `normalize_draft` with `EngineProfile`, normalized tracks/segments/material refs, checked source/target timerange arithmetic, source material duration bounds, visual stack order, and non-renderable diagnostics for muted tracks/unavailable materials.
- Added `resolve_frame_state`, `resolve_render_range`, and `frame_index_to_microseconds` so preview/export callers can sample active visual layers, audio segments, and text overlays from the same normalized draft path.
- Added `TextLayoutProfile::mvp_default()` and resolved text overlays with pinned `PingFang SC` font policy, fallback candidate identities, safe area, wrapping policy, alignment, and integer layout dimensions.

## Task Commits

1. **Task 05-01-01 RED:** `890eca6` test(05-01): add failing normalization tests
2. **Task 05-01-01 GREEN:** `ed1eefe` feat(05-01): normalize draft semantics for preview export
3. **Task 05-01-02 RED:** `60157fe` test(05-01): add failing frame state snapshots
4. **Task 05-01-02 GREEN:** `191a216` feat(05-01): resolve normalized frame state and ranges
5. **Task 05-01-03 RED:** `c965242` test(05-01): add failing text layout snapshots
6. **Task 05-01-03 GREEN:** `d12e1f9` feat(05-01): pin deterministic text layout semantics

## Files Created/Modified

- `crates/engine_core/src/normalize.rs` - Normalized draft, track, segment, material, diagnostics, and classified engine errors.
- `crates/engine_core/src/frame_state.rs` - Frame-state and render-range evaluation with rational frame sampling.
- `crates/engine_core/src/text_layout.rs` - Deterministic MVP text layout profile and resolved text overlay data.
- `crates/engine_core/src/lib.rs` - Public exports for normalization, frame-state, and text layout APIs.
- `crates/engine_core/Cargo.toml` - Local `draft_model` plus serde dependencies for engine data and tests.
- `Cargo.lock` - Lockfile update for the new engine_core dependency edges.
- `crates/engine_core/tests/normalization.rs` - TEST-03 normalization, diagnostics, checked timerange, and stack-order tests.
- `crates/engine_core/tests/frame_state_snapshots.rs` - TEST-03/TEXT-03 frame-state, render-range, and text-layout snapshot tests.

## Decisions Made

- Kept material `Missing` and `ProbeFailed` as classified non-renderable diagnostics during normalization instead of mutating or rejecting otherwise valid drafts.
- Made text font availability an explicit caller-provided policy identity; `engine_core` validates the policy but performs no filesystem probing.
- Used stable JSON-like assertions through `serde_json` rather than adding a snapshot crate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Test Bug] Corrected text overlay source-position expectation**
- **Found during:** Task 05-01-02
- **Issue:** The initial frame-state test expected a text overlay source position of `600000`, but the segment target range starts at `500000`, so integer source mapping produces `100000`.
- **Fix:** Corrected the expected source position to `100000`.
- **Files modified:** `crates/engine_core/tests/frame_state_snapshots.rs`
- **Verification:** `cargo test -p engine_core frame_state -- --nocapture`
- **Committed in:** `191a216`

**2. [Rule 1 - Implementation Bug] Fixed internal text-layout helper import**
- **Found during:** Task 05-01-03
- **Issue:** `frame_state.rs` imported `resolve_text_overlay` from the crate root before it was exported.
- **Fix:** Imported the helper from `crate::text_layout`.
- **Files modified:** `crates/engine_core/src/frame_state.rs`
- **Verification:** `cargo test -p engine_core text_layout -- --nocapture`
- **Committed in:** `d12e1f9`

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bugs)
**Impact on plan:** Both fixes were task-local correctness fixes. No architecture or scope changes.

## Issues Encountered

- `pnpm run test:phase3-source-guards` exits with status 0 but prints existing renderer matches from its negative `rg` checks. The executable gate passed and no Phase 5 files were implicated.

## Known Stubs

None. Stub scan found only normal empty vector initialization and `TextLayoutProfile::invalid_for_tests()`, which is an explicit negative-test helper.

## Threat Flags

None. New trust-boundary handling matches the plan threat model: draft input normalization, checked integer time mapping, and caller-provided text layout policy validation.

## Verification

- `cargo test -p engine_core normalization -- --nocapture` - PASS
- `cargo test -p engine_core frame_state -- --nocapture` - PASS
- `cargo test -p engine_core text_layout -- --nocapture` - PASS
- `cargo test -p engine_core -- --nocapture` - PASS
- `pnpm run test:phase3-source-guards` - PASS

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Plan 05-02. `render_graph` can now consume one Rust-owned `NormalizedDraft` plus `FrameState`/`RenderRangeState` instead of reading raw draft timelines or renderer-derived layer lists.

## Self-Check: PASSED

- Found key created files: `normalize.rs`, `frame_state.rs`, `text_layout.rs`, `normalization.rs`, `frame_state_snapshots.rs`.
- Found task commits: `890eca6`, `ed1eefe`, `60157fe`, `191a216`, `c965242`, `d12e1f9`.
- Plan requirements listed in frontmatter: `TEXT-03`, `EXP-02`, `TEST-03`.
- No unexpected tracked file deletions found in task commits.

---
*Phase: 05-preview-and-export-pipeline*
*Completed: 2026-06-17*
