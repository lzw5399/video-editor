---
phase: 11-realtime-preview-runtime-and-gpu-render-backend
plan: 02
subsystem: realtime-preview-runtime
tags: [rust, realtime-preview, render-graph, diagnostics, tdd]

requires:
  - phase: 11-realtime-preview-runtime-and-gpu-render-backend
    provides: realtime preview runtime session, request, clock, telemetry contracts from 11-01
provides:
  - Renderer-neutral realtime preview graph preparation through engine_core and render_graph
  - Supported/degraded/unsupported realtime graph capability reports
  - Serializable preview/export parity diagnostics
affects: [phase-11, realtime-preview-runtime, render-graph, preview-export-parity]

tech-stack:
  added: []
  patterns:
    - TDD RED/GREEN commits for runtime graph preparation and capability diagnostics
    - Runtime capability classification over render_graph intent before backend execution

key-files:
  created:
    - crates/realtime_preview_runtime/src/graph_prepare.rs
    - crates/realtime_preview_runtime/src/capabilities.rs
    - crates/realtime_preview_runtime/src/parity.rs
    - crates/realtime_preview_runtime/tests/capability_matrix.rs
  modified:
    - crates/realtime_preview_runtime/src/lib.rs
    - crates/realtime_preview_runtime/src/diagnostics.rs
    - crates/realtime_preview_runtime/tests/parity_diagnostics.rs

key-decisions:
  - "Realtime graph preparation uses engine_core normalization/range resolution and render_graph graph construction directly, with no FFmpeg compiler/runtime/cache/GPU ownership."
  - "Realtime capability classification is renderer-neutral and emits serializable diagnostics before any backend execution."
  - "Preview/export divergence is represented as parity diagnostics comparing realtime capability outcomes against export graph intent."

patterns-established:
  - "prepare_realtime_preview_graph: draft plus target microseconds plus preview dimensions becomes EngineProfile, single-frame RenderRangeState, RenderGraph, and diagnostics."
  - "RealtimePreviewCapabilityClassifier: backend/surface/text parity capability inputs classify existing graph intent without constructing graph semantics."
  - "realtime_preview_parity_diagnostics: report known preview/export support divergence as serializable data."

requirements-completed: [RTPREV-01, RTPREV-02, RTPREV-04]

duration: 68min
completed: 2026-06-18
---

# Phase 11 Plan 02: Render Graph Preparation And Capability Diagnostics Summary

**Rust-owned realtime preview graph preparation with explicit capability classification and serializable preview/export parity diagnostics**

## Performance

- **Duration:** 68 min
- **Started:** 2026-06-18T15:00:00Z
- **Completed:** 2026-06-18T16:08:50Z
- **Tasks:** 2
- **Files modified:** 7 code/test files plus planning metadata

## Accomplishments

- Added `prepare_realtime_preview_graph`, `RealtimePreviewGraphInput`, and `PreparedRealtimePreviewGraph` to prepare single-frame realtime graph intent through `EngineProfile::from_draft_canvas`, `normalize_draft`, `resolve_render_range`, and `build_render_graph`.
- Added `RealtimePreviewCapabilityClassifier`, `RealtimePreviewCapabilityReport`, and `RealtimePreviewGraphSupport` for supported/degraded/unsupported classification of canvas, material frames, visual layers, transforms, opacity, fit modes, keyframe sampled state, text parity, effects, filters, transitions, masks, blends, surface availability, and backend availability.
- Added `RealtimePreviewParityDiagnostic` snapshots so realtime/export divergence is explicit serialized data instead of silent drift.

## Task Commits

1. **Task 11-02-01 RED:** `424d878` test - failing graph preparation tests
2. **Task 11-02-01 GREEN:** `09fc5f5` feat - realtime graph preparation helper
3. **Task 11-02-02 RED:** `570f7b2` test - failing capability/parity tests
4. **Task 11-02-02 GREEN:** `e998e50` feat - graph capability classifier and parity diagnostics

## Files Created/Modified

- `crates/realtime_preview_runtime/src/graph_prepare.rs` - Realtime graph input/result/error types and graph preparation helper.
- `crates/realtime_preview_runtime/src/capabilities.rs` - Capability classifier and support report types.
- `crates/realtime_preview_runtime/src/parity.rs` - Preview/export parity diagnostic snapshot generation.
- `crates/realtime_preview_runtime/src/diagnostics.rs` - Shared diagnostic constructor for graph/classifier diagnostics.
- `crates/realtime_preview_runtime/src/lib.rs` - Public exports for graph preparation, classifier, and parity APIs.
- `crates/realtime_preview_runtime/tests/capability_matrix.rs` - Capability matrix coverage for supported, degraded, and unsupported graph states.
- `crates/realtime_preview_runtime/tests/parity_diagnostics.rs` - Graph preparation and parity serialization tests.

## Decisions Made

- Used existing `engine_core` and `render_graph` types as the only semantic preparation path; realtime runtime does not compile FFmpeg jobs, create GPU commands, own cache keys, or duplicate timeline/keyframe/text logic.
- Treated canvas-only positions as valid graph output because current render_graph semantics can represent an empty active timeline over a canvas. Invalid range testing uses true integer overflow instead.
- Kept `render_graph` unchanged; classifier additions live in `realtime_preview_runtime` to avoid GPU/backend/cache/runtime concerns leaking into renderer-neutral graph intent.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The exact Cargo filter `cargo test -p render_graph render_graph_snapshots -- --nocapture` filters by test name and runs zero existing tests. The required plan-level `cargo test -p render_graph -- --nocapture` was run and passed all render_graph tests.
- The GSD state helper could not parse the current STATE.md plan counters and produced incorrect metadata on first attempt. Those helper changes were reverted for the specific files and replaced with a narrow Plan 02 metadata patch.

## Known Stubs

None.

## Authentication Gates

None.

## Verification

- `cargo test -p realtime_preview_runtime capability_matrix -- --nocapture` - passed, 4 tests.
- `cargo test -p realtime_preview_runtime parity_diagnostics -- --nocapture` - passed, 3 tests.
- `cargo test -p render_graph -- --nocapture` - passed, 10 tests.
- `cargo check --workspace --locked` - passed.

## TDD Gate Compliance

- RED commit present for Task 11-02-01: `424d878`.
- GREEN commit present for Task 11-02-01: `09fc5f5`.
- RED commit present for Task 11-02-02: `570f7b2`.
- GREEN commit present for Task 11-02-02: `e998e50`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 11 can proceed to frame provider contracts and H.264 software video frame cache work. The next plan can consume prepared realtime graph intent and capability reports without moving graph construction, fallback decisions, or preview/export parity semantics into Electron renderer code.

## Self-Check: PASSED

- Found created files: `graph_prepare.rs`, `capabilities.rs`, `parity.rs`, `capability_matrix.rs`, and `parity_diagnostics.rs`.
- Found task commits: `424d878`, `09fc5f5`, `570f7b2`, and `e998e50`.
- Stub scan found no TODO/FIXME/placeholder/empty UI stub patterns in created or modified runtime files.

---
*Phase: 11-realtime-preview-runtime-and-gpu-render-backend*
*Completed: 2026-06-18*
