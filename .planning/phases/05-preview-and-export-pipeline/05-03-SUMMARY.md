---
phase: 05-preview-and-export-pipeline
plan: 03
subsystem: ffmpeg-compiler
tags: [rust, ffmpeg_compiler, ffmpeg, ass, render_graph, snapshots]

requires:
  - phase: 05-preview-and-export-pipeline
    provides: render_graph RenderGraphPlan and preview/export output profiles from Plan 05-02
provides:
  - Structured FfmpegJob data with inputs, OsString args, derived sidecars, encode settings, and validation expectations
  - Deterministic FFmpeg filter script generation with range-clipped source timing
  - Deterministic ASS text sidecars with font capability classification
affects: [preview_service, media_runtime, bindings_node, export, TEST-04]

tech-stack:
  added: [draft_model path dependency, render_graph path dependency, serde, engine_core dev dependency, serde_json dev dependency]
  patterns:
    - Structured FFmpeg jobs as derived Rust data, never shell strings
    - Compiler capabilities injected through CompileContext
    - ASS and filter scripts represented as sidecar definitions, not project state

key-files:
  created:
    - crates/ffmpeg_compiler/src/job.rs
    - crates/ffmpeg_compiler/src/filters.rs
    - crates/ffmpeg_compiler/src/ass.rs
    - crates/ffmpeg_compiler/tests/ffmpeg_job_snapshots.rs
    - crates/ffmpeg_compiler/tests/ass_snapshots.rs
    - crates/ffmpeg_compiler/tests/capability_snapshots.rs
    - crates/ffmpeg_compiler/tests/common/mod.rs
  modified:
    - Cargo.lock
    - crates/ffmpeg_compiler/Cargo.toml
    - crates/ffmpeg_compiler/src/lib.rs
    - crates/engine_core/src/text_layout.rs
    - crates/engine_core/src/lib.rs
    - crates/engine_core/tests/frame_state_snapshots.rs
    - crates/render_graph/src/lib.rs
    - crates/render_graph/tests/render_graph_snapshots.rs

key-decisions:
  - "Resolved text overlays now carry style color, stroke, shadow, and background so compiler output does not lose Jianying text semantics."
  - "FFmpeg filter scripts and ASS events are clipped to the output profile target range rather than using whole segment timeranges."
  - "CompileContext carries encoder/filter/font capabilities; ffmpeg_compiler classifies missing support without executing FFmpeg."

patterns-established:
  - "FfmpegJob args are Vec<OsString> with generated filter/ASS sidecars as derived artifact definitions."
  - "Compiler code may emit FFmpeg syntax, but it must not depend on media_runtime, desktop process execution, Electron, project_store, or .veproj persistence."
  - "Text font resolution checks VE_TEXT_FONT_PATH first, then pinned fallback candidates, and fails closed when none resolve."

requirements-completed: [TEXT-03, EXP-01, EXP-02, TEST-04]

duration: 21 min
completed: 2026-06-17
---

# Phase 05 Plan 03: FFmpeg Compiler Summary

**Structured FFmpeg job compilation with deterministic filter scripts, ASS sidecars, and classified capability failures**

## Performance

- **Duration:** 21 min
- **Started:** 2026-06-17T17:52:50Z
- **Completed:** 2026-06-17T18:13:08Z
- **Tasks:** 3
- **Files modified:** 15

## Accomplishments

- Added `compile_ffmpeg_job(RenderGraphPlan, CompileContext)` producing `FfmpegJob` with material inputs, `Vec<OsString>` arguments, derived filter/ASS sidecars, encode settings, output kind, and validation expectations.
- Added deterministic filter generation for preview frame, preview segment, and export MP4 profiles, including stable labels, overlay composition, audio trims/mixes, and output-range clipping.
- Added deterministic ASS generation with PingFang SC font policy, `VE_TEXT_FONT_PATH` resolution, Simplified Chinese-safe text escaping, style colors/stroke/shadow/background, and classified missing capability errors.
- Extended `engine_core::ResolvedTextOverlay` to preserve text style data so export/preview rendering does not drop Jianying text semantics.

## Task Commits

1. **Tasks 05-03-01..03:** `69c958f` feat(05-03): compile render graphs to ffmpeg jobs

## Files Created/Modified

