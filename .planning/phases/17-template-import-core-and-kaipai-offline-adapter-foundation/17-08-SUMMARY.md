---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
plan: "08"
subsystem: template-import-testing
tags: [template-import, kaipai, preview, export, no-fallback, rust, ffmpeg]

requires:
  - phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
    provides: Project-session offline Kaipai import and localized resource persistence from Plan 17-06
  - phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
    provides: Static center-anchor rotation preview/export parity from Plan 17-07
provides:
  - Imported fixture export evidence for MP4 metadata, layer order, visible text, audio streams, canonical project JSON, and adaptation reports
  - Imported fixture realtime preview evidence through the render-graph product path with fallback/mock/artifact/CPU/Android success rejected
  - Phase 17 aggregate scripts for Rust import, source guards, fixture export, fixture preview, no-product-fallback, workspace check, and contract drift gates
affects: [phase17-verification, template-import, adapter-kaipai, realtime-preview, export]

tech-stack:
  added: [adapter_kaipai dev dependency in testkit, draft_import dev dependency in testkit, project_store dev dependency in testkit]
  patterns:
    - Import fixture evidence resolves bundle-local resource refs only for test execution while keeping saved project JSON canonical
    - Preview success is classified through realtime render-graph support diagnostics, not artifact or fallback evidence
    - Export fixture evidence validates media output and adaptation report snapshots together

key-files:
  created:
    - crates/testkit/tests/template_import_exports.rs
    - crates/testkit/tests/template_import_preview.rs
  modified:
    - Cargo.lock
    - crates/testkit/Cargo.toml
    - crates/adapter_kaipai/src/lib.rs
    - crates/adapter_kaipai/src/mapper.rs
    - crates/adapter_kaipai/tests/mapper.rs
    - crates/adapter_kaipai/tests/mapper_fixtures.rs
    - package.json
    - scripts/phase17-source-guards.sh

key-decisions:
  - "Imported template fixture acceptance is tied to real preview/export product-path evidence plus adaptation report classifications; successful file creation alone is insufficient."
  - "Test execution may resolve localized bundle-relative resources to filesystem paths, but persisted .veproj/project.json remains canonical and provider/runtime-ref free."
  - "Phase 17 source guards now fail raw provider formula semantics, remote runtime/render URLs, Android/live provider dependencies, and fallback success evidence."

patterns-established:
  - "Template import fixture gates combine report snapshot checks with output media assertions so unsupported provider behavior cannot be hidden by a successful export."
  - "Realtime preview fixture tests assert render-graph support and inspect diagnostics for fallback evidence before product preview is considered supported."

requirements-completed:
  - COMP-01
  - COMP-02
  - PRODFX-05
  - NO-FALLBACK-01

duration: 28 min
completed: 2026-06-24
status: complete
---

# Phase 17 Plan 08: Template Import Evidence Summary

**Imported Kaipai fixture drafts now prove canonical preview/export behavior through repo-owned render-graph and FFmpeg paths with no fallback success.**

## Performance

- **Duration:** 28 min
- **Started:** 2026-06-24T09:50:00Z
- **Completed:** 2026-06-24T10:18:00Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Added export fixture gates for main video, PIP, text sticker, BGM/audio, missing-resource, and native-effect fixtures. The gates validate adaptation report snapshots, canonical project JSON, non-empty MP4 output, dimensions, frame rate, duration, visible text, layer order, and audio streams where applicable.
- Added realtime preview fixture gates that prepare imported drafts through the render-graph preview path and reject fallback/mock/artifact/CPU/Android evidence.
- Wired Phase 17 package scripts so `test:phase17` runs Rust import/adapter/session/preview/export gates, source guards, no-product-fallback, workspace check, and contract drift checks.
- Strengthened Phase 17 source guards for raw formula JSON as runtime semantics, remote render/runtime URLs, Android/live provider dependencies, and fallback success evidence.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Add imported fixture preview and export evidence tests** - `34b90c3` (test)
2. **Task 1 GREEN: Implement template import evidence gates** - `f378465` (feat)
3. **Task 2: Wire full Phase 17 automated gates** - `c9b0bf4` (chore)
4. **Close-out correction: Format template import evidence gates** - `f1fbb32` (style)

