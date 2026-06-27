# Feature/UAT Research: v1.1 Usability & Export

**Domain:** Desktop video editor usability, export closure, and product UAT  
**Researched:** 2026-06-27  
**Overall confidence:** HIGH  
**Downstream consumer:** v1.1 requirements and Phase 20+ roadmap

## Executive Summary

v1.1 should be a closure and usability milestone. v1.0 already established the production core: Rust-owned `.veproj` semantics, project sessions, high-frequency interaction sessions, realtime GPU preview, scheduler isolation, audio preview/export parity, template import reporting, and Phase 19 retime/effect/transition/mask/blend semantics. The next milestone should not broaden the product with a large effect library, live provider integrations, mobile UI, or proprietary compatibility. It should prove that the existing editor can survive real editing sessions.

The central user-visible promise for v1.1 is: an editor can create or open a project, import mixed media, build and revise a longer multi-track timeline, use shortcuts and direct manipulation without fighting the UI, save/reopen repeatedly, preview through the production compositor, export through the production path, and receive clear diagnostics when a supported path fails or an unsupported/degraded semantic is present.

Roadmap phases should therefore be organized around product evidence, not isolated implementation completeness. Each feature area below needs packaged Playwright/Electron evidence where the normal UI performs the workflow, native preview evidence proves real `renderGraphGpuComposited` output when preview success is claimed, and exported media evidence proves the result instead of merely creating a file.

## Product Workflows v1.1 Should Prove

| Workflow | What Real Editors Should Be Able To Do | Acceptance Evidence |
|----------|----------------------------------------|---------------------|
| New project to exported video | Create a project, import video/image/audio, add text or subtitles, edit segments, preview, save, reopen, export H.264/AAC, and continue editing after export. | Packaged Electron E2E; `.veproj/project.json` round-trip comparison; native preview evidence; ffprobe output for duration/fps/resolution/audio; extracted exported frames for visible text/effect/crop checks. |
| Longer mixed-media timeline | Work on a realistic multi-track sequence with many segments, mixed source durations, repeated trims/moves/splits, text/subtitle overlays, audio, transitions, retiming, and supported effects without timeline controls becoming sluggish or visually unstable. | Deterministic long-timeline fixture plus product E2E at required desktop sizes; latency/queue telemetry; no layout overlap; preview remains visible and responsive while scrolling, zooming, selecting, scrubbing, and editing. |
| Repeated edit/save/reopen/export loop | Perform multiple edit cycles in one session, close/reopen the project, verify no semantic drift, then export after reopening. | E2E repeats at least three save/reopen/edit/export cycles; project revision and undo behavior are sane; derived artifacts remain derived; stale cache/render graph data does not appear after reopen. |
| High-frequency direct manipulation | Scrub, drag the playhead, move/trim segments, drag preview transform/crop handles, adjust inspector sliders, move keyframe markers, edit retime/effect/mask parameters, cancel with Escape, and commit with one undoable mutation. | Interaction-session observations show provisional updates do not increment revision, save, or push undo; commit creates exactly one canonical mutation; cancel leaves project unchanged; stale base revisions are rejected. |
| Crop/export closure | Crop controls and crop-containing drafts render the same supported visual result in preview and export, including small source media and aspect-ratio mismatch cases. Invalid crop rectangles are rejected, clamped, or diagnosed before FFmpeg execution. | Focused crop compiler tests; re-enabled crop coverage for the deferred Kaipai fixture; product E2E exports crop cases; no `Invalid too big or non positive size` runtime failure for valid user-visible crop states. |
| Phase 19 parity closure | Supported retiming, dissolve transitions, Gaussian blur, basic color/opacity adjustment, rectangle/ellipse masks, and supported blend/mask behavior preview and export consistently for first-party semantics. Unsupported/degraded semantics are explicit. | Preview/export parity matrix over Phase 19 features; exported frame/audio checks; product UI capability chips match actual preview/export support; unsupported features cannot be applied as successful first-party semantics. |
| Diagnostics and recovery | Understand missing media, unsupported effects, degraded export paths, runtime/export failures, and adapter degradations without seeing raw backend jargon in default UI. | Product diagnostics panel/report rows identify affected segment/time/effect/material, severity, user action, and whether export is blocked; default UI hides raw FFmpeg/render graph/provider internals; developer details remain opt-in. |

## Table Stakes For v1.1