- `crates/ffmpeg_compiler/src/job.rs` - Compile context, capability model, structured job/input/sidecar/output/validation/error types, and FFmpeg argument generation.
- `crates/ffmpeg_compiler/src/filters.rs` - Deterministic filter script generation with range-clipped video/audio source timing.
- `crates/ffmpeg_compiler/src/ass.rs` - Deterministic ASS sidecar generation and text font/filter capability checks.
- `crates/ffmpeg_compiler/tests/*` - Snapshot and classification coverage for jobs, filters, ASS sidecars, and capabilities.
- `crates/engine_core/src/text_layout.rs` - Resolved text overlays now carry style data needed by the compiler.
- `crates/render_graph/src/lib.rs` - Re-exported output codec/container types for compiler use.

## Decisions Made

- Used sidecar definitions for both filter scripts and ASS text rather than writing derived artifacts into `.veproj/project.json`.
- Kept compiler capability checks injected through `CompileContext` so actual runtime probing can remain in later runtime/service layers.
- Treated missing H.264/AAC/font/ASS support as classified compile errors instead of silently degrading text rendering or changing fonts.

## Deviations From Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Boundary Detail] Preserved text style in resolved overlays**
- **Found during:** ASS sidecar implementation
- **Issue:** `RenderTextOverlay` lacked color, stroke, shadow, and background data, so ASS output would have dropped text semantics.
- **Fix:** Added `ResolvedTextStyle` to `engine_core::ResolvedTextOverlay` and updated engine/render_graph snapshots.
- **Files modified:** `crates/engine_core/src/text_layout.rs`, `crates/engine_core/src/lib.rs`, `crates/engine_core/tests/frame_state_snapshots.rs`, `crates/render_graph/tests/render_graph_snapshots.rs`
- **Verification:** `cargo test -p engine_core -- --nocapture`, `cargo test -p render_graph -- --nocapture`
- **Committed in:** `69c958f`

**2. [Rule 1 - Implementation Bug] Clipped compiler timing to output target range**
- **Found during:** self-review after initial filter/ASS tests passed
- **Issue:** Initial filter and ASS generation used whole segment source/target ranges, which would render from the wrong media time for partial preview/export ranges.
- **Fix:** Added target-range clipping for video/audio filter trims and ASS event start/end times.
- **Files modified:** `crates/ffmpeg_compiler/src/filters.rs`, `crates/ffmpeg_compiler/src/ass.rs`, `crates/ffmpeg_compiler/tests/*`
- **Verification:** `cargo test -p ffmpeg_compiler -- --nocapture`
- **Committed in:** `69c958f`

---

**Total deviations:** 2 auto-fixed (1 missing critical detail, 1 implementation bug)
**Impact on plan:** Both fixes tightened the planned compiler contract and prevent preview/export timing and text-style drift.

## Issues Encountered

- A string split over `-text-text-a` incorrectly recovered segment ID `a`; fixed by adding explicit `segment_id` metadata to ASS sidecars.
- The initial test assertion required preview and export filter scripts to be byte-identical; corrected it because output dimensions and sidecar paths are profile-specific while the compiler path remains shared.
- The local working tree still contains unrelated root `Cargo.toml` license changes and untracked reference files. They were not staged or modified by this plan.

## Verification

- `cargo test -p ffmpeg_compiler ffmpeg_job -- --nocapture` - PASS
- `cargo test -p ffmpeg_compiler ass -- --nocapture` - PASS
- `cargo test -p ffmpeg_compiler filters -- --nocapture` - PASS
- `cargo test -p ffmpeg_compiler capability -- --nocapture` - PASS
- `cargo test -p ffmpeg_compiler -- --nocapture` - PASS
- `cargo test -p engine_core -- --nocapture` - PASS
- `cargo test -p render_graph -- --nocapture` - PASS
- `cargo check -p ffmpeg_compiler --locked` - PASS
- `cargo fmt --all --check` - PASS
- Boundary scan for `media_runtime`, desktop process execution, Electron, project_store, preview cache, and `.veproj` persistence in `crates/ffmpeg_compiler` - PASS

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Plan 05-04 and Plan 05-07. `preview_service` can now compile preview frame/segment artifacts from the shared render graph, and `media_runtime` can consume structured `FfmpegJob` data without deciding editing semantics.

---
*Phase: 05-preview-and-export-pipeline*
*Completed: 2026-06-17*
