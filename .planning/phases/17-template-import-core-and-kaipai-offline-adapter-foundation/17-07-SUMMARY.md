---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
plan: "07"
subsystem: export-rendering
tags: [ffmpeg-compiler, render-graph, rotation, preview-export-parity, no-fallback]

requires:
  - phase: 17-01
    provides: [provider-neutral adaptation report contracts, Phase 17 source guards]
provides:
  - Generic static center-anchor rotation support in FFmpeg export/preview compilation
  - Rotation filtergraph snapshots with explicit supported and unsupported diagnostic cases
  - FFmpeg-backed preview/export pixel evidence for rotation and layer ordering
affects: [template-import, ffmpeg-compiler, render-graph, preview-export-parity, adapter-kaipai]

tech-stack:
  added: []
  patterns:
    - Generic FFmpeg `rotate` filter stage after scale and before opacity
    - Rotated layer bounds drive overlay placement for center-anchor transforms
    - Unsupported rotation diagnostics are reserved for animated or non-center-anchor cases

key-files:
  created:
    - .planning/phases/17-template-import-core-and-kaipai-offline-adapter-foundation/17-07-SUMMARY.md
  modified:
    - crates/ffmpeg_compiler/src/filters.rs
    - crates/ffmpeg_compiler/tests/transform_snapshots.rs
    - crates/testkit/tests/preview_export_parity.rs
    - crates/render_graph/src/graph.rs

key-decisions:
  - "Static center-anchor rotation is compiled generically in ffmpeg_compiler instead of adapter/provider code."
  - "Render graph diagnostics no longer classify supported static center-anchor rotation as unsupported."
  - "Non-center-anchor rotation and animated visual rotation remain explicit unsupported diagnostics."

patterns-established:
  - "Rotation parity tests must sample visual pixels for ignored rotation and layer ordering, not only assert non-empty output."
  - "Provider-specific terms stay out of ffmpeg_compiler and render_graph; Phase 17 source guards enforce the boundary."

requirements-completed: [PRODFX-05, NO-FALLBACK-01]

duration: 11 min
completed: 2026-06-24
status: complete
---

# Phase 17 Plan 07: Static Rotation Export Parity Summary

**Generic FFmpeg static center-anchor rotation with snapshot diagnostics and preview/export pixel evidence for rotated layer order.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-06-24T08:10:27Z
- **Completed:** 2026-06-24T08:21:27Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added TDD RED tests proving current export ignored static rotation and reported it unsupported.
- Implemented generic FFmpeg `rotate` filter generation with rotated bounds used for layer placement.
- Added parity evidence that samples rotated overlay pixels and a top overlay pixel to catch ignored rotation and wrong layer ordering.
- Kept animated visual rotation and non-center-anchor static rotation explicitly classified as unsupported.

## Task Commits

1. **Task 1: Add generic rotation export parity tests** - `0dd018c` (test)
2. **Task 2: Implement generic static rotation in export compiler** - `2e080fe` (feat)

## Files Created/Modified

- `crates/ffmpeg_compiler/src/filters.rs` - Adds static rotation filter generation, alpha-safe rotation, rotated bounds, and placement adjustment.
- `crates/ffmpeg_compiler/tests/transform_snapshots.rs` - Adds static center-anchor rotation snapshot coverage and non-center-anchor diagnostic coverage.
- `crates/testkit/tests/preview_export_parity.rs` - Adds FFmpeg-backed visual pixel evidence for rotated overlay output and layer order.
- `crates/render_graph/src/graph.rs` - Updates diagnostics so supported static center-anchor rotation is not reported unsupported.

## Decisions Made

- Implemented rotation in shared compiler semantics, not in Kaipai/provider import code.
- Limited supported static rotation diagnostics to center-anchor semantics; non-center anchors remain explicit unsupported diagnostics.
- Used generated local test media and sampled RGB evidence to prove visual rotation and stacking behavior.

## Verification

All verification passed:

- `cargo test -p ffmpeg_compiler transform -- --nocapture`
- `cargo test -p testkit preview_export_parity -- --nocapture`
- `pnpm run test:phase17-source-guards`

RED evidence before implementation:

- `cargo test -p ffmpeg_compiler transform_snapshot_compiles_static_center_anchor_rotation_and_preserves_layer_order -- --nocapture` failed because static rotation was classified as unsupported.
- `cargo test -p testkit preview_export_parity_static_rotation_samples_visual_layer_order -- --nocapture` failed because ignored rotation sampled red where the rotated result expected green.

Warnings observed:

- Existing Rust warning in `media_runtime_desktop` for deprecated `AVAsset::tracksWithMediaType`.
- Existing pnpm Node engine warning: project wants Node `24.12.0`, current runtime is `24.15.0`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated render graph rotation diagnostics at the source**
- **Found during:** Task 2 (Implement generic static rotation in export compiler)
- **Issue:** The unsupported static rotation diagnostic originates in `render_graph`, before `ffmpeg_compiler` returns the compiled job. Leaving it there would still classify supported static center-anchor rotation as unsupported.
- **Fix:** Updated `crates/render_graph/src/graph.rs` so center-anchor static rotation is no longer unsupported, while non-center-anchor rotation remains an explicit `rotationAnchor` unsupported diagnostic.
- **Files modified:** `crates/render_graph/src/graph.rs`, `crates/ffmpeg_compiler/tests/transform_snapshots.rs`
- **Verification:** `cargo test -p ffmpeg_compiler transform -- --nocapture`
- **Committed in:** `2e080fe`

**Total deviations:** 1 auto-fixed (1 missing critical functionality).
**Impact on plan:** The deviation is required for the planned diagnostic behavior; no provider-specific or adapter code was introduced.

## Issues Encountered

- The first parity RED fixture accidentally let the top overlay use default full-canvas stretch, so the sample hit the top overlay instead of the rotated lower overlay. The fixture was corrected before the RED commit by using canonical `Fit` plus explicit scale values.
- No blockers remain.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Stub scan found no placeholder/TODO/empty-data patterns in files touched by this plan.

## Threat Flags

None. The plan modifies existing compiler/render graph semantics and tests; it does not add new network endpoints, auth paths, file-access trust boundaries, or schema persistence surfaces.

## TDD Gate Compliance

- RED commit present: `0dd018c`
- GREEN commit present after RED: `2e080fe`
- No refactor commit was needed.

## Next Phase Readiness

Ready for downstream Phase 17 import plans to classify static center-anchor rotated overlays as supported when all other required resources and semantics are supported. Provider-specific Kaipai logic still must stay outside `ffmpeg_compiler`.

## Self-Check: PASSED

- Key files exist: `crates/ffmpeg_compiler/src/filters.rs`, `crates/ffmpeg_compiler/tests/transform_snapshots.rs`, `crates/testkit/tests/preview_export_parity.rs`, and `crates/render_graph/src/graph.rs`.
- Task commits `0dd018c` and `2e080fe` exist in git history.
- Plan-level verification commands passed after the final task commit.

---
*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Completed: 2026-06-24*
