---
phase: 20-long-timeline-product-uat-and-guard-baseline
verified: 2026-06-27T21:36:59Z
status: passed
score: 15/15 must-haves verified
behavior_unverified: 0
overrides_applied: 0
gaps: []
---

# Phase 20: Long Timeline Product UAT And Guard Baseline Verification Report

**Phase Goal:** Users can prove real long editing sessions stay responsive and canonical across preview, save/reopen, export, and continued editing.
**Verified:** 2026-06-27T21:36:59Z
**Status:** passed
**Re-verification:** Yes - closes the prior packaged canonical export gap.

## Goal Achievement

All Phase 20 observable truths are verified. The previous gap was a packaged Electron export state race: the UI could display a stale terminal export status while a new export command was still starting, and the Rust export registry used the same stable compiled job id for repeated attempts of the same draft/profile. The fix splits compiled render identity from attempt-scoped runtime/export job identity and clears stale terminal UI state at the start of a new export attempt.

## Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | User can complete a packaged product session that imports/opens mixed media, edits a long multi-track timeline, previews, saves, reopens, continues editing, exports, and exports again. | VERIFIED | `pnpm run test:phase20` passed, including packaged `product-long-timeline-uat.spec.ts` canonical reopen/export. |
| 2 | User can repeat edit/save/reopen/export cycles without `.veproj/project.json` gaining derived artifacts or changing canonical semantics unexpectedly. | VERIFIED | Packaged canonical UAT completed both exports and canonical summary comparisons. |
| 3 | User can select, scroll, zoom, scrub, move, trim, split, undo, redo, and preview on a long timeline while documented responsiveness budgets are met. | VERIFIED | Packaged responsiveness UAT passed inside `pnpm run test:phase20`. |
| 4 | User can keep scrubbing, editing inspector values, receiving preview frames, and committing or canceling interactions while export/probe/artifact/cache work run. | VERIFIED | Packaged scheduler pressure UAT passed inside `pnpm run test:phase20`. |
| 5 | Product success cannot come only from fallback, mock, artifact, CPU probe, DOM overlay, native-video proof, first-frame snapshot, or file-exists-only export proof. | VERIFIED | `scripts/phase20-source-guards.sh`, `scripts/no-product-fallback-guards.sh`, ffprobe metadata, and sampled-frame evidence gates passed. |
| 6 | Rust can generate a 180 segments/track x 3-track video/audio/text product fixture with 1s segments. | VERIFIED | `cargo test -p testkit --test long_timeline_product_fixture -- --nocapture` passed 7 tests. |
| 7 | Rust blocking pressure covers 1000 segments/track with structural boundedness; 3000 segments/track is diagnostic only. | VERIFIED | `phase20_blocking_1000_segments_per_track_keeps_localized_diff_bounded` passed; diagnostic 3000-segment test remains excluded from aggregate. |
| 8 | Generated `.veproj` is saved/reopened through `project_store` and compared by canonical draft facts. | VERIFIED | Materializer and canonical reopen checks passed in Rust and packaged UAT. |
| 9 | Playwright requests a Rust-generated long `.veproj` instead of synthesizing segment semantics in TypeScript. | VERIFIED | Source guard and UAT helper wiring passed. |
| 10 | Product runs have lightweight success summaries and rich failure evidence with product-readable and developer details. | VERIFIED | Phase 20 Playwright UAT produced success evidence for responsiveness, canonical/export, and pressure runs. |
| 11 | Evidence helpers compare normalized canonical draft facts and reject fallback/source-only product proof. | VERIFIED | Evidence helper and no-fallback guards passed. |
| 12 | Phase 20 UAT stays scoped to already-stable baseline semantics and does not expand into detailed Phase 19 parity. | VERIFIED | UAT covers baseline long-session operations; broader parity remains deferred to later phases. |
| 13 | Phase closeout requires packaged Electron evidence and blocks fallback/mock/artifact/CPU/DOM/native-video/first-frame/file-exists-only success. | VERIFIED | Aggregate `test:phase20` includes packaged Electron UAT and all source/fallback guards. |
| 14 | The 3000 segments/track pressure run is available as non-blocking diagnostic and excluded from blocking aggregate. | VERIFIED | `test:phase20-diagnostic` remains separate; aggregate does not invoke it. |
| 15 | Aggregate gates preserve success/failure evidence paths and fail source-only product proof. | VERIFIED | Phase 20 source guard and shared no-product-fallback guard passed. |

**Score:** 15/15 truths verified.

## Gap Closure

| Prior Gap | Resolution | Evidence |
|---|---|---|
| Packaged canonical reopen/export failed on the first export with stale `failed 0%` UI state. | Rust export runtime job ids are now attempt-scoped, while compiled sidecar identity stays stable. Starting a new UI export clears previous terminal phase/validation/diagnostic state immediately. Export settings are locked while an export is active so the UI cannot lose the current job id. | `crates/editor_runtime/src/export.rs`, `apps/desktop-electron/src/renderer/App.tsx`, `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx`, `cargo test -p bindings_node --test scheduler_export -- --nocapture --test-threads=1`, `pnpm --filter @video-editor/desktop exec playwright test tests/export-modal.spec.ts --reporter=line --workers=1`, `pnpm run test:phase20`. |

## Verification Commands

| Command | Result |
|---|---|
| `cargo test -p bindings_node scheduler_export_restarts_same_draft_profile_as_fresh_attempt --test scheduler_export` | PASS |
| `cargo test -p bindings_node --test scheduler_export -- --nocapture --test-threads=1` | PASS, 7 tests |
| `pnpm --filter @video-editor/desktop build` | PASS |
| `pnpm --filter @video-editor/desktop build:electron` | PASS |
| `pnpm --filter @video-editor/desktop exec playwright test tests/export-modal.spec.ts --reporter=line --workers=1` | PASS, 3 tests |
| `pnpm run test:phase20-desktop` | PASS, 3 packaged UAT tests |
| `pnpm run test:phase20` | PASS |

## Requirements Coverage

| Requirement | Status | Evidence |
|---|---|---|
| UAT11-01 | SATISFIED | Packaged long product session completed responsiveness, canonical reopen/export, and pressure UAT. |
| UAT11-02 | SATISFIED | Reopen/export cycles completed without derived artifact pollution or canonical semantic drift. |
| LONG11-01 | SATISFIED | Long timeline responsiveness UAT passed. |
| LONG11-02 | SATISFIED | Pressure UAT passed while export/probe/artifact/cache work was active. |
| GATE11-01 | SATISFIED | Source guards, no-fallback guards, bundled runtime ffprobe metadata, and sampled-frame evidence passed. |

## Human Verification Required

None. The phase has executable product UAT and aggregate gates, and all passed.

---

_Verified: 2026-06-27T21:36:59Z_
_Verifier: Codex_
