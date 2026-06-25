---
phase: 19-production-effects-retiming-and-transition-semantics
plan: "05"
subsystem: audio
tags: [retiming, audio-engine, dsp-timeline, preview-export-parity, source-guards]

# Dependency graph
requires:
  - phase: 19-04
    provides: Render graph, compiler, preview, and testkit retime facts for preview/export parity
provides:
  - Retime-aware audio DSP source sample mapping
  - Audio mix intent retime follow-speed support facts and diagnostics
  - Testkit audio preview/export parity diagnostics for retime source mismatches and unsupported follow-speed
  - Phase 19 retiming-audio source guard coverage
affects: [phase-19, production-effects, audio-retiming, preview-export-parity]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Audio DSP retime sample maps derive from engine_core retime mapping
    - Audio mix intent carries retime facts for preview/export parity
    - Source guards stage audio retime coverage under --retiming-audio

key-files:
  created: []
  modified:
    - Cargo.lock
    - crates/audio_engine/Cargo.toml
    - crates/audio_engine/src/dsp_timeline.rs
    - crates/audio_engine/src/lib.rs
    - crates/audio_engine/src/mix_intent.rs
    - crates/audio_engine/tests/dsp_timeline.rs
    - crates/testkit/src/audio_parity.rs
    - crates/testkit/tests/audio_preview_export_parity.rs
    - scripts/phase19-source-guards.sh

key-decisions:
  - "Audio DSP retime source samples use engine_core source_position_for_retime and retimed_source_range instead of duplicating time mapping in audio_engine."
  - "AudioRetimeMixIntent is carried on both DspSegment and AudioMixSegment so preview/export parity can compare audio retime facts directly."
  - "Unsupported preserve-pitch and degraded speed-curve follow-speed are typed DSP/parity diagnostics, not silent audio success."

patterns-established:
  - "AudioRetimeSourceSampleMap stores original and retimed source ranges plus sampled target-to-source sample points."
  - "Testkit audio parity reports RetimeSourceSampleMismatch and UnsupportedAudioFollowSpeed as separate categories."
  - "Phase 19 source guards require audio_engine retime intent plus testkit parity coverage for --retiming-audio."

requirements-completed: [PRODFX-01]

# Metrics
duration: 11 min
completed: 2026-06-25
status: complete
---

# Phase 19 Plan 05: Audio Retime DSP And Parity Summary

**Audio retiming now travels through DSP timeline evaluation, mix intent, preview/export parity diagnostics, and Phase 19 source guards.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-06-25T09:18:55Z
- **Completed:** 2026-06-25T09:29:09Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- Added retime-aware audio source sample maps for constant speed and speed curves using integer microsecond/sample math.
- Carried `AudioRetimeMixIntent` through `DspSegment` and `AudioMixSegment`, including follow-speed policy, support state, and reason text.
- Added typed DSP diagnostics for unsupported preserve-pitch, degraded speed-curve follow-speed, muted unsupported retime, and unsupported audio effect slots.
- Extended testkit audio parity with retime-specific source-sample mismatch and unsupported follow-speed categories.
- Strengthened `scripts/phase19-source-guards.sh --retiming-audio` to require audio_engine retime intent and audio parity coverage.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add audio DSP retime representation**
   - `1e19b09` test(19-05): add failing audio DSP retime tests
   - `76889fb` feat(19-05): implement audio DSP retime intent
2. **Task 2: Add audio parity and staged guard coverage**
   - `83cf60f` test(19-05): add failing audio retime parity tests
   - `05da7d2` feat(19-05): classify retimed audio parity

## Files Created/Modified

