# Roadmap: Video Editor

## Overview

v1.1 continues after the shipped v1.0 production core and focuses on product truth: long editing sessions, high-frequency interaction reliability, crop/export parity, existing Phase 19 parity, diagnostics, and final UI acceptance. Every v1.1 phase requires product-level evidence; unit-only, fallback, mock, artifact, CPU, DOM, native-video, first-frame, or file-exists-only evidence cannot satisfy user-visible success.

## Milestones

- ✅ **v1.0 Production Core** — Phases 1-19 plus inserted production-hardening phases (shipped 2026-06-26)
- 🚧 **v1.1 Usability & Export** — Phases 20-24 (planned)

## Phases

<details>
<summary>✅ v1.0 Production Core (25 phases, 187 plans) — SHIPPED 2026-06-26</summary>

Phases 1-19 plus inserted phases 04.1, 10.1, 15.1, 15.2, 15.3, and 17.1 shipped the Rust-owned draft model, command semantics, Jianying-style desktop workspace, production preview/export path, runtime/cache/audio/scheduler foundations, offline template adapter, portable bindings, and Phase 19 retime/effect/filter/mask/blend/transition semantics.

Full phase details are archived in `.planning/milestones/v1.0-ROADMAP.md`.

</details>

### 🚧 v1.1 Usability & Export (Planned)

**Milestone Goal:** Make the editor feel reliably usable in real editing sessions while closing crop/export/effects parity gaps for the existing Phase 19 capability set.

- [x] **Phase 20: Long Timeline Product UAT And Guard Baseline** - Prove real long mixed-media editing sessions, save/reopen/export loops, responsiveness, and no-fallback product evidence. (completed 2026-06-27)
- [ ] **Phase 21: High-Frequency Interaction And Shortcut Session Hardening** - Make common shortcuts and visible high-frequency controls reliable through Rust-owned interaction sessions.
- [ ] **Phase 22: Crop And Export Parity Closure** - Close crop/export parity through Rust compiler/runtime preflight, export state feedback, and product diagnostics.
- [ ] **Phase 23: Existing Phase 19 Parity And Diagnostics Closure** - Prove the current retime/effect/filter/transition/mask/blend support set and diagnostics without expanding library breadth.
- [ ] **Phase 24: UI Polish And Product Acceptance Sweep** - Polish only backed or explicitly gated controls and run the full v1.1 acceptance sweep.

## Phase Details

### Phase 20: Long Timeline Product UAT And Guard Baseline

**Goal**: Users can prove real long editing sessions stay responsive and canonical across preview, save/reopen, export, and continued editing.
**Depends on**: Phase 19 (shipped v1.0 baseline)
**Requirements**: UAT11-01, UAT11-02, LONG11-01, LONG11-02, GATE11-01
**Success Criteria** (what must be TRUE):

  1. User can complete a packaged product session that imports mixed media, edits a long multi-track timeline, scrubs and previews through the production compositor, saves, reopens, continues editing, exports, and exports again.
  2. User can repeat edit/save/reopen/export cycles without `.veproj/project.json` gaining derived artifacts or changing canonical semantics unexpectedly.
  3. User can select, scroll, zoom, scrub, move, trim, split, undo, redo, and preview on a long timeline while documented responsiveness budgets are met.
  4. User can keep scrubbing, editing inspector values, receiving preview frames, and committing or canceling interactions while export, probing, artifact generation, and cache work run.
  5. User never sees product success when evidence comes only from fallback, mock, artifact, CPU probe, DOM overlay, native-video proof, first-frame snapshot, or file-exists-only export proof; those states fail closed with diagnostics.

**Plans**: 4/4 plans complete
Plans:
**Wave 1**

- [x] 20-01-PLAN.md — Rust/testkit long fixture, canonical materialization, and pressure gates
- [x] 20-02-PLAN.md — Playwright long fixture, canonical comparison, preview/export evidence, and bundle helpers
- [x] 20-03-PLAN.md — Packaged long-session UAT with responsiveness, reopen/export cycles, scheduler pressure, and commit/cancel coverage

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 20-04-PLAN.md — No-fallback/source guards and `pnpm run test:phase20` aggregate wiring

### Phase 21: High-Frequency Interaction And Shortcut Session Hardening

