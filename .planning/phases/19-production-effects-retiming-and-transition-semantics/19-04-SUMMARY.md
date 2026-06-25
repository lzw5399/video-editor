---
phase: 19-production-effects-retiming-and-transition-semantics
plan: "04"
subsystem: rendering
tags: [retiming, render-graph, ffmpeg-compiler, realtime-preview, testkit, source-guards]

# Dependency graph
requires:
  - phase: 19-03
    provides: Rust retiming commands, draft contracts, and engine_core source mapping
provides:
  - Render graph retime intent with engine-owned source mapping and audio follow-speed facts
  - Compiler-owned retime video/audio filter helpers and export diagnostics
  - Realtime preview retime capability/parity diagnostics
  - Testkit preview/export retime parity evidence and retime source guard ownership scan
affects: [phase-19, production-effects, retiming, preview-export-parity, audio-retiming]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Typed render graph intent is the only source for retime compiler filters
    - Testkit parity compares typed source mapping facts instead of FFmpeg-only success
    - Source guards enforce retime filter string ownership in ffmpeg_compiler

key-files:
  created:
    - crates/ffmpeg_compiler/src/effects.rs
  modified:
    - crates/render_graph/src/graph.rs
    - crates/render_graph/src/lib.rs
    - crates/render_graph/tests/production_effects.rs
    - crates/ffmpeg_compiler/src/filters.rs
    - crates/ffmpeg_compiler/src/lib.rs
    - crates/ffmpeg_compiler/tests/common/mod.rs
    - crates/ffmpeg_compiler/tests/production_effects.rs
    - crates/ffmpeg_compiler/tests/transform_snapshots.rs
    - crates/realtime_preview_runtime/src/capabilities.rs
    - crates/realtime_preview_runtime/src/parity.rs
    - crates/realtime_preview_runtime/tests/gpu_subset.rs
    - crates/realtime_preview_runtime/tests/production_effects.rs
    - crates/testkit/tests/production_effects_preview.rs
    - crates/testkit/tests/production_effects_exports.rs
    - crates/testkit/tests/realtime_preview_parity.rs
    - crates/draft_import/src/validation.rs
    - crates/adapter_kaipai/src/mapper.rs
    - scripts/phase19-source-guards.sh

key-decisions:
  - "Retimed source ranges are derived by engine_core and carried through render graph intent before compiler or preview code consumes them."
  - "FFmpeg retime timestamp/audio filters are generated only in ffmpeg_compiler and guarded against appearing elsewhere."
  - "Testkit retime parity uses graph source-mapping facts and typed diagnostics, not artifact fallback or FFmpeg string assertions outside the compiler crate."

patterns-established:
  - "Retime support facts travel as RenderRetimeIntent, including source mapping, capability support, and audio policy diagnostics."
  - "Compiler helpers expose video/audio retime fragments from typed graph intent while returning RenderAudioMixDiagnostic for unsupported/degraded audio."
  - "Phase 19 source guards can stage plan-specific artifact requirements under --retiming without requiring later transition/effect waves."

requirements-completed: [PRODFX-01]

# Metrics
duration: 37min
completed: 2026-06-25
status: complete
---

# Phase 19 Plan 04: Retiming Render Graph, Compiler, Preview, And Parity Summary

**Retiming now survives render graph intent, fingerprints, FFmpeg compiler output, realtime preview diagnostics, and testkit preview/export parity evidence.**

## Performance

- **Duration:** 37 min
- **Started:** 2026-06-25T08:32:28Z
- **Completed:** 2026-06-25T09:08:37Z
- **Tasks:** 3
- **Files modified:** 19

## Accomplishments

- Added typed render graph retime intent carrying mode, speed curve, engine-owned source mapping, export support, and audio follow-speed policy diagnostics.
- Added compiler-owned retime helpers for video timestamp filters and audio follow-speed chains, with degraded/unsupported audio diagnostics returned as typed render graph facts.
- Extended realtime preview capability/parity classification for retime support and audio policy divergence.
- Added deterministic testkit preview/export retime parity tests for constant speed and speed curve cases.
- Extended the Phase 19 `--retiming` source guard to require graph/compiler/testkit coverage and block retime FFmpeg filter strings outside `ffmpeg_compiler`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add retime render graph intent and fingerprints**
   - `aa4741b` test(19-04): add failing retime render graph intent tests
   - `e815a5f` feat(19-04): propagate retime intent through render graph
