# Architecture Research: v1.1 Usability & Export

**Project:** Video Editor
**Researched:** 2026-06-27
**Mode:** Architecture research
**Decision:** partially correct
**Confidence:** HIGH for roadmap boundaries, MEDIUM for exact residual code gaps

## Decision

The v1.1 direction is partially correct.

The ownership model is correct and should be preserved: UI emits commands, intents, or interaction-session updates; Rust owns project, timeline, preview/export, cache, retime/effect/transition, adapter, and diagnostic semantics. The v1.0 planning and source checks show this direction is not aspirational anymore: `editor_runtime`, `ProjectInteractionSession`, `RealtimePreviewRuntime`, the render graph, `ffmpeg_compiler`, product no-fallback gates, and Phase 19 capability contracts exist.

The v1.1 risk is sequencing. Treating long-timeline UAT, shortcut polish, crop/export closure, effect parity, diagnostics, and UI polish as peer UI tasks would be the wrong direction. v1.1 should first preserve and extend the existing Rust-owned chains under realistic editing pressure, then close known preview/export parity gaps, then polish UI surfaces only after the core path has product evidence.

Recommended v1.1 phase order:

1. Phase 20: Long Timeline Product UAT and Guard Baseline.
2. Phase 21: High-Frequency Interaction and Shortcut Session Hardening.
3. Phase 22: Crop/Export Parity Closure.
4. Phase 23: Existing Phase 19 Parity and Diagnostics Closure.
5. Phase 24: UI Polish and Product Acceptance Sweep.

## Current Chain Based On Planning Docs

This is the chain v1.0 claims and the source spot-checks support.

```text
Electron renderer
  -> product UI controls, geometry measurement, visible product state only
  -> explicit preload/main APIs
  -> bindings_node as Node-API transport adapter
  -> editor_runtime project/session/export authority
  -> project_store .veproj/project.json persistence
  -> draft_commands / ProjectInteractionSession edit semantics
  -> engine_core normalized draft, frame state, retime mapping
  -> render_graph typed render/effect/transition intent
  -> realtime_preview_runtime GPU compositor + audio graph/scheduler paths
  -> ffmpeg_compiler structured FFmpeg jobs for export
  -> media_runtime_desktop bundled FFmpeg execution
  -> product-safe status plus developer diagnostics
```

Confirmed current facts from planning artifacts:

| Area | Current chain |
|------|---------------|
| Project format | `.veproj/project.json` is canonical; thumbnails, waveforms, render graphs, FFmpeg scripts, proxies, previews, and exports are derived artifacts. |
| Desktop UI | Phase 15.3 moved the product to project entry, five-zone Jianying-style workspace, top-right export modal, product-safe diagnostics, screenshot-backed layout regression, and no default debug console UI. |
| Preview | Phase 15.2 reclosed product playback after manual UAT invalidated the first closeout. Current product success requires `renderGraphGpuComposited`, visible preview pixel motion, native surface placement, timeline sync, and native audio evidence. |
| No fallback | Product playback cannot pass through `requestPreviewFrame`, preview artifacts, mock/offscreen output, native single-video bridge evidence, DOM overlay motion, CPU hashes, or playhead-only advancement. |
| Interaction sessions | Phase 17.1 added Rust-owned `ProjectInteractionSession` lifecycle, provisional updates, stale rejection, cancel, and commit. Updates do not save, increment revision, or push undo; commit produces one canonical mutation. |
| Export | Phase 18 moved project/export authority into `editor_runtime`; Node, C, server, and future mobile surfaces are adapters. Desktop product export uses `startProjectSessionExport`, not renderer-owned draft payloads. |
| Effects/retime/transitions | Phase 19 added typed retime, transition, effect/filter, mask, blend, and capability registry semantics. First-party support is typed; external provider IDs remain diagnostics/report evidence. |
| Known gap | v1.0 milestone audit and Phase 19 deferred item both preserve a known crop export limitation. The suggested fix is a focused crop export compiler guard that validates or clamps crop dimensions against decoded source dimensions before FFmpeg execution, then re-enables crop fixture coverage. |

Important path note: the requested file `.planning/milestones/v1.0-phases/15.3-realtime-preview-and-native-media-io-hardening/15.3-VERIFICATION.md` is not present. The repo contains `15.2-p0-real-gpu-realtime-compositor-closure/15.2-VERIFICATION.md` for realtime preview closure and `15.3-p0-jianying-style-production-ui-convergence/15.3-VERIFICATION.md` for UI convergence. This research uses both.

