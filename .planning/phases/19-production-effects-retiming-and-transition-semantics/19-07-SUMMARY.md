---
phase: 19-production-effects-retiming-and-transition-semantics
plan: "07"
subsystem: render-preview-export
tags: [rust, render-graph, realtime-preview, ffmpeg-compiler, transitions, tdd]

# Dependency graph
requires:
  - phase: 19-production-effects-retiming-and-transition-semantics
    provides: "Plan 19-06 validated first-party transition relationships between adjacent segment IDs."
provides:
  - "Track-level transition relationships projected into typed render graph transition intents with endpoint-aware windows."
  - "Transition fingerprints and dirty ranges include duration, reference, support state, and endpoint identity."
  - "Realtime preview capability reports supported dissolve transitions and rejects external transitions as product success."
  - "FFmpeg compiler-owned dissolve export filters generated from typed render graph transition intents."
  - "Phase 19 transition source guard blocks transition FFmpeg filter strings outside ffmpeg_compiler."
affects: [render_graph, realtime_preview_runtime, ffmpeg_compiler, engine_core, draft_model, phase19-source-guards]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Render graph transition intents carry canonical from/to segment IDs before preview/export consumers see them."
    - "FFmpeg transition filter syntax is isolated to ffmpeg_compiler and guarded by source scans."

key-files:
  created:
    - ".planning/phases/19-production-effects-retiming-and-transition-semantics/19-07-SUMMARY.md"
  modified:
    - "crates/engine_core/src/normalize.rs"
    - "crates/render_graph/src/effects.rs"
    - "crates/render_graph/src/fingerprint.rs"
    - "crates/render_graph/src/graph.rs"
    - "crates/render_graph/src/incremental.rs"
    - "crates/render_graph/src/lib.rs"
    - "crates/render_graph/tests/production_effects.rs"
    - "crates/realtime_preview_runtime/tests/production_effects.rs"
    - "crates/draft_model/src/effects.rs"
    - "crates/ffmpeg_compiler/src/effects.rs"
    - "crates/ffmpeg_compiler/src/filters.rs"
    - "crates/ffmpeg_compiler/tests/production_effects.rs"
    - "scripts/phase19-source-guards.sh"

key-decisions:
  - "Canonical track-level transition relationships are carried through engine normalization and render graph intents rather than inferred in preview or compiler layers."
  - "Dissolve export support is implemented as compiler-owned xfade filter generation from RenderTransitionIntent."
  - "External transition references remain report-only diagnostics and never become FFmpeg filter semantics."
  - "Transition FFmpeg filter strings are allowed only inside ffmpeg_compiler."

patterns-established:
  - "Transition node identity and fingerprints include both relationship endpoints."
  - "Compiler transition labels are endpoint-based and split per segment to support chained relationships without label reuse."
  - "Source guards enforce ownership boundaries for retime and transition FFmpeg filters."

requirements-completed: [PRODFX-02]

# Metrics
duration: 21 min
completed: 2026-06-25
status: complete
---

# Phase 19 Plan 07: Transition Graph Compiler Semantics Summary

**First-party dissolve transitions now flow from canonical track relationships into render graph intent, preview diagnostics, compiler-owned FFmpeg output, and source-boundary guards.**

## Performance

- **Duration:** 21 min
- **Started:** 2026-06-25T10:14:06Z
- **Completed:** 2026-06-25T10:35:09Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments

- Render graph transition intent now carries from/to segment IDs, endpoint-aware overlap windows, capability state, fingerprints, and dirty-range inputs.
- Realtime preview production-effects tests prove first-party dissolve support is explicit and external transitions are unsupported diagnostics, not product success.
- FFmpeg compiler now emits deterministic dissolve `xfade` filters from typed graph intents and reports unsupported external transitions without compiling proprietary IDs.
- Phase 19 source guards now require command, graph, preview, compiler, and boundary coverage for transition semantics.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Add failing transition graph preview tests** - `552f910` (test)
2. **Task 1 GREEN: Add transition graph preview intents** - `25e90b8` (feat)
3. **Task 2 RED: Add failing dissolve compiler tests** - `7a18ce2` (test)
4. **Task 2 GREEN: Compile dissolve transition export** - `fb4ee67` (feat)

_Note: Both plan tasks were TDD tasks, so RED and GREEN commits are recorded separately._

## Files Created/Modified

