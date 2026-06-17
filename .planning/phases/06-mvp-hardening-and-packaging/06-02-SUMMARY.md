---
phase: 06-mvp-hardening-and-packaging
plan: 02
subsystem: runtime
tags: [ffmpeg, capabilities, bindings, generated-contracts]

requires:
  - phase: 06-mvp-hardening-and-packaging
    provides: "06-01 packaged desktop directory smoke and native binding packaged path"
provides:
  - "Rust-owned runtime capability report with external FFmpeg posture"
  - "probeRuntimeCapabilities command in generated Rust/TypeScript contracts"
  - "Node binding route for runtime readiness diagnostics"
affects: [desktop-runtime-diagnostics, real-workflow-e2e, release-gates]

tech-stack:
  added: []
  patterns:
    - "Runtime capability interpretation stays in media_runtime and bindings_node service conversion"
    - "Generated TypeScript remains a transport contract produced from draft_model"

key-files:
  created:
    - crates/media_runtime/src/capabilities.rs
    - crates/media_runtime/tests/runtime_capability.rs
    - crates/bindings_node/src/runtime_capability_service.rs
    - crates/bindings_node/tests/runtime_capabilities.rs
  modified:
    - crates/media_runtime/src/lib.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
    - crates/bindings_node/src/lib.rs

key-decisions:
  - "probeRuntimeCapabilities is a separate Rust-owned command instead of overloading renderer or Electron runtime probing."
  - "The MVP report always states externalRuntime=true and redistributableBuild=false."
  - "bindings_node converts media_runtime report types into draft_model transport types so draft_model stays runtime-independent."

patterns-established:
  - "Runtime readiness commands return standardized CommandResultEnvelope values through executeCommand."
  - "Missing FFmpeg/ffprobe for capability probing maps to runtimeDiscoveryFailed with actionable Chinese copy."

requirements-completed: [TEST-06, TEST-07]

duration: 14 min
completed: 2026-06-17
---

# Phase 06 Plan 02: Runtime Capability Command Summary

**Rust-owned FFmpeg capability reporting exposed through generated command contracts and the Node binding route**

## Performance

- **Duration:** 14 min resumed execution
- **Started:** 2026-06-17T21:06:13Z
- **Completed:** 2026-06-17T21:20:08Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- Added `media_runtime::probe_runtime_capabilities` with bounded FFmpeg/ffprobe version/configure probing, encoder/filter readiness, font readiness, diagnostics, and external runtime license posture.
- Added `probeRuntimeCapabilities` to the Rust command contract and regenerated JSON schema plus TypeScript command/result contracts.
- Added a binding service and tests proving configured and missing runtime paths flow through standardized envelopes without renderer-owned probing.

## Task Commits

1. **Task 1: Add media runtime capability probing** - `5a4d5bc` (feat)
2. **Task 2: Add generated command contract and binding route** - `517df92` (feat)

## Files Created/Modified

- `crates/media_runtime/src/capabilities.rs` - Runtime capability report and probe implementation.
- `crates/media_runtime/tests/runtime_capability.rs` - Capability report and discovery classification tests.
- `crates/bindings_node/src/runtime_capability_service.rs` - Converts `media_runtime` capability reports into `draft_model` transport reports.
- `crates/bindings_node/tests/runtime_capabilities.rs` - Command route tests for configured FFmpeg/ffprobe and missing runtime errors.
- `crates/draft_model/src/lib.rs` - Generated command payload and runtime report transport types.
- `crates/draft_model/tests/schema_exports.rs` - Contract generation and runtime capability coverage tests.
- `schemas/command.schema.json` - Regenerated command schema.
- `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Regenerated command envelope TypeScript.
- `apps/desktop-electron/src/generated/CommandResultEnvelope.ts` - Regenerated result contract TypeScript.
- `crates/bindings_node/src/lib.rs` - `probeRuntimeCapabilities` allowlist, route, and Chinese runtime discovery error envelope.

## Decisions Made

- Kept `draft_model` independent from `media_runtime`; the binding service owns type conversion at the IPC boundary.
- Used temporary executable scripts in binding tests so the capability command gate does not depend on local Homebrew FFmpeg.
- Preserved the old `probeMediaRuntime` behavior and added the richer capability report as a separate command for Phase 06 diagnostics UI.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Initial `cargo test -p bindings_node runtime_capabilities -- --nocapture` selected the integration test binary but filtered out every test because test names did not include `runtime_capabilities`. The test names were fixed before commit, and the gate now runs 3 assertions.

## Verification

- `cargo test -p media_runtime runtime_capability -- --nocapture`
- `cargo test -p bindings_node runtime_capabilities -- --nocapture`
- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture`
- `cargo test -p draft_model schema_exports -- --nocapture`
- `git diff --exit-code schemas apps/desktop-electron/src/generated`

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 06 Plan 03 can now build the Chinese runtime diagnostics UI from `probeRuntimeCapabilities` without renderer-side FFmpeg probing.

---
*Phase: 06-mvp-hardening-and-packaging*
*Completed: 2026-06-17*
