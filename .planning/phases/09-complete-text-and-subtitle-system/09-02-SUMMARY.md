---
phase: 09-complete-text-and-subtitle-system
plan: 02
subsystem: render-pipeline
tags: [rust, engine-core, render-graph, ffmpeg, ass, text, subtitle]
requires:
  - phase: 09-complete-text-and-subtitle-system
    provides: Complete defaulted TextSegment schema from Plan 09-01
provides:
  - Deterministic engine_core text layout metrics from segment-level text box and layout region fields
  - Render graph text/subtitle overlays carrying source, font refs, line metrics, spacing, layout, and unsupported diagnostics
  - ASS sidecar compilation for supported static text styling, margins, background, line breaks, and letter spacing
  - Classified compiler errors for unsupported text font refs, bubbles, and effects
affects: [engine-core, render-graph, ffmpeg-compiler, phase-09-subtitle-import, phase-09-ui]
tech-stack:
  added: []
  patterns: [tdd-red-green, segment-level-text-layout, explicit-unsupported-text-resource]
key-files:
  created:
    - .planning/phases/09-complete-text-and-subtitle-system/09-02-SUMMARY.md
  modified:
    - crates/engine_core/src/text_layout.rs
    - crates/engine_core/tests/canvas_profile.rs
    - crates/engine_core/tests/frame_state_snapshots.rs
    - crates/engine_core/tests/normalization.rs
    - crates/render_graph/tests/render_graph_snapshots.rs
    - crates/ffmpeg_compiler/src/ass.rs
    - crates/ffmpeg_compiler/src/job.rs
    - crates/ffmpeg_compiler/tests/ass_snapshots.rs
    - crates/ffmpeg_compiler/tests/capability_snapshots.rs
    - crates/ffmpeg_compiler/tests/common/mod.rs
key-decisions:
  - "Segment-level text box and layout region fields take precedence over profile safe-area defaults during frame-state resolution."
  - "Unsupported text font refs, bubbles, and effects stop ASS compilation with an explicit UnsupportedTextResource error instead of silently approximating proprietary resources."
patterns-established:
  - "Render graph can preserve complete text intent by carrying engine_core FrameTextOverlay data without FFmpeg syntax."
  - "ASS sidecars include trace comments for text box, layout region, and line height while compiling only supported ASS fields into style/dialogue data."
requirements-completed: [TEXT2-01, TEXT2-02, TEXT2-03]
duration: 10 min
completed: 2026-06-18
---

# Phase 09 Plan 02: Text Engine And ASS Propagation Summary

**Segment-level text/subtitle layout propagation with ASS sidecars for the supported static subset and explicit unsupported resource errors.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-18T03:50:15Z
- **Completed:** 2026-06-18T04:00:38Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Extended `ResolvedTextOverlay` with text/subtitle source, font refs, resolved text box/layout region pixels, derived margins, wrapping, line height, letter spacing, and unsupported bubble/effect diagnostics.
- Kept render graph text overlays compiler-agnostic while preserving the complete engine-resolved text intent and diagnostics.
- Compiled ASS sidecars with deterministic font resolution, colors, stroke, shadow, background border style, layout-region margins, escaped line breaks, and letter spacing.
- Added compiler classification for unsupported text `fontRef`, proprietary bubble refs, and 花字/effect refs.

## Task Commits

1. **Task 09-02-01: Resolve complete text layout in engine_core and render_graph** - `8d92c0a` (test RED), `773fcdb` (feat GREEN)
2. **Task 09-02-02: Compile supported text fields into ASS sidecars** - `5f2b4ee` (test RED), `beea4ce` (feat GREEN)

## Files Created/Modified

