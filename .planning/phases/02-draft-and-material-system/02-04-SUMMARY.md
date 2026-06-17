---
phase: 02-draft-and-material-system
plan: 04
subsystem: material-import
tags: [rust, bindings-node, draft-model, material-service, generated-contracts, ffprobe]
requires:
  - phase: 02-draft-and-material-system
    provides: Draft schema, project-store persistence, URI helpers, media-runtime probing, and generated material fixtures
provides:
  - Binding-facing `bindings_node::material_service` import/list/missing-material orchestration
  - Pure `draft_model` material registry mutation helpers with validation rollback
  - Rust-generated draft JSON Schema and TypeScript contracts
  - Recoverable missing-material diagnostics with original URI and last-known resolved path details
  - Runtime-boundary documentation for material import ownership and derived artifact exclusions
affects: [bindings-node, desktop-material-commands, generated-contracts, material-bin, project-store]
tech-stack:
  added: [project_store local dependency in bindings_node, test-only media_runtime_desktop, testkit, tempfile]
  patterns: [binding-facing-service-orchestration, pure-registry-helpers, rust-generated-draft-contracts, recoverable-missing-material-diagnostics]
key-files:
  created:
    - crates/bindings_node/src/material_service.rs
    - crates/bindings_node/tests/material_service.rs
    - schemas/draft.schema.json
    - apps/desktop-electron/src/generated/Draft.ts
  modified:
    - Cargo.lock
    - crates/bindings_node/Cargo.toml
    - crates/bindings_node/src/lib.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/src/material.rs
    - crates/draft_model/tests/draft_schema.rs
    - crates/draft_model/tests/schema_exports.rs
    - docs/runtime-boundaries.md
key-decisions:
  - "Material import orchestration lives in `bindings_node::material_service`; `project_store` remains limited to persistence and URI helpers."
  - "Material registry mutation helpers stay pure in `draft_model` and validate/roll back failed mutations."
  - "Draft schema and TypeScript draft contracts are generated from Rust and exclude derived cache/probe/render artifacts."
patterns-established:
  - "Binding-facing services coordinate project-store URI helpers, media-runtime probes, draft-model registry helpers, validation, and save."
  - "Missing materials remain draft entries and are returned as classified diagnostics with original URI and last-known resolved path."
requirements-completed: [MAT-01, MAT-02, MAT-04, DRAFT-04]
duration: 9min
completed: 2026-06-17
---

# Phase 02 Plan 04: Material Import Service Summary

**Binding-facing Rust material import orchestration with pure registry helpers, recoverable missing diagnostics, and generated draft contracts.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-17T02:39:26Z
- **Completed:** 2026-06-17T02:48:06Z
- **Tasks:** 2
- **Files modified:** 12 code/test/generated/doc files plus planning metadata

## Accomplishments

- Added pure `draft_model` helpers for adding/upserting materials and marking available, missing, or probe-failed states with validation rollback.
- Extended Rust contract generation to emit `schemas/draft.schema.json` and `apps/desktop-electron/src/generated/Draft.ts`.
- Added `bindings_node::material_service` for video/image/audio import, material listing, missing-material diagnostics, validation, and project-bundle save orchestration.
- Added service tests proving generated video, image, and audio imports persist normalized metadata and missing files remain recoverable draft state.
- Updated runtime-boundary docs to make material import ownership explicit and keep thumbnails, waveforms, raw probe JSON, preview caches, render graphs, FFmpeg scripts, proxies, and exports outside `project.json`.

## Task Commits

1. **Task 1: Add pure material registry helpers and generated draft contracts** - `17becb5` (feat)
2. **Task 2: Implement material import service outside project_store** - `960579b` (feat)

**Plan metadata:** committed with this summary.

## Files Created/Modified