- `Cargo.lock` - Records the new workspace path dependency edge for `audio_engine`.
- `crates/audio_engine/Cargo.toml` - Adds the `engine_core` workspace dependency so DSP retime mapping consumes the canonical engine mapper.
- `crates/audio_engine/src/dsp_timeline.rs` - Evaluates retime source sample maps, gain source samples, follow-speed support facts, and typed diagnostics.
- `crates/audio_engine/src/mix_intent.rs` - Adds `AudioRetimeMixIntent`, `AudioRetimeSourceSampleMap`, sampled retime points, and support state.
- `crates/audio_engine/src/lib.rs` - Exports the new retime mix intent and diagnostic types.
- `crates/audio_engine/tests/dsp_timeline.rs` - Covers constant-speed, speed-curve, and unsupported preserve-pitch audio retime behavior.
- `crates/testkit/src/audio_parity.rs` - Compares retime source mapping and audio follow-speed support between preview mix intent and render graph export mixes.
- `crates/testkit/tests/audio_preview_export_parity.rs` - Covers retime source mismatch, unsupported follow-speed, and source guard coverage.
- `scripts/phase19-source-guards.sh` - Requires retiming-audio DSP and parity coverage.

## Decisions Made

- Reused `engine_core` retime mapping from `audio_engine` to keep retiming ownership on the same Rust-owned semantic path as video/render graph retime.
- Treated speed-curve audio follow-speed as degraded DSP intent until sample-accurate time-stretch output support is implemented.
- Treated non-1x preserve-pitch audio retime as unsupported typed diagnostic rather than parity success.

## Verification

- `cargo test -p audio_engine dsp_timeline -- --nocapture` - passed, 6 filtered tests.
- `cargo test -p testkit audio_preview_export_parity -- --nocapture` - passed, 6 filtered tests.
- `bash scripts/phase19-source-guards.sh --retiming-audio` - passed.

Known warning: `media_runtime_desktop` still reports the pre-existing macOS `tracksWithMediaType` deprecation during `testkit` builds.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added engine_core dependency to audio_engine**
- **Found during:** Task 1 (Add audio DSP retime representation)
- **Issue:** The plan did not list `crates/audio_engine/Cargo.toml` or `Cargo.lock`, but duplicating retime sample mapping inside `audio_engine` would violate the Phase 19 ownership rule that `engine_core` owns source-time mapping.
- **Fix:** Added a workspace path dependency on `engine_core` and used `source_position_for_retime` / `retimed_source_range` for DSP source sample mapping.
- **Files modified:** `crates/audio_engine/Cargo.toml`, `Cargo.lock`, `crates/audio_engine/src/dsp_timeline.rs`
- **Verification:** `cargo test -p audio_engine dsp_timeline -- --nocapture`
- **Committed in:** `76889fb`

---

**Total deviations:** 1 auto-fixed (1 Rule 2 missing critical)
**Impact on plan:** The adjustment preserves the intended production ownership boundary and avoids duplicate retime math. No external dependencies were added.

## Issues Encountered

- The initial Task 1 GREEN run passed the `dsp_timeline` filter without running the new tests because their names did not include `dsp_timeline`; the tests were renamed before commit so the required gate runs all retime cases.
- The retimed audio parity fixture initially reused a 2s material duration while assigning a 4s source range; the fixture duration was corrected to satisfy existing normalization validation.

## Known Stubs

None. Stub scan found no TODO/FIXME/placeholder or hardcoded empty UI data paths in the files modified by this plan.

## Threat Flags

None. The new trust-boundary work is the planned draft retime semantics to audio_engine/testkit parity surface covered by T-19-10A and T-19-10B.

## User Setup Required

None - no external service configuration required.

## TDD Gate Compliance

- RED commits exist for Task 1 and Task 2: `1e19b09`, `83cf60f`.
- GREEN commits exist after RED for each task: `76889fb`, `05da7d2`.

## Next Phase Readiness

Plan 19-06 can build on retime-aware audio DSP and parity diagnostics while transition/effect waves continue using typed Rust-owned semantic facts. PRODFX-01 now has draft, engine_core, render graph, compiler, preview, audio graph, parity, and source guard coverage.

## Self-Check: PASSED

- Found summary file at `.planning/phases/19-production-effects-retiming-and-transition-semantics/19-05-SUMMARY.md`.
- Verified commits exist: `1e19b09`, `76889fb`, `83cf60f`, `05da7d2`.
- Verified plan gates passed: `cargo test -p audio_engine dsp_timeline -- --nocapture`, `cargo test -p testkit audio_preview_export_parity -- --nocapture`, `bash scripts/phase19-source-guards.sh --retiming-audio`.

---
*Phase: 19-production-effects-retiming-and-transition-semantics*
*Completed: 2026-06-25*
