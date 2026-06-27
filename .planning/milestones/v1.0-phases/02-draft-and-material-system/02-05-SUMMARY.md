---
phase: 02-draft-and-material-system
plan: 05
subsystem: bindings-electron-smoke
tags: [rust, bindings-node, electron, generated-contracts, material-commands, playwright]
requires:
  - phase: 02-draft-and-material-system
    provides: Binding-facing material_service import/list/missing orchestration and generated Draft contracts
provides:
  - Rust-owned `importMaterial`, `listMaterials`, and `listMissingMaterials` command contracts
  - `execute_command` routing from bindings_node to material_service with standardized command envelopes
  - Generated command schema and TypeScript payload/result contracts for material commands
  - Electron smoke material row displaying generated material metadata through the bridge
  - Playwright coverage for material metadata display and renderer FFmpeg/ffprobe absence
affects: [bindings-node, generated-contracts, desktop-material-bin, phase-02-final-gates]
tech-stack:
  added: [media_runtime_desktop local production dependency in bindings_node]
  patterns: [rust-owned-material-command-contracts, binding-route-service-delegation, smoke-draft-material-display]
key-files:
  created:
    - .planning/phases/02-draft-and-material-system/02-05-SUMMARY.md
  modified:
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/schema_exports.rs
    - crates/bindings_node/Cargo.toml
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/tests/binding_smoke.rs
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
    - apps/desktop-electron/src/renderer/App.tsx
    - apps/desktop-electron/src/renderer/styles.css
    - apps/desktop-electron/tests/electron-smoke.spec.ts
    - .planning/STATE.md
    - .planning/ROADMAP.md
    - .planning/REQUIREMENTS.md
key-decisions:
  - "Material command payloads carry serialized Draft values and bundle/material paths through Rust-generated contracts."
  - "The binding uses desktop runtime/filesystem injection only to call material_service; it does not construct FFmpeg/ffprobe process commands or directly mutate project JSON."
  - "Electron remains a smoke surface by calling listMaterials with a generated-contract smoke Draft instead of adding import UI or timeline behavior."
patterns-established:
  - "CommandEnvelope material variants route to material_service and return typed ok/error/events envelopes."
  - "Generated command TS files import Draft.ts types instead of hand-writing parallel material contracts."
requirements-completed: [MAT-01, MAT-02, MAT-03, MAT-04]
duration: 18min
completed: 2026-06-17
---

# Phase 02 Plan 05: Material Commands and Electron Smoke Summary

**Rust-owned material command contracts now route through the binding service and drive a smoke-level Electron material metadata row.**

## Performance

- **Duration:** 18 min
- **Started:** 2026-06-17T02:48:30Z
- **Completed:** 2026-06-17T03:06:20Z
- **Tasks:** 2
- **Files modified:** 11 code/test/generated files plus planning metadata

## Accomplishments

- Added `importMaterial`, `listMaterials`, and `listMissingMaterials` command payloads/results to `draft_model`.
- Regenerated `schemas/command.schema.json`, `CommandEnvelope.ts`, and `CommandResultEnvelope.ts` from Rust.
- Routed material commands in `bindings_node::execute_command` to `material_service` with stable command error kinds.
- Added binding smoke tests for import, list, missing-material diagnostics, unsupported commands, and env-isolated runtime probes.
- Updated the Electron smoke workbench to display material display name, kind, duration microseconds, dimensions/audio detail, and status from the generated `listMaterials` command.
- Added Playwright assertions for the material row and renderer source checks that keep FFmpeg/ffprobe strings out of UI source.

## Task Commits

1. **Task 1: Add material command contracts and binding routes** - `e437191` (feat)
2. **Task 2: Add Electron material metadata smoke display** - `fd5d8ca` (feat)

**Plan metadata:** committed with this summary.

## Files Created/Modified

