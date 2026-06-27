# Project Research Summary: v1.1 Usability & Export

**Project:** Video Editor
**Domain:** Desktop-first Jianying-style video editor with Rust-owned editing, preview, and export core
**Researched:** 2026-06-27
**Confidence:** HIGH for roadmap direction, MEDIUM for exact residual implementation gaps

## Executive Summary

Video Editor v1.1 should be a closure and product-truth milestone, not a feature expansion milestone. v1.0 already established the production core: Rust-owned `.veproj/project.json` draft semantics, project sessions, interaction sessions, realtime GPU preview, scheduler isolation, render graph to FFmpeg export, offline adapter boundaries, and typed Phase 19 retime/effect/filter/mask/blend/transition semantics. v1.1 should prove those capabilities compose under real editing pressure: longer mixed timelines, repeated edit/save/reopen/export loops, shortcuts and direct manipulation, crop/export parity, existing Phase 19 parity, and user-readable diagnostics.

The recommended approach is to keep the current stack and architecture, then harden it with product-level gates. Electron, React/TypeScript, Node-API, Rust workspace crates, FFmpeg/ffprobe runtime, Playwright/Electron E2E, Rust tests, golden fixtures, and source guards are still the right foundation. The roadmap should start by creating failing end-to-end product UAT for long sessions, then close high-frequency interaction/session behavior, crop/export, existing Phase 19 parity, diagnostics, and finally UI rough edges. UI polish must be allowed only where behavior is backed by Rust-owned commands, interaction sessions, preview/export evidence, or explicit unsupported/degraded diagnostics.

The key risk is false confidence: tests or UI can pass while the actual production chain falls back to DOM evidence, preview artifacts, CPU probes, mock/native-video paths, file-exists-only export checks, per-sample canonical commands, stale preview generations, invalid FFmpeg crop filters, unsupported effect success, or provider IDs leaking into first-party semantics. Mitigate this by making each phase fail the known bad states with packaged product E2E, native preview evidence, exported-media validation, `.veproj` round-trip checks, source guards, telemetry budgets, and product-safe diagnostic assertions.

## Key Findings

### Recommended Stack

Do not replace the stack for v1.1. The current Electron desktop shell plus React/TypeScript UI, thin Node-API bridge, Rust-owned editor/runtime crates, render graph to FFmpeg export path, realtime preview runtime, and Playwright/Rust test strategy are appropriate. v1.1 needs runtime hardening and acceptance depth, not a new desktop shell, rendering framework, binding layer, or project format.

**Core technologies:**
- **Electron 42.4.1 desktop shell:** mature desktop filesystem/menu/package surface for the first product; keep explicit preload/main APIs.
- **React 19.2.7 + TypeScript 6.0.3 + Vite 8.0.16:** suitable for dense editor UI and Playwright coverage; UI remains command/intent only.
- **Node-API via thin binding crate:** transport adapter only; Rust owns project, draft, preview, export, cache, and diagnostics.
- **Rust workspace, edition 2024, rust-version 1.95.0:** durable semantics across desktop, future mobile, server, and adapter paths.
- **`.veproj/project.json`:** canonical semantic source of truth; render graphs, FFmpeg scripts, thumbnails, waveforms, proxies, preview caches, and exports remain derived artifacts.
- **FFmpeg/ffprobe desktop runtime:** compiler/runtime path remains debuggable and reproducible; v1.1 should add preflight and diagnostic depth rather than UI-side command generation.
- **Rust tests, golden/testkit fixtures, Playwright Electron product E2E, and source guards:** required evidence stack for long sessions, preview/export parity, no fallback success, and product UI behavior.

**Version and toolchain watch items:**
- Normalize Node/pnpm drift before packaged UAT failures are interpreted as product regressions. Current research notes Node engine 24.12.0 with a local Node 24.15.0 warning.
- Keep the Rust toolchain, generated contracts, Electron packaging, Playwright Electron harness, and FFmpeg distribution/licensing posture explicit in v1.1 requirements.

### Expected Features

