---
phase: 01-foundation-and-golden-harness
plan: 02
subsystem: rust-core
tags: [rust, cargo, serde, schemars, ts-rs, contracts]

requires:
  - phase: 01-01
    provides: Root Rust workspace metadata, locked Cargo resolution, and command entrypoints
provides:
  - Compile-safe pure semantic crate shells for draft, command, engine, render graph, and FFmpeg compiler layers
  - Rust-owned Phase 1 command envelope and standardized result contracts
  - Contract tests proving ping/version deserialization, result serialization, and unknown-field rejection
affects: [phase-1-foundation, rust-core, bindings-node, schema-generation]

tech-stack:
  added: [serde-1.0.228, serde_json-1.0.150, schemars-1.2.1, ts-rs-12.0.1]
  patterns:
    - Pure semantic crates contain boundary notes but no platform/runtime dependencies
    - Rust serde types own command/result envelopes before Electron bindings consume them
    - TDD contract work uses red test commit followed by implementation commit

key-files:
  created:
    - crates/draft_model/Cargo.toml
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/contract.rs
    - crates/draft_commands/Cargo.toml
    - crates/draft_commands/src/lib.rs
    - crates/engine_core/Cargo.toml
    - crates/engine_core/src/lib.rs
    - crates/render_graph/Cargo.toml
    - crates/render_graph/src/lib.rs
    - crates/ffmpeg_compiler/Cargo.toml
    - crates/ffmpeg_compiler/src/lib.rs
  modified:
    - Cargo.toml
    - Cargo.lock
  deleted:
    - crates/workspace_anchor/src/lib.rs

key-decisions:
  - "Replaced the temporary workspace anchor with the first real Phase 1 semantic crate members."
  - "Kept Phase 1 command scope to ping/version envelopes and standardized unsupported-command errors."
  - "Used generic CommandResultEnvelope<T> so typed ping/version responses can travel through the same ok/error/events result shape."

patterns-established:
  - "Semantic crate shells document future Jianying-aligned responsibilities without implementing timeline or render behavior early."
  - "Boundary structs use serde camelCase plus deny_unknown_fields where untrusted JSON enters Rust."
  - "Approved Rust contract dependencies are local to draft_model until later crates need them."

requirements-completed: [FOUND-01, FOUND-02, TEST-01]

duration: 5 min
completed: 2026-06-17
---

# Phase 1 Plan 02: Pure Rust Semantic Crates And Command Contracts Summary

**Compile-safe Rust semantic crate shells with serde-owned ping/version command envelopes and contract tests for unknown-field rejection.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-16T21:24:07Z
- **Completed:** 2026-06-16T21:29:08Z
- **Tasks:** 2
- **Files modified:** 14

## Accomplishments

- Added `draft_model`, `draft_commands`, `engine_core`, `render_graph`, and `ffmpeg_compiler` as real Cargo workspace members.
- Defined `CommandEnvelope`, `CommandName`, `CommandPayload`, `CommandResultEnvelope`, `CommandError`, `CommandErrorKind`, `CommandEvent`, `PingResponse`, and `VersionResponse` in `draft_model`.
- Added Rust contract tests covering valid ping/version envelopes, ok/error/events serialization, and rejection of unknown top-level command fields.

## Task Commits

1. **Task 01-W1-01: Create compile-safe pure crate shells** - `4ae3176` (chore)
2. **Task 01-W1-02 RED: Add failing command contract tests** - `3041e73` (test)
3. **Task 01-W1-02 GREEN: Implement Rust command contracts** - `12dac64` (feat)

_No REFACTOR commit was needed; the green implementation remained small and direct._

## Files Created/Modified

- `Cargo.toml` - Replaced the temporary anchor member with real Phase 1 semantic crate members.
- `Cargo.lock` - Locked the approved `draft_model` contract dependencies.
- `crates/draft_model/Cargo.toml` - Added `serde`, `serde_json`, `schemars`, and `ts-rs`.
- `crates/draft_model/src/lib.rs` - Defines the Rust-owned Phase 1 command/result contract symbols.
- `crates/draft_model/tests/contract.rs` - Contract tests for envelope deserialization, result serialization, and unknown-field rejection.
- `crates/draft_commands/src/lib.rs` - Boundary notes for future draft edit command semantics.
- `crates/engine_core/src/lib.rs` - Boundary notes for future timeline normalization and frame-state evaluation.
- `crates/render_graph/src/lib.rs` - Boundary notes for future typed render intents.
- `crates/ffmpeg_compiler/src/lib.rs` - Boundary notes for future FFmpeg plan compilation without process execution.

## Verification

- `cargo check -p draft_commands -p engine_core -p render_graph -p ffmpeg_compiler --locked` - PASS
- `grep -R "std::process\\|which::\\|FfmpegExecutor\\|PlatformFileSystem\\|PreviewRenderer" crates/draft_model crates/draft_commands crates/engine_core && exit 1 || true` - PASS
- `cargo test -p draft_model contract -- --nocapture` - PASS, 3 contract tests passed
- `cargo check -p draft_model --locked` - PASS

## TDD Gate Compliance

- **RED:** `3041e73` added failing tests for the command contract; failure showed missing `draft_model` contract symbols and missing `serde_json`.
- **GREEN:** `12dac64` implemented the contract types and approved dependencies; the targeted contract test passed.
- **REFACTOR:** Not needed.

## Decisions Made

- Used a generic `CommandResultEnvelope<T>` to keep ping and version response data typed while preserving one standardized `ok`/`data`/`error`/`events` envelope.
- Kept `CommandName` limited to `Ping` and `Version`; future edit commands remain out of scope for Phase 1.
- Used string event kinds for `CommandEvent` so later command/event taxonomy can be introduced deliberately without inventing edit semantics in this plan.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated root workspace membership and lockfile**
- **Found during:** Task 01-W1-01 (Create compile-safe pure crate shells)
- **Issue:** The plan file list did not include root `Cargo.toml`, but `cargo check -p draft_commands -p engine_core -p render_graph -p ffmpeg_compiler --locked` cannot work unless the new crates are workspace members. The first locked check also failed until `Cargo.lock` reflected the new workspace shape.
- **Fix:** Added the real semantic crates to root workspace membership, removed the now-obsolete temporary anchor from the workspace, regenerated the lockfile, then reran the locked gate.
- **Files modified:** `Cargo.toml`, `Cargo.lock`, `crates/workspace_anchor/src/lib.rs`
- **Verification:** `cargo check -p draft_commands -p engine_core -p render_graph -p ffmpeg_compiler --locked`
- **Committed in:** `4ae3176`

---

**Total deviations:** 1 auto-fixed (Rule 3: 1)
**Impact on plan:** Required to make the planned crate checks executable. No platform runtime, FFmpeg execution, Electron binding, or timeline editing behavior was added.

## Issues Encountered

- Context7 documentation lookup was unavailable because `ctx7` is not installed. The implementation used the approved versions and derive patterns from `01-RESEARCH.md`, then validated the actual API surface with Cargo tests/checks.

## Known Stubs

None.

## Threat Flags

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Plan 01-03 to add service-boundary crate shells and runtime-boundary docs while keeping pure semantic crates free of platform traits.

## Self-Check: PASSED

- Key files exist on disk.
- Task commits `4ae3176`, `3041e73`, and `12dac64` exist in git history.
- Plan verification commands passed.
- Stub scan over touched code files found no placeholder/TODO/FIXME or hardcoded empty UI data patterns.

---
*Phase: 01-foundation-and-golden-harness*
*Completed: 2026-06-17*
