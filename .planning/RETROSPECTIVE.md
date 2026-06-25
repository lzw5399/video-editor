# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — Production Core

**Shipped:** 2026-06-26
**Phases:** 25 | **Plans:** 187 | **Tasks:** 385

### What Was Built

- A Rust-owned `.veproj` editing core with draft/material/timeline semantics, commands, undo/redo, snapping, render graph, FFmpeg compiler, realtime preview/runtime contracts, and export validation.
- A Jianying-style Electron desktop editor with Simplified Chinese product UI, project entry, material library, preview/player, contextual inspector, timeline, and export modal.
- Production foundations for GPU preview, media IO, graph/cache coherence, artifact store, audio DSP, scheduler isolation, portable bindings, server runtime, template import, and Phase 19 retime/effects/transitions.

### What Worked

- Hard source guards and aggregate test scripts prevented renderer-owned FFmpeg, draft mutation, fallback success, and provider-ID leakage from returning.
- Later aggregate phases successfully superseded earlier blocked audit findings instead of preserving partial legacy paths.
- Independent UI and code-review passes caught real interaction/session and destructive confirmation risks before final Phase 19 verification.

### What Was Inefficient

- Planning artifacts accumulated stale quick-task/debug/verification state even after product behavior was fixed.
- `REQUIREMENTS.md` traceability drifted after the roadmap expanded from MVP to production architecture.
- Some phase verification artifacts were uneven: later validation and aggregate gates covered behavior, but root `*-VERIFICATION.md` consistency was not uniform.

### Patterns Established

- Rust owns semantics; Electron owns presentation and transport only.
- High-frequency UI uses immediate local affordance plus Rust-owned interaction sessions and coalesced commits.
- Product success must be visible user-flow evidence, not mock, artifact, first-frame, DOM, CPU probe, or fallback output.
- External template/provider adapters produce reports and first-party draft semantics; provider-native IDs stay external references.

### Key Lessons

1. Treat planning traceability as a release artifact, not just execution bookkeeping; stale traceability creates closeout friction even when code is correct.
2. Aggregate gates should explicitly include every changed product regression path, or late review will expose coverage gaps.
3. UI polish and ownership boundaries need independent review after implementation, not only design-time approval.
4. Fail-closed product policy is expensive early but prevents long-lived false-success paths.

### Cost Observations

- Model mix: Codex primary orchestration with GSD executor/reviewer/auditor subagents.
- Sessions: multi-day milestone execution ending with milestone close.
- Notable: The largest context cost came from long phase history and milestone archive generation; future milestones should archive earlier and keep active planning files compact.

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | multi-day | 25 | Moved from MVP scaffold to production Rust-owned editor architecture with explicit no-fallback and product E2E gates |

### Cumulative Quality

| Milestone | Gates | Coverage | Known Closeout Debt |
|-----------|-------|----------|---------------------|
| v1.0 | Rust tests, schema/contracts, source guards, Playwright/Electron, packaged E2E, no-product-fallback, UI audit, code review | 187/187 plans complete; 11/11 integration flows wired | Planning traceability cleanup, stale quick/debug artifacts, app metadata/icon, documented crop/export limitation |

### Top Lessons

1. Keep active ROADMAP/REQUIREMENTS compact after each milestone; archive full detail before starting the next cycle.
2. Tie every product-facing success state to real user-flow evidence and source guards.
3. Re-run traceability cleanup whenever roadmap scope changes materially.