v1.1 must prove the editor is usable as an actual desktop editor over sustained work, not merely that isolated features work. The feature research consistently prioritizes product workflows: create/open project, import mixed media, build and revise a longer multi-track timeline, use shortcuts and direct manipulation, preview through the production compositor, save/reopen repeatedly, export through the production path, and understand failure or degraded states.

**Must have, table stakes:**
- **Real editing UAT suite:** packaged Electron workflows for import/edit/preview/save/reopen/export and continued editing.
- **Long timeline usability/performance gates:** deterministic mixed-media fixture, timeline responsiveness budgets, scheduler telemetry, dirty range checks, and no whole-draft invalidation without explicit reason.
- **Repeated edit/save/reopen/export loops:** prove `.veproj/project.json` remains canonical and reopened export uses Rust session semantics, not in-memory UI artifacts.
- **Shortcut coverage:** common editing shortcuts for play/pause, frame step, split, delete, undo/redo, save, import/export, zoom/fit, and Escape cancel, with focus-safe behavior.
- **High-frequency interaction closure:** drag/scrub/trim/crop/retime/effect/keyframe/inspector controls use Rust `ProjectInteractionSession` semantics with provisional updates, cancel, stale rejection, and one commit.
- **Crop/export correctness:** validate, clamp, or reject crop against decoded source dimensions before FFmpeg runtime execution; preview and export use the same policy.
- **Existing Phase 19 parity:** close preview/export reliability for the existing retime, dissolve transition, first-party effect/filter, mask, blend, crop, transform, text, and audio support set.
- **Product-safe diagnostics:** distinguish unsupported, degraded, missing media, invalid crop, export preflight failure, FFmpeg runtime failure, stale generation, scheduler pressure, and fallback-disallowed states.

**Should have, differentiators to preserve:**
- **One consistent preview/export model:** supported edits preview and export through the same Rust-owned semantics.
- **Rust-owned direct manipulation:** live interaction without undo/save/revision storms.
- **Navigable diagnostics:** report rows can focus, seek, or select canonical targets when possible; report-only rows stay clearly non-editable.
- **Jianying-style dense workflow:** polish the existing five-zone desktop workspace, shortcut discoverability, hit targets, product units, disabled states, and long-timeline density without broad redesign.

**Defer to v2+ or later milestones:**
- Broad new effect/filter/transition library.
- Full proprietary Jianying/CapCut/Kaipai parity or live provider integrations.
- Mobile app UI, cloud rendering product UX, marketplace preset libraries, collaboration, or server fleet operations.
- AI oral-video workflows, ASR, auto-highlight generation, template intelligence, or digital-human workflows.
- Direct Kdenlive/MLT runtime integration or copied GPL assets/presets/XML.

### Architecture Approach

The v1.1 architecture direction is partially correct because the ownership boundary is already right, but the sequencing must keep it right under pressure. UI emits product intents, semantic handles, and interaction-session updates. Electron main validates and forwards explicit APIs. Node-API adapts transport. Rust `editor_runtime` and project sessions own draft, command, preview, export, cache, adapter, effect, retime, transition, crop, and diagnostic semantics. Preview and export consume the same accepted Rust semantics, with divergences represented as typed diagnostics rather than hidden success.

**Production target chain:**

```text
User gesture / shortcut / export action
  -> renderer sends a narrow intent, semantic handle, or interaction-session update
  -> Electron main validates sender and forwards explicit native API
  -> Node binding adapts JSON only
  -> editor_runtime/project session checks session id and expected revision
  -> Rust accepts, rejects, previews, cancels, or commits
  -> CommandDelta/provisional delta drives invalidation and view model update
  -> realtime preview renders through Rust GPU compositor for supported product paths
  -> export builds render graph and FFmpeg job in Rust only
  -> typed diagnostics report supported, degraded, unsupported, blocked, or failed state
  -> product UI shows bounded user copy; developer mode may reveal raw details
```

