# Phase 19: Production Effects, Retiming, And Transition Semantics - Context

**Gathered:** 2026-06-25T05:00:00+08:00
**Status:** Ready for research and planning
**Source:** User-approved GSD continuation after Phase 18 closeout

<domain>
## Phase Boundary

Phase 19 restores production editing semantics for retiming, speed curves,
transitions, filters/effects, masks, blur, blend modes, and template-fidelity
fixtures on top of the Phase 18 shared runtime/binding foundation.

This phase is about core editor capability first. External draft adapters,
including Kaipai compatibility, must remain adapter layers that map external
references into the self-owned draft/effect/transition model plus compatibility
reports. Phase 19 should not chase byte-for-byte proprietary parity. The target
is strong local rendering behavior, deterministic preview/export semantics, and
clear supported/degraded/unsupported reporting.
</domain>

<decisions>
## Implementation Decisions

### Production Architecture

- Rust owns retiming, effect, filter, transition, mask, blend, keyframe, preview,
  render graph, export, cache invalidation, and compatibility-report semantics.
- Electron and renderer UI emit explicit commands and display runtime state. UI
  code must not own time mapping, effect evaluation, transition windows, FFmpeg
  filter construction, fallback selection, or render semantics.
- `.veproj/project.json` remains canonical. Render graphs, preview cache,
  thumbnails, effect indexes, compatibility reports, and FFmpeg jobs are derived.
- Use integer microseconds, frame indices, or rational frame rates for persisted
  retiming and transition math. Do not persist naked floating-point timeline
  semantics.
- Destructive production refactors are allowed. If an existing boundary cannot
  support production retiming/effects/transitions, replace it rather than adding
  compatibility shims.

### Capability Model

- Add a first-party capability registry before adding many individual effects.
  The registry maps semantic effect/filter/transition intent to GPU preview and
  export/compiler implementations where supported.
- Capability reports must distinguish supported, degraded, unsupported, and
  external/proprietary references. Unsupported paths must fail or report
  degradation explicitly; they cannot satisfy product success.
- Private Jianying/Kaipai effect IDs are compatibility references only. They
  must not become internal render semantics.

### Retiming And Time Mapping

- Retiming/speed curves are typed draft semantics evaluated by `engine_core` and
  represented in render graph/audio graph outputs.
- Segment source-to-target mapping must be deterministic under split, trim,
  move, snapping, main-track magnet, transition overlap, keyframes, preview, and
  export.
- Audio follow-speed behavior must be explicit. Unsupported pitch/time-stretch
  combinations should produce typed diagnostics rather than hidden success.

### Transitions

- Transitions are first-class adjacent/overlap relationships with type,
  duration, parameters, timeline validation, undoable commands, preview state,
  render graph representation, and export/compiler behavior.
- Timeline edit commands must validate how transitions affect overlap, trim,
  snapping, and main-track magnet. Renderer-generated transition deltas are not
  accepted.

### Effects, Filters, Masks, Blends

- First-party effects should start narrow and real: e.g. opacity/blur/basic
  color/filter primitives and one or more simple transitions that can be proven
  in preview/export.
- Masks, blend modes, blur, and complex effects must use the production GPU
  preview path for realtime interaction where supported and classify unsupported
  export paths.
- Render graph fingerprints, dirty ranges, cache keys, and preview invalidation
  must include effect, transition, retiming, mask, and blend semantics.

### High-Frequency Interaction

- Direct manipulation for transform, keyframes, transition duration, effect
  sliders, mask handles, speed handles, and timeline retiming must use Rust-owned
  interaction sessions.
- Frontend may render immediate visual feedback only as a UI-local ghost/proxy
  while Rust accepts coalesced provisional updates. Product preview state,
  save/revision/undo, and committed semantics remain Rust-owned.
- Do not save, increment revision, or push undo entries on every mouse move.
  Coalesced Rust updates must still keep interaction live; commit/cancel closes
  the session deterministically.

### UI And Product Quality

- Phase 19 has UI impact. Plans must include a UI-SPEC and post-implementation
  independent UI audit when visible controls are changed.
- Controls should feel like a restrained Jianying-style editor: resource panel
  categories for effects/transitions/filters, compact inspector controls,
  familiar icons, swatches, sliders, segmented controls, timeline transition
  handles, and no explanatory marketing text.
- It is acceptable to improve surrounding UI layout, typography, spacing, and
  interaction affordances when necessary for production quality.

### Testing And Evidence

- Verification must include Rust unit/integration tests, golden fixtures,
  preview/export parity, compatibility reports, source guards, Playwright UI/E2E
  where visible workflows change, and no-product-fallback gates.
- Complex Jianying/Kaipai-like fixtures should verify local preview/export
  parity, compatibility reporting, and performance budgets without requiring
  proprietary effect parity.
- Passing tests must fail the known bad states: renderer-owned time math,
  FFmpeg strings constructed in UI, unsupported effects treated as supported,
  artifact/CPU/DOM/mock fallback counted as product success, and every mousemove
  becoming full save/revision/undo.

