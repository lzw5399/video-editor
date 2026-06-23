---
phase: 16-task-scheduler-job-isolation-and-performance-telemetry
plan: "01"
subsystem: runtime
tags: [rust, scheduler, freshness, playback-generation, timeline-clock]

requires:
  - phase: 15.3
    provides: Rust-owned realtime preview cadence and product no-fallback boundary
provides:
  - task_runtime workspace crate as the Rust-owned scheduler boundary
  - canonical PlaybackGeneration, TimelineClock, playback state, playback rate, and target timeline freshness contracts
  - realtime_preview_runtime compatibility re-exports for canonical freshness types
  - local task_runtime dependency edges for scheduler-adjacent crates
affects: [phase-16, realtime-preview-runtime, audio-engine, artifact-store, project-store, bindings-node]

tech-stack:
  added: [task_runtime]
  patterns:
    - dependency-light Rust runtime boundary crate
    - canonical freshness type owner with compatibility re-exports
    - integer microsecond and rational playback-rate serialization

key-files:
  created:
    - crates/task_runtime/Cargo.toml
    - crates/task_runtime/src/lib.rs
    - crates/task_runtime/src/freshness.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/realtime_preview_runtime/Cargo.toml
    - crates/realtime_preview_runtime/src/clock.rs
    - crates/realtime_preview_runtime/src/lib.rs
    - crates/audio_engine/Cargo.toml
    - crates/artifact_store/Cargo.toml
    - crates/project_store/Cargo.toml
    - crates/bindings_node/Cargo.toml

key-decisions:
  - "Moved freshness ownership to task_runtime and kept realtime_preview_runtime as a source-compatible re-export surface."
  - "Kept Phase 16-01 limited to portable contracts and local dependency edges; scheduler queues, telemetry, adapters, and config policy remain deferred."
  - "Used plain git commits and skipped STATE.md/ROADMAP.md updates because the local GSD helper is known to fail in this repository."

patterns-established:
  - "Scheduler-adjacent crates depend on task_runtime for portable Rust contracts before integration adapters are added."
  - "Preview compatibility modules can re-export task_runtime contracts without duplicating freshness definitions."

requirements-completed: [SCHED-01, SCHED-03]

duration: 10 min
completed: 2026-06-23
status: complete
---

# Phase 16 Plan 01: Scheduler Boundary Freshness Summary

**Rust-owned task_runtime freshness contracts with one canonical PlaybackGeneration shared through preview/audio/artifact/project/binding dependency edges**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-23T14:13:14Z
- **Completed:** 2026-06-23T14:23:32Z
- **Tasks:** 3
- **Files modified:** 13

## Accomplishments

- Added `crates/task_runtime` as an explicit Rust workspace member and dependency-light scheduler boundary crate.
- Moved the playback freshness vocabulary into `task_runtime`, including `PlaybackGeneration`, `TimelineClock`, `PlaybackRate`, `PlaybackState`, and `TimelineFreshness`.
- Replaced `realtime_preview_runtime`'s local clock definitions with compatibility re-exports from `task_runtime`.
- Added local `task_runtime` dependencies to `audio_engine`, `artifact_store`, `project_store`, and `bindings_node` without adding queues, telemetry, adapters, or desktop policy.

## Task Commits

1. **Task 16-01-01 RED: Add failing task runtime freshness contract** - `730f958` (test)
2. **Task 16-01-01 GREEN: Implement task runtime freshness contracts** - `9aa6ae7` (feat)
3. **Task 16-01-02: Re-export canonical freshness through realtime preview runtime** - `7a4a248` (feat)
4. **Task 16-01-03: Add scheduler boundary dependencies** - `e373110` (feat)

## Files Created/Modified

- `Cargo.toml` - Registers `crates/task_runtime` as an explicit workspace member.
- `Cargo.lock` - Records the new local package and local dependency edges.
- `crates/task_runtime/Cargo.toml` - Defines the scheduler boundary crate manifest.
- `crates/task_runtime/src/lib.rs` - Documents the Rust-owned task runtime boundary and exports freshness contracts.
- `crates/task_runtime/src/freshness.rs` - Owns canonical playback generation, timeline clock, rational playback rate, playback state, and target timeline freshness payloads.
- `crates/realtime_preview_runtime/Cargo.toml` - Adds local `task_runtime` dependency.
- `crates/realtime_preview_runtime/src/clock.rs` - Re-exports task runtime freshness types for compatibility.
- `crates/realtime_preview_runtime/src/lib.rs` - Re-exports canonical freshness types at the preview crate root.
- `crates/audio_engine/Cargo.toml` - Adds local `task_runtime` dependency.
- `crates/artifact_store/Cargo.toml` - Adds local `task_runtime` dependency.
- `crates/project_store/Cargo.toml` - Adds local `task_runtime` dependency.
- `crates/bindings_node/Cargo.toml` - Adds local `task_runtime` dependency.
- `.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-01-SUMMARY.md` - Records plan completion.

## Decisions Made

- `task_runtime` is the canonical owner of freshness types. `realtime_preview_runtime` remains source-compatible by re-exporting the same concrete types instead of defining duplicates.
- `TimelineFreshness` uses preview-compatible camelCase fields: `targetTime`, `playbackGeneration`, `projectSessionId`, and `expectedRevision`.
- The crate uses only existing local Rust contracts plus `serde`/`serde_json`; no scheduler queues, telemetry, adapters, resource policy, Electron fields, FFmpeg paths, worker names, or external packages were added.
- Per the user's local workflow constraint, `STATE.md` and `ROADMAP.md` were not updated and `gsd-tools.cjs` was not used for commits or state updates.

## Verification

- `cargo check -p task_runtime --locked` - passed.
- `cargo test -p task_runtime freshness -- --nocapture` - passed, 4 tests.
- `cargo check -p realtime_preview_runtime -p audio_engine -p artifact_store -p project_store -p bindings_node --locked` - passed with one pre-existing transitive deprecation warning in `media_runtime_desktop`.
- `rg -n "struct PlaybackGeneration" crates | wc -l | awk '{exit ($1 == 1 ? 0 : 1)}'` - passed.
- `git diff --check -- . ':!reference'` - passed.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The first `cargo check -p realtime_preview_runtime --locked` after adding the dependency failed because `Cargo.lock` had not yet recorded the new local dependency edge. Resolved by running the same check once with `--offline` to update the lockfile, then rerunning the required `--locked` check successfully.
- The cross-crate cargo check reports a pre-existing `objc2_av_foundation::AVAsset::tracksWithMediaType` deprecation warning in `crates/media_runtime_desktop/src/platform/macos.rs`; it is unrelated to this plan and was not changed.

## Known Stubs

None in files created or modified by this plan. The stub scan only found the pre-existing root workspace metadata `planned-members = []`, which is not a runtime or UI data stub.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 16 can now build scheduler core and integration plans against one portable Rust freshness owner. Later plans can add scheduler queues, cancellation policy, telemetry, adapters, and config without creating another generation counter or moving preview-owned compatibility imports.

## Self-Check

PASSED

- Created files exist: `crates/task_runtime/Cargo.toml`, `crates/task_runtime/src/lib.rs`, `crates/task_runtime/src/freshness.rs`, and this summary.
- Task commits found in git history: `730f958`, `9aa6ae7`, `7a4a248`, and `e373110`.
- Summary frontmatter includes `status: complete` and `requirements-completed: [SCHED-01, SCHED-03]`.

---
*Phase: 16-task-scheduler-job-isolation-and-performance-telemetry*
*Completed: 2026-06-23*