- `crates/draft_model/src/material.rs` - Pure material registry mutation and status helpers.
- `crates/draft_model/src/lib.rs` - Public exports for material helper APIs.
- `crates/draft_model/tests/draft_schema.rs` - Registry helper success and rollback coverage.
- `crates/draft_model/tests/schema_exports.rs` - Draft schema and TypeScript contract generation.
- `schemas/draft.schema.json` - Rust-generated draft/material/timeline JSON Schema.
- `apps/desktop-electron/src/generated/Draft.ts` - Rust-generated TypeScript draft contracts.
- `crates/bindings_node/src/material_service.rs` - Binding-facing material import and missing diagnostics service.
- `crates/bindings_node/tests/material_service.rs` - Video/image/audio import and missing-material preservation tests.
- `crates/bindings_node/Cargo.toml` and `Cargo.lock` - Local service/test dependency edges.
- `crates/bindings_node/src/lib.rs` - Public material service module export.
- `docs/runtime-boundaries.md` - Material import orchestration and derived artifact boundary documentation.

## Decisions Made

- Kept `project_store` out of import orchestration: it classifies/resolves material URIs and saves validated drafts only.
- Used deterministic caller-supplied IDs in tests and retained a deterministic URI-hash fallback helper instead of adding a UUID dependency.
- Persisted normalized metadata and bounded error text only; raw ffprobe JSON and cache/render artifacts remain derived outputs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected missing-material diagnostic priority**
- **Found during:** Task 2 (`cargo test -p bindings_node material_service -- --nocapture`)
- **Issue:** A locally missing file already marked `Missing` was classified as `markedMissing` instead of `missingFile`.
- **Fix:** Prioritized resolved local path absence over stored missing status in `diagnostic_for_material`.
- **Files modified:** `crates/bindings_node/src/material_service.rs`
- **Verification:** `cargo test -p bindings_node material_service -- --nocapture` passed.
- **Committed in:** `960579b`

---

**Total deviations:** 1 auto-fixed (Rule 1 bug)
**Impact on plan:** The fix was required for MAT-04 diagnostic correctness and did not expand scope.

## Issues Encountered

- `gsd-tools` was not on PATH in this shell; the local shim at `/Users/zhiwen/.codex/get-shit-done/bin/gsd-tools.cjs` was available for state reads, and tracked planning metadata was updated directly.

## Known Stubs

None.

## Authentication Gates

None.

## Threat Flags

None - the plan implemented the local material path, project-store, media-runtime, and generated-contract trust boundaries already covered by the threat model. No network endpoints, auth paths, or raw probe JSON persistence were introduced.

## Verification

- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - passed.
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - passed.
- `cargo test -p draft_model material_registry -- --nocapture` - passed.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed.
- `grep -R "FfmpegExecutor\|PlatformFileSystem\|std::fs\|std::process" crates/draft_model/src && exit 1 || true` - passed.
- `cargo test -p bindings_node material_service -- --nocapture` - passed.
- `cargo test -p media_runtime material_probe -- --nocapture` - passed.
- `cargo test -p project_store round_trip -- --nocapture` - passed.
- `grep -R "probe_material_metadata\|FfmpegExecutor\|ffprobe\|ffmpeg" crates/project_store/src && exit 1 || true` - passed.

## Self-Check: PASSED

- Found all key created/modified files on disk.
- Found task commits `17becb5` and `960579b` in git history.
- Stub scan found no placeholder/TODO/FIXME or hardcoded empty UI-data stubs in files changed by this plan.
- Generated draft schema and TypeScript scans found no thumbnail, waveform, raw probe JSON, preview cache, render graph, FFmpeg script, or export contract fields.
- No unexpected tracked file deletions were introduced.

## User Setup Required

None - no external service configuration required. FFmpeg and ffprobe must remain available through `VE_FFMPEG_PATH` / `VE_FFPROBE_PATH` or PATH for media-backed tests.

## Next Phase Readiness

Plan 02-05 can expose material commands through the binding command surface and add Electron smoke coverage using the generated `Draft.ts` contracts and the service APIs from this plan.

---
*Phase: 02-draft-and-material-system*
*Completed: 2026-06-17*
