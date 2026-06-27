# Stack/Runtime Research: v1.1 Usability & Export

**Project:** Video Editor
**Milestone:** v1.1 Usability & Export
**Researched:** 2026-06-27
**Research scope:** Stack/runtime capabilities needed for long-session editing, save/reopen/export parity, high-frequency interactions, crop/export closure, Phase 19 parity fixes, and diagnostics.
**Overall confidence:** HIGH for repo baseline and roadmap implications; MEDIUM for external framework guidance.

## Recommendation

Do not change the core stack for v1.1. Keep the Electron + React/TypeScript desktop shell, thin Node-API bridge, Rust-owned editor/runtime semantics, render graph to FFmpeg export path, realtime preview runtime, Playwright/Electron product tests, and Rust golden/testkit gates.

v1.1 should invest in runtime hardening and acceptance depth, not broad stack replacement. The production baseline is strong enough: v1.0 already owns draft/timeline/preview/export/effects semantics in Rust and rejects fallback/mock/DOM evidence as product success. The gap is that the current gates prove representative workflows, while v1.1 needs sustained product workflows: longer mixed timelines, repeated edit/save/reopen/export cycles, shortcut-heavy and pointer-heavy editing, crop/export parity, and user-readable diagnostics when capability limits are hit.

## Current Production Baseline From v1.0

| Area | Current Baseline | v1.1 Implication |
|------|------------------|------------------|
| Desktop shell | Electron 42.4.1, Vite 8.0.16, React 19.2.7, TypeScript 6.0.3 | Keep the shell; improve product-flow tests and shortcut/input handling through existing preload/main contracts. |
| Package/runtime manager | pnpm 10.32.1, Node engine 24.12.0, audit noted local Node 24.15.0 warning | Normalize toolchain enforcement or document accepted drift before treating packaged UAT failures as product failures. |
| Native binding | `@napi-rs/cli` 3.7.2 over `crates/bindings_node` and shared `editor_runtime` | Preserve thin adapter shape; add long-session leak/lifetime assertions and avoid broad generic IPC. |
| Rust workspace | Rust edition 2024, rust-version 1.95.0; crates for draft model, commands, engine, render graph, preview runtime, audio, FFmpeg compiler/runtime, artifact store, project store, bindings, server runtime, testkit | Keep semantics in Rust; v1.1 requirements should target crate-level hardening and gates, not new ownership boundaries. |
| Project format | `.veproj/project.json` canonical; derived artifacts remain generated | Long-session gates must verify save/reopen canonical parity and deterministic derived artifact rebuild/invalidation. |
| Realtime preview | Rust-owned render-graph GPU path with scheduler, media IO handoff, audio sync, no-fallback product evidence | Use preview telemetry as acceptance evidence for responsiveness and no fallback under pressure. |
| Export | Render graph to FFmpeg compiler/runtime, progress/cancel/log/error classification | Add compiler preflight for crop/effect invalid cases before FFmpeg runtime failure. |
| High-frequency edits | Rust-owned `ProjectInteractionSession` with base revision, monotonic sequence, provisional updates, cancel, coalesced commit, one undo/save/revision on commit | Extend usage to v1.1 shortcuts and polish; do not reintroduce per-pointer canonical command loops. |
| Scheduler | Phase 16 priority queues, cancellation, backpressure, telemetry, starvation gates | Build longer product stress gates that combine export, import/probe, playback, scrubbing, inspector edits, and save/reopen. |
| Effects/retime/transitions | Phase 19 typed capability registry, retime/effect/filter/mask/blend/transition semantics, preview/export diagnostics | Fix parity for existing supported capabilities; do not expand into a large new effect library. |
| Test harness | Rust tests, golden fixtures, testkit large timeline generator, Playwright Electron dev/packaged product workflows, source guards, no-product-fallback gates | Compose existing gates into v1.1 aggregate acceptance and add focused new gates for crop, long timeline, diagnostics, and repeated reopen/export. |

## Stack/Runtime Gaps v1.1 Must Close

### 1. Long-session product UAT depth

