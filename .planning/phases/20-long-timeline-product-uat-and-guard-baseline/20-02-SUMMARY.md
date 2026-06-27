---
phase: 20-long-timeline-product-uat-and-guard-baseline
plan: 02
subsystem: testing
tags: [playwright, long-timeline, evidence, veproj, ffmpeg-runtime]

requires:
  - phase: 20-long-timeline-product-uat-and-guard-baseline
    provides: Phase 20 Rust materializer CLI and long `.veproj` fixture contracts from Plan 20-01.
provides:
  - Playwright helper for Rust-owned Phase 20 long `.veproj` materialization.
  - Canonical draft summary and derived artifact pollution checks for save/reopen cycles.
  - Production preview evidence and export media validation helpers for Phase 20 UAT.
  - Product-readable success/failure evidence summary helpers with developer detail paths.
affects: [phase20-product-uat, phase20-source-guards, phase20-aggregate-gate]

tech-stack:
  added: []
  patterns:
    - Playwright helpers orchestrate Rust/testkit fixture materialization instead of constructing draft semantics.
    - Product evidence helpers require renderGraphGpuComposited preview proof and bundled ffprobe/ffmpeg runtime discovery.

key-files:
  created:
    - apps/desktop-electron/tests/helpers/longTimelineFixture.ts
    - apps/desktop-electron/tests/helpers/longTimelineEvidence.ts
    - apps/desktop-electron/tests/longTimelineFixture.spec.ts
    - apps/desktop-electron/tests/longTimelineEvidence.spec.ts
  modified: []

key-decisions:
  - "Keep Phase 20 TypeScript fixture code limited to path/process orchestration; Rust testkit owns the 540 segment draft semantics."
  - "Canonical draft summaries exclude evidence paths and only preserve materials, tracks, segments, timing, visual, audio, text, canvas, and revision facts."
  - "Export proof uses app runtime-discovered bundled ffprobe/ffmpeg for metadata and sampled frame evidence; file existence is only a prerequisite."

patterns-established:
  - "Phase20LongTimelineFixtures: one run root with exports and evidence directories, two distinct export paths, long media fixture paths, locked scale facts, and parsed Rust materializer output."
  - "CanonicalDraftSummary: normalized `.veproj/project.json` facts without derived artifacts, runtime handles, preview/export caches, or path-dependent evidence fields."
  - "Phase 20 evidence bundles: product-readable workflow/stage/status summary plus separate developerDetails for telemetry, native observations, screenshots, project summaries, ffprobe, and sampled frames."

requirements-completed: [UAT11-02, GATE11-01]

duration: 12 min
completed: 2026-06-27
status: complete
---

# Phase 20 Plan 02: Playwright Long-Timeline Evidence Helpers Summary

**Playwright evidence helpers that materialize Rust-owned long `.veproj` fixtures and reject fallback preview/export proof**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-27T18:05:16Z
- **Completed:** 2026-06-27T18:16:50Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added `generatePhase20LongTimelineFixture`, which creates `test-results/phase20/<run-id>/exports`, `evidence`, two export outputs, repo long media paths, locked 180 x 3 x 1s scale facts, and invokes `cargo run -p testkit --bin phase20_long_fixture`.
- Added canonical draft summary helpers that preserve `.veproj/project.json` material/track/segment/timing/visual/audio/text/canvas/revision facts and reject derived artifact pollution.
- Added Phase 20 preview and export evidence helpers that require `renderGraphGpuComposited`, `renderGraphGpu`, diagnostic source `none`, unchanged artifact preview-frame request counts, bundled runtime discovery, ffprobe metadata, and sampled frame evidence.
- Added evidence summary and failure collection helpers with product-readable workflow/stage/status fields and separate developer details for telemetry, native observations, screenshots, project summaries, ffprobe, and sampled frames.

## Task Commits

Each task was committed with TDD RED/GREEN gates:

1. **Task 1: Add the Playwright long fixture helper**
   - `dbcd32b` test: failing long fixture helper contract
   - `c3f6663` feat: Phase 20 long fixture helper
2. **Task 2: Add canonical, preview, export, and evidence bundle helpers**
   - `ba42c49` test: failing long evidence helper contract
   - `26a7711` feat: Phase 20 evidence helpers

## Files Created/Modified