_Task 1 followed TDD: the first commit added the failing evidence tests, then the GREEN commit fixed the adapter/test harness path and made the tests pass._

## Files Created/Modified

- `crates/testkit/tests/template_import_exports.rs` - Imports fixture bundles, saves/reopens canonical drafts, runs FFmpeg exports, validates media metadata, and asserts text/layer/audio/report evidence.
- `crates/testkit/tests/template_import_preview.rs` - Imports supported fixture bundles, prepares realtime render-graph preview work, and rejects fallback evidence.
- `crates/testkit/Cargo.toml` and `Cargo.lock` - Added testkit dev dependencies on `adapter_kaipai`, `draft_import`, and `project_store`.
- `crates/adapter_kaipai/src/mapper.rs` - Fixed fixture mapping correctness for source ranges and renderable material metadata.
- `crates/adapter_kaipai/tests/mapper.rs` - Extended mapper assertions for corrected duration and material metadata behavior.
- `crates/adapter_kaipai/src/lib.rs` and `crates/adapter_kaipai/tests/mapper_fixtures.rs` - Formatter-only changes from the close-out rustfmt pass.
- `package.json` - Added/updated Phase 17 aggregate scripts for Rust, export fixture, preview fixture, source guard, no-fallback, workspace check, and contracts.
- `scripts/phase17-source-guards.sh` - Added required script checks plus negative guard coverage for raw provider/runtime/fallback evidence.

## Decisions Made

- Imported fixture evidence must combine adaptation reports with preview/export output assertions so unsupported provider behavior cannot be counted as success.
- Test-only resource URI resolution is allowed for executing export jobs, but the saved `.veproj/project.json` must remain canonical and free of raw provider/runtime references.
- Source guards are part of Phase 17 acceptance, not advisory lint, because provider/runtime/fallback leakage would violate the import ownership boundary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created bundle directories before resource localization**
- **Found during:** Task 1 (Add imported fixture preview and export evidence tests)
- **Issue:** The RED tests showed resource localization needed the `.veproj` bundle directory before copied resources could be resolved for preview/export execution.
- **Fix:** Test helpers now create bundle/source roots before mapping/import application.
- **Files modified:** `crates/testkit/tests/template_import_exports.rs`, `crates/testkit/tests/template_import_preview.rs`
- **Verification:** `cargo test -p testkit template_import_exports -- --nocapture`; `cargo test -p testkit template_import_preview -- --nocapture`
- **Committed in:** `f378465`

**2. [Rule 1 - Bug] Corrected imported video material source duration**
- **Found during:** Task 1 (Add imported fixture preview and export evidence tests)
- **Issue:** Nonzero source starts could create source ranges past material duration because mapped video material duration used source duration instead of source end.
- **Fix:** Video materials now use `source_end` for metadata duration so canonical source timeranges are valid.
- **Files modified:** `crates/adapter_kaipai/src/mapper.rs`, `crates/adapter_kaipai/tests/mapper.rs`
- **Verification:** `cargo test -p adapter_kaipai offline_mapper -- --nocapture`; `pnpm run test:phase17`
- **Committed in:** `f378465`

**3. [Rule 2 - Missing Critical] Added renderable metadata for PIP/image/sticker materials**
- **Found during:** Task 1 (Add imported fixture preview and export evidence tests)
- **Issue:** Renderable non-source video/image/sticker materials lacked dimensions, which made realtime preview capability classification reject supported PIP/overlay fixtures.
- **Fix:** Mapper now records dimensions for renderable imported resources so preview/export can classify and render them through generic draft semantics.
- **Files modified:** `crates/adapter_kaipai/src/mapper.rs`, `crates/adapter_kaipai/tests/mapper.rs`
- **Verification:** `cargo test -p testkit template_import_preview -- --nocapture`; `pnpm run test:phase17`
- **Committed in:** `f378465`