## Production Target Chain For v1.1

v1.1 should not introduce a new architecture. It should harden the existing chain under realistic product workflows:

```text
User gesture / shortcut / export action
  -> renderer sends a narrow intent, semantic handle, or interaction-session update
  -> Electron main validates sender and forwards explicit native API
  -> Node binding adapts JSON only
  -> editor_runtime/project session checks session id + expected revision
  -> Rust accepts, rejects, previews, cancels, or commits
  -> CommandDelta / provisional delta drives invalidation and view model update
  -> realtime preview renders through Rust GPU compositor for supported product paths
  -> export builds graph and FFmpeg job in Rust only
  -> diagnostics flow as typed support/degraded/unsupported facts
  -> product UI shows bounded user copy; developer mode may reveal raw details
```

Key target properties:

- Renderer never owns canonical `Draft`, raw `Track`/`Segment`, selection semantics, undo/redo, snapping, crop math, retime math, transition adjacency, effect evaluation, cache keys, dirty ranges, render graphs, FFmpeg args, export jobs, or fallback selection.
- High-frequency input uses `ProjectInteractionSession` for every draft-mutating drag, scrub, slider, crop handle, retime handle, transition handle, and keyframe adjustment.
- Save, autosave, revision increment, and undo happen only at interaction commit or canonical command commit.
- Preview and export consume the same accepted Rust semantics. Any preview/export divergence is a typed diagnostic, not hidden product success.
- Product default mode fails closed when a production path is unavailable. Diagnostics may explain fallback reasons, but fallback cannot satisfy product acceptance.

## Gaps And Required Actions

| Gap | Why it matters | Required action |
|-----|----------------|-----------------|
| Long-timeline product pressure is not yet a v1.1 gate | v1.0 proved many focused flows, but v1.1 is about real editing sessions. Without long-timeline gates, scheduler/cache/session regressions can hide behind small fixtures. | Add mixed-media long timeline product UAT before feature polish. Exercise import, drag edit, trim/split/move, scrub, preview, effect edits, save/reopen, export, and repeated operations while artifact/export work is active. |
| Crop export limitation is known and deferred | Crop is already a supported visual concept in preview and UI, but Phase 19 deferred an export failure where crop can reach FFmpeg as an invalid crop. This is a parity and diagnostics issue. | Fix in Rust compiler/runtime path, not UI clamping. Validate or clamp crop against decoded source dimensions before runtime execution; emit typed diagnostics when unsupported; re-enable the crop fixture and add preview/export parity evidence. |
| High-frequency session coverage must expand with v1.1 polish | Phase 17.1 proved the session model, but v1.1 shortcuts, crop handles, retime/effect sliders, and long timeline interactions can accidentally reintroduce per-sample commands or UI-local provisional semantics. | Require every new high-frequency surface to name its `ProjectInteractionSession` kind, update payload, coalescing behavior, stale/cancel behavior, commit command, and preview evidence before implementation. |
| Effects parity can drift into broad library expansion | v1.1 should close existing Phase 19 parity gaps, not chase full proprietary Jianying/CapCut effect libraries. | Freeze the supported set per capability registry. For each visible effect/filter/transition/mask/blend/retime control, either prove GPU preview plus export parity or gate it as unsupported/degraded with product-safe diagnostics. |
| Blend export has known non-success behavior | Phase 19 compiler tests intentionally report non-normal blend export as unsupported and set product success false. | v1.1 must choose explicitly: implement alpha-correct export for the existing visible blend modes, or gate those controls out of product success. Do not silently export normal overlay while presenting multiply/screen as supported. |
| Diagnostics need one shared path | Raw FFmpeg/backend/runtime strings in product UI violate the v1.0 UI boundary, but vague errors make export/effects unusable. | Build typed diagnostic aggregation from capability registry, render graph, compiler, runtime, and adapter reports. Product UI receives bounded localized status; developer diagnostics receive structured internals. |
| UI polish can mask core bypasses | Visual fixes may reintroduce DOM-only preview evidence, renderer state projection, local crop/effect math, or unsupported functional-looking controls. | UI polish phases must be screenshot-backed and source-guarded, but may not add semantics unless the Rust-owned path and product E2E are already in place. |
| Planning traceability has stale rows and path mismatches | v1.0 audit notes stale requirements traceability and some missing root verification files. v1.1 planning should not inherit ambiguous evidence names. | Phase 20 should update v1.1 requirement IDs and gate names around the actual v1.0 artifacts: 15.2 preview closure, 15.3 UI convergence, 17.1 interaction sessions, 18 runtime boundaries, and 19 effects semantics. |

