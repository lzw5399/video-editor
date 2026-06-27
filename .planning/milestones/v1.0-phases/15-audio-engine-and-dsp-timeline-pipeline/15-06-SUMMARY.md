---
phase: 15-audio-engine-and-dsp-timeline-pipeline
plan: "06"
subsystem: audio
tags: [rust, audio-engine, render-graph, ffmpeg-compiler, testkit, parity]

requires:
  - phase: 15-audio-engine-and-dsp-timeline-pipeline
    provides: AudioMixIntent and DSP timeline semantics from Plan 15-02
  - phase: 05-preview-and-export-pipeline
    provides: render_graph and ffmpeg_compiler export path
provides:
  - RenderAudioMix fields for gain, pan, fades, volume keyframes, effect slots, and classification
  - FFmpeg compiler-owned audio filter generation for Rust-owned audio mix intent
  - Typed audio preview/export parity diagnostics and sample summaries in testkit
affects: [audio_engine, engine_core, render_graph, ffmpeg_compiler, testkit, phase-16]

tech-stack:
  added: []
  patterns:
    - Accepted SegmentAudio is normalized through engine_core before render graph export intent
    - FFmpeg audio syntax remains compiler-owned and emits typed diagnostics for unsupported effect slots
    - Audio parity tests compare typed summaries before media-binary rendering

key-files:
  created:
    - crates/testkit/src/audio_parity.rs
    - crates/testkit/tests/audio_preview_export_parity.rs
    - .planning/phases/15-audio-engine-and-dsp-timeline-pipeline/15-06-SUMMARY.md
  modified:
    - Cargo.lock
    - crates/engine_core/src/normalize.rs
    - crates/render_graph/src/graph.rs
    - crates/render_graph/src/lib.rs
    - crates/render_graph/src/fingerprint.rs
    - crates/render_graph/tests/render_graph_snapshots.rs
    - crates/ffmpeg_compiler/src/filters.rs
    - crates/ffmpeg_compiler/src/job.rs
    - crates/ffmpeg_compiler/tests/common/mod.rs
    - crates/ffmpeg_compiler/tests/ffmpeg_job_snapshots.rs
    - crates/testkit/Cargo.toml
    - crates/testkit/src/lib.rs

key-decisions:
  - "Render graph reads accepted SegmentAudio through engine_core normalization instead of depending directly on audio_engine, avoiding the audio_engine -> realtime_preview_runtime -> render_graph crate cycle."
  - "FFmpeg audio filter strings remain localized to ffmpeg_compiler; renderer code receives no audio FFmpeg syntax."
  - "Audio preview/export parity diagnostics live in testkit as typed Rust data comparing AudioMixIntent and RenderAudioMix without raw sample buffers."

patterns-established:
  - "RenderAudioMix carries compiler-ready audio facts while preserving existing volumeLevelMillis compatibility."
  - "Audio parity diagnostics classify semantic differences rather than collapsing them into generic failures."

requirements-completed: [AUDIO2-04]

duration: 14 min
completed: 2026-06-19
---

# Phase 15 Plan 06: Audio Export Mix And Parity Summary

**Rust-owned audio mix intent now reaches render graph and FFmpeg export compilation with typed preview/export parity diagnostics**

## Performance

- **Duration:** 14 min
- **Started:** 2026-06-19T10:09:39Z
- **Completed:** 2026-06-19T10:23:29Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments

- Added render graph audio mix facts for gain, pan, fade-in/out, volume keyframes, effect-slot classification, and mix classification.
- Updated FFmpeg compiler audio filter generation to consume `RenderAudioMix` typed fields for volume, pan, fades, and unsupported effect diagnostics.
- Added deterministic testkit audio parity diagnostics comparing preview `AudioMixIntent` and export `RenderAudioMix` summaries without FFmpeg media rendering.

## Task Commits

1. **Task 15-06-01 RED:** `fccaf8a` (test) add failing audio export mix tests.
2. **Task 15-06-01 GREEN:** `a2a830c` (feat) compile Rust-owned audio mix intent for export.
3. **Task 15-06-02 RED:** `4ababa9` (test) add failing audio parity diagnostics tests.
4. **Task 15-06-02 GREEN:** `59b3e99` (feat) add typed audio parity diagnostics.

## Verification

- `cargo test -p render_graph audio -- --nocapture` - passed; ran 2 focused render graph audio tests.
- `cargo test -p ffmpeg_compiler audio -- --nocapture` - passed; ran 2 focused FFmpeg compiler audio tests.
- `cargo test -p testkit audio_preview_export_parity -- --nocapture` - passed; ran 3 audio parity tests.
- `cargo test -p testkit preview_export_parity -- --nocapture` - passed; ran 4 existing preview/export parity tests plus the filtered audio parity tests.
- `rg -n "filter_complex|atrim|amix|volume=|pan=" apps/desktop-electron/src` - no matches.