### the agent's Discretion

- Planner may split Phase 19 into multiple plans/waves. Prefer sequencing:
  capability registry and contracts, retiming/speed, transitions, first-party
  effects/filters, masks/blends, UI integration, template-fidelity gates, then
  aggregate validation.
- Planner may choose exact crate/module names, but should preserve existing
  ownership boundaries: `draft_model`, `draft_commands`, `engine_core`,
  `render_graph`, `ffmpeg_compiler`, `realtime_preview_runtime`,
  `preview_service`, `task_runtime`, `editor_runtime`, `bindings_node`,
  `adapter_kaipai`, and `testkit`.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Direction And Constraints

- `AGENTS.md` — Rust-owned semantics, production refactor policy, no fallback
  success, `.veproj/project.json` canonical source, Jianying terminology.
- `.planning/PROJECT.md` — Core product value, Jianying-style editor direction,
  and long-term canvas/transform/effects/transition goals.
- `.planning/ROADMAP.md` §Phase 19 — Phase goal, dependencies, requirements,
  success criteria, and sequencing hint.
- `.planning/REQUIREMENTS.md` §PRODFX-01..PRODFX-05 — Production effects,
  retiming, transition, capability registry, masks/blends, and template fixture
  requirements.
- `docs/no-product-fallback-policy.md` — Product success cannot be satisfied by
  fallback/mock/artifact/CPU/debug/DOM evidence.
- `docs/product-e2e-acceptance-policy.md` — Product-facing features need real
  workflow evidence.
- `docs/refactor-and-legacy-cleanup-policy.md` — Destructive refactor posture
  when a boundary is structurally wrong.
- `docs/runtime-boundaries.md` — Current runtime ownership map after Phase 18.

### Historical Scope Inputs

- `ROADMAP_PHASES_11_13_ARCHIVE.md` — Archived speed, filter/effect, and
  transition scope to restore under the new Phase 19 foundation.
- `.planning/phases/10-*/` — Typed keyframe schema, commands, evaluation, and UI
  controls.
- `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/` —
  Realtime preview runtime, GPU compositor, diagnostics, and preview evidence.
- `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/` —
  Media IO, frame/texture lifetime, fallback diagnostics, and device identity.
- `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/`
  — Dirty ranges, render graph snapshots, cache keys, filter/transition
  fingerprint lessons.
- `.planning/phases/15.2-p0-real-gpu-realtime-compositor-closure/` and
  `.planning/phases/15.3-*` where present — Product playback, real GPU
  compositor closure, and editor interaction UX decisions.
- `.planning/phases/17.1-interaction-session-and-template-import-main-chain-hardening/`
  — Rust-owned high-frequency interaction sessions and template report flow.
- `.planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/` —
  Shared runtime, portable binding, server runtime, C ABI, and mobile contracts.

### Current Implementation Anchors

- `crates/draft_model/src/` — Draft material/track/segment/keyframe/filter-like
  persisted schema and generated TS contracts.
- `crates/draft_commands/src/` — Undoable timeline edit commands and validation.
- `crates/engine_core/src/` — Normalization and frame-state evaluation.
- `crates/render_graph/src/` — Typed render graph and graph fingerprints.
- `crates/ffmpeg_compiler/src/` — Render graph to FFmpeg job compilation.
- `crates/realtime_preview_runtime/src/` — GPU preview runtime, compositor,
  diagnostics, scheduler, and native surface path.
- `crates/editor_runtime/src/project_session_node.rs` — Node-shaped runtime
  project-session/interaction command surface below adapters.
- `apps/desktop-electron/src/renderer/` — Desktop workspace, resource panel,
  preview monitor, inspector, and timeline UI.
- `apps/desktop-electron/tests/` — Product E2E patterns for no fallback,
  preview cadence, scheduler stress, and runtime diagnostics.
- `crates/testkit/tests/template_import_preview.rs` and
  `crates/testkit/tests/template_import_exports.rs` — Template-fidelity and
  compatibility-report evidence.
</canonical_refs>

<specifics>
## Specific Ideas

- Start with capability registry and typed semantic contracts rather than
  individual UI controls.
- First supported slice should be small but end-to-end: schema → commands →
  engine evaluation → render graph → GPU preview/export → UI control → tests.
- Retiming and transitions should include high-frequency UI manipulation only
  after core time mapping and validation are correct.
- Kaipai support in Phase 19 should use fixture/report-driven mapping onto local
  semantics, not a separate rendering engine.
</specifics>

<deferred>
## Deferred Ideas

- Full proprietary Jianying/CapCut/Kaipai effect parity is deferred.
- Large preset/effect marketplace and cloud effect resource distribution are
  deferred.
- Mobile UI and store-ready mobile apps are deferred.
- Server multi-tenant rendering, auth, billing, and remote storage are deferred.
</deferred>

---

*Phase: 19-production-effects-retiming-and-transition-semantics*
*Context gathered: 2026-06-25 via user-approved GSD continuation*
