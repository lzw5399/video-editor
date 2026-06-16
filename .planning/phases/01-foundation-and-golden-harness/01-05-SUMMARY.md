---
phase: 01-foundation-and-golden-harness
plan: 05
subsystem: media-runtime
tags: [rust, ffmpeg, ffprobe, discovery, node-api, contracts]

requires:
  - phase: 01-03
    provides: Service-boundary crates and `media_runtime::FfmpegExecutor`
  - phase: 01-04
    provides: Node-API binding crate and standardized command envelope
  - phase: 01-06
    provides: Generated command schema and TypeScript contract artifacts
provides:
  - Env/PATH FFmpeg and ffprobe discovery with version probes
  - Structured runtime discovery errors with bounded stdout/stderr summaries
  - Non-editing `probeMediaRuntime` command routed through `execute_command`
  - Updated Rust-owned schema and TypeScript contracts for the runtime probe command
affects: [phase-1-foundation, media-runtime, bindings-node, command-contracts, render-smoke]

tech-stack:
  added: [which-8.0.4, thiserror-2.0.18]
  patterns:
    - Explicit `VE_FFMPEG_PATH` and `VE_FFPROBE_PATH` discovery precedes PATH lookup
    - Runtime process probes use `Command::new(...).args([...])` argument arrays
    - Runtime probe command stays inside the Rust-owned ok/error/events envelope

key-files:
  created:
    - crates/media_runtime/src/discovery.rs
    - crates/media_runtime/src/error.rs
    - crates/media_runtime/tests/discovery.rs
  modified:
    - Cargo.lock
    - crates/media_runtime/Cargo.toml
    - crates/media_runtime/src/lib.rs
    - crates/media_runtime_desktop/src/lib.rs
    - crates/bindings_node/Cargo.toml
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/tests/binding_smoke.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/contract.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts

key-decisions:
  - "Kept FFmpeg discovery local-only: env vars and PATH are probed, but no FFmpeg binaries are downloaded, bundled, or redistributed."
  - "Added `probeMediaRuntime` to the Rust-owned command contract so the binding probe does not bypass D-06 contract generation."
  - "Mapped discovery failures to `RuntimeDiscoveryFailed` command errors with bounded process output embedded in the stable envelope message."

patterns-established:
  - "Runtime discovery returns `RuntimeConfig` with `DiscoveredBinary` entries for ffmpeg and ffprobe."
  - "Discovery errors carry kind, binary kind, checked paths, remediation, and optional bounded stdout/stderr summaries."
  - "Binding smoke tests use fake local binaries to verify runtime probe success and failure without requiring a system FFmpeg install."

requirements-completed: [FOUND-03, FOUND-02]

duration: 10 min
completed: 2026-06-17
---

# Phase 1 Plan 05: FFmpeg Discovery And Runtime Probe Summary

**Env/PATH FFmpeg discovery with structured bounded errors and a non-editing runtime probe command in the Phase 1 binding envelope.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-16T22:08:50Z
- **Completed:** 2026-06-16T22:19:08Z
- **Tasks:** 2
- **Files modified:** 16

## Accomplishments

- Implemented `discover_runtime_config`, `resolve_binary`, and `probe_binary_version` for ffmpeg and ffprobe.
- Added `DiscoveryError` and `DiscoveryErrorKind` with missing-binary, failed-probe, and unsupported-version classifications.
- Added discovery tests for env precedence, PATH fallback, missing binary remediation, bad binary failures, bounded stderr, and both binary kinds.
- Extended `execute_command` with `probeMediaRuntime` while preserving unsupported editing command behavior.
- Regenerated command schema and TypeScript contracts from Rust-owned command types.

## Task Commits

1. **Task 01-W3-01 RED: FFmpeg discovery tests** - `9479bd9` (test)
2. **Task 01-W3-01 GREEN: FFmpeg discovery implementation** - `f932618` (feat)
3. **Task 01-W3-02 RED: Runtime probe binding tests** - `306df9c` (test)
4. **Task 01-W3-02 GREEN: Runtime probe command routing** - `04a6259` (feat)