## Files Created/Modified

- `crates/engine_core/src/normalize.rs` - Carries accepted `SegmentAudio` through normalized segments for downstream render/export intent.
- `crates/render_graph/src/graph.rs` - Adds `RenderAudioMix` audio intent fields, effect-slot classifications, and diagnostics.
- `crates/render_graph/src/fingerprint.rs` - Includes new audio mix facts in node semantic fingerprints.
- `crates/render_graph/src/lib.rs` - Exports new audio mix parity/diagnostic types.
- `crates/render_graph/tests/render_graph_snapshots.rs` - Covers gain, pan, fades, keyframes, effect slots, and absence of FFmpeg syntax in render graph snapshots.
- `crates/ffmpeg_compiler/src/filters.rs` - Compiles audio gain, pan, and fades into FFmpeg filters and emits effect-slot diagnostics.
- `crates/ffmpeg_compiler/src/job.rs` - Carries filter-script audio diagnostics on compiled jobs.
- `crates/ffmpeg_compiler/tests/common/mod.rs` - Adds deterministic audio mix fixture facts.
- `crates/ffmpeg_compiler/tests/ffmpeg_job_snapshots.rs` - Covers compiler-owned audio filter output and unsupported effect diagnostics.
- `crates/testkit/Cargo.toml` and `Cargo.lock` - Adds direct testkit dependencies needed for typed audio parity helpers.
- `crates/testkit/src/audio_parity.rs` - Defines typed audio sample summaries, parity status, differences, and diagnostic comparison.
- `crates/testkit/src/lib.rs` - Exports audio parity helper types and function.
- `crates/testkit/tests/audio_preview_export_parity.rs` - Covers matching parity, unsupported effect slots, sample-rate mismatch, missing material, export-only, preview-only, and muted-track silence cases.

## Decisions Made

- Avoided a direct `render_graph -> audio_engine` dependency because it would create a crate cycle through `audio_engine -> realtime_preview_runtime -> render_graph`.
- Kept render graph as the export intent carrier and FFmpeg compiler as the only owner of audio filter strings.
- Kept parity diagnostics in testkit as reusable typed Rust data suitable for later UI/export status transport without exposing raw samples or FFmpeg filters.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Avoided render_graph/audio_engine crate cycle**
- **Found during:** Task 15-06-01 GREEN
- **Issue:** A direct `render_graph` dependency on `audio_engine::AudioMixIntent` would cycle through `audio_engine -> realtime_preview_runtime -> render_graph`.
- **Fix:** Added accepted `SegmentAudio` to `engine_core::NormalizedSegment` and mapped those Rust-owned semantics into `RenderAudioMix`.
- **Files modified:** `crates/engine_core/src/normalize.rs`, `crates/render_graph/src/graph.rs`
- **Verification:** `cargo test -p render_graph audio -- --nocapture` and `cargo test -p ffmpeg_compiler audio -- --nocapture` passed.
- **Committed in:** `a2a830c`

---

**Total deviations:** 1 auto-fixed (1 Rule 3 blocking issue).
**Impact on plan:** Preserved the intended Rust-owned audio/export path without introducing a cyclic crate dependency or moving semantics into UI code.

## Issues Encountered

- An initial direct `rustfmt` invocation lacked the workspace edition and failed on existing Rust 2024 let-chain syntax; reran formatting with `rustfmt --edition 2024` for only plan-touched files.
- A broad `cargo fmt` invocation briefly formatted unrelated files; those specific unintended changes were reverted before any commit.

## Known Stubs

None - stub scan found only format-string literals and existing error text, not incomplete audio/export behavior.

## Authentication Gates

None.

## Threat Flags

None - modified trust-boundary surfaces are the planned audio mix intent, render graph to FFmpeg compiler, and typed parity diagnostic boundaries covered by T-15-06-01 through T-15-06-04.

## TDD Gate Compliance

- RED commits present: `fccaf8a`, `4ababa9`.
- GREEN commits present after RED: `a2a830c`, `59b3e99`.
- Refactor commits: none needed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 16 can consume typed audio parity diagnostics and compiler-owned filter diagnostics for scheduler/export status without adding renderer-owned audio graph or FFmpeg semantics.

## Self-Check: PASSED

- Verified key created files exist on disk.
- Verified commits `fccaf8a`, `a2a830c`, `4ababa9`, and `59b3e99` exist in git history.
- Re-ran all plan-level automated verification commands successfully.

---
*Phase: 15-audio-engine-and-dsp-timeline-pipeline*
*Completed: 2026-06-19*