| Feature Surface | Why It Is Expected | Complexity | Notes |
|-----------------|--------------------|------------|-------|
| Product-level UAT suite | v1.0 shipped capabilities individually; v1.1 needs proof they compose in normal editing sessions. | High | Treat as the first phase or hard gate for every phase. |
| Long timeline interaction performance | Real editing sessions expose scheduler, view-model, cache, and UI density problems that short demos miss. | High | Use deterministic fixtures with many segments/tracks and measurable latency budgets. |
| Shortcut coverage for common edits | Desktop editors feel rough without keyboard-driven split/delete/undo/redo/play/step/zoom/save/export flows. | Medium | Shortcuts must respect focus contexts and text/numeric input editing. |
| Timeline and preview direct manipulation polish | Move/trim/scrub/handles are the highest-frequency surfaces. | High | Must use Rust interaction sessions for provisional updates and commit/cancel semantics. |
| Crop export correctness | A known v1.0 deferred issue can make export fail before Phase 19 evidence is evaluated. | Medium | Validate/clamp crop dimensions against decoded source dimensions before runtime execution. |
| Preview/export parity for existing supported effects | Phase 19 added semantics; v1.1 should close gaps in shipped behavior before adding more. | High | Build a parity matrix for supported, degraded, unsupported, and blocked cases. |
| Product-safe diagnostics | Unsupported/degraded/failure states must guide users without leaking debug internals. | Medium | Diagnostics should be navigable and tied to canonical draft targets when available. |
| UI rough-edge cleanup | Real use reveals friction in density, labels, hit targets, disabled states, and viewport stability. | Medium | Scope to roughness blocking real workflows; avoid ornamental redesign. |

## High-Frequency Interactions Requiring Live Preview/Session Handling

Every visible high-frequency editor control in v1.1 should either use the Rust-owned `ProjectInteractionSession` path or be hidden/gated until it can. Local React-only ghost state is acceptable only as same-frame visual affordance reconciled by Rust provisional results.

| Interaction | Required Behavior | UAT Evidence |
|-------------|-------------------|--------------|
| Playhead scrub and ruler drag | Scrubbing updates preview/audio targets through session/runtime state, rejects stale frames, and does not drive repeated artifact preview generation. | Native preview motion at scrubbed times; stale generation rejection; no product success from artifact/DOM evidence. |
| Timeline segment move | Dragging streams provisional Rust legality/snapping results; release commits one undoable move. | Observed begin/update/commit; no repeated canonical command loop; snapping/magnet comes from Rust view model. |
| Timeline edge trim | Dragging trim handles previews target/source range changes and rejects invalid trims before commit. | 16px or larger effective hit area; invalid trim state visible; one undo item after release. |
| Cross-track move and layer reorder | Moving across video/audio/text tracks respects Rust track compatibility, z-order, lock/visibility/mute state, and collision rules. | Product E2E over multi-track fixture; rejected moves leave project unchanged. |
| Keyframe marker drag | Marker movement uses segment-relative integer microseconds, replace-at semantics, and collision rejection. | Nearby/focused keyframe can be moved/deleted without exact playhead equality. |
| Preview transform handles | Drag/scale/rotate handles produce live preview and commit/cancel semantics. | Native preview evidence, handle labels/tooltips, stable selection overlay, Escape cancel. |
| Crop handles and crop numeric controls | Crop preview must match export semantics and invalid crop cannot reach FFmpeg as an impossible filter. | Preview/export crop parity; small-source fixture; failure classified before runtime. |
| Inspector sliders and steppers | Scale, rotation, opacity, volume, text style/layout, retime speed, effect strength, mask feather/opacity stream provisional updates and commit once. | No revision/save/undo during drag; units displayed as percent, degrees, seconds, or product labels. |
| Transition duration and retime handles | Visible handles must be capability-backed and update preview/export diagnostics live. | Transition adjacency validation; retime source mapping stays Rust-owned; unsupported audio pitch cases report degradation. |
| Effect/filter/mask/blend parameters | Supported first-party parameters update live; degraded/unsupported paths show clear state and cannot masquerade as success. | Capability chips and export diagnostics agree with actual render/export result. |
| Template/report navigation | Clicking report rows focuses/seek/selects canonical targets when they exist; report-only rows are clearly non-editable. | Keyboard/click navigation; no raw provider JSON or provider IDs in default UI. |

## UI And Shortcut Polish Areas

### Shortcut Scope

v1.1 should define and test a shortcut map for the common desktop editing loop. Exact key choices can be finalized during UI spec, but these operations need product-level coverage:

| Operation | Expected Behavior |
|-----------|-------------------|
| Play/pause | Works from preview and timeline contexts; does not trigger while editing text unless intentionally handled. |
| Previous/next frame | Advances by rational frame duration and updates preview/playhead consistently. |
| Split at playhead | Uses Rust session playhead and selected segment; no renderer-derived split time. |
| Delete selected segment/keyframe/effect | Destructive actions are scoped, confirm where needed, and leave undo history correct. |
| Undo/redo | Works after high-frequency commits as one user action, not as every slider sample. |
| Copy/paste or duplicate where supported | If exposed, it must be Rust-owned and tested; otherwise keep hidden. |
| Save project | Persists `.veproj/project.json` only as canonical semantics; derived artifacts remain derived. |
| Import media and export | Shortcut opens the same product flow as toolbar/menu actions, not a hidden test path. |
| Timeline zoom and fit | Zoom is stable, does not resize controls unpredictably, and keeps playhead/selection discoverable. |
| Escape cancel | Cancels active interaction sessions, inline confirmations, and modals through the same production cancel path. |

