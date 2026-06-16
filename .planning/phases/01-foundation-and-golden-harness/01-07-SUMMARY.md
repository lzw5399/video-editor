---
phase: 01-foundation-and-golden-harness
plan: 07
subsystem: render-smoke-testkit
tags: [rust, ffmpeg, ffprobe, testkit, goldens]

requires:
  - phase: 01-03
    provides: Service-boundary crates and `media_runtime::FfmpegExecutor`
  - phase: 01-05
    provides: FFmpeg/ffprobe discovery and bounded runtime errors
provides:
  - Tiny FFmpeg lavfi MP4 generation in temporary media-generated directories
  - Required ffprobe metadata-only render smoke helper and integration test
  - Golden harness scope documentation without binary media baselines
affects: [phase-1-foundation, testkit, media-runtime, render-smoke, goldens]

tech-stack:
  added: [tempfile-3.27.0]
  patterns:
    - Generated media lives in temporary `media-generated` directories and is never committed
    - Render smoke uses FFmpeg/ffprobe argument arrays through runtime boundaries
    - Phase 1 smoke asserts output existence and ffprobe metadata only

key-files:
  created:
    - crates/testkit/tests/render_smoke.rs
    - fixtures/media-generated/.gitkeep
    - goldens/README.md
  modified:
    - Cargo.lock
    - crates/media_runtime/src/lib.rs
    - crates/media_runtime_desktop/src/lib.rs
    - crates/testkit/Cargo.toml
    - crates/testkit/src/lib.rs

key-decisions:
  - "Extended `media_runtime::FfmpegExecutor` with a generic argument-array process runner so smoke tests do not shell-concatenate FFmpeg commands."
  - "Represented smoke duration as integer microseconds and frame rate as a rational pair in `SmokeMetadata`."
  - "Kept Phase 1 golden scope to generated media plus ffprobe metadata; full draft/render golden cases remain deferred."

patterns-established:
  - "Testkit helpers return temp-dir-owned media handles so generated MP4 files disappear when tests finish."
  - "Render smoke fails when FFmpeg or ffprobe discovery fails, preserving D-15 as a required gate."

requirements-completed: [FOUND-04]

duration: 5 min
completed: 2026-06-16
---

# Phase 1 Plan 07: Tiny FFmpeg Render Smoke Harness Summary

**Deterministic FFmpeg lavfi media generation with ffprobe metadata assertions and no committed binary media fixtures.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-16T22:43:30Z
- **Completed:** 2026-06-16T22:49:23Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Added `generate_tiny_lavfi_media` for temporary MP4 generation from `testsrc2` video and `sine` audio.
- Added `run_tiny_render_smoke`, `probe_media_metadata`, `SmokeMetadata`, and `assert_tiny_smoke_metadata`.
- Added a required render-smoke integration test that verifies file existence, duration, 160x90 resolution, 10 fps, and video/audio streams.
- Added generated-media and golden harness structure without committing MP4/MOV/WAV/AAC/PNG assets.

## Task Commits

1. **Task 01-W4-01 RED: Tiny lavfi generation test** - `08233fb` (test)
2. **Task 01-W4-01 GREEN: Tiny lavfi generation helper** - `75aceb7` (feat)
3. **Task 01-W4-02 RED: Render smoke metadata test** - `b921d31` (test)
4. **Task 01-W4-02 GREEN: ffprobe metadata smoke** - `0f91a20` (feat)

## Files Created/Modified

- `Cargo.lock` - Added the approved `tempfile` dependency resolution and transitive support crates.
- `crates/media_runtime/src/lib.rs` - Extended `FfmpegExecutor` with a generic argument-array `run` method.
- `crates/media_runtime_desktop/src/lib.rs` - Runs FFmpeg-family processes with `Command::new(binary).args(args)`.
- `crates/testkit/Cargo.toml` - Added local runtime dependencies plus approved `tempfile` and existing `serde_json`.
- `crates/testkit/src/lib.rs` - Implements temporary lavfi media generation, ffprobe metadata parsing, and smoke assertions.
- `crates/testkit/tests/render_smoke.rs` - Verifies render smoke output and metadata.
- `fixtures/media-generated/.gitkeep` - Keeps the generated-media directory convention without media assets.
- `goldens/README.md` - Documents Phase 1 golden harness scope and deferred full golden cases.

