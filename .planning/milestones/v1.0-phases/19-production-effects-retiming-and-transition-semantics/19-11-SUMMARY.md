---
phase: 19-production-effects-retiming-and-transition-semantics
plan: "11"
subsystem: rendering
tags: [rust, render-graph, realtime-preview, ffmpeg-compiler, mask, blend]
requires:
  - phase: 19-production-effects-retiming-and-transition-semantics
    provides: "19-10 mask/blend command semantics and draft fields"
provides:
  - "Typed mask/blend render graph intents and semantic fingerprints"
  - "Native/GPU realtime preview mask/blend pass evidence"
  - "FFmpeg compiler-owned first-party mask alpha export filters"
  - "Typed unsupported diagnostics and failed product-success metadata for unsupported mask/blend export paths"
  - "Phase 19 source guards for mask/blend compiler and preview ownership"
affects: [render_graph, realtime_preview_runtime, ffmpeg_compiler, phase19-source-guards]
tech-stack:
  added: []
  patterns:
    - "Rust render graph carries typed visual semantics; compiler emits export filters or diagnostics"
    - "Unsupported visual export paths remain explicit diagnostics and cannot be product-successful"
key-files:
  created:
    - ".planning/phases/19-production-effects-retiming-and-transition-semantics/19-11-SUMMARY.md"
  modified:
    - "crates/render_graph/src/graph.rs"
    - "crates/render_graph/src/fingerprint.rs"
    - "crates/render_graph/src/lib.rs"
    - "crates/render_graph/tests/production_effects.rs"
    - "crates/realtime_preview_runtime/src/effects.rs"
    - "crates/realtime_preview_runtime/src/gpu/compositor.rs"
    - "crates/realtime_preview_runtime/src/gpu/pipelines.rs"
    - "crates/realtime_preview_runtime/tests/production_effects.rs"
    - "crates/realtime_preview_runtime/tests/gpu_subset.rs"
    - "crates/ffmpeg_compiler/src/effects.rs"
    - "crates/ffmpeg_compiler/src/filters.rs"
    - "crates/ffmpeg_compiler/src/job.rs"
    - "crates/ffmpeg_compiler/tests/production_effects.rs"
    - "scripts/phase19-source-guards.sh"
key-decisions:
  - "First-party rectangle/ellipse masks compile in ffmpeg_compiler as RGBA alpha expressions derived from typed render graph intent."
  - "Multiply/screen blend export remains unsupported until an alpha-correct FFmpeg blend compositor is implemented; jobs carry unsupported diagnostics and product-success metadata is false."
  - "External mask/blend provider IDs stay out of filter scripts and appear only in typed diagnostics."
patterns-established:
  - "Mask/blend preview evidence must come from realtime_preview_runtime native/GPU passes."
  - "Compiler diagnostics that affect product success are attached to FfmpegJob.visual_diagnostics."
requirements-completed: [PRODFX-04]
duration: 26min
completed: 2026-06-25
status: complete
---

# Phase 19 Plan 11: Mask/Blend Semantics Summary

**Mask/blend semantics now flow through graph fingerprints, native/GPU preview, compiler-owned mask export, unsupported blend diagnostics, and source-boundary guards.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-06-25T11:54:38Z
- **Completed:** 2026-06-25T12:20:36Z
- **Tasks:** 2
- **Files modified:** 15

## Accomplishments

- Added `RenderMaskIntent` and `RenderBlendIntent` to video layers/text overlays, including semantic fingerprint participation for mask geometry, opacity, feathering, inversion, and blend mode.
- Added realtime preview mask/blend pass evidence through the native/GPU compositor path, including mask uniforms and normal/multiply/screen WGPU blend pipelines.
- Added FFmpeg compiler-owned first-party mask alpha filters for rectangle/ellipse masks.
- Classified non-normal blend export and external mask/blend references as typed unsupported diagnostics, with compiled product-success metadata set false.
- Extended `scripts/phase19-source-guards.sh --mask-blend` to require graph, GPU preview, compiler, diagnostics, and source ownership evidence.

## Task Commits

1. **Task 1 RED: Add failing mask blend preview tests** - `c7604c3`
2. **Task 1 GREEN: Implement mask blend graph preview support** - `fe5959c`
3. **Task 2 RED: Add failing mask blend export tests** - `7b32742`
4. **Task 2 GREEN: Implement mask blend export diagnostics** - `a6c9d5e`

## Files Created/Modified