**Goal**: Users can perform common editing operations quickly through shortcuts and session-backed direct manipulation without save, undo, or revision storms.
**Depends on**: Phase 20
**Requirements**: INT11-01, SHORT11-01
**Success Criteria** (what must be TRUE):

  1. User can use common desktop editing shortcuts for play/pause, frame step, split, delete, undo/redo, save, import/export, zoom/fit, and Escape cancel with focus-safe behavior across timeline, preview, inspector, text input, numeric input, and modal contexts.
  2. User can drag, scrub, trim, and adjust visible high-frequency controls with live production preview while updates leave the canonical draft, saved project, revision, and undo stack unchanged.
  3. User can commit a high-frequency interaction and receive exactly one canonical mutation, one revision advance, one undo entry, and one save/autosave decision.
  4. User can cancel an interaction or hit a stale target and see the canonical draft remain unchanged.

**Plans**: TBD
**UI hint**: yes

### Phase 22: Crop And Export Parity Closure

**Goal**: Users can rely on crop-bearing previews and exports, with export state feedback backed by the Rust compiler/runtime path rather than UI-only clamping.
**Depends on**: Phase 21
**Requirements**: CROP11-01, CROP11-02, EXP11-01
**Success Criteria** (what must be TRUE):

  1. User can export supported crop behavior for video, image, imported template, and small-source fixtures and see exported frames match production preview within documented tolerance.
  2. User sees invalid or impossible crop blocked before FFmpeg runtime execution with product-language diagnostics, not a late invalid-size FFmpeg failure.
  3. User sees export progress, cancel, success, blocked, degraded, unsupported, and failed states in the export modal with actionable product copy.
  4. User can save, reopen, and export a crop-bearing project without semantic drift or derived artifact pollution of `.veproj/project.json`.
  5. User cannot use direct crop handles as active controls until undo, preview, export, diagnostics, and session behavior are backed; unsupported states are hidden or gated.

**Plans**: TBD
**UI hint**: yes

### Phase 23: Existing Phase 19 Parity And Diagnostics Closure

**Goal**: Users can trust the existing Phase 19 retime, effect, filter, transition, mask, blend, crop, transform, text, and audio support set before any broader effect-library expansion.
**Depends on**: Phase 22
**Requirements**: FX11-01, FX11-02, DIAG11-01, ADAPT11-01
**Success Criteria** (what must be TRUE):

  1. User can use only visible Phase 19 controls that have Rust-backed support facts for preview, export, persistence, undo/redo, and product E2E evidence.
  2. User sees unsupported or degraded retime, effect, filter, transition, mask, blend, crop, transform, text, or audio paths hidden, gated, or reported with product success false.
  3. User can open diagnostics views and see typed codes, product-safe copy, affected draft targets where possible, and opt-in developer details.
  4. User can import adapter-derived projects and see provider IDs or raw provider payloads confined to adapter reports or provenance, never canonical `.veproj`, render graph, or export semantics.
  5. User can review the preview/export parity matrix for the existing Phase 19 support set without the product implying broad new proprietary or first-party effect-library breadth.

**Plans**: TBD
**UI hint**: yes

### Phase 24: UI Polish And Product Acceptance Sweep

**Goal**: Users see a polished, backed desktop editor surface after semantic, export, diagnostics, and session behavior is stable.
**Depends on**: Phase 23
**Requirements**: UI11-01, UI11-02
**Success Criteria** (what must be TRUE):

  1. User can click through every changed visible control and either perform backed behavior or see an explicit gated state for unsupported behavior.
  2. User can work at 1120x720, 1280x800, and in a crowded long-timeline state without overlapping, clipped, debug, or raw backend copy.
  3. User can read shortcut, capability, export, crop, and effect diagnostics in product language with accessible labels or tooltips where needed.
  4. User can run the full v1.1 acceptance sweep in dev and packaged Electron workflows with production preview/export evidence still required.

**Plans**: TBD
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in order: 20 → 21 → 22 → 23 → 24.

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 20. Long Timeline Product UAT And Guard Baseline | v1.1 | 4/4 | Complete   | 2026-06-27 |
| 21. High-Frequency Interaction And Shortcut Session Hardening | v1.1 | 0/TBD | Not started | - |
| 22. Crop And Export Parity Closure | v1.1 | 0/TBD | Not started | - |
| 23. Existing Phase 19 Parity And Diagnostics Closure | v1.1 | 0/TBD | Not started | - |
| 24. UI Polish And Product Acceptance Sweep | v1.1 | 0/TBD | Not started | - |
