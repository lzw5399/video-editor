---
phase: 18-mobile-server-binding-architecture-and-runtime-ports
plan: "05"
subsystem: server-runtime
tags: [rust, server-runtime, veproj, export, ffmpeg, task-runtime]
requires:
  - phase: "18-01"
    provides: "editor_runtime shared project-session and export contracts"
  - phase: "18-02"
    provides: "Phase 18 staged source guard and server package script"
  - phase: "18-03"
    provides: "editor_runtime-owned export service used by adapter surfaces"
provides:
  - "Electron-free server_runtime library API for project open, export start/status/cancel, and wait"
  - "server_runtime CLI that opens .veproj bundles and prints JSON export progress/status events"
  - "Bundle-relative material URI resolution before server export"
  - "Real server export smoke tests for output validation, progress telemetry, cancellation, and CLI JSON status"
affects: [phase-18, server-runtime, editor-runtime, project-store, export, future-server-rendering]
tech-stack:
  added:
    - "server_runtime dependency on draft_model, media_runtime, project_store, serde, and serde_json"
    - "server_runtime dev-dependency on testkit and media_runtime_desktop for generated smoke media"
  patterns:
    - "Server runtime delegates project session and export authority to editor_runtime"
    - "Server export resolves filesystem-backed material URIs against the opened bundle without mutating project.json"
key-files:
  created:
    - crates/server_runtime/src/main.rs
    - crates/server_runtime/tests/server_export_smoke.rs
  modified:
    - Cargo.lock
    - crates/server_runtime/Cargo.toml
    - crates/server_runtime/src/lib.rs
key-decisions:
  - "server_runtime is an adapter over editor_runtime::ProjectSessionService and editor_runtime::ExportService, not a duplicate export scheduler."
  - "Bundle-relative material URIs are resolved just before server export so .veproj/project.json remains canonical and derived export payloads use concrete file paths."
  - "The Phase 18 server CLI emits newline-delimited structured JSON events and uses the library API rather than owning a separate runtime path."
patterns-established:
  - "Server tests build real .veproj bundles with relative video/image/audio/text materials and validate exported H.264/AAC output metadata."
  - "Plan 05 source guard runs in staged mode after server library/bin/test artifacts exist."
requirements-completed: [PLAT-02, BIND-04, BIND-05]
duration: 16 min
completed: 2026-06-25
status: complete
---

# Phase 18 Plan 05: Server Runtime Summary

**Electron-free server runtime for opening .veproj bundles, exporting through shared Rust runtime services, reporting progress/cancel status, and validating real output media.**

## Performance

- **Duration:** 16 min
- **Started:** 2026-06-25T02:12:05Z
- **Completed:** 2026-06-25T02:28:23Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Added `ServerRuntime` library APIs: `open_project`, `start_export`, `get_export_status`, `cancel_export`, and `wait_for_export`.
- Added `server_runtime export <bundle.veproj> <output.mp4>` CLI that opens the bundle, starts export, polls status, and prints structured JSON events.
- Resolved bundle-relative material URIs through `project_store` before export while preserving `.veproj/project.json` as the canonical source.
- Added real server smoke coverage that generates media fixtures, saves `.veproj` bundles, exports H.264/AAC output, validates duration/fps/dimensions/audio metadata, checks scheduler progress, cancels an export, and verifies CLI JSON output.

## Task Commits

Each task was committed atomically with RED and GREEN TDD gates:

1. **Task 1 RED: Server runtime API and CLI contract tests** - `c08fcfc` (test)
2. **Task 1 GREEN: Server runtime library and CLI** - `42064b7` (feat)
3. **Task 2 RED: Server export smoke tests** - `1b16b02` (test)
4. **Task 2 GREEN: Bundle-relative server export resolution** - `d49454b` (feat)

## Files Created/Modified

- `crates/server_runtime/Cargo.toml` - Adds runtime/test dependencies for the server adapter and smoke fixtures.
- `crates/server_runtime/src/lib.rs` - Implements server runtime session, project open, export start/status/cancel/wait, error mapping, and export material URI resolution.
- `crates/server_runtime/src/main.rs` - Adds the JSON-emitting server export CLI.
- `crates/server_runtime/tests/server_export_smoke.rs` - Adds real `.veproj` export, progress/cancel, and CLI smoke coverage.
- `Cargo.lock` - Locks the updated server runtime dependency graph.

## Decisions Made