Current product E2E proves important workflows, but mostly as bounded phase gates. v1.1 needs a realistic edit-session matrix that keeps one project alive through many operations: import mixed media, add many clips, edit repeatedly, play/scrub, save, reopen, continue editing, export, reopen again, and export again.

Required runtime capability:

- Stable project-session lifetime over repeated operations.
- No leaked native handles after project close/reopen.
- No stale generation presentation after repeated scrubs/exports.
- Product-safe telemetry snapshots for queue latency, preview cadence, fallback count, export status, and diagnostic counts.
- Deterministic `.veproj/project.json` round trip with derived caches rebuildable.

### 2. Long timeline usability/performance budgets

`crates/testkit/src/large_timeline.rs` already generates large drafts and Phase 13 verifies incremental graph behavior. v1.1 should turn that into product acceptance: timeline UI remains responsive, localized edits do not cause whole-draft invalidation, and preview/export parity survives realistic segment counts.

Required runtime capability:

- A repository-owned "long mixed timeline" fixture or generator exposed to desktop E2E.
- Measured budgets for first usable workspace, timeline scroll/zoom, playhead seek, selection, localized trim/move, inspector update, preview refresh, and export preflight.
- Delta/dirty-range assertions that localized edits produce bounded invalidation rather than full draft/cache churn unless required.
- Playwright trace/screenshot artifacts on failure, not just unit timing.

### 3. Shortcut and high-frequency interaction closure

Phase 17.1 proves interaction sessions for preview, timeline, inspector, keyframes, playhead, and template report navigation. v1.1 shortcut work must reuse the same command/session boundary.

Required runtime capability:

- Shortcut actions resolve to typed project intents or interaction-session lifecycle calls.
- Repeated keypresses and pointer samples are coalesced where they represent one user gesture.
- Commit/cancel paths are idempotent. Phase 19 already found and fixed one duplicate finish race; v1.1 should guard the class of issue.
- Undo/redo stacks contain user-level commits, not every sample.
- Source guards reject renderer-owned draft mutation, UI-owned retime/effect math, direct FFmpeg/render graph construction, and generic "execute arbitrary native command" shortcuts.

### 4. Save/reopen/export parity for real workflows

v1.0 has save/reopen and export tests. v1.1 needs parity after longer edit sequences and after Phase 19 controls have been used.

Required runtime capability:

- Canonical project save after committed edits only.
- Reopen reconstructs material bin, timeline selection-safe state, effect/retime/crop semantics, capability diagnostics, and preview/export readiness from `project.json`.
- Export uses reopened semantic state, not in-memory-only session artifacts.
- Derived artifact manifests, preview caches, thumbnails, waveforms, and proxy/cache rows cannot become product success criteria.

### 5. Crop/export compiler preflight

The known v1.0 deferred issue is concrete: a Kaipai fixture crop can compile to an invalid FFmpeg crop against small desktop media, failing with invalid too-big/non-positive dimensions before the intended Phase 19 export evidence runs.

Required runtime capability:

- Validate or clamp normalized crop rectangles against decoded source dimensions before filtergraph generation.
- Preserve preview/export parity: GPU preview crop and FFmpeg export crop must use the same normalized-to-pixel policy.
- Emit typed compiler diagnostics that name segment/material, source dimensions, requested crop, adjusted crop, support state, and user-facing remediation.
- Add fixtures that re-enable the crop-bearing template case rather than removing crop from the test variant.

### 6. Phase 19 existing capability parity closure

Phase 19 is complete for typed semantics, first-party support boundaries, and product evidence. v1.1 should close bugs in the existing support set only.

Required runtime capability:

- Parity matrix for retime, dissolve transition, first-party effect/filter subset, mask, blend, crop, transform, text, audio, and export diagnostics across preview and export.
- Diagnostics for unsupported/degraded combinations remain first-class and user-visible where relevant.
- No provider-private IDs or external preset names can become internal render semantics.

### 7. Better unsupported/degraded/failure diagnostics

Diagnostics exist across runtime capability, preview, scheduler, FFmpeg runtime, and Phase 19 support. v1.1 needs a product-facing diagnostic taxonomy and evidence that failures are understandable without exposing raw runtime internals by default.

Required runtime capability:

- A single mapping layer from Rust diagnostic kinds to product-safe messages and optional developer details.
- Distinct states for unsupported, degraded, missing media, invalid crop, export preflight failure, runtime FFmpeg failure, preview unavailable, stale generation rejected, scheduler pressure, and fallback disallowed.
- Desktop UI tests that assert product copy in default mode and developer details only when developer diagnostics are enabled.

## Recommended Requirement Candidates

Use these stable IDs or names as v1.1 requirement seeds.

| ID | Name | Requirement Candidate | Why It Belongs In v1.1 |
|----|------|-----------------------|-------------------------|
| V11-UAT-01 | Real Editing Session Matrix | Packaged Electron UAT performs a long mixed-media editing session with repeated edit/play/save/reopen/export cycles and verifies visible preview, timeline state, saved draft, and exported media. | This is the milestone's central acceptance requirement. |
| V11-LONG-01 | Long Timeline Product Fixture | Repository-owned long mixed timeline fixture/generator is usable from Rust tests and Playwright product tests, with explicit segment/track/media mix. | Prevents product usability claims from resting on tiny fixtures. |
| V11-LONG-02 | Long Timeline Responsiveness Budgets | Timeline selection, scroll/zoom, playhead seek, localized trim/move, inspector update, preview refresh, and export preflight stay within documented budgets under long timeline load. | Turns "feels usable" into executable gates. |
| V11-SESSION-01 | Long-Session Handle Hygiene | Project/session/media/frame/texture/artifact handles are released on close/reopen; leak diagnostics fail the gate when handles survive beyond owner session. | Protects Electron/Node-API/Rust long-running desktop behavior. |
| V11-INTERACT-01 | Shortcut Command Ownership | Every shortcut and high-frequency action routes through typed project intents or interaction sessions; renderer cannot mutate draft, render graph, FFmpeg, cache, or effect semantics. | Preserves the v1.0 ownership boundary while adding polish. |
| V11-INTERACT-02 | Coalesced Interaction Commit Semantics | Pointer/slider/key-repeat gestures do not save, increment revision, or push undo during updates; commit/cancel is idempotent and creates at most one canonical mutation. | Extends Phase 17.1/19 guarantees to v1.1 polish. |
| V11-SAVE-01 | Reopen Semantic Parity | After long edits, reopened `.veproj/project.json` restores materials, tracks, segments, retime/effects/crop/mask/blend/transition state, and diagnostics without semantic drift. | Ensures canonical project format remains trustworthy. |
| V11-EXPORT-01 | Reopened Export Parity | Export after reopen uses the same normalized draft/render graph/compiler/runtime path and matches preview within documented tolerances for the supported set. | Catches in-memory-only state and derived-artifact coupling. |
| V11-CROP-01 | Crop Compiler Preflight | FFmpeg compiler validates/clamps crop rectangles against source dimensions and reports typed diagnostics before runtime execution. | Directly closes the v1.0 deferred crop export limitation. |
| V11-CROP-02 | Crop Preview/Export Parity | Crop behavior is proven in GPU preview and exported media for video/image/template fixtures, including small-source edge cases. | Prevents compiler-only fixes from diverging from preview. |
| V11-FXPARITY-01 | Existing Phase 19 Parity Matrix | Existing retime/effect/filter/transition/mask/blend support has preview/export parity fixtures and diagnostic assertions; no broad new library expansion. | Matches milestone scope and avoids scope creep. |
| V11-DIAG-01 | Product Diagnostic Taxonomy | Rust diagnostics map to product-safe default copy plus developer details; unsupported/degraded/failure states are distinct and tested. | Makes failures actionable without exposing raw internals. |
| V11-DIAG-02 | Export Failure Evidence | Export failures include preflight diagnostics, FFmpeg runtime classification, logs/developer details, and user-facing remediation where possible. | Export closure needs better failure understanding. |
| V11-GATE-01 | Acceptance Artifact Policy | Product UAT retains screenshots/traces/videos/log snippets for failures and selected visual changes; roadmap phases cannot close on unit-only evidence for user-visible behavior. | Keeps v1.1 product-quality work evidence-backed. |
| V11-BOUNDARY-01 | No Product Fallback Regression | Existing no-fallback guards extend to v1.1 long-session, crop, diagnostics, shortcut, and Phase 19 parity flows. | Prevents false product success during polish work. |