## Destructive Refactor Boundaries

If any of these are found during v1.1 implementation, do not patch around them. Replace the boundary and delete or gate the obsolete path.

| Boundary | Destructive refactor trigger | Replace with |
|----------|------------------------------|--------------|
| Preview frame pump | Electron main, preload, renderer, or Node binding drives playback by repeatedly requesting preview frames, waiting synchronously per frame, or accepting artifact/native-video evidence. | Rust-owned realtime scheduler, `PlaybackGeneration`, GPU compositor presentation, subscription telemetry, and product fail-closed state. |
| Renderer draft/session state | Renderer stores canonical `draft`, `commandState`, `selection`, raw tracks/segments, or computes timeline projection/capabilities. | Rust project-session `viewModel`, semantic handles, `CommandDelta`, and expected-revision checks. |
| High-frequency edits | Pointer/slider/scrub/keyframe samples execute canonical commands, save project, increment revision, or push undo per sample. | `ProjectInteractionSession` begin/update/commit/cancel with provisional deltas and one commit. |
| Export ownership | UI or Electron constructs FFmpeg args, render graphs, filter strings, output validation, crop correction, retime filters, effect filters, or export success policy. | `editor_runtime::ExportService` -> `engine_core` -> `render_graph` -> `ffmpeg_compiler` -> `media_runtime`. |
| Crop/export fix location | Crop invalidity is fixed by hiding values in the UI, clamping only React form state, or relying on FFmpeg failure text. | Rust compiler/runtime validation against actual source dimensions, typed support diagnostics, and preview/export parity tests. |
| Effects and transitions | Renderer validates transition adjacency, evaluates CSS filters as production preview, maps provider IDs into first-party effects, or emits FFmpeg filter snippets. | Rust draft commands, capability registry, render graph intent, realtime preview passes, compiler-owned export filters, adapter reports for proprietary IDs. |
| Diagnostics | Product default UI displays raw backend names, FFmpeg paths, graph/cache internals, or developer logs; or product UI hides support failures as success. | Product-safe diagnostic summaries backed by structured developer diagnostics and product-success booleans. |
| Adapter/runtime portability | Node, C, server, or future mobile adapters duplicate project/export/session/handle semantics. | `editor_runtime` shared authority with adapter-owned transport only. |
| Derived artifacts | `.veproj/project.json` stores generated graph/cache/export/proxy/thumbnail/waveform facts as canonical semantics. | Derived artifacts under project-local derived/artifact stores with invalidation facts from Rust. |

## Phase-Boundary Recommendations

### Phase 20: Long Timeline Product UAT And Guard Baseline

**Purpose:** Establish v1.1 product acceptance under realistic editing pressure before polishing or expanding controls.

Build:

- Repo-owned long-timeline fixtures with mixed video, image, audio, text/subtitles, transitions, first-party filters/effects, masks/blends where currently visible, and crop cases scoped to known support.
- Product UAT flows for repeated import/edit/play/scrub/save/reopen/export cycles.
- Scheduler/cache/session telemetry assertions for queue latency, stale rejection, cancellation, dropped frames, export isolation, and artifact work isolation.
- Source guard refresh for v1.1 to keep generic command, fallback preview, renderer draft, renderer FFmpeg, raw diagnostics, and per-sample command loops out.

Do not build:

- New effect library scope.
- UI-only performance illusions.
- Crop workarounds in React.

Exit gates:

- Long timeline product E2E passes with real native preview evidence and validated export for the currently supported set.
- Export/artifact jobs do not block scrub/drag/preview responsiveness in the product stress flow.
- No product fallback guard remains wired and passes.
- Any discovered wrong ownership boundary is either destructively refactored in this phase or blocks later phases.

### Phase 21: High-Frequency Interaction And Shortcut Session Hardening

**Purpose:** Make common editing operations feel live without save/undo/revision storms.

Build:

- Shortcut command routing through Rust project-session intents, not renderer semantic mutation.
- Session-backed move, trim, crop handle, retime handle, transition duration, effect slider, keyframe, playhead, preview transform, inspector visual/text/audio workflows where exposed.
- Product telemetry that proves update coalescing, stale rejection, cancel, commit-once, and no canonical command loop.
- Keyboard and pointer UAT for timeline, preview, inspector, and playhead workflows.

Do not build:

- UI-local provisional drafts.
- Per-sample `updateSegmentVisual`, effect, retime, or transition commands.
- Shortcut handlers that infer timeline semantics in TypeScript.