**Major components:**
1. **Desktop renderer:** dense product UI, geometry measurement, visible state, accessibility labels, shortcuts, and product copy; no canonical semantics or FFmpeg/render graph construction.
2. **Preload/Electron main:** explicit API and sender validation; no generic command envelope or semantic interpretation.
3. **`bindings_node`:** thin Node-API transport to Rust; owns marshaling and opaque handles, not editor logic.
4. **`editor_runtime` and project sessions:** authority for open project, expected revision, command routing, save/reopen, export start, diagnostics, and session lifetime.
5. **`draft_model` / `draft_commands` / `ProjectInteractionSession`:** canonical draft/material/track/segment schema, integer/rational time, undo/redo, snapping, provisional interaction updates, cancel, stale rejection, and one commit.
6. **`engine_core`, `render_graph`, `realtime_preview_runtime`:** normalized draft, frame state, typed render intents, GPU compositor preview, audio sync, scheduler/backpressure, and stale generation rejection.
7. **`ffmpeg_compiler` / `media_runtime_desktop`:** Rust-owned export compilation, crop/effect/retime/transition validation, FFmpeg job execution, progress, cancel, and classified runtime errors.
8. **`project_store`, artifact/cache stores, and testkit:** canonical `.veproj` persistence, derived artifact invalidation, fixtures, golden checks, and product evidence helpers.
9. **External adapters:** provider-neutral import/export adapters and compatibility reports only; proprietary IDs remain adapter/report/provenance facts, not first-party render semantics.

### Critical Pitfalls

1. **False product success from fallback or permissive evidence:** prevent with no-product-fallback gates, native preview evidence, exported frame/audio validation, and source guards that reject DOM/artifact/CPU/mock/native-video success.
2. **High-frequency interaction commit storms:** prevent by requiring `ProjectInteractionSession` for visible drag/scrub/slider/keyframe/crop/retime/effect operations, with zero save/revision/undo during updates and one canonical commit.
3. **Long timeline full recompute or scheduler starvation:** prevent with deterministic long-timeline fixtures, dirty-range and cache-reuse assertions, queue latency budgets, stale generation rejection, and export/artifact isolation under product stress.
4. **Preview/export drift, especially crop and Phase 19 capabilities:** prevent with Rust compiler preflight, shared normalized-to-pixel policies, capability matrices, exported-media checks, and failure-before-FFmpeg for impossible crop.
5. **Unsupported/degraded effects masquerading as support:** prevent with Rust capability-backed UI states, product success booleans, default product diagnostics, and strict provider-ID isolation.
6. **UI polish masking architecture gaps:** prevent by requiring source guards and product click-through evidence for every behavior-changing UI control touched by polish phases.
7. **Save/reopen/revision and handle lifetime drift:** prevent with repeated edit/save/reopen/export loops, expected-revision checks, deterministic session close/cancel paths, leak diagnostics, and stale completion rejection.

## Implications for Roadmap

Based on the combined research, v1.1 should start at Phase 20 and use product evidence as the organizing principle. The phase order below intentionally puts failing UAT and guard baselines before polish or parity closure so later phases cannot pass locally while the real editing workflow remains broken.

### Phase 20: Long Timeline Product UAT And Guard Baseline

**Rationale:** v1.1 needs a product truth harness first. Without it, shortcut, crop, effects, diagnostics, and UI work can pass isolated tests while failing a real editing session.

**Delivers:** packaged Electron UAT for a long mixed-media session; deterministic long-timeline fixture/generator; save/reopen/export loop evidence; scheduler/cache/session telemetry; refreshed no-fallback and ownership source guards.

**Addresses:** real editing UAT, long timeline usability, repeated edit/save/reopen/export, no product fallback, canonical `.veproj` round trip.

**Avoids:** false product success, tiny-fixture confidence, full recompute hiding, stale preview presentation, file-exists-only export validation, and inherited v1.0 traceability ambiguity.

**Exit gates should include:**
- Real packaged app flow imports media, builds or opens a long mixed timeline, edits, scrubs, previews, saves, reopens, continues editing, exports, reopens, and exports again.
- Preview success includes `renderGraphGpuComposited` or equivalent native preview evidence and visible preview-region change.
- Export success includes ffprobe metadata plus representative frame/audio facts, not only file existence.
- Source guards fail renderer-owned draft mutation, direct FFmpeg/render graph construction, generic native commands, fallback preview/export success, and raw default diagnostics.