- `crates/engine_core/src/text_layout.rs` - Resolves segment text layout fields into deterministic overlay metrics and unsupported diagnostics.
- `crates/engine_core/tests/frame_state_snapshots.rs` - Covers multiple text/subtitle overlays, layout metrics, spacing, line height, and unsupported diagnostics.
- `crates/engine_core/tests/canvas_profile.rs` - Verifies segment-level layout precedence over profile safe-area defaults.
- `crates/engine_core/tests/normalization.rs` - Updates text test fixture literals for the complete 09-01 schema.
- `crates/render_graph/tests/render_graph_snapshots.rs` - Verifies render graph preservation of text/subtitle intent and absence of FFmpeg syntax.
- `crates/ffmpeg_compiler/src/ass.rs` - Compiles supported ASS text fields and rejects unsupported proprietary text resources.
- `crates/ffmpeg_compiler/src/job.rs` - Adds `UnsupportedTextResource` compiler error classification.
- `crates/ffmpeg_compiler/tests/ass_snapshots.rs` - Covers ASS layout margins, background, spacing, line breaks, and trace comments.
- `crates/ffmpeg_compiler/tests/capability_snapshots.rs` - Covers unsupported font ref, bubble, and effect classification.
- `crates/ffmpeg_compiler/tests/common/mod.rs` - Adds complete text fixture semantics and unsupported-resource plan helper.

## Decisions Made

- Segment `TextSegment.textBox` and `TextSegment.layoutRegion` now drive resolved text layout before profile defaults; profile text layout remains the deterministic fallback/policy container.
- Unsupported proprietary text resources are compiler errors for ASS output. This avoids exporting a video that appears to support a bubble/effect/font ref that the local compiler cannot render.
- ASS sidecars carry non-rendering comments for line height, text box, and layout region so derived artifacts remain debuggable without pretending unsupported ASS features exist.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated stale text test fixture literals**
- **Found during:** Task 09-02-01 RED verification
- **Issue:** `engine_core` test fixtures in `canvas_profile.rs` and `normalization.rs` still used pre-09-01 `TextSegment` and `TextStyle` struct literals, preventing the targeted text tests from compiling.
- **Fix:** Added defaulted 09-01 text fields to those fixtures without changing their behavioral assertions.
- **Files modified:** `crates/engine_core/tests/canvas_profile.rs`, `crates/engine_core/tests/normalization.rs`
- **Verification:** `cargo test -p engine_core text -- --nocapture`
- **Committed in:** `8d92c0a`

**2. [Rule 3 - Blocking] Ensured ASS snapshot runs under the plan's filtered command**
- **Found during:** Task 09-02-02 GREEN verification
- **Issue:** The ASS snapshot test name did not include `text`, so `cargo test -p ffmpeg_compiler text -- --nocapture` compiled it but filtered it out.
- **Fix:** Renamed the test to include `text`, making the plan gate execute the ASS assertions.
- **Files modified:** `crates/ffmpeg_compiler/tests/ass_snapshots.rs`
- **Verification:** `cargo test -p ffmpeg_compiler text -- --nocapture`
- **Committed in:** `beea4ce`

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both fixes made the planned verification executable. No feature scope was added beyond the plan.

## Known Stubs

None.

## Threat Flags

None.

## Issues Encountered

- `cargo fmt` reformatted several draft-model files outside this plan. Those formatting-only changes were discarded for the specific files so Plan 09-02 stayed scoped.
- Some `gsd-tools query` state handlers could not parse this repo's current `Plan: 1/5` STATE shape or non-checkbox TEXT2 requirements. Legacy CLI handlers were used where possible, then planning docs were repaired narrowly to keep STATE/ROADMAP/REQUIREMENTS consistent.

## Verification

- `cargo test -p engine_core text -- --nocapture` - passed
- `cargo test -p render_graph text -- --nocapture` - passed
- `cargo test -p ffmpeg_compiler text -- --nocapture` - passed

## User Setup Required

None.

## Next Phase Readiness

Phase 09 Plan 03 can build subtitle SRT import on top of the shared text segment path; imported subtitles will now resolve through the same frame-state, render graph, and ASS sidecar pipeline as normal text segments.

## Self-Check: PASSED

- Found `.planning/phases/09-complete-text-and-subtitle-system/09-02-SUMMARY.md`.
- Found key implementation files `crates/engine_core/src/text_layout.rs` and `crates/ffmpeg_compiler/src/ass.rs`.
- Found task commits `8d92c0a`, `773fcdb`, `5f2b4ee`, and `beea4ce`.
- No tracked file deletions were introduced.

---
*Phase: 09-complete-text-and-subtitle-system*
*Completed: 2026-06-18*