Exit gates:

- During update: zero save, zero revision increment, zero undo push.
- On commit: exactly one accepted canonical mutation, one revision increment, one undo entry, and one save/autosave decision.
- Product preview evidence changes through `renderGraphGpuComposited` for visible visual interactions.
- Source guards fail repeated canonical command loops and renderer-owned timing/retime/effect/crop math.

### Phase 22: Crop/Export Parity Closure

**Purpose:** Close the known crop export limitation as a production preview/export parity issue.

Build:

- Rust compiler validation or clamping of crop dimensions against decoded source dimensions before FFmpeg runtime execution.
- Shared crop diagnostics that distinguish invalid draft crop, unsupported source dimensions, compiler-clamped crop, and runtime failure.
- Preview/export parity fixtures for small media, portrait media, overlay media, template-import crop, save/reopen/export, and direct crop interaction if the control is visible.
- Re-enable or replace the Phase 19 crop fixture that was removed to avoid unrelated crop failure.

Do not build:

- UI-only crop bounds enforcement as the primary fix.
- Silent crop fallback, normal-fit export, or ignored crop semantics.
- FFmpeg runtime failure as the expected diagnostic path for known invalid crop dimensions.

Exit gates:

- Invalid crop cannot reach FFmpeg as `Invalid too big or non positive size`.
- Supported crop produces matching preview/export evidence within documented tolerance.
- Unsupported crop reports typed diagnostics and product success false.
- Direct crop handles remain hidden or gated until undo, preview, export, and diagnostics are all proven.

### Phase 23: Existing Phase 19 Parity And Diagnostics Closure

**Purpose:** Make the already introduced Phase 19 capability set trustworthy without expanding into proprietary parity.

Build:

- Capability matrix for current first-party retime, dissolve transition, Gaussian blur, basic color adjustment, opacity adjustment, rectangle/ellipse masks, normal/multiply/screen blend status, and supported template mappings.
- Preview/export parity tests for the current supported set under long-timeline and template-like scenarios.
- Diagnostics for degraded/unsupported external provider effects, unsupported blend export, preserve-pitch/follow-speed audio retime limits, transition adjacency/duration rejection, and missing effect resources.
- Product UI mapping from typed diagnostics to bounded product copy, with raw detail behind developer diagnostics only.

Do not build:

- Full Jianying/CapCut/Kaipai proprietary effect parity.
- Provider-native IDs as first-party render semantics.
- CSS/DOM effect previews as product GPU preview evidence.

Exit gates:

- Every visible "supported" Phase 19 control has Rust command semantics, realtime GPU preview evidence, export compiler evidence, save/reopen persistence, undo/redo, and product E2E.
- Every unsupported/degraded control is hidden, gated, or reports typed diagnostics with product success false.
- FFmpeg effect/retime/transition/mask/blend strings appear only in `ffmpeg_compiler` or testkit export assertions.
- Provider IDs remain in adapter/report diagnostics only.

### Phase 24: UI Polish And Product Acceptance Sweep

**Purpose:** Polish the editor after the semantic, preview, export, diagnostics, and interaction boundaries are stable.

Build:

- Screenshot-backed refinements for long timeline density, inspector readability, shortcut discoverability, export diagnostics, crop/effect diagnostics, panel alignment, preview/native surface placement, and no-overlap states at required desktop sizes.
- Product copy cleanup for supported, degraded, unsupported, and failed export/effect states.
- Final v1.1 aggregate acceptance combining long timeline UAT, crop parity, Phase 19 parity, interaction sessions, save/reopen/export, no-fallback, source guards, package, and contract drift.

Do not build:

- New semantic controls without Rust preview/export backing.
- Marketing-style UI surfaces or diagnostic panels in default product mode.
- Mock/native-video/artifact screenshots as preview evidence.

Exit gates:

- Product UI screenshots include native preview surface evidence when playback or visual state matters.
- Default product mode has no raw FFmpeg/backend/cache/graph/log leakage.
- Unsupported controls are not functional-looking by default.
- Full v1.1 product UAT passes in dev and packaged Electron workflows.

## Verification Gates

### Cross-Phase Gates

