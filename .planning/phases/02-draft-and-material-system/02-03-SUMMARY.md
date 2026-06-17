---
phase: 02-draft-and-material-system
plan: 03
subsystem: media-runtime
tags: [rust, ffprobe, media-runtime, testkit, material-probe]
requires:
  - phase: 01-foundation-and-golden-harness
    provides: FFmpeg/ffprobe runtime boundary, desktop executor, and render smoke testkit patterns
  - phase: 02-draft-and-material-system
    provides: Draft material metadata schema and project-store derived-artifact boundary
provides:
  - Normalized `media_runtime::probe_material_metadata` API backed by `FfmpegExecutor`
  - Typed material probe metadata and classified bounded probe errors
  - Temp-dir-backed generated video, image, and audio material fixtures in `testkit`
affects: [material-import, bindings-node, generated-contracts, phase-02]
tech-stack:
  added: [serde_json in media_runtime, testkit dev-dependency for media_runtime tests]
  patterns: [argument-array-ffprobe, bounded-output-errors, temp-dir-generated-media, integer-microsecond-duration, rational-frame-rate]
key-files:
  created:
    - crates/media_runtime/src/probe.rs
    - crates/media_runtime/tests/material_probe.rs
    - crates/testkit/tests/material_fixtures.rs
  modified:
    - Cargo.lock
    - crates/media_runtime/Cargo.toml
    - crates/media_runtime/src/lib.rs
    - crates/testkit/src/lib.rs
    - .planning/STATE.md
    - .planning/ROADMAP.md
    - .planning/REQUIREMENTS.md
key-decisions:
  - "Normalized material probing belongs in `media_runtime` behind `FfmpegExecutor` and `RuntimeConfig`; it does not persist drafts or mutate material registries."
  - "Generated video, image, and audio fixtures live in `testkit` temp directories and are not committed under fixtures or goldens."
patterns-established:
  - "Material probes return normalized typed metadata with integer microseconds and rational frame rates instead of raw ffprobe JSON."
  - "Probe failures carry classified error kinds plus bounded stdout/stderr summaries."
requirements-completed: [MAT-01, MAT-02, DRAFT-04]
duration: 9min
completed: 2026-06-17
---

# Phase 02 Plan 03: Material Probe Runtime Summary

**FFprobe-backed material metadata normalization with temp-dir generated video, image, and audio fixtures.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-17T02:24:18Z
- **Completed:** 2026-06-17T02:33:11Z
- **Tasks:** 2
- **Files modified:** 7 code/test files plus planning metadata

## Accomplishments

- Added `media_runtime::probe_material_metadata` with normalized material kind, integer duration microseconds, dimensions, rational frame rate, stream flags, audio metadata, and classified probe errors.
- Added probe tests for generated video, image, audio-only media, missing/corrupt inputs, malformed JSON, bounded output, and timeout classification.
- Added reusable `testkit` generated material fixtures backed by temp directories, then rewired media-runtime probe tests to consume those helpers.

## Task Commits

1. **Task 1: Normalize ffprobe metadata in media_runtime** - `e94b35c` (feat)
2. **Task 2: Extend testkit with generated material fixtures** - `f00d46b` (feat)

**Plan metadata:** committed with this summary.

## Files Created/Modified

- `crates/media_runtime/src/probe.rs` - FFprobe argument-array material probe API, normalized metadata structs, rational fps parsing, and classified errors.
- `crates/media_runtime/src/lib.rs` - Public probe API exports.
- `crates/media_runtime/Cargo.toml` - Adds approved existing `serde_json` plus dev-only test dependencies.
- `crates/media_runtime/tests/material_probe.rs` - Generated media and failure coverage for material probes.
- `crates/testkit/src/lib.rs` - Temp-dir-backed generated video, image, and audio material fixture helpers with expected metadata assertions.
- `crates/testkit/tests/material_fixtures.rs` - Fixture lifecycle and probe expectation integration test.
- `Cargo.lock` - Lockfile update for the new crate dependency edges.

## Decisions Made

- Kept material probing in `media_runtime` and did not add any `project_store`, draft registry, Electron, or material ID dependency.
- Returned normalized typed metadata and classified errors instead of exposing raw ffprobe JSON.
- Used generated temp media only; no binary media fixtures were committed.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The `gsd-tools` command was not on PATH in this shell; the local Codex shim at `/Users/zhiwen/.codex/get-shit-done/bin/gsd-tools.cjs` was used for state advance.
- FFprobe reports a `25/1` stream frame rate for generated PNG still images. The fixture expectation records that normalized rational value while keeping image duration as `None`.

## Known Stubs

- `crates/media_runtime/tests/material_probe.rs` uses the byte string `placeholder` in two fake-input tests to exercise fake executor error paths. This is test-only data and does not flow to UI rendering or persisted draft semantics.

## Authentication Gates

None.

## Verification

- `cargo test -p media_runtime material_probe -- --nocapture` - passed.
- `cargo test -p testkit material -- --nocapture` - passed.
- `grep -R "project_store\|MaterialId\|save_project_bundle\|create_project_bundle" crates/media_runtime/src crates/media_runtime/tests && exit 1 || true` - passed.
- `grep -R "duration_seconds\|durationSeconds\|f32\|f64" crates/media_runtime/src crates/testkit/src && exit 1 || true` - passed.
- `find fixtures goldens -type f \( -name '*.mp4' -o -name '*.mov' -o -name '*.wav' -o -name '*.aac' -o -name '*.png' \) | grep . && exit 1 || true` - passed.

## Self-Check: PASSED

- Found created files: `crates/media_runtime/src/probe.rs`, `crates/media_runtime/tests/material_probe.rs`, and `crates/testkit/tests/material_fixtures.rs`.
- Found task commits `e94b35c` and `f00d46b` in git history.
- Stub scan found only intentional test-only fake input bytes noted above.
- Threat surface matches the plan threat model: local path probing, external ffprobe JSON parsing, and temp generated media only. No network endpoints, auth paths, draft persistence, registry mutation, or raw probe JSON persistence were introduced.

## User Setup Required

None - no external service configuration required. FFmpeg and ffprobe must remain available through `VE_FFMPEG_PATH` / `VE_FFPROBE_PATH` or PATH for these media tests.

## Next Phase Readiness

Plan 02-04 can build material import orchestration on top of `probe_material_metadata`, map normalized metadata into draft material records, and keep missing-material diagnostics separate from project persistence.

---
*Phase: 02-draft-and-material-system*
*Completed: 2026-06-17*