### UI Detail Cleanup

| Area | v1.1 Expectation |
|------|------------------|
| Hit targets | Timeline trim/playhead/keyframe handles and preview transform/crop handles have reliable effective targets; no pixel-perfect clicking for common edits. |
| Focus contexts | Keyboard shortcuts do not break text editing, numeric entry, color input, path fields, or modal controls. |
| Tooltips and labels | Icon controls have Chinese `aria-label` and `title`; shortcut hints appear in menus/tooltips where helpful. |
| Product units | User controls display percent, seconds, degrees, fps, resolution, bitrate, and track/segment labels rather than raw microseconds or backend ranges. |
| Disabled/unavailable states | Unsupported categories or capabilities show `暂不可用`/`暂不支持` style product copy and cannot look active. |
| Viewport stability | 1120x720 and 1280x800 remain required gates, with no overlapping text, clipped toolbar controls, or debug copy. |
| Timeline density | Long timelines need stable row heights, compact track headers, readable segment labels, and no hover-driven layout jumps. |
| Material panel | Import/search/filter/drag-to-timeline stays primary; missing/probe-failed states remain actionable without per-card debug noise. |
| Export modal | Export path, preset, progress, cancel, diagnostics, and open-location states should remain top-right modal flow. |

## Requirement Candidates And Acceptance Evidence

These are candidate v1.1 requirement IDs for downstream requirements authoring. They intentionally build on completed v1.0 requirement families instead of redefining the core architecture.

| Candidate ID | Requirement | Acceptance Evidence |
|--------------|-------------|---------------------|
| UAT11-01 | A packaged product E2E proves a realistic mixed-media editing session from project create/open through import, edit, preview, save, reopen, export, and continued edit. | Playwright/Electron normal UI workflow; native preview evidence; `.veproj` round trip; exported media metadata and frame checks. |
| UAT11-02 | Repeated edit/save/reopen/export cycles do not drift semantics, lose selection-relevant state, corrupt derived artifacts, or reuse stale preview/export data. | Multi-cycle E2E; project JSON semantic comparison; revision/cache invalidation observations. |
| LONG11-01 | A long multi-track timeline remains usable for selection, scroll, zoom, scrub, move, trim, split, undo/redo, and preview under realistic segment counts. | Deterministic long-timeline fixture; latency/telemetry budgets; no fallback success; 1120/1280 layout screenshots. |
| LONG11-02 | Export, artifact generation, probing, and cache work do not block playhead scrub, inspector edits, preview delivery, or interaction-session commit/cancel. | Scheduler telemetry under concurrent export/artifact workload; product E2E while export is active or queued. |
| INT11-01 | All visible high-frequency controls use Rust-owned provisional interaction sessions with coalesced updates, stale rejection, cancel, and single canonical commit. | Source guards; native observation of begin/update/commit/cancel; undo stack has one item per committed user action. |
| SHORT11-01 | Common desktop editing shortcuts are defined, discoverable, focus-safe, and tested against the real product UI. | Playwright keyboard matrix across preview, timeline, inspector, text input, numeric input, and modal contexts. |
| UI11-01 | UI rough-edge cleanup preserves Jianying-style five-zone density and removes workflow friction without broad redesign. | Screenshot regression at 1120x720 and 1280x800; no overlap/clipping/debug copy; accessible labels/tooltips. |
| CROP11-01 | Crop semantics are validated against source dimensions before export and have preview/export parity for supported cases. | Rust compiler tests; product crop E2E; re-enabled crop-containing template fixture; classified invalid-crop diagnostics. |
| EXP11-01 | Export reports progress, cancel, success, blocked, degraded, unsupported, and failed states in product language with affected draft targets where possible. | Export modal/report E2E; failure injection; no raw FFmpeg/backend jargon in default UI; developer details opt-in. |
| FX11-01 | Existing Phase 19 supported first-party retime/effect/filter/transition/mask/blend semantics have a preview/export parity matrix and product evidence. | GPU preview evidence; exported frames/audio; capability chips match registry; unsupported/degraded cases cannot produce success labels. |
| DIAG11-01 | Unsupported/degraded/failure diagnostics are navigable from product UI to the relevant material, segment, time range, effect, transition, or report-only row. | Report row navigation tests; keyboard accessibility; canonical target focus/seek/select or clear report-only state. |
| BOUND11-01 | v1.1 does not reintroduce renderer-owned semantics, FFmpeg command construction, generic command envelopes, fallback-as-success paths, or provider IDs in first-party render semantics. | Source guards extending v1.0 no-fallback and ownership checks; negative tests for old paths. |