- `crates/engine_core/src/normalize.rs` - Carries canonical track transition relationships into normalized tracks.
- `crates/render_graph/src/effects.rs` - Classifies transition capabilities for render graph consumers.
- `crates/render_graph/src/fingerprint.rs` - Includes transition endpoints, windows, references, support, and reasons in semantic fingerprints.
- `crates/render_graph/src/graph.rs` - Projects transition relationships into `RenderTransitionIntent` values with endpoint-aware windows and stable node IDs.
- `crates/render_graph/src/incremental.rs` - Updates transition node identity generation for endpoint-based relationship keys.
- `crates/render_graph/src/lib.rs` - Exposes transition capability helpers.
- `crates/render_graph/tests/production_effects.rs` - Covers transition graph intent, fingerprints, dirty ranges, and endpoint changes.
- `crates/realtime_preview_runtime/tests/production_effects.rs` - Covers supported first-party dissolve and unsupported external transition diagnostics.
- `crates/draft_model/src/effects.rs` - Keeps capability copy backend-neutral so FFmpeg-specific syntax stays in the compiler.
- `crates/ffmpeg_compiler/src/effects.rs` - Adds compiler-owned dissolve transition filter generation.
- `crates/ffmpeg_compiler/src/filters.rs` - Wires transition filters into export filter scripts with per-segment split allocation.
- `crates/ffmpeg_compiler/tests/production_effects.rs` - Covers dissolve export output, chained transition labels, and unsupported external transition diagnostics.
- `scripts/phase19-source-guards.sh` - Adds transition coverage and source-boundary guard checks.

## Decisions Made

- Canonical transition relationships are consumed from normalized Rust draft state and not reconstructed by UI, preview, or compiler code.
- First-party dissolve export support is intentionally narrow: it compiles only supported `TransitionKind::Dissolve` intents and leaves external references as diagnostics.
- Compiler labels are endpoint-based and segment split outputs are allocated once per segment, preventing chained transitions from reusing a consumed FFmpeg label.
- Source-boundary enforcement treats transition FFmpeg syntax as compiler-only implementation detail.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Normalized tracks now carry track transition relationships**
- **Found during:** Task 1 (Add transition graph fingerprints and preview support states)
- **Issue:** Render graph could not consume canonical 19-06 track-level transition relationships if normalization only exposed segment-local transition fields.
- **Fix:** Added `transitions: Vec<TrackTransition>` to normalized tracks and projected those relationships into render graph transition intents.
- **Files modified:** `crates/engine_core/src/normalize.rs`, `crates/render_graph/src/graph.rs`, `crates/render_graph/tests/production_effects.rs`
- **Verification:** `cargo test -p render_graph production_effects -- --nocapture`
- **Committed in:** `25e90b8`

**2. [Rule 2 - Missing Critical] Kept FFmpeg transition wording out of draft capability metadata**
- **Found during:** Task 2 (Compile first-party dissolve transition and guards)
- **Issue:** The new source guard must reject transition FFmpeg filter strings outside `ffmpeg_compiler`; draft capability copy mentioned `xfade`.
- **Fix:** Changed the draft-model capability description to backend-neutral transition export wording.
- **Files modified:** `crates/draft_model/src/effects.rs`
- **Verification:** `bash scripts/phase19-source-guards.sh --transition`
- **Committed in:** `fb4ee67`

**3. [Rule 1 - Bug] Prevented chained transition FFmpeg label reuse**
- **Found during:** Task 2 (Compile first-party dissolve transition and guards)
- **Issue:** A middle segment participating in two transitions would need one main output plus two transition taps; splitting per transition could reuse a consumed FFmpeg label.
- **Fix:** Allocated split outputs once per segment and added a chained dissolve compiler test.
- **Files modified:** `crates/ffmpeg_compiler/src/filters.rs`, `crates/ffmpeg_compiler/tests/production_effects.rs`
- **Verification:** `cargo test -p ffmpeg_compiler production_effects -- --nocapture`
- **Committed in:** `fb4ee67`

---

**Total deviations:** 3 auto-fixed (2 missing critical, 1 bug)
**Impact on plan:** All auto-fixes reinforced the planned Rust-owned transition semantics and compiler-only FFmpeg boundary. No scope creep beyond transition correctness.

## Issues Encountered

- Initial Task 2 GREEN work exposed label details in the compiler tests. The final implementation preserves endpoint-based labels and adds chained-transition coverage.
- No authentication gates or external setup were required.

## Verification

- `cargo test -p render_graph production_effects -- --nocapture` - passed, 6 tests
- `cargo test -p realtime_preview_runtime production_effects -- --nocapture` - passed, 5 tests
- `cargo test -p ffmpeg_compiler production_effects -- --nocapture` - passed, 7 tests
- `bash scripts/phase19-source-guards.sh --transition` - passed

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

PRODFX-02 is complete for first-party dissolve transition graph intent, preview diagnostics, compiler export output, and source ownership guards. Later transition kinds can add compiler support by extending typed `TransitionReference` handling inside `ffmpeg_compiler` while keeping external provider references diagnostic-only.

## Self-Check: PASSED

- Found `.planning/phases/19-production-effects-retiming-and-transition-semantics/19-07-SUMMARY.md`.
- Found task commits `552f910`, `25e90b8`, `7a18ce2`, and `fb4ee67`.

---
*Phase: 19-production-effects-retiming-and-transition-semantics*
*Completed: 2026-06-25*
