---
phase: 05-preview-and-export-pipeline
plan: 09
subsystem: testing
tags: [preview, export, parity, ffmpeg, source-guards, gates]

requires:
  - phase: 05-preview-and-export-pipeline
    provides: preview service, render graph, FFmpeg compiler, export runtime, desktop preview/export UI
provides:
  - Preview/export frame parity gate through one Rust render path
  - Final Phase 5 renderer/source ownership guards
  - Root `pnpm run test` and `just test` Phase 5 gate chaining
affects: [phase-05, phase-06, testkit, preview-service, export-runtime, desktop-electron]

tech-stack:
  added: []
  patterns:
    - FFmpeg capability probes return classified setup errors
    - Preview/export parity compares RGB tolerance instead of byte-perfect media
    - Public gates include Phase-specific render-core and workspace subsets

key-files:
  created:
    - crates/testkit/src/render_compare.rs
    - crates/testkit/tests/preview_export_parity.rs
  modified:
    - Cargo.lock
    - crates/testkit/Cargo.toml
    - crates/testkit/src/lib.rs
    - scripts/phase5-source-guards.sh
    - package.json
    - justfile

key-decisions:
  - "Phase 5 preview/export parity uses generated local media and the same Rust path from draft normalization through render graph, FFmpeg compilation, preview_service, media_runtime export, and ffprobe validation."
  - "Parity is exact for dimensions/frame metadata and tolerant for encoded pixels: mean RGB delta <= 8.0 and p99 RGB delta <= 24."
  - "Missing libx264, AAC, ASS/subtitles filters, or deterministic text fonts are classified setup failures with remediation text, not skipped tests."

patterns-established:
  - "testkit::render_compare owns media frame extraction, capability probes, and parity tolerance helpers."
  - "Phase 5 source guards allow renderer command envelope helpers but reject renderer ownership of FFmpeg, render graph, export script, process, validation, and cache semantics."

requirements-completed:
  - TEXT-03
  - PREV-01
  - PREV-02
  - PREV-03
  - PREV-04
  - EXP-01
  - EXP-02
  - EXP-03
  - EXP-04
  - TEST-03
  - TEST-04
  - TEST-05

duration: 17 min
completed: 2026-06-18
---

# Phase 05 Plan 09: Preview/Export Parity And Final Gates Summary

**Golden preview/export parity now runs through the shared Rust render path, and Phase 5 gates are part of both root test entrypoints.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-06-17T19:40:00Z
- **Completed:** 2026-06-17T19:57:04Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Added `testkit::render_compare` helpers for FFmpeg capability probes, deterministic font setup checks, raw RGB frame extraction, frame metadata assertions, and Phase 5 pixel tolerance checks.
- Added `preview_export_parity.rs`, which generates a small video/audio/text draft, requests a preview frame through `preview_service`, exports through `render_graph -> ffmpeg_compiler -> media_runtime`, validates output metadata, and compares frames.
- Tightened Phase 5 source guards and chained render-core, source-guard, preview/export workspace, and contract-drift checks into `pnpm run test` and `just test`.

## Task Commits

Each task was committed atomically:

1. **Task 05-09-01: Add preview/export parity helpers and golden test** - `73067fc` (test)
2. **Task 05-09-02: Add final Phase 5 source guards and public gates** - `7933fa6` (test)

**Plan metadata:** pending in the closeout commit.

## Files Created/Modified

- `crates/testkit/src/render_compare.rs` - Capability probing, classified setup errors, frame metadata, RGB extraction, and tolerance comparison helpers.
- `crates/testkit/tests/preview_export_parity.rs` - TEST-05 golden parity test with video, audio, and text through the shared Rust path.
- `crates/testkit/Cargo.toml` - Testkit dependencies for Phase 5 render comparison.
- `crates/testkit/src/lib.rs` - Exposes the new render comparison module.
- `Cargo.lock` - Workspace dependency graph update for testkit.
- `scripts/phase5-source-guards.sh` - Final renderer/schema ownership guard for Phase 5 preview/export boundaries.
- `package.json` - Adds `test:phase5-render-core`, `test:phase5-workspace`, and chains final Phase 5 gates into `test`.
- `justfile` - Adds Phase 5 render-core, source-guard, workspace, and contract gates to `just test`.

## Decisions Made

- Pixel parity uses the documented Phase 5 tolerance rather than byte-perfect comparison, because local H.264 encoding and text rasterization can differ slightly while still proving shared-path behavior.
- The parity fixture uses generated 160x90, 30fps, one-second video/audio media to keep the gate fast while still covering video, audio, text, preview, export, and validation.
- `commandHelpers.ts` remains allowed to mention preview cache invalidation fields only as Rust command-envelope payloads; other renderer files remain guarded from owning cache, render, process, validation, or FFmpeg semantics.

## Deviations from Plan

None - plan executed exactly as written.

---

**Total deviations:** 0 auto-fixed.
**Impact on plan:** No scope creep.

## Issues Encountered

The shell initially did not have `just` on `PATH`. `cargo install just --locked` reported `just v1.53.0` was already installed under `/Users/zhiwen/.cargo/bin`, so the gate was run successfully via `/Users/zhiwen/.cargo/bin/just test`.

## Verification

- `cargo fmt --all --check` - passed
- `cargo test -p testkit preview_export_parity -- --nocapture` - passed
- `pnpm run test:phase5-render-core` - passed
- `pnpm run test:phase5-source-guards` - passed
- `pnpm run test:phase5-workspace` - passed
- `pnpm run test` - passed
- `/Users/zhiwen/.cargo/bin/just test` - passed
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 5 is complete. Phase 6 can plan MVP hardening and packaging on top of a working shared preview/export pipeline, with public gates already covering render core parity, desktop preview/export UI, source ownership, and generated contract drift.

---
*Phase: 05-preview-and-export-pipeline*
*Completed: 2026-06-18*
