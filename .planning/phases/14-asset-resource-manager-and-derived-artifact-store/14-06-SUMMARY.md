---
phase: 14-asset-resource-manager-and-derived-artifact-store
plan: "06"
subsystem: bindings
tags: [rust, generated-contracts, node-bindings, preview-service, artifact-store]

requires:
  - phase: 14-05
    provides: Rust-owned artifact store status, GC, quota, generation jobs, and local manifest semantics
provides:
  - Generated artifact status, task, quota, generation action, and maintenance command contracts
  - Node binding routing for artifact status, quota, GC, retry, resume, and cancel commands
  - Rust/project-owned preview artifact root resolution with renderer cacheRoot deprecated for normal project preview
affects: [phase-14, bindings-node, preview-service, desktop-renderer, generated-contracts]

tech-stack:
  added: [artifact_store path dependency for preview_service]
  patterns:
    - "Generated command contracts carry safe display/status data only; caller payloads do not supply roots, cache keys, fingerprints, graph keys, dirty ranges, SQLite details, or FFmpeg args."
    - "bindings_node adapts JSON transport to Rust services and delegates artifact decisions to artifact_store."
    - "preview_service resolves project-local preview artifacts under `.veproj/derived/blobs/preview` when a bundle path is available."

key-files:
  created:
    - crates/bindings_node/src/artifact_store_service.rs
    - crates/bindings_node/tests/artifact_store_commands.rs
  modified:
    - Cargo.lock
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
    - apps/desktop-electron/src/renderer/App.tsx
    - apps/desktop-electron/src/renderer/commandHelpers.ts
    - crates/bindings_node/Cargo.toml
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/src/preview_export_service.rs
    - crates/bindings_node/tests/preview_commands.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/schema_exports.rs
    - crates/preview_service/Cargo.toml
    - crates/preview_service/src/service.rs
    - schemas/command.schema.json

key-decisions:
  - "Artifact commands are Rust-generated transport contracts; renderer code receives safe summaries and action flags, not derived-store internals."
  - "Artifact binding commands open the project-local artifact store and delegate status/quota/GC/job actions to artifact_store APIs."
  - "Preview requests now accept `bundlePath` and optional/deprecated `cacheRoot`; project preview output resolves through Rust to `.veproj/derived/blobs/preview`."

patterns-established:
  - "Command/payload mismatch protection now covers artifact maintenance commands."
  - "Bindings classify artifact store failures without panics or partial renderer-owned state."
  - "Project-local preview root resolution keeps renderer command helpers free of the old `/tmp/video-editor-preview-cache` default."

requirements-completed: [ASSET-01, ASSET-02, ASSET-03, ASSET-04, ASSET-05]

duration: 34 min
completed: 2026-06-19
---

# Phase 14 Plan 06: Generated Contracts And Binding Commands Summary

**Artifact store status/maintenance transport is generated from Rust and routed through bindings without moving artifact semantics into Electron.**

## Performance

- **Duration:** 34 min
- **Completed:** 2026-06-19T06:16:45Z
- **Tasks:** 3
- **Production commits:** 6

## Accomplishments

- Added generated artifact status, task, quota, generation action, and maintenance payload/result contracts in `draft_model`, `schemas/command.schema.json`, and desktop generated TypeScript.
- Added `artifact_store_service.rs` and binding tests for status, quota, dry-run/apply GC, retry/resume/cancel error classification, and mismatched command/payload rejection.
- Updated preview command payloads and preview service config so normal project preview requests can omit renderer `cacheRoot` and resolve artifacts under Rust-owned `.veproj/derived/blobs/preview`.
- Removed the desktop renderer's hard-coded `/tmp/video-editor-preview-cache` production path in favor of passing the current project `bundlePath`.

## Task Commits

1. **Task 14-06-01 RED: artifact contract coverage** - `a93a2d2` (test)
2. **Task 14-06-01 GREEN: artifact command contracts** - `f40fde4` (feat)
3. **Task 14-06-02 RED: artifact binding coverage** - `6f31385` (test)
4. **Task 14-06-02 GREEN: artifact store binding commands** - `f6610e2` (feat)
5. **Task 14-06-03 RED: project preview root coverage** - `a39cbd6` (test)
6. **Task 14-06-03 GREEN: Rust preview artifact roots** - `d5e89bd` (feat)

## Verification

- `cargo test -p draft_model schema_exports -- --nocapture` - PASS
- `cargo test -p bindings_node artifact_store_commands -- --nocapture` - PASS
- `cargo test -p bindings_node preview_commands -- --nocapture` - PASS
- `cargo test -p preview_service preview -- --nocapture` - PASS
- `pnpm run test:contracts` - PASS after committing generated contract updates
- `pnpm run test:phase14-source-guards` - PASS

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Removed renderer default preview cache root**
- **Found during:** Task 14-06-03
- **Issue:** The planned Rust/project-owned preview artifact root made `cacheRoot` optional, but production renderer code still supplied the old hard-coded `/tmp/video-editor-preview-cache` value.
- **Fix:** Updated desktop command helpers and App preview requests to pass `bundlePath` and omit default `cacheRoot` for normal project preview.
- **Files modified:** `apps/desktop-electron/src/renderer/App.tsx`, `apps/desktop-electron/src/renderer/commandHelpers.ts`
- **Verification:** `cargo test -p bindings_node preview_commands -- --nocapture`; `pnpm run test:phase14-source-guards`
- **Committed in:** `d5e89bd`

**Total deviations:** 1 auto-fixed (1 missing critical).
**Impact on plan:** Required to satisfy D-10 and keep renderer code from owning production artifact roots.

## Issues Encountered

- The executor stream disconnected during Task 14-06-03 after committing the RED test. The orchestrator spot-checked partial commits and dirty worktree state, completed the GREEN implementation, and re-ran the full plan verification before writing this summary.
- `pnpm run test:contracts` failed before `d5e89bd` because generated schema/TypeScript files were intentionally dirty. It passed after committing the generated updates.

## Known Stubs

None.

## Threat Flags

None - artifact command payloads, binding delegation, status response safety, and preview root ownership were covered by the plan threat model T-14-17 through T-14-20.

## User Setup Required

None.

## Next Phase Readiness

Plan 14-07 can build production resource/artifact status UI on top of generated command contracts and safe binding responses. The renderer can request artifact status/actions and project preview without computing artifact roots, SQLite details, cache keys, fingerprints, graph keys, dirty ranges, or FFmpeg arguments.

## Self-Check: PASSED

- Created files exist: `crates/bindings_node/src/artifact_store_service.rs`, `crates/bindings_node/tests/artifact_store_commands.rs`, and this summary.
- Task commits `a93a2d2`, `f40fde4`, `6f31385`, `f6610e2`, `a39cbd6`, and `d5e89bd` exist in git history.
- Required verification commands passed.
- Worktree has no uncommitted plan changes; only untouched untracked `reference/` remains.

---
*Phase: 14-asset-resource-manager-and-derived-artifact-store*
*Completed: 2026-06-19*