- Kept export scheduler, render graph build, FFmpeg compile, validation, progress, and cancellation authority inside `editor_runtime::ExportService`.
- Used `media_runtime::discover_runtime_config` so the server path reuses configured bundled FFmpeg/ffprobe resources and does not search PATH.
- Resolved only filesystem-backed material URIs to `file://` paths for export; text/external URI semantics are preserved.
- Kept the CLI intentionally narrow: local export execution and JSON progress/status/error events.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added server smoke artifact before Task 2 so staged guard could pass**
- **Found during:** Task 1 (server runtime library and CLI)
- **Issue:** `bash scripts/phase18-source-guards.sh --plan 05` requires `crates/server_runtime/tests/server_export_smoke.rs`, but the plan assigned real smoke content to Task 2.
- **Fix:** Added the test artifact during Task 1, then replaced it with real export/progress/cancel smoke coverage in Task 2.
- **Files modified:** `crates/server_runtime/tests/server_export_smoke.rs`
- **Verification:** `bash scripts/phase18-source-guards.sh --plan 05` passed after the artifact existed and again after Task 2.
- **Committed in:** `42064b7` and `1b16b02`

**2. [Rule 2 - Missing Critical] Resolved bundle-relative material URIs for server export**
- **Found during:** Task 2 RED smoke
- **Issue:** Server export opened `.veproj` bundles correctly, but the export payload used raw relative material URIs, causing FFmpeg execution to fail outside the bundle directory.
- **Fix:** Added export-time material URI resolution against the opened bundle path before delegating to `editor_runtime::ExportService`.
- **Files modified:** `crates/server_runtime/src/lib.rs`, `crates/server_runtime/Cargo.toml`
- **Verification:** `cargo test -p server_runtime --test server_export_smoke -- --nocapture`; `pnpm run test:phase18-server`; `bash scripts/phase18-source-guards.sh --plan 05`
- **Committed in:** `d49454b`

---

**Total deviations:** 2 auto-fixed (1 Rule 3 blocking, 1 Rule 2 missing critical)
**Impact on plan:** Both fixes were required to satisfy the planned server runtime boundary and evidence gates. No adapter-owned export scheduler or desktop UI dependency was introduced.

## Issues Encountered

- `pnpm` emitted the existing engine warning: package expects Node `24.12.0`, current runtime is `v24.15.0`. Verification still passed.
- Rust emitted the existing macOS `AVAsset::tracksWithMediaType` deprecation warning in `media_runtime_desktop`; it is pre-existing and out of scope.
- `requirements.mark-complete PLAT-02 BIND-04 BIND-05` returned `not_found` because the current requirements file stores those IDs as narrative requirement rows rather than SDK-markable checklist entries. No manual requirements rewrite was made outside the SDK.

## Verification

- `cargo test -p server_runtime --lib -- --nocapture` - passed, 3 tests.
- `cargo check -p server_runtime --locked` - passed with the pre-existing macOS deprecation warning.
- `cargo test -p server_runtime --test server_export_smoke -- --nocapture` - passed, 3 tests.
- `pnpm run test:phase18-server` - passed with the existing Node engine warning.
- `bash scripts/phase18-source-guards.sh --plan 05` - passed.

## TDD Gate Compliance

- RED gate present for Task 1: `c08fcfc`.
- GREEN gate present for Task 1: `42064b7`.
- RED gate present for Task 2: `1b16b02`.
- GREEN gate present for Task 2: `d49454b`.

## Known Stubs

None. Stub scan found no TODO/FIXME/placeholder text or UI-flowing hardcoded empty data in the created/modified server runtime files.

## Threat Mitigations

- **T-18-17 / server bundle path and material URI tampering:** `.veproj` opens through `editor_runtime::ProjectSessionService`, which uses `project_store`, and export-time filesystem material paths are resolved through `project_store` classification.
- **T-18-18 / material information disclosure:** Relative in-bundle material paths are resolved explicitly; no absolute local fallback is treated as success.
- **T-18-19 / export evidence repudiation:** Smoke tests assert output file existence, validation metadata, progress, cancellation, and CLI status JSON.
- **T-18-20 / export job denial of service:** Export execution, status, cancellation, validation, and scheduler telemetry remain delegated to `editor_runtime::ExportService` and `task_runtime`.
- **T-18-SC / package install tampering:** No new external package install was performed; existing workspace crates and locked Rust dependencies were used.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 18-06 can add mobile contract documentation and run the full Phase 18 aggregate guard set with the server runtime artifact, real smoke tests, C ABI, and staged source guards in place.

## Self-Check: PASSED

- Found key files on disk: `crates/server_runtime/src/main.rs`, `crates/server_runtime/tests/server_export_smoke.rs`, `crates/server_runtime/src/lib.rs`, `crates/server_runtime/Cargo.toml`, and this summary.
- Confirmed all task commits exist in git history: `c08fcfc`, `42064b7`, `1b16b02`, and `d49454b`.

---
*Phase: 18-mobile-server-binding-architecture-and-runtime-ports*
*Completed: 2026-06-25*