| Gate | Must fail when |
|------|----------------|
| `test:no-product-fallback` | Product preview/export success uses mock/offscreen/artifact/CPU/DOM/native-video evidence, or a product path reports fallback as success. |
| `test:v1-1-source-guards` | Renderer/main/preload constructs FFmpeg/render graph/effect/retime/crop semantics, stores canonical draft/session state, exposes generic command IPC, or emits raw product diagnostics. |
| `test:v1-1-long-timeline` | Long timeline edit/scrub/preview/export workflows fail, export blocks interactive preview, stale work presents, or dropped-frame/latency budgets are exceeded. |
| `test:v1-1-interactions` | High-frequency updates save, increment revision, push undo, execute canonical commands per sample, or lack cancel/stale/coalescing evidence. |
| `test:v1-1-crop-parity` | Supported crop preview/export diverges, invalid crop reaches FFmpeg runtime as an unclassified failure, or direct crop handles are visible without full Rust-owned behavior. |
| `test:v1-1-effects-parity` | A visible supported Phase 19 control lacks command, preview, export, undo, persistence, or diagnostics evidence. |
| `test:v1-1-diagnostics` | Product mode exposes raw backend/FFmpeg/cache/graph/log details, or unsupported/degraded paths are shown as success. |
| `test:v1-1-product-acceptance` | Dev or packaged product workflow fails import/edit/play/save/reopen/export with visible native preview and exported media evidence. |

### Required Evidence Patterns

- Preview playback: `renderGraphGpuComposited`, visible preview-region pixel motion, presented-frame/timeline sync, no fallback active, no preview artifact loop.
- Interaction sessions: `revisionUnchanged` on update, coalesced sequence telemetry, stale rejection, cancel evidence, one commit mutation.
- Export: project-session export start with `sessionId` and `expectedRevision`, Rust-built render graph and FFmpeg job, bundled runtime execution, output validation, typed diagnostics.
- Crop: compiler-level source-dimension validation/clamping or unsupported diagnostic before FFmpeg runtime, plus extracted export frame comparison.
- Effects: capability registry support state, render graph intent, realtime preview pass where supported, compiler output or diagnostic, product success boolean.
- UI: screenshots at 1120x720 and 1280x800 plus long-timeline layout, with product-safe copy and no debug leakage.

## Research Flags For Phase Planning

- Phase 20 needs exact performance budgets before implementation starts. Use existing Phase 16 telemetry names where possible, but define user-facing pass/fail thresholds for long timeline responsiveness.
- Phase 22 needs deeper code research into `ffmpeg_compiler` crop dimensions, material/source dimension resolution, odd/even dimension requirements, fit-mode interaction, and realtime preview crop math before choosing clamp versus reject behavior.
- Phase 23 needs an explicit support matrix before any UI work. The matrix must say which current Phase 19 controls are supported, degraded, unsupported, hidden, or developer-only.
- Phase 24 should be allowed to polish only backed behavior. If a desired UI affordance exposes unsupported semantics, it belongs in Phase 21, 22, or 23 first.

## Sources

- `.planning/PROJECT.md` - v1.1 goal, non-negotiables, active requirements, and key decisions.
- `.planning/STATE.md` - accumulated architecture decisions and quick-task boundary hardening.
- `.planning/milestones/v1.0-ROADMAP.md` - Phase 15.2, 15.3, 16, 17.1, 18, and 19 scope and ordering.
- `.planning/milestones/v1.0-REQUIREMENTS.md` - preview/export, no-fallback, scheduler, binding, and production effects requirements.
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md` - milestone closure status and deferred crop/effect parity scope.
- `.planning/milestones/v1.0-phases/15.2-p0-real-gpu-realtime-compositor-closure/15.2-VERIFICATION.md` - realtime preview product evidence and invalidated/reclosed UAT history.
- `.planning/milestones/v1.0-phases/15.3-p0-jianying-style-production-ui-convergence/15.3-VERIFICATION.md` - production UI convergence gates.
- `.planning/milestones/v1.0-phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-VERIFICATION.md` - interaction-session verification.
- `.planning/milestones/v1.0-phases/18-mobile-server-binding-architecture-and-runtime-ports/18-VERIFICATION.md` - runtime/binding/export authority verification.
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/19-VERIFICATION.md` - Phase 19 parity verification and deferred scope.
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/deferred-items.md` - crop export limitation.
- `docs/runtime-boundaries.md` - source-checked runtime boundary map.
- `scripts/no-product-fallback-guards.sh`, `scripts/phase17-1-source-guards.sh`, `scripts/phase18-source-guards.sh`, `scripts/phase19-source-guards.sh` - source-checked guard boundaries.
- `crates/ffmpeg_compiler/src/filters.rs`, `crates/realtime_preview_runtime/src/gpu/compositor.rs`, `crates/draft_model/src/validation.rs` - source-checked crop ownership and validation anchors.
