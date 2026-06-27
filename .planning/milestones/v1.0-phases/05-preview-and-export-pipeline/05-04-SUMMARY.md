---
phase: 05-preview-and-export-pipeline
plan: 04
subsystem: preview-service
tags: [rust, preview, cache, render-graph, ffmpeg]
requires:
  - phase: 05-01
    provides: normalized draft and frame-state evaluation
  - phase: 05-02
    provides: typed renderer-neutral render graph
  - phase: 05-03
    provides: deterministic FFmpeg job compilation and sidecars
provides:
  - Rust preview frame request service
  - Rust preview segment request service
  - Preview cache keys, cache entries, artifact metadata, and invalidation helpers
  - PREV-01/PREV-02/PREV-03/PREV-04 service-level tests
affects: [preview_service, bindings_node, desktop-preview, export-pipeline]
tech-stack:
  added: [serde, serde_json, tempfile]
  patterns:
    - derived preview artifacts remain outside .veproj/project.json
    - preview cache identity belongs to preview_service
    - preview generation reuses engine_core -> render_graph -> ffmpeg_compiler
key-files:
  created:
    - crates/preview_service/src/cache.rs
    - crates/preview_service/src/service.rs
    - crates/preview_service/tests/cache_invalidation.rs
    - crates/preview_service/tests/preview_generation.rs
  modified:
    - Cargo.lock
    - crates/preview_service/Cargo.toml
    - crates/preview_service/src/lib.rs
key-decisions:
  - "Preview cache keys include target timerange, output profile, semantic fingerprint, material dependencies, and artifact path."
  - "Preview frame and segment generation compile through the shared engine/render graph/FFmpeg compiler path instead of renderer-owned logic."
  - "Timeline, text, audio, and material invalidation helpers live in preview_service so renderer code does not calculate cache overlap."
patterns-established:
  - "PreviewServiceConfig owns cache root, FFmpeg binary path, compiler capabilities, and preview dimensions."
  - "Preview service errors are classified before crossing service boundaries."
  - "Derived preview artifacts and sidecars are written under the configured cache root only."
requirements-completed: [PREV-01, PREV-02, PREV-03, PREV-04, EXP-02]
duration: resumed
completed: 2026-06-17
---

# Phase 05 Plan 04: Preview Cache Service Summary

**Rust preview frame/segment services with deterministic cache keys, range/material invalidation, and shared render graph to FFmpeg compilation**

## Performance

- **Duration:** resumed from partial implementation
- **Started:** prior session; resumed 2026-06-17T18:16:00Z
- **Completed:** 2026-06-17T18:30:04Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- Added `preview_service::cache` with cache key, entry, artifact, profile, invalidation request, and invalidation result types.
- Added `preview_service::service` with preview frame and preview segment request orchestration through `engine_core`, `render_graph`, `ffmpeg_compiler`, and `media_runtime::FfmpegExecutor`.
- Added deterministic tests for cache metadata shape, range invalidation, material invalidation, preview frame generation, preview segment cache reuse, and runtime failure classification.
- Added timeline/text/audio accepted edit invalidation helpers so future command/binding layers can pass Rust-owned invalidation inputs without calculating overlap in the renderer.

## Task Commits

Each task was committed atomically:

1. **Task 05-04-01: Define preview cache keys, entries, and range invalidation** - `f5bc9fb` (feat)
2. **Task 05-04-02: Generate deterministic preview frames and cached preview segments** - `f5bc9fb` (feat)
3. **Task 05-04-03: Connect accepted edit ranges to preview cache invalidation inputs** - `f5bc9fb` (feat)

**Plan metadata:** this summary commit

## Files Created/Modified

- `crates/preview_service/src/cache.rs` - Defines cache profiles, keys, entries, artifact metadata, range/material invalidation, and accepted edit invalidation helpers.
- `crates/preview_service/src/service.rs` - Implements preview frame/segment orchestration and classified service errors.
- `crates/preview_service/tests/cache_invalidation.rs` - Covers cache metadata, range overlap, text/audio/timeline invalidation inputs, and material dependency invalidation.
- `crates/preview_service/tests/preview_generation.rs` - Covers shared preview generation path, segment cache reuse, and runtime failure classification.
- `crates/preview_service/Cargo.toml` - Adds local Rust crate dependencies and test dependency.
- `crates/preview_service/src/lib.rs` - Exports the preview cache and service APIs.
- `Cargo.lock` - Records preview service dependency graph.

## Decisions Made

- Preview cache identity uses semantic fingerprints and material dependencies so derived cache entries can be invalidated without storing cache metadata in `.veproj/project.json`.
- Preview frame and segment requests return FFmpeg job metadata for testability while artifact generation still runs through `media_runtime::FfmpegExecutor`.
- Accepted edit invalidation is represented as explicit target timeranges and material IDs in `preview_service`; no renderer-side cache math is introduced.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The resumed partial implementation already covered frame/segment generation tests. The missing acceptance gap was Task 05-04-03's explicit accepted edit invalidation helpers and coverage; this was added before close-out.
- PREV-01 is complete at the Rust service level in this plan. Desktop center player UI integration remains the responsibility of Phase 05 Plan 06.

## Verification

- `cargo test -p preview_service -- --nocapture` - passed, 9 tests.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for `05-05` binding commands to expose preview frame/segment requests. `05-07` can proceed independently on export runtime primitives.

---
*Phase: 05-preview-and-export-pipeline*
*Completed: 2026-06-17*