## Verification Gates Suitable For Roadmap Phases

### Gate A: Stack/Boundary Regression

Purpose: prove v1.1 work preserves v1.0 architecture.

Suggested command shape:

```bash
pnpm run test:no-product-fallback
pnpm run test:phase17-1:guards
pnpm run test:phase19-source-guards
pnpm run test:contracts
cargo check --workspace --locked
```

Additional v1.1 guard expectations:

- No new renderer-owned draft mutation helpers.
- No renderer/main FFmpeg filter strings or render graph construction.
- No shortcut path bypassing project-session intent or interaction-session APIs.
- No product success from artifact PNG, DOM overlay, CPU probe, native-video bridge, mock runtime, or fallback preview.

### Gate B: Long Timeline Rust Runtime

Purpose: prove runtime data structures and invalidation remain bounded before product UI relies on them.

Suggested command shape:

```bash
cargo test -p testkit large_timeline -- --nocapture
cargo test -p testkit large_timeline_incremental -- --nocapture
cargo test -p draft_commands long_timeline -- --nocapture
cargo test -p preview_service dirty_propagation -- --nocapture
cargo test -p task_runtime starvation -- --nocapture
```

Add v1.1 tests for:

- Localized edit delta size and dirty ranges on a long mixed timeline.
- Save/reopen semantic equality after many edits.
- Preview/export graph fingerprints before and after localized edits.
- Handle/session cleanup after repeated open/close.

### Gate C: Long Product Session UAT

Purpose: prove a real user workflow, not isolated behavior.

Suggested command shape:

```bash
pnpm --filter @video-editor/desktop package:dir
pnpm --filter @video-editor/desktop exec playwright test tests/v1-1-long-session.spec.ts --reporter=line --workers=1
```

Required assertions:

- Uses real packaged app, real project bundle, real media fixtures, and production runtime capabilities.
- Imports video/image/audio, creates many timeline segments, uses shortcuts and pointer interactions, edits transform/keyframe/crop/retime/effect/filter/mask/blend/transition controls from the existing supported set.
- Plays and scrubs with `renderGraphGpuComposited` evidence and no preview frame artifact fallback.
- Saves, closes, reopens, verifies timeline/material/inspector state, continues editing, exports, reopens, exports again.
- Checks scheduler telemetry: bounded queue latency, zero fallback success, no rejected normal product work, no stale generation presentation.
- Retains Playwright screenshot/trace/video artifacts on failure.

### Gate D: Crop/Export Closure

Purpose: close the known deferred crop limitation.

Suggested command shape:

```bash
cargo test -p ffmpeg_compiler crop -- --nocapture
cargo test -p testkit production_effects_exports -- --nocapture
cargo test -p testkit preview_export_parity -- --nocapture
pnpm --filter @video-editor/desktop exec playwright test tests/v1-1-crop-export.spec.ts --reporter=line --workers=1
```

Required assertions:

- Crop preflight rejects or clamps impossible rectangles before FFmpeg execution.
- Diagnostics include segment/material/source dimensions and product-safe remediation.
- GPU preview and export use the same crop policy for video, image, and imported template fixtures.
- The Phase 19 crop-bearing fixture variant is re-enabled.
- Invalid crop cannot be hidden by removing crop from test data.

### Gate E: Existing Phase 19 Parity Closure

Purpose: prove existing capabilities are reliable without expanding scope.

Suggested command shape:

```bash
pnpm run test:phase19
cargo test -p testkit production_effects -- --nocapture
pnpm --filter @video-editor/desktop exec playwright test tests/production-effects.spec.ts tests/v1-1-effects-parity.spec.ts --reporter=line --workers=1
```

Required assertions:

- Retime/speed, dissolve transition, supported filters/effects, mask, blend, transform, crop, text, and audio have preview/export evidence or typed unsupported/degraded diagnostics.
- Unsupported provider-private effects stay disabled/diagnostic.
- Product controls only appear enabled when Rust capabilities support them.

### Gate F: Diagnostics Product Acceptance