### Phase 21: High-Frequency Interaction And Shortcut Session Hardening

**Rationale:** v1.0 proved the interaction-session model; v1.1 must apply it consistently to the shortcut-heavy and pointer-heavy surfaces that make the editor feel usable.

**Delivers:** focus-safe shortcut map; session-backed timeline move/trim/split/delete, playhead scrub, inspector sliders, preview transform, keyframe, retime/effect/mask/transition controls where visible; high-sample drag tests; cancel/stale/revision/undo/save assertions.

**Addresses:** shortcut coverage, direct manipulation polish, live preview/session handling, undo/revision correctness, UI hit-target roughness for common operations.

**Avoids:** per-pointer canonical command loops, UI-local semantic ghosts, debounce-as-ownership, undo storms, autosave storms, stale target commits, and shortcuts that infer timeline semantics in TypeScript.

**Exit gates should include:**
- During updates: zero save, zero revision increment, zero undo push, bounded queue latency.
- On commit: exactly one canonical mutation, one revision increment, one undo item, and one save/autosave decision.
- Cancel and stale samples leave canonical draft unchanged.
- Product preview evidence changes through the production compositor for visible visual interactions.

### Phase 22: Crop And Export Parity Closure

**Rationale:** Crop/export is the concrete v1.0 deferred limitation and a high-value parity risk. It must be fixed in the Rust compiler/runtime path, not masked in UI values or fixtures.

**Delivers:** crop validation or clamping against decoded source dimensions before FFmpeg runtime; shared preview/export crop policy; typed crop diagnostics; re-enabled or dedicated crop-bearing fixtures; product E2E for video/image/template/small-source crop cases.

**Addresses:** crop/export closure, preview/export parity, failure diagnostics, save/reopen/export after crop, Phase 19 crop-bearing fixture debt.

**Avoids:** FFmpeg discovering invalid crop first, React-only crop bounds, silently ignored crop, fixture removal as a "fix", and preview/export rounding divergence.

**Exit gates should include:**
- Invalid crop cannot reach FFmpeg as `Invalid too big or non positive size`.
- Supported crop exports frames that match preview within documented tolerance.
- Unsupported or impossible crop produces typed product diagnostics and product success false.
- Direct crop handles remain hidden or gated until undo, preview, export, diagnostics, and session behavior are all proven.

### Phase 23: Existing Phase 19 Parity And Diagnostics Closure

**Rationale:** v1.1 should make the current Phase 19 support set trustworthy before expanding library breadth or proprietary compatibility promises.

**Delivers:** capability matrix for current first-party retime, dissolve transition, filters/effects, masks, blend states, crop, transform, text, audio, and template mappings; preview/export parity fixtures; product diagnostic taxonomy; developer-details path; navigable report rows.

**Addresses:** Phase 19 parity, unsupported/degraded/failure diagnostics, product-safe copy, export failure evidence, provider-neutral adapter reports.

**Avoids:** fake support, unsupported blend export shown as success, raw FFmpeg/backend leakage in default UI, provider-native IDs as first-party semantics, and broad new effect-library scope.

**Exit gates should include:**
- Every visible "supported" Phase 19 control has Rust command semantics, realtime GPU preview evidence, export compiler evidence, save/reopen persistence, undo/redo, and product E2E.
- Every unsupported/degraded path is hidden, gated, or reports typed diagnostics with product success false.
- FFmpeg effect/retime/transition/mask/blend strings appear only in `ffmpeg_compiler` or testkit export assertions.
- Provider IDs remain in adapter/report/provenance boundaries only.

### Phase 24: UI Polish And Product Acceptance Sweep

**Rationale:** Visual and interaction rough edges should be polished only after semantic, preview, export, diagnostics, and session backing are stable. This phase should close the milestone, not introduce new behavior.

**Delivers:** screenshot-backed refinements for long timeline density, inspector readability, shortcut discoverability, hit targets, disabled states, export diagnostics, crop/effect diagnostics, preview/native surface placement, and no-overlap states; aggregate v1.1 acceptance run.

**Addresses:** UI detail cleanup, product language consistency, accessibility labels/tooltips, viewport stability at 1120x720 and 1280x800, final acceptance artifacts.