## Files Created/Modified

- `crates/media_runtime/src/discovery.rs` - Env/PATH discovery, version probing, runtime config types, and output summary bounding.
- `crates/media_runtime/src/error.rs` - Structured discovery error kinds, remediation, checked paths, and display/error implementations.
- `crates/media_runtime/tests/discovery.rs` - Fake-binary tests for env, PATH, missing, bad-binary, stderr bounding, and ffmpeg/ffprobe coverage.
- `crates/media_runtime/src/lib.rs` - Re-exports discovery/error APIs and extends `FfmpegExecutor` with an argument-array version probe method.
- `crates/media_runtime_desktop/src/lib.rs` - Implements desktop `run_version_probe` through `Command::new(binary).args(["-version"])`.
- `crates/bindings_node/src/lib.rs` - Routes `probeMediaRuntime` to runtime discovery and maps discovery failures to command errors.
- `crates/bindings_node/tests/binding_smoke.rs` - Adds binding-level runtime probe success and bounded-failure coverage.
- `crates/draft_model/src/lib.rs` - Adds `ProbeMediaRuntime` command/payload and `RuntimeDiscoveryFailed` error kind.
- `crates/draft_model/tests/contract.rs` - Covers runtime probe envelope deserialization.
- `crates/draft_model/tests/schema_exports.rs` - Exports the new runtime probe payload type.
- `schemas/command.schema.json` - Regenerated command schema with `probeMediaRuntime`.
- `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Regenerated TypeScript command envelope contract.
- `apps/desktop-electron/src/generated/CommandResultEnvelope.ts` - Regenerated TypeScript result envelope error kind.
- `crates/media_runtime/Cargo.toml`, `crates/bindings_node/Cargo.toml`, `Cargo.lock` - Add approved local/runtime dependencies.

## Verification

- `cargo test -p media_runtime discovery -- --nocapture` - PASS, 4 discovery tests passed.
- `cargo test -p bindings_node execute_command -- --nocapture` - PASS, 4 execute-command tests passed.
- `cargo test -p draft_model contract -- --nocapture` - PASS, 3 contract tests passed.
- `cargo test -p draft_model schema -- --nocapture` - PASS, generated schema and fixtures validated.
- `grep -R "sh -c\\|Command::new(.*ffmpeg.* .*\\|format!(.*ffmpeg" crates/media_runtime crates/media_runtime_desktop && exit 1 || true` - PASS.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - PASS after generated contract artifacts were committed.
- `cargo fmt --all --check` - PASS.

## TDD Gate Compliance

- **Task 01-W3-01 RED:** `9479bd9` added discovery tests; they failed because `BinaryKind`, `DiscoverySource`, `DiscoveryErrorKind`, and `discover_runtime_config` did not exist.
- **Task 01-W3-01 GREEN:** `f932618` implemented discovery, structured errors, approved dependencies, and desktop argument-array version probing.
- **Task 01-W3-02 RED:** `306df9c` added binding smoke tests; they failed because `RuntimeDiscoveryFailed` and `probeMediaRuntime` did not exist.
- **Task 01-W3-02 GREEN:** `04a6259` added the runtime probe command contract, route, generated artifacts, and bounded error mapping.

## Decisions Made

- Explicit env vars are authoritative: if `VE_FFMPEG_PATH` or `VE_FFPROBE_PATH` is set but invalid, discovery returns a structured error instead of silently falling back to PATH.
- Runtime probe output is summarized into the existing `CommandError.message` field because the Phase 1 `CommandError` contract does not yet have structured detail fields.
- The binding-level runtime probe is a non-editing command named `probeMediaRuntime`; real import, preview, export, and edit commands remain out of scope.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Test Bug] Made discovery tests match the planned cargo filter**
- **Found during:** Task 01-W3-01 (Implement FFmpeg and ffprobe discovery errors)
- **Issue:** The initial discovery test names used `discover_...`, so `cargo test -p media_runtime discovery -- --nocapture` compiled but filtered out every integration test.
- **Fix:** Renamed test functions to include `discovery_...` and reran the exact planned verification command.
- **Files modified:** `crates/media_runtime/tests/discovery.rs`
- **Verification:** `cargo test -p media_runtime discovery -- --nocapture`
- **Committed in:** `f932618`

**2. [Rule 2 - Missing Critical] Added runtime probe to the Rust-owned command contract**
- **Found during:** Task 01-W3-02 (Route media runtime probe through execute_command)
- **Issue:** The task file list only named binding files, but accepting `probeMediaRuntime` without updating `draft_model` and generated artifacts would bypass D-06, where Rust serde types are the source of truth.
- **Fix:** Added `ProbeMediaRuntime` command/payload and `RuntimeDiscoveryFailed` to `draft_model`, updated contract tests, and regenerated schema plus TypeScript artifacts.
- **Files modified:** `crates/draft_model/src/lib.rs`, `crates/draft_model/tests/contract.rs`, `crates/draft_model/tests/schema_exports.rs`, `schemas/command.schema.json`, `apps/desktop-electron/src/generated/CommandEnvelope.ts`, `apps/desktop-electron/src/generated/CommandResultEnvelope.ts`
- **Verification:** `cargo test -p draft_model contract -- --nocapture`; `cargo test -p draft_model schema -- --nocapture`; `git diff --exit-code schemas apps/desktop-electron/src/generated`
- **Committed in:** `04a6259`

### Tooling Lookup Deviations

**1. Context7 CLI unavailable**
- **Found during:** Task 01-W3-01 dependency/API verification
- **Issue:** The required Context7 CLI fallback reported `ctx7 not found`.
- **Fix:** Used approved versions from `01-RESEARCH.md` and verified local crate metadata with `cargo info which@8.0.4` and `cargo info thiserror@2.0.18`.
- **Files modified:** None
- **Verification:** `cargo test -p media_runtime discovery -- --nocapture`

---

**Total deviations:** 2 auto-fixed (Rule 1: 1, Rule 2: 1), 1 tooling lookup fallback.
**Impact on plan:** The fixes were required for the plan's exact verification command and Rust-owned command-contract consistency. Scope stayed within FFmpeg/ffprobe discovery and a non-editing runtime probe; no FFmpeg download, bundling, redistribution, import, preview, export, or editing semantics were added.

## Issues Encountered

- A test helper formatting issue was corrected before the RED discovery commit so the committed RED failure represented the missing discovery API rather than a broken helper.
- `ctx7` was unavailable in this environment; dependency API confidence came from the approved research and Cargo metadata.

## Known Stubs

None. Stub scan over created/modified files found no TODO/FIXME/placeholder markers or UI-facing empty/mock data stubs.

## Threat Flags

None. The new env/PATH, OS process, process-output, and runtime-probe surfaces are covered by this plan's threat model and mitigated by version probes, checked paths, argument arrays, and bounded summaries.

## User Setup Required

None - no external service configuration required. Systems without FFmpeg/ffprobe will receive actionable `MissingBinary` discovery errors rather than setup being required for this plan's fake-binary tests.

## Next Phase Readiness

Ready for Plan 01-07 to build the tiny FFmpeg render smoke harness on top of `discover_runtime_config` and the structured discovery errors.

## Self-Check: PASSED

- Key files exist on disk: `crates/media_runtime/src/discovery.rs`, `crates/media_runtime/src/error.rs`, `crates/media_runtime/tests/discovery.rs`, `crates/bindings_node/src/lib.rs`, `crates/bindings_node/tests/binding_smoke.rs`, `schemas/command.schema.json`, `apps/desktop-electron/src/generated/CommandEnvelope.ts`, and `apps/desktop-electron/src/generated/CommandResultEnvelope.ts`.
- Task commits `9479bd9`, `f932618`, `306df9c`, and `04a6259` exist in git history.
- Plan verification commands passed.
- Stub scan found no blocking stubs.

---
*Phase: 01-foundation-and-golden-harness*
*Completed: 2026-06-17*