- `apps/desktop-electron/tests/helpers/longTimelineFixture.ts` - Phase 20 run directory, export/evidence path, media path, locked scale, and Rust materializer orchestration helper.
- `apps/desktop-electron/tests/helpers/longTimelineEvidence.ts` - Canonical draft, no-derived-artifact, preview proof, export proof, success summary, and failure evidence helpers.
- `apps/desktop-electron/tests/longTimelineFixture.spec.ts` - TDD contract for Rust materializer delegation and no TypeScript-authored segment semantics.
- `apps/desktop-electron/tests/longTimelineEvidence.spec.ts` - TDD contract for canonical summaries, evidence JSON shape, and fallback/file-exists-only rejection source checks.

## Decisions Made

- TypeScript path helpers parse the Rust materializer summary and assert the locked scale facts, but never write material, track, or segment arrays.
- Canonical summaries intentionally omit `projectJsonPath` and other evidence/runtime paths so save/reopen comparisons are path-independent.
- Export validation treats output file existence as a prerequisite only; success requires bundled ffprobe metadata and sampled frames produced through bundled ffmpeg.

## Verification

- `pnpm --filter @video-editor/desktop exec playwright test tests/longTimelineFixture.spec.ts --reporter=line --workers=1` - passed, 2 tests.
- `pnpm --filter @video-editor/desktop exec playwright test tests/longTimelineEvidence.spec.ts --reporter=line --workers=1` - passed, 3 tests.
- `pnpm --filter @video-editor/desktop exec playwright test tests/longTimelineFixture.spec.ts tests/longTimelineEvidence.spec.ts --reporter=line --workers=1` - passed, 5 tests.
- `pnpm --filter @video-editor/desktop build` - passed.
- Acceptance checks confirmed required helper exports, the Rust materializer command string, no TypeScript-authored 540 segment arrays, production preview evidence requirements, bundled ffprobe/ffmpeg discovery, sampled frame validation, and product/developer evidence summary fields.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Removed path-dependent facts from canonical summaries**
- **Found during:** Task 2 GREEN implementation review
- **Issue:** The first canonical summary shape included `projectJsonPath`, which would make semantic equality depend on temp bundle locations and contradict D-15's requirement to ignore absolute temp/evidence paths.
- **Fix:** Removed `projectJsonPath` from `CanonicalDraftSummary`; evidence paths remain in developer details instead.
- **Files modified:** `apps/desktop-electron/tests/helpers/longTimelineEvidence.ts`
- **Verification:** `pnpm --filter @video-editor/desktop exec playwright test tests/longTimelineEvidence.spec.ts --reporter=line --workers=1`; `pnpm --filter @video-editor/desktop build`
- **Committed in:** `26a7711`

---

**Total deviations:** 1 auto-fixed (1 Rule 2 missing critical)
**Impact on plan:** The fix tightened the planned canonical comparison behavior and avoided path-sensitive false drift. No product scope was added.

## Issues Encountered

- `pnpm` warned that the current Node version is `v24.15.0` while the repo engine asks for `24.12.0`; commands still completed successfully.
- The desktop build emitted existing Rust warnings in `media_runtime_desktop` and `bindings_node`; these are pre-existing and outside this helper plan.

## Known Stubs

None. Stub scan only found normal empty-array initialization and null checks in helper/test logic; nothing flows to UI rendering as placeholder product evidence.

## Authentication Gates

None.

## Threat Flags

None. The local Rust materializer process, bundled runtime discovery, and evidence JSON persistence surfaces were already covered by T20-04 through T20-07 in the plan threat model.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 20-03 can use `generatePhase20LongTimelineFixture`, `readCanonicalDraftSummary`, `expectCanonicalDraftStable`, `expectPhase20PreviewProductionEvidence`, `expectPhase20ExportMedia`, `writePhase20EvidenceSummary`, and `collectPhase20FailureEvidence` to build the packaged long-session UAT.

## Self-Check

PASSED.

- Found all four key files created by the plan.
- Found all four task commits in git history: `dbcd32b`, `c3f6663`, `ba42c49`, `26a7711`.
- Verified `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-02-SUMMARY.md` exists on disk.

---
*Phase: 20-long-timeline-product-uat-and-guard-baseline*
*Completed: 2026-06-27*