**Avoids:** functional-looking unsupported controls, marketing-style redesign, debug dashboard creep, DOM-only preview claims, and product copy that hides real failures.

**Exit gates should include:**
- Screenshot regression at 1120x720, 1280x800, and at least one crowded long-timeline state.
- Product click-through for every visible changed control.
- Default product mode has no raw FFmpeg/backend/cache/graph/log leakage.
- Full v1.1 product UAT passes in dev and packaged Electron workflows.

### Phase Ordering Rationale

- **Phase 20 first:** creates the product-truth gate that all later work must satisfy.
- **Phase 21 before crop/effects polish:** shortcut and direct-manipulation changes can destabilize save/undo/revision/preview semantics, so harden sessions before broad parity closure.
- **Phase 22 before Phase 19 sweep:** crop is a known deferred export failure and should be closed as a focused preview/export compiler problem.
- **Phase 23 before UI sweep:** support states and diagnostics must be correct before UI presents them as polished behavior.
- **Phase 24 last:** visual cleanup and milestone acceptance should validate backed behavior, not hide missing core work.

### Research Flags

Phases likely needing deeper `$gsd-plan-phase --research-phase <N>` planning research:
- **Phase 20:** exact long-timeline performance budgets, fixture size, scheduler telemetry names, trace artifact policy, and packaged UAT runtime constraints.
- **Phase 22:** `ffmpeg_compiler` crop source-dimension resolution, crop rounding, even-dimension requirements, fit/fill/stretch interaction, keyframed crop handling, and GPU preview crop math.
- **Phase 23:** explicit Phase 19 support matrix for supported, degraded, unsupported, hidden, and developer-only states, including blend export decisions and diagnostics taxonomy.

Phases with mostly standard patterns where additional research can be skipped unless implementation reveals gaps:
- **Phase 21:** interaction-session ownership model is already established by Phase 17.1; planning should mainly enumerate surfaces and gates.
- **Phase 24:** UI polish should use existing product UI, screenshot, accessibility, source-guard, and Playwright patterns; no new architecture research should be needed.

## Requirements Implications

The downstream requirements file should define v1.1 around executable product gates. Recommended seed requirements:

| ID | Requirement | Acceptance evidence |
|----|-------------|---------------------|
| V11-UAT-01 | Normal product E2E covers long mixed timelines, repeated edits, save/reopen, scrub, preview, and export with real fixture media. | Packaged Electron workflow, native preview evidence, `.veproj` round trip, exported media metadata and frame/audio checks. |
| V11-NOFALLBACK-01 | Product success cannot be satisfied by fallback, mock, artifact, CPU probe, DOM overlay, first-frame snapshot, native single-video proof, or file-exists-only export proof. | Source guards and negative product tests. |
| V11-LONG-01 | Long-timeline tests verify graph diff cost, dirty range accuracy, queue latency, stale rejection, cache reuse, and export consistency. | Rust/testkit fixtures plus product stress telemetry. |
| V11-INTERACT-01 | High-frequency controls use Rust interaction sessions with provisional updates, stale rejection, cancel, and one commit. | 300-1000 sample interaction tests; zero save/revision/undo during update; one undo item after commit. |
| V11-SHORTCUT-01 | Common desktop editing shortcuts are focus-safe, discoverable, and routed through Rust-owned intents or sessions. | Playwright keyboard matrix across preview, timeline, inspector, text input, numeric input, and modals. |
| V11-SAVE-01 | Repeated edit/save/reopen/export loops preserve canonical semantics and reject stale expected revisions. | Semantic project JSON comparison, revision checks, reopened export evidence. |
| V11-CROP-01 | Crop export validates/clamps/rejects against decoded source dimensions before FFmpeg runtime execution. | Compiler tests, crop product E2E, typed diagnostics, re-enabled crop fixture coverage. |
| V11-PARITY-01 | Preview/export parity covers crop, transform, retime, transitions, filters, masks, blends, text, and audio for existing supported Phase 19 semantics. | GPU preview evidence, exported frames/audio, capability matrix. |
| V11-FX-01 | Every exposed effect/control displays Rust capability-backed preview/export support or is product-gated. | UI capability state tests, source guards, product success booleans. |
| V11-DIAG-01 | Export/effect/crop diagnostics have typed codes, product-safe copy, affected draft targets where possible, and developer details behind explicit diagnostics. | Product diagnostic E2E and developer-mode detail checks. |
| V11-ADAPTER-01 | External adapter IDs and raw provider payloads remain adapter/report/provenance only. | Canonical project JSON, render graph, compiler, and source guard checks. |
| V11-UI-01 | UI polish phases click every visible changed control and prove backed behavior or gated state. | Screenshot/accessibility regression plus click-through product tests. |
| V11-LIFETIME-01 | Sessions, handles, listeners, preview/audio/export jobs, and native resources cancel deterministically on selection change, unmount, close, undo/redo, import, and stale generation. | Rust owner/generation tests, leak diagnostics, stale completion rejection. |

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| False product success from permissive UAT or fallback evidence | Critical | Carry v1.0 no-product-fallback policy into every phase; require native preview and exported-media evidence. |
| Interaction save/undo/revision storms | Critical | Require Rust interaction sessions, high-sample drag tests, source guards, and telemetry assertions. |
| Long timeline full recompute or scheduler starvation | Critical | Create long mixed fixtures, dirty-range budgets, cache-reuse assertions, and concurrent export/artifact stress tests. |
| Crop/export and Phase 19 preview/export drift | Critical | Fix crop in Rust compiler/runtime; build parity matrices and exported frame/audio checks. |
| Unsupported/degraded effects shown as support | High | Capability-backed UI states, product success booleans, typed diagnostics, and unsupported/degraded product E2E. |
| UI polish hides architecture gaps | High | Allow polish only with backed behavior, source guards, screenshots, and click-through tests. |
| External adapter leakage | High | Keep provider-native IDs in adapter/report/provenance only; scan core/render/session/export paths. |
| Save/reopen/revision drift and native handle leaks | High | Multi-cycle product UAT, expected revision checks, deterministic cancel/close, leak diagnostics, and stale completion rejection. |
| Diagnostics too raw or too thin | Medium | Shared Rust diagnostic taxonomy, product-safe copy, machine-readable codes, and opt-in developer details. |

## Non-Goals

- Do not replace Electron, React, TypeScript, Node-API, Rust, Playwright, FFmpeg, or the current crate structure for v1.1.
- Do not expand into a broad new effect/filter/transition library before existing Phase 19 parity is reliable.
- Do not make Jianying/CapCut/Kaipai drafts the primary project format.
- Do not treat proprietary IDs, provider-native effect names, raw formula JSON, or remote provider payloads as internal first-party render semantics.
- Do not use derived artifacts, cache files, thumbnails, waveforms, FFmpeg scripts, preview PNGs, DOM overlays, CPU probes, or native-video bridge evidence as product success.
- Do not let renderer, Electron main, or UI code construct FFmpeg commands, render graphs, cache semantics, crop policy, retime/effect semantics, or timeline semantics.
- Do not persist naked floating-point time semantics for convenience.
- Do not hide unsupported/degraded/failure states behind silent fallback or best-effort export success.
- Do not productize mobile clients, cloud rendering UX, AI oral-video workflows, live provider integrations, marketplace presets, or full proprietary draft parity in v1.1.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Current repo manifests, v1.0 artifacts, Rust workspace structure, Electron package setup, and test harnesses support keeping the stack. External framework guidance is secondary and does not change the recommendation. |
| Features | HIGH | v1.1 scope is explicit in `.planning/PROJECT.md` and consistent across STACK, FEATURES, ARCHITECTURE, and PITFALLS research. |
| Architecture | HIGH for boundaries, MEDIUM for exact residual code gaps | Research cross-checks v1.0 planning, verification, guards, runtime boundaries, and source anchors. Exact implementation gaps still need phase-level code inspection. |
| Pitfalls | MEDIUM | Risks are strongly supported by project-local artifacts and v1.0 audit history, but no fresh v1.1 long-session stress run exists yet. |