**4. [Rule 3 - Blocking] Formatted Phase 17 import/adapter evidence files**
- **Found during:** Close-out verification after Task 2
- **Issue:** `cargo fmt --all --check` failed on Phase 17 adapter and new test files, including the Plan 17-08 evidence tests.
- **Fix:** Ran `cargo fmt --all` and committed the formatter-only result.
- **Files modified:** `crates/adapter_kaipai/src/lib.rs`, `crates/adapter_kaipai/src/mapper.rs`, `crates/adapter_kaipai/tests/mapper.rs`, `crates/adapter_kaipai/tests/mapper_fixtures.rs`, `crates/testkit/tests/template_import_exports.rs`, `crates/testkit/tests/template_import_preview.rs`
- **Verification:** `cargo fmt --all --check`; `pnpm run test:phase17`
- **Committed in:** `f1fbb32`

---

**Total deviations:** 4 auto-fixed (2 blocking, 1 bug, 1 missing critical)
**Impact on plan:** All fixes were required to make the planned preview/export evidence trustworthy. No UI, runtime ownership, or provider-specific render semantics were added.

## Verification

All planned verification commands passed:

- `cargo test -p testkit template_import_exports -- --nocapture`
- `cargo test -p testkit template_import_preview -- --nocapture`
- `cargo test -p adapter_kaipai offline_mapper -- --nocapture`
- `cargo test -p bindings_node project_session_import_kaipai -- --nocapture`
- `pnpm run test:phase17-source-guards`
- `pnpm run test:phase17-rust`
- `pnpm run test:phase17-export-fixtures`
- `pnpm run test:phase17-preview`
- `pnpm run test:no-product-fallback`
- `cargo check --workspace --locked`
- `pnpm run test:contracts`
- `pnpm run test:phase17`
- `cargo fmt --all --check`

Observed non-blocking warnings:

- `pnpm` reported the package engine expects Node `24.12.0`; the shell used Node `v24.15.0`. Commands still passed.
- Rust emitted the existing `tracksWithMediaType` deprecation warning in `crates/media_runtime_desktop/src/platform/macos.rs`; this plan did not touch that path.

## Known Stubs

None. Stub scan found only intentional negative-test/source-guard literals such as injected `rawFormula` and remote URL strings.

## Threat Flags

None. New threat surface is limited to automated tests and source guards; no runtime endpoints, auth paths, schema trust boundary changes, or product fallback paths were introduced.

## Issues Encountered

- The TDD RED tests initially failed because test harness setup lacked a bundle directory and execution-local resource URI resolution. Fixed in `f378465`.
- The close-out formatter check failed on Phase 17 import/adapter evidence files. Fixed in `f1fbb32`.

## User Setup Required

None - no external service configuration required.

## TDD Gate Compliance

- RED gate present: `34b90c3` (`test(17-08): add failing template import evidence tests`)
- GREEN gate present after RED: `f378465` (`feat(17-08): implement template import evidence gates`)
- Refactor/style cleanup present after GREEN: `f1fbb32` (`style(17-08): format template import evidence gates`)

## Next Phase Readiness

Plan 17-08 is complete. Phase 17 can continue with downstream UI/report integration only after preserving these backend import, preview/export, source guard, and no-fallback gates.

## Self-Check: PASSED

- Found summary file: `.planning/phases/17-template-import-core-and-kaipai-offline-adapter-foundation/17-08-SUMMARY.md`
- Found task commits: `34b90c3`, `f378465`, `c9b0bf4`, `f1fbb32`
- No tracked deletions were introduced by the close-out style commit.

---
*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Completed: 2026-06-24*