### Suggested Evidence Matrix

| Evidence Type | Must Prove |
|---------------|------------|
| Packaged product E2E | Normal user paths work outside dev-only fixtures and without hidden shortcuts. |
| Native preview evidence | Claimed preview success is visible render-graph GPU compositor output, not DOM/artifact/fallback/native-player evidence. |
| Export media validation | Output exists, duration/fps/resolution/audio match expectation, and selected frames/audio facts reflect edits. |
| Project round-trip validation | `.veproj/project.json` remains canonical, portable, and free of derived artifacts or provider runtime refs. |
| Interaction observations | Provisional updates are coalesced, cancelable, stale-safe, and commit as one undoable mutation. |
| Scheduler telemetry | Heavy jobs are isolated from interactive preview and edit responsiveness. |
| Screenshot/accessibility regression | Dense editor UI remains usable at target desktop sizes with stable labels and hit targets. |
| Source guards | Old fallback, renderer-owned, generic command, and direct FFmpeg construction paths fail fast. |

## Differentiators To Preserve

| Differentiator | Value Proposition | v1.1 Treatment |
|----------------|-------------------|----------------|
| One consistent preview/export model | Users can trust that supported edits preview and export the same way. | Strengthen parity evidence; do not add unsupported UI controls. |
| Rust-owned interaction sessions | Direct manipulation feels live without corrupting undo/save/revision semantics. | Apply uniformly to every visible high-frequency control. |
| Product diagnostics instead of silent fallback | Users know whether a result is supported, degraded, unsupported, blocked, or failed. | Make diagnostics navigable and product-safe. |
| Jianying-style desktop workflow | Familiar editing model with self-owned internals. | Polish the existing five-zone UI rather than rebuilding it. |

## Anti-Features And Non-Goals

| Non-Goal | Why Avoid In v1.1 | What To Do Instead |
|----------|-------------------|--------------------|
| Broad new effect library | Would expand surface area before existing Phase 19 parity is stable. | Close preview/export parity for the current supported first-party set. |
| 1:1 Jianying/CapCut/Kaipai proprietary parity | Private effects/resources are unstable, constrained, and not first-party semantics. | Keep adapters separate and report unsupported/degraded inputs clearly. |
| Live provider integrations | Would mix external transport/product scope into a closure milestone. | Keep offline/imported adapter evidence bounded and provider-neutral. |
| Mobile app UI or cloud product UX | v1.1 is desktop usability/export closure. | Preserve portable runtime boundaries, but do not productize new clients. |
| AI oral-video workflows | Not the current product identity. | Keep general-purpose editing workflows central. |
| New primary project format | `.veproj/project.json` is canonical. | Keep external drafts as adapter inputs with compatibility reports. |
| Debug dashboard expansion | Default UI must feel like an editor, not an engineering console. | Keep diagnostics product-safe by default and developer details opt-in. |
| Renderer-side quick fixes for semantics | Violates the established ownership boundary and creates parity drift. | Replace wrong boundaries with Rust-owned commands/sessions/render/export paths. |

## Phase Ordering Implications

1. **Phase 20: v1.1 Product UAT Baseline** - Build the end-to-end mixed-media, long-session, save/reopen/export acceptance suite first. This gives later closure work a failing product gate and prevents local fixes from passing without real workflow evidence.
2. **Phase 21: Long Timeline Usability And Interaction Performance** - Use the baseline to harden timeline density, view-model scale, scheduler isolation, and interactive responsiveness.
3. **Phase 22: Shortcuts And High-Frequency Interaction Polish** - Finalize shortcut map, focus contexts, hit targets, and session handling across timeline, preview, inspector, retime/effect/mask controls.
4. **Phase 23: Crop And Export Diagnostics Closure** - Fix crop compiler/runtime validation, re-enable crop fixture coverage, and productize export failure/degradation states.
5. **Phase 24: Phase 19 Preview/Export Parity Closure** - Close parity gaps for the existing supported retime/effect/filter/transition/mask/blend set and make unsupported/degraded paths unambiguous.
6. **Phase 25: v1.1 Acceptance Audit** - Run aggregate product UAT, visual/accessibility checks, source guards, parity matrix, and known-limits update before milestone close.

## Sources

- `.planning/PROJECT.md`
- `.planning/STATE.md`
- `.planning/milestones/v1.0-ROADMAP.md`
- `.planning/milestones/v1.0-REQUIREMENTS.md`
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md`
- `.planning/milestones/v1.0-phases/04.1-professional-jianying-workspace-ui-refinement/04.1-UI-REVIEW.md`
- `.planning/milestones/v1.0-phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-UI-REVIEW.md`
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/19-UI-AUDIT.md`
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/deferred-items.md`
