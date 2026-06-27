# Phase 20: Long Timeline Product UAT And Guard Baseline - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-28
**Phase:** 20-Long Timeline Product UAT And Guard Baseline
**Areas discussed:** Long Timeline Scale, Product UAT Path, Performance Budgets, Evidence Artifacts

---

## Long Timeline Scale

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid baseline | Product E2E uses a stable medium-long timeline; Rust/testkit runs larger pressure cases. | ✓ |
| Heavy product baseline | Product E2E uses about `300 segments/track x 3 tracks`, increasing realism but raising packaged test flake risk. | |
| Small product baseline first | Product E2E uses about `50-100 segments/track`, making the flow stable but weakening long-timeline evidence. | |

**User's choice:** Hybrid baseline.
**Notes:** Product E2E baseline was then fixed at `180 segments/track x 3 tracks`, `1s/segment`, about 3 minutes total. Rust/testkit blocking pressure was fixed at `1000 segments/track`, with `3000 segments/track` as non-blocking diagnostic pressure.

---

## Product UAT Path

| Option | Description | Selected |
|--------|-------------|----------|
| Open generated project, then edit through UI | Generate the long `.veproj` through Rust/testkit and use product UI for meaningful edits, preview, save/reopen, and export. | ✓ |
| Build long timeline entirely through UI | Construct all long timeline segments through UI, maximizing user realism but making the test slow and fragile. | |
| Two-lane product UAT | Use both a short UI-created project path and a long generated-project path. | |

**User's choice:** Open generated project, then edit through UI.
**Notes:** The UI core edit set is selection, scroll/zoom, scrub/play, move, trim, split, undo/redo, inspector visual edit, save/reopen, and export. The UAT must run two reopen cycles and two exports. Packaged Electron is the blocking gate; dev flow is diagnostic or shortened.

---

## Performance Budgets

| Option | Description | Selected |
|--------|-------------|----------|
| Pragmatic budgets | Inspector edit `<= 2.5s`; selection/move/trim/split/undo/redo `<= 2s`; scroll/zoom/scrub visible feedback `<= 1.5s`. | ✓ |
| Aggressive budgets | Most operations `<= 1s` or below, stronger but likely flaky at this stage. | |
| Loose budgets | Most operations `<= 5s`, stable but too weak to catch long-timeline problems. | |

**User's choice:** Pragmatic budgets.
**Notes:** Scheduler/preview budget uses the current stress baseline: queue latency p95 `<= 2s`, normal product rejected count `0`, fallback count `0`, stale generations may be rejected but must not present, and visible preview center must change under pressure. Export validates metadata and sampled semantic frames without a strict wall-clock export duration gate. Rust/testkit focuses on bounded diff, dirty range, and cache invalidation assertions; wall-clock is diagnostic only.

---

## Evidence Artifacts

| Option | Description | Selected |
|--------|-------------|----------|
| Structured evidence bundle | Success keeps lightweight JSON summaries; failure keeps full trace/screenshot/video and developer details. | ✓ |
| Full artifacts always | Always keep trace/screenshot/video/export frames for maximum auditability at higher storage/noise cost. | |
| Failure-only artifacts | Keep heavy artifacts only on failure, saving space but weakening baseline evidence. | |

**User's choice:** Structured evidence bundle.
**Notes:** Save/reopen evidence uses semantic normalized comparison. No-fallback/source guards are blocking phase gates and must cover long session, generated `.veproj`, packaged UAT, and export evidence. Failure diagnostics should include product summary plus developer details.

---

## the agent's Discretion

- Choose exact helper names, test file names, JSON evidence schema details, export timeout values, sampled-frame extraction implementation, and non-blocking diagnostic command wiring.

## Deferred Ideas

- Crop/export parity decisions belong to Phase 22.
- Existing Phase 19 effect/filter/transition/mask/blend parity and diagnostics taxonomy belong to Phase 23.
- Shortcut and high-frequency interaction hardening belongs to Phase 21.
- UI polish and final acceptance sweep belong to Phase 24.
