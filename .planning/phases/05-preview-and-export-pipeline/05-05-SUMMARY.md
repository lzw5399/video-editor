---
phase: 05-preview-and-export-pipeline
plan: 05
subsystem: bindings-preview
tags: [rust, napi, electron, preview, contracts]
requires:
  - phase: 05-04
    provides: preview_service frame/segment generation and cache invalidation
provides:
  - Rust-owned preview command payload and response contracts
  - Generated command schema and TypeScript preview command contracts
  - Node binding routes for preview frame, preview segment, and preview cache invalidation
  - Renderer command-only preview envelope helpers
  - Phase 5 source guard for renderer/render/cache ownership boundaries
affects: [draft_model, bindings_node, desktop-renderer, generated-contracts]
tech-stack:
  added: []
  patterns:
    - preview commands are generated from Rust contracts
    - bindings_node composes preview_service rather than duplicating preview semantics
    - renderer helpers build envelopes only
key-files:
  created:
    - crates/bindings_node/src/preview_export_service.rs
    - crates/bindings_node/tests/preview_commands.rs
    - scripts/phase5-source-guards.sh
  modified:
    - Cargo.lock
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
    - crates/bindings_node/Cargo.toml
    - crates/bindings_node/src/lib.rs
    - apps/desktop-electron/src/renderer/commandHelpers.ts
    - package.json
key-decisions:
  - "Preview payloads expose draft, cache root, target microsecond time/range, and invalidation references, but not FFmpeg args, render graph data, cache-key formulas, or derived scripts."
  - "bindings_node owns desktop composition of runtime discovery, DesktopFfmpegExecutor, and preview_service config."
  - "Renderer helpers remain command-envelope builders only."
patterns-established:
  - "Preview route errors map to CommandErrorKind::PreviewServiceFailed."
  - "Preview cache invalidation references contain target range, material dependencies, profile, and artifact path, not internal cache-key formulas."
  - "Phase 5 source guards reject renderer-owned FFmpeg/render graph/cache/process semantics."
requirements-completed: [PREV-01, PREV-02, PREV-03, PREV-04, EXP-02]
duration: 10 min
completed: 2026-06-17
---

# Phase 05 Plan 05: Preview Binding Commands Summary

**Rust-generated preview command contracts with Node binding routes and command-only Electron renderer helpers**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-17T18:40:00Z
- **Completed:** 2026-06-17T18:49:16Z
- **Tasks:** 3
- **Files modified:** 13

## Accomplishments

- Added `requestPreviewFrame`, `requestPreviewSegment`, and `invalidatePreviewCache` to Rust command contracts.
- Added generated JSON schema and TypeScript contract updates for preview payloads, artifact responses, invalidation responses, statuses, and diagnostics.
- Added `bindings_node::preview_export_service` to adapt command payloads into `preview_service` requests without duplicating cache, render graph, or FFmpeg semantics.
- Added renderer helper builders for preview commands while keeping renderer code command-only.
- Added `test:phase5-source-guards` to reject renderer-owned FFmpeg/render graph/cache/process behavior and ensure preview generated contracts exist.

## Task Commits

Each task was committed atomically:

1. **Task 05-05-01: Add Rust-owned preview command contracts and generated artifacts** - `c2ffa8d` (feat)
2. **Task 05-05-02: Route preview commands through bindings_node preview service** - `c2ffa8d` (feat)
3. **Task 05-05-03: Add renderer preview envelope helpers without semantic ownership** - `c2ffa8d` (feat)

**Plan metadata:** this summary commit

## Files Created/Modified

- `crates/draft_model/src/lib.rs` - Adds preview commands, payloads, response contracts, status, diagnostics, and `PreviewServiceFailed`.
- `crates/draft_model/tests/schema_exports.rs` - Adds preview contract coverage and schema/type generation.
- `schemas/command.schema.json` - Generated command schema with preview contracts.
- `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Generated preview command payload types.
- `apps/desktop-electron/src/generated/CommandResultEnvelope.ts` - Generated preview response/status/diagnostic types.
- `crates/bindings_node/src/preview_export_service.rs` - Binding adapter for preview frame, segment, and invalidation routes.
- `crates/bindings_node/src/lib.rs` - Adds preview commands to executeCommand allowlist and dispatch.
- `crates/bindings_node/tests/preview_commands.rs` - Covers preview service adapter, invalidation route, and mismatched preview command rejection.
- `apps/desktop-electron/src/renderer/commandHelpers.ts` - Adds preview command envelope builders.
- `scripts/phase5-source-guards.sh` - Adds Phase 5 renderer/source guard checks.
- `package.json` - Adds `test:phase5-source-guards`.

## Decisions Made

- Kept preview command contracts Rust-owned and generated into TypeScript instead of hand-writing desktop-only types.
- Did not expose FFmpeg args, render graph, internal cache keys, or generated scripts through the command payload/response contracts.
- Kept real preview generation behind the binding/service adapter; renderer helper functions only build command envelopes.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The first source-guard draft checked `git diff --exit-code` directly, which would fail during implementation before generated files are committed. This was replaced with explicit generated-header and preview-contract existence checks; `test:contracts` remains the dedicated generated artifact drift gate.

## Verification

- `cargo test -p draft_model schema_exports_include_preview_command_contracts -- --nocapture` - passed.
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` - passed.
- `cargo test -p bindings_node preview_commands -- --nocapture` - passed, 3 tests.
- `cargo test -p bindings_node -- --nocapture` - passed, 19 tests.
- `pnpm run test:phase5-source-guards` - passed.
- `pnpm --filter @video-editor/desktop test` - passed, 11 Playwright tests.
- `pnpm run test:contracts` - passed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for `05-06` to connect the desktop preview monitor UI to `requestPreviewFrame`, `requestPreviewSegment`, and `invalidatePreviewCache` through `window.videoEditorCore.executeCommand`.

---
*Phase: 05-preview-and-export-pipeline*
*Completed: 2026-06-17*