Purpose: make failure modes understandable and non-fallback.

Suggested command shape:

```bash
pnpm --filter @video-editor/desktop test:runtime-diagnostics
pnpm --filter @video-editor/desktop exec playwright test tests/v1-1-diagnostics.spec.ts --reporter=line --workers=1
cargo test -p media_runtime output_validation -- --nocapture
cargo test -p ffmpeg_compiler diagnostics -- --nocapture
```

Required assertions:

- Default UI shows product-safe copy, not raw graph/cache/FFmpeg internals.
- Developer diagnostics mode reveals technical details and logs.
- Missing media, unsupported effect, degraded preview, invalid crop, export preflight failure, FFmpeg runtime failure, and stale preview generation are distinct.
- Export modal/report surfaces actionable remediation and preserves logs for debugging.

## Explicit Non-Goals

- Do not replace Electron, React, Node-API, Rust, Playwright, FFmpeg, or the current crate structure for v1.1.
- Do not add a broad new effect/filter/transition library. v1.1 closes reliability for the existing Phase 19 capability set.
- Do not make Jianying/CapCut/Kaipai drafts the primary project format.
- Do not treat derived artifacts, cache files, thumbnails, waveforms, FFmpeg scripts, preview frame PNGs, DOM overlays, CPU probes, or native-video bridge evidence as product success.
- Do not let UI, renderer, or Electron main construct FFmpeg commands, render graphs, cache semantics, retime/effect semantics, or crop policy.
- Do not use floating-point persisted time semantics for convenience in UI polish.
- Do not hide unsupported/degraded/failure states behind silent fallback or "best effort" export success.
- Do not require always-on Playwright tracing/video for every test; enable artifacts for failure and focused UAT to keep the suite usable.

## Sources

### Repo Sources

- `.planning/PROJECT.md` - v1.0 shipped baseline, v1.1 milestone scope, hard constraints. Confidence: HIGH.
- `.planning/STATE.md` - current milestone state and v1.0 phase completion context. Confidence: HIGH.
- `.planning/milestones/v1.0-ROADMAP.md` - Phase 16 scheduler, Phase 17.1 interaction sessions, Phase 18 binding ports, Phase 19 production effects. Confidence: HIGH.
- `.planning/milestones/v1.0-REQUIREMENTS.md` - validated requirements and traceability context. Confidence: HIGH, with known traceability debt noted by audit.
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md` - closeout status, tech debt, deferred crop limitation. Confidence: HIGH.
- `.planning/milestones/v1.0-phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-VERIFICATION.md` - interaction-session verification. Confidence: HIGH.
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/19-VERIFICATION.md` - Phase 19 verification. Confidence: HIGH.
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/deferred-items.md` - crop export limitation. Confidence: HIGH.
- `package.json`, `apps/desktop-electron/package.json`, `Cargo.toml` - current toolchain and workspace versions. Confidence: HIGH.
- `crates/testkit/src/large_timeline.rs`, `apps/desktop-electron/tests/product-scheduler-stress.spec.ts`, `apps/desktop-electron/tests/real-workflow.spec.ts`, `apps/desktop-electron/tests/production-effects.spec.ts` - existing gate assets. Confidence: HIGH.

### External Sources

- Electron IPC and context isolation docs: https://www.electronjs.org/docs/latest/tutorial/ipc and https://www.electronjs.org/docs/latest/tutorial/context-isolation. Confidence: MEDIUM through GSD `brave` verified tier.
- Node-API docs for async/thread-safe native patterns and external handles: https://nodejs.org/api/n-api.html. Confidence: MEDIUM through GSD `brave` verified tier.
- Playwright Electron, screenshots, video, and trace docs: https://playwright.dev/docs/api/class-electronapplication, https://playwright.dev/docs/screenshots, https://playwright.dev/docs/videos, https://playwright.dev/docs/trace-viewer. Confidence: MEDIUM through GSD `brave` verified tier.
- FFmpeg crop and progress/filter diagnostics docs: https://ffmpeg.org/ffmpeg-filters.html#crop and https://ffmpeg.org/ffmpeg.html. Confidence: MEDIUM through GSD `brave` verified tier.