- `crates/render_graph/src/graph.rs` - Added typed mask/blend render intents and diagnostics wiring.
- `crates/render_graph/src/fingerprint.rs` - Included mask/blend semantics in graph fingerprints.
- `crates/render_graph/src/lib.rs` - Exported mask/blend render intent types.
- `crates/render_graph/tests/production_effects.rs` - Covered mask/blend graph intent and fingerprint invalidation.
- `crates/realtime_preview_runtime/src/effects.rs` - Added mask/blend preview pass construction and GPU pass diagnostics.
- `crates/realtime_preview_runtime/src/gpu/compositor.rs` - Added mask alpha shader logic and blend-mode pipeline selection.
- `crates/realtime_preview_runtime/src/gpu/pipelines.rs` - Added mask/blend pipeline metadata markers.
- `crates/realtime_preview_runtime/tests/production_effects.rs` - Covered first-party GPU mask/blend preview and external rejection.
- `crates/realtime_preview_runtime/tests/gpu_subset.rs` - Updated graph fixtures for new layer intent fields.
- `crates/ffmpeg_compiler/src/effects.rs` - Added compiler-owned mask alpha filters and mask/blend export diagnostics.
- `crates/ffmpeg_compiler/src/filters.rs` - Applied first-party mask filters in the filtergraph builder.
- `crates/ffmpeg_compiler/src/job.rs` - Surfaced mask/blend diagnostics on `FfmpegJob` and set product-success metadata false for unsupported visual export paths.
- `crates/ffmpeg_compiler/tests/production_effects.rs` - Covered mask alpha export, unsupported blend diagnostics, and external reference diagnostics.
- `scripts/phase19-source-guards.sh` - Added mask/blend guard requirements and non-compiler FFmpeg mask/blend ownership scan.

## Decisions Made

- First-party masks are implemented in the compiler with `format=rgba` plus `geq` alpha expressions so mask pixels are not evaluated by UI or renderer code.
- Multiply/screen export is explicitly unsupported for now because correct output needs alpha-aware blend compositing, not a normal overlay fallback.
- Unsupported visual export diagnostics make compile validation metadata false (`must_exist=false`, `must_be_non_empty=false`) so unsupported mask/blend output is not represented as product-successful.
- Text overlay mask/blend export is classified unsupported in the compiler because the current ASS subtitle path cannot apply those visual compositing semantics.

## Deviations from Plan

### Auto-Fixed Issues

**1. [Rule 3 - Blocking] Updated support files required by new graph fields**
- **Found during:** Task 1
- **Issue:** Adding mask/blend fields to render graph layers required public exports and fixture updates outside the task file list.
- **Fix:** Exported the new render intent types from `crates/render_graph/src/lib.rs` and updated `crates/realtime_preview_runtime/tests/gpu_subset.rs`.
- **Files modified:** `crates/render_graph/src/lib.rs`, `crates/realtime_preview_runtime/tests/gpu_subset.rs`
- **Verification:** `cargo test -p render_graph production_effects -- --nocapture`; `cargo test -p realtime_preview_runtime production_effects -- --nocapture`
- **Committed in:** `fe5959c`

**2. [Rule 2 - Missing Critical] Surfaced compiler diagnostics on product job metadata**
- **Found during:** Task 2
- **Issue:** Unsupported blend/export diagnostics needed to reach `FfmpegJob.visual_diagnostics` and make product-success metadata false; helper-only diagnostics would not satisfy the plan.
- **Fix:** Added mask/blend export diagnostics to `crates/ffmpeg_compiler/src/job.rs` and made unsupported visual diagnostics set compile validation `must_exist=false` and `must_be_non_empty=false`.
- **Files modified:** `crates/ffmpeg_compiler/src/job.rs`
- **Verification:** `cargo test -p ffmpeg_compiler production_effects -- --nocapture`; `bash scripts/phase19-source-guards.sh --mask-blend`
- **Committed in:** `a6c9d5e`

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 missing critical)
**Impact on plan:** Both were required to keep the typed Rust ownership boundary correct. No UI fallback or renderer-owned mask/blend evaluation was introduced.

## Issues Encountered

None beyond the documented deviations. The planned TDD RED/GREEN gates failed and passed as expected.

## Known Stubs

None. Stub scan found no blocking placeholder/TODO/FIXME or hardcoded empty UI data introduced by this plan.

## Auth Gates

None.

## User Setup Required

None.

## Verification

- `cargo test -p render_graph production_effects -- --nocapture` - passed
- `cargo test -p realtime_preview_runtime production_effects -- --nocapture` - passed
- `cargo test -p ffmpeg_compiler production_effects -- --nocapture` - passed
- `bash scripts/phase19-source-guards.sh --mask-blend` - passed
- FFmpeg `geq` mask expression syntax smoke check - passed

## Next Phase Readiness

PRODFX-04 mask/blend semantics are ready for downstream validation. Future export work can replace the current non-normal blend unsupported diagnostic with an alpha-correct FFmpeg compositor without changing UI ownership boundaries.

## Self-Check: PASSED

- Summary file exists.
- Key modified files exist.
- Task commits found: `c7604c3`, `fe5959c`, `7b32742`, `a6c9d5e`.
- Required verification commands passed.

---
*Phase: 19-production-effects-retiming-and-transition-semantics*
*Completed: 2026-06-25*