**Overall confidence:** HIGH for phase structure and non-goals; MEDIUM for exact performance budgets and crop/effects implementation details.

### Gaps to Address

- **Long-session budgets:** define concrete p95 latency, stale rejection, cache reuse, first usable workspace, scrub/seek, drag, save/reopen, and export preflight thresholds in Phase 20 planning.
- **Crop policy:** decide clamp versus reject behavior after source-level research into source dimensions, fit modes, rounding, even dimensions, keyframes, and preview crop math.
- **Phase 19 support matrix:** freeze exactly which retime/effect/filter/transition/mask/blend/crop/text/audio states are supported, degraded, unsupported, hidden, or developer-only before UI work.
- **Diagnostics taxonomy:** unify capability, render graph, compiler, runtime, scheduler, adapter, and project-store diagnostics into product-safe messages plus developer details.
- **Node/toolchain drift:** document or normalize Node/pnpm/Electron packaging assumptions before packaged UAT is used as a product gate.
- **FFmpeg licensing/distribution:** keep LGPL/GPL/nonfree build options, notices, and commercial obligations visible as export work deepens.

## Sources

### Primary, high confidence

- `.planning/PROJECT.md` - v1.1 goal, scope, validated v1.0 baseline, constraints, out-of-scope list, and key decisions.
- `.planning/research/STACK.md` - current runtime stack, crate layout, toolchain notes, v1.1 requirement candidates, and verification gates.
- `.planning/research/FEATURES.md` - user workflows, table stakes, high-frequency interactions, UI/shortcut polish, requirement candidates, and phase ordering.
- `.planning/research/ARCHITECTURE.md` - current chain, production target chain, destructive refactor boundaries, phase-boundary recommendations, and cross-phase gates.
- `.planning/research/PITFALLS.md` - critical/high/moderate pitfalls, requirement seeds, phase gates, red flags, and deferred risks.
- `.planning/milestones/v1.0-ROADMAP.md` - Phase 16 scheduler, Phase 17.1 interaction sessions, Phase 18 binding/runtime ports, Phase 19 production effects.
- `.planning/milestones/v1.0-REQUIREMENTS.md` - v1.0 preview/export, no-fallback, scheduler, interaction, binding, and effects requirements.
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md` - v1.0 closure status, deferred crop limitation, and traceability notes.
- `.planning/milestones/v1.0-phases/15.2-p0-real-gpu-realtime-compositor-closure/15.2-VERIFICATION.md` - realtime preview closure and invalidated/reclosed UAT context.
- `.planning/milestones/v1.0-phases/15.3-p0-jianying-style-production-ui-convergence/15.3-VERIFICATION.md` - UI convergence gates.
- `.planning/milestones/v1.0-phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-VERIFICATION.md` - interaction-session verification.
- `.planning/milestones/v1.0-phases/18-mobile-server-binding-architecture-and-runtime-ports/18-VERIFICATION.md` - runtime/binding/export authority verification.
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/19-VERIFICATION.md` - Phase 19 support and parity verification.
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/deferred-items.md` - known crop export limitation.
- `docs/runtime-boundaries.md`, guard scripts, `Cargo.toml`, `package.json`, `apps/desktop-electron/package.json`, and selected crate/testkit anchors cited by the research files.

### Secondary, medium confidence

- Electron IPC and context isolation documentation - confirms explicit preload/main/API boundary expectations.
- Node-API documentation - confirms native async/thread-safe and opaque handle patterns.
- Playwright Electron, screenshot, video, and trace documentation - confirms product E2E and artifact strategies.
- FFmpeg crop, filter, progress, and diagnostic documentation - confirms compiler preflight and runtime diagnostic considerations.

### Notes

- The source research notes a few missing or renamed historical files, especially around Phase 15.3 realtime preview naming and Phase 17.1 review naming. The synthesis uses the adjacent present verification, validation, review, and audit artifacts rather than treating stale paths as authoritative.
- External web research was secondary. The roadmap recommendation is grounded primarily in project-local v1.0 evidence and the current v1.1 project constraints.

---
*Research completed: 2026-06-27*
*Ready for roadmap: yes*