- `crates/draft_model/src/lib.rs` - Material command names, payloads, results, diagnostics, and error kinds.
- `crates/draft_model/tests/schema_exports.rs` - Rust-owned schema/TS generation for material command contracts.
- `crates/bindings_node/Cargo.toml` - Promoted local `media_runtime_desktop` dependency for binding runtime injection.
- `crates/bindings_node/src/lib.rs` - `execute_command` routing for import/list/missing material commands.
- `crates/bindings_node/tests/binding_smoke.rs` - Binding command smoke tests for material import/list/missing diagnostics.
- `schemas/command.schema.json` - Generated material command schema.
- `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Generated material command payload types.
- `apps/desktop-electron/src/generated/CommandResultEnvelope.ts` - Generated material command result and diagnostic types.
- `apps/desktop-electron/src/renderer/App.tsx` - Smoke material row rendered from `listMaterials` command output.
- `apps/desktop-electron/src/renderer/styles.css` - Compact material row metadata styling.
- `apps/desktop-electron/tests/electron-smoke.spec.ts` - Playwright assertions for material metadata display and renderer FFmpeg/ffprobe absence.
- `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md` - Plan progress and MAT-03 completion updates.

## Decisions Made

- Used serialized `Draft` payloads for material commands so the binding does not open or mutate draft state outside the material service path.
- Mapped material-service and project-store failures to stable command envelope errors: `invalidProject`, `projectIoFailed`, and `materialProbeFailed`.
- Kept Electron smoke data intentionally narrow: a generated-contract smoke Draft exercises `listMaterials`, while real import UI remains Phase 4 scope.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Isolated material command tests from runtime env mutation**
- **Found during:** Task 1 (`cargo test -p bindings_node -- --nocapture`)
- **Issue:** A new material import command smoke test could race existing tests that temporarily override `VE_FFMPEG_PATH` and `VE_FFPROBE_PATH`.
- **Fix:** Reused the existing `ENV_LOCK` guard around material command tests that trigger runtime discovery.
- **Files modified:** `crates/bindings_node/tests/binding_smoke.rs`
- **Verification:** `cargo test -p bindings_node -- --nocapture` passed.
- **Committed in:** `e437191`

**2. [Rule 2 - Missing Critical] Added material row styling outside the initial file list**
- **Found during:** Task 2
- **Issue:** The plan listed renderer/test files, but the existing `.material-row` CSS fixed the row at a single short line and would not fit the required metadata fields cleanly.
- **Fix:** Updated `apps/desktop-electron/src/renderer/styles.css` with compact title/metadata layout rules.
- **Verification:** `pnpm --filter @video-editor/desktop test` passed and Playwright asserted the metadata row contents.
- **Committed in:** `fd5d8ca`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 missing critical UI layout requirement)
**Impact on plan:** Both fixes were required for reliable verification and smoke display quality. No scope was added beyond material command routing and smoke metadata display.

## Issues Encountered

- Bare `gsd-tools` was not on PATH at executor start; the local shim at `$HOME/.codex/get-shit-done/bin/gsd-tools.cjs` was available. Planning state was updated manually before summary creation to preserve the requested close-out order.

## Known Stubs

- `apps/desktop-electron/src/renderer/App.tsx` defines an intentional `smokeDraft` fixture to exercise the generated `listMaterials` command. This does not block Plan 02-05 because Phase 2 explicitly keeps Electron as a smoke consumer; real material-bin import UI remains Phase 4 scope.

## Authentication Gates

None.

## Threat Flags

None - the implementation stayed inside the planned Electron/main binding, material_service, and renderer display trust boundaries. No network endpoints, auth paths, raw probe JSON persistence, direct renderer filesystem mutation, or renderer FFmpeg/ffprobe command construction were introduced.

## Verification

- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - passed.
- `cargo test -p bindings_node -- --nocapture` - passed.
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - passed.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed after the Task 1 commit.
- `pnpm --filter @video-editor/desktop test` - passed.
- `grep -R "ffmpeg\|ffprobe" apps/desktop-electron/src && exit 1 || true` - passed.
- `git diff --check` - passed.

## Self-Check: PASSED

- Found all key code, generated, test, and planning files on disk.
- Found task commits `e437191` and `fd5d8ca` in git history.
- Stub scan found only intentional smoke fixture data and legitimate null/optional checks.
- Threat scan found no new unplanned security surface in implementation files.
- No unexpected tracked file deletions were introduced.

## User Setup Required

None - no external service configuration required. FFmpeg and ffprobe must remain available through `VE_FFMPEG_PATH` / `VE_FFPROBE_PATH` or PATH for media-backed binding tests.

## Next Phase Readiness

Plan 02-06 can run final Phase 2 fixture and gate coverage with material command contracts, binding routes, generated artifacts, and Electron metadata smoke already in place.

---
*Phase: 02-draft-and-material-system*
*Completed: 2026-06-17*