2. **Task 2: Compile retime export and preview diagnostics**
   - `ec227a3` test(19-04): add failing compiler preview retime tests
   - `88339c9` feat(19-04): compile retime filters and preview diagnostics
3. **Task 3: Prove retime preview export parity in testkit**
   - `e596f93` test(19-04): add failing retime testkit parity guards
   - `9af42d8` feat(19-04): prove retime preview export parity

Deviation fix commits:

- `734ef50` fix(19-04): align compiler fixtures with typed effect contracts
- `bc9e93b` fix(19-04): align draft import filter validation
- `31c790f` fix(19-04): align Kaipai transition mapping
- `3dd5753` fix(19-04): align testkit preview parity filter fixture

## Files Created/Modified

- `crates/render_graph/src/graph.rs` - Carries `RenderRetimeIntent` and engine-owned source mapping on video and audio graph layers.
- `crates/render_graph/src/lib.rs` - Exports new retime graph types.
- `crates/render_graph/tests/production_effects.rs` - Covers retime source mapping, fingerprints, dirty domains, and stable node identity.
- `crates/ffmpeg_compiler/src/effects.rs` - Adds compiler-owned retime video/audio filter helpers and source-range clipping helper.
- `crates/ffmpeg_compiler/src/filters.rs` - Wires retime source ranges, video timestamp filters, audio timing filters, and retime audio diagnostics into filter script generation.
- `crates/ffmpeg_compiler/src/lib.rs` - Exports retime compiler helpers and production-effects module surface.
- `crates/ffmpeg_compiler/tests/common/mod.rs`, `crates/ffmpeg_compiler/tests/transform_snapshots.rs` - Align compiler fixtures with typed filter/transition contracts.
- `crates/ffmpeg_compiler/tests/production_effects.rs` - Covers compiler-owned retime filter generation and unsupported preserve-pitch diagnostics.
- `crates/realtime_preview_runtime/src/capabilities.rs` - Classifies retime and audio retime support without accepting fallback success for supported paths.
- `crates/realtime_preview_runtime/src/parity.rs` - Includes retime/audio support in preview/export parity decisions.
- `crates/realtime_preview_runtime/tests/gpu_subset.rs`, `crates/realtime_preview_runtime/tests/production_effects.rs` - Align preview tests with typed retime facts and unsupported audio policy behavior.
- `crates/testkit/tests/production_effects_preview.rs` - Adds constant-speed and speed-curve preview retime parity evidence.
- `crates/testkit/tests/production_effects_exports.rs` - Adds constant-speed and speed-curve export parity evidence tied to graph source mapping and diagnostics.
- `crates/testkit/tests/realtime_preview_parity.rs` - Aligns stale test fixture with typed external filters.
- `crates/draft_import/src/validation.rs` - Aligns import validation with typed filters.
- `crates/adapter_kaipai/src/mapper.rs` - Aligns Kaipai transition mapping with typed transition constructors.
- `scripts/phase19-source-guards.sh` - Adds staged retime graph/compiler/testkit coverage checks and non-compiler retime filter ownership scan.

## Decisions Made

- Retime source mapping remains owned by `engine_core`; render graph, compiler, preview, and testkit consume typed facts instead of recomputing source/target time math.
- Constant-speed audio follow-speed exports through compiler-owned atempo chains; preserve-pitch and speed-curve audio emit explicit typed diagnostics when unsupported or degraded.
- Testkit retime parity avoids asserting raw FFmpeg filter strings outside `ffmpeg_compiler`; it checks semantic source mapping and diagnostics instead.
- The `--retiming` guard now enforces only artifacts expected by completed retiming waves, while later transition/effect waves remain separately gated.

## Verification

- `cargo test -p render_graph production_effects -- --nocapture` - passed
- `cargo test -p ffmpeg_compiler production_effects -- --nocapture` - passed
- `cargo test -p realtime_preview_runtime production_effects -- --nocapture` - passed
- `cargo test -p testkit production_effects -- --nocapture` - passed
- `bash scripts/phase19-source-guards.sh --retiming` - passed