## Verification

- `cargo fmt --all --check` - PASS
- `cargo test -p testkit generate_tiny -- --nocapture` - PASS
- `cargo test -p testkit render_smoke -- --nocapture` - PASS, 2 render-smoke tests passed.
- `cargo test -p testkit -- --nocapture` - PASS, unit and integration tests passed.
- `cargo test -p media_runtime -- --nocapture` - PASS, 4 discovery tests passed.
- `cargo test -p media_runtime_desktop -- --nocapture` - PASS
- `find fixtures goldens -type f \( -name '*.mp4' -o -name '*.mov' -o -name '*.wav' -o -name '*.aac' -o -name '*.png' \) | grep .` - PASS, no binary media found.
- `grep -R "hash\|pixel" crates/testkit/tests/render_smoke.rs goldens/README.md` - PASS, no forbidden comparison terms found.

## TDD Gate Compliance

- **Task 01-W4-01 RED:** `08233fb` added a failing test for `generate_tiny_lavfi_media`; failure was the missing planned helper.
- **Task 01-W4-01 GREEN:** `75aceb7` implemented temporary lavfi generation and the generated-media marker.
- **Task 01-W4-02 RED:** `b921d31` added failing render-smoke integration tests; failure was the missing metadata helper API.
- **Task 01-W4-02 GREEN:** `0f91a20` implemented ffprobe metadata parsing, smoke assertions, and golden scope docs.

## Decisions Made

- Used `tempfile` for media lifetime ownership so generated MP4 outputs are removed automatically.
- Extended the existing runtime trait rather than adding shell execution in testkit, preserving the Phase 1 runtime boundary.
- Stored smoke duration as integer microseconds and frame rate as numerator/denominator to avoid semantic time drift.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Extended runtime executor beyond version probes**
- **Found during:** Task 01-W4-01 (Generate tiny lavfi media during tests)
- **Issue:** The existing runtime trait only supported `run_version_probe`, but the plan required FFmpeg execution through runtime boundaries using argument arrays.
- **Fix:** Added `FfmpegExecutor::run(&Path, &[String])` and implemented it in `DesktopFfmpegExecutor`.
- **Files modified:** `crates/media_runtime/src/lib.rs`, `crates/media_runtime_desktop/src/lib.rs`
- **Verification:** `cargo test -p testkit -- --nocapture`; `cargo test -p media_runtime -- --nocapture`; `cargo test -p media_runtime_desktop -- --nocapture`
- **Committed in:** `75aceb7`

---

**Total deviations:** 1 auto-fixed (Rule 2: 1)
**Impact on plan:** Required for the planned threat mitigation. Scope stayed within the existing runtime boundary; no real draft, render graph, export preset, preview cache, FFmpeg download, bundling, or redistribution behavior was added.

## Issues Encountered

None.

## Known Stubs

None. Stub scan over touched files found no TODO/FIXME/placeholder markers or UI-facing empty/mock data stubs.

## Threat Flags

None. New FFmpeg/ffprobe process execution and ffprobe JSON parsing are covered by this plan's threat model and mitigated through runtime discovery, version probes, argument arrays, bounded process summaries, and metadata-only assertions.

## User Setup Required

None - no external service configuration required. Systems without FFmpeg/ffprobe will fail the smoke test with actionable discovery remediation rather than silently skipping it.

## Next Phase Readiness

Ready for Plan 01-09 to include `cargo test -p testkit render_smoke -- --nocapture` in the full `just test` and CI gates.

## Self-Check: PASSED

- Key files exist on disk: `crates/testkit/Cargo.toml`, `crates/testkit/src/lib.rs`, `crates/testkit/tests/render_smoke.rs`, `fixtures/media-generated/.gitkeep`, and `goldens/README.md`.
- Task commits `08233fb`, `75aceb7`, `b921d31`, and `0f91a20` exist in git history.
- Plan verification commands passed.
- Stub scan found no blocking stubs.

---
*Phase: 01-foundation-and-golden-harness*
*Completed: 2026-06-16*