Known warning: `media_runtime_desktop` still reports the pre-existing macOS `tracksWithMediaType` deprecation during preview/testkit builds.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Aligned compiler fixtures with typed effect contracts**
- **Found during:** Task 2 (Compile retime export and preview diagnostics)
- **Issue:** Existing compiler fixtures still used old free-form filter/transition shapes, blocking compiler production-effects tests after typed effect contracts landed.
- **Fix:** Updated compiler fixture helpers and snapshots to typed first-party/external filter and transition contracts.
- **Files modified:** `crates/ffmpeg_compiler/tests/common/mod.rs`, `crates/ffmpeg_compiler/tests/transform_snapshots.rs`, related compiler test fixtures
- **Verification:** `cargo test -p ffmpeg_compiler production_effects -- --nocapture`
- **Committed in:** `734ef50`

**2. [Rule 3 - Blocking] Aligned draft import filter validation with typed filters**
- **Found during:** Task 3 (Prove retime preview export parity in testkit)
- **Issue:** `draft_import` validation referenced removed `Filter.name` and `Filter.parameters` fields, preventing `testkit` from compiling.
- **Fix:** Updated validation to inspect `FilterKind::ExternalReference` and retain provider semantic leakage checks on typed external references.
- **Files modified:** `crates/draft_import/src/validation.rs`
- **Verification:** `cargo check -p draft_import`
- **Committed in:** `bc9e93b`

**3. [Rule 3 - Blocking] Aligned Kaipai transition mapping with typed transitions**
- **Found during:** Task 3 (Prove retime preview export parity in testkit)
- **Issue:** `adapter_kaipai` constructed the removed transition `name` field, preventing `testkit` dev dependencies from compiling.
- **Fix:** Switched supported imported fade/dissolve transitions to `Transition::dissolve` while preserving imported duration.
- **Files modified:** `crates/adapter_kaipai/src/mapper.rs`
- **Verification:** `cargo check -p adapter_kaipai`
- **Committed in:** `31c790f`

**4. [Rule 3 - Blocking] Aligned testkit preview parity fixture with typed filters**
- **Found during:** Task 3 (Prove retime preview export parity in testkit)
- **Issue:** `testkit` preview parity test fixture still constructed old free-form filters, blocking the testkit crate.
- **Fix:** Updated the fixture to use a typed external filter with the same `cinematic-lut` display evidence.
- **Files modified:** `crates/testkit/tests/realtime_preview_parity.rs`
- **Verification:** `cargo test -p testkit production_effects -- --nocapture`
- **Committed in:** `3dd5753`

---

**Total deviations:** 4 auto-fixed (4 Rule 3 blocking fixes)
**Impact on plan:** All fixes were schema/fixture drift blockers required to run the planned production-effects retime tests. No dependency or architecture changes were introduced.

## Issues Encountered

- The `testkit` crate exposed stale typed effect/transition fixtures in dependent crates before the new retime parity tests could run. These were fixed as blocking schema drift and committed separately.
- The retime source guard initially exited on an empty non-compiler filter scan because of `set -e`; the guard now explicitly treats no matches as success.

## Known Stubs

None. Stub scan found no TODO/FIXME/placeholder UI data paths in the files changed by this plan. The scan produced only legitimate FFmpeg filter string false positives in `ffmpeg_compiler`.

## Threat Flags

None. The new trust-boundary surface is the planned `ffmpeg_compiler::effects` retime compiler helper; it is covered by T-19-10 and guarded by `scripts/phase19-source-guards.sh --retiming`.

## User Setup Required

None - no external service configuration required.

## TDD Gate Compliance

- RED commits exist for Task 1, Task 2, and Task 3: `aa4741b`, `ec227a3`, `e596f93`.
- GREEN commits exist after RED for each task: `e815a5f`, `88339c9`, `9af42d8`.

## Next Phase Readiness

Plan 19-05 can consume the retime audio intent and typed diagnostics to represent retiming in `audio_engine` and testkit audio parity. Later transition/effect waves can follow the same render graph -> preview diagnostics -> compiler -> source guard pattern established here.

## Self-Check: PASSED

- Found summary file at `.planning/phases/19-production-effects-retiming-and-transition-semantics/19-04-SUMMARY.md`.
- Verified commits exist: `aa4741b`, `e815a5f`, `734ef50`, `ec227a3`, `88339c9`, `bc9e93b`, `31c790f`, `3dd5753`, `e596f93`, `9af42d8`.

---
*Phase: 19-production-effects-retiming-and-transition-semantics*
*Completed: 2026-06-25*
