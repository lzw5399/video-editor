# Phase 20: Long Timeline Product UAT And Guard Baseline - Research

**Researched:** 2026-06-28 CST
**Domain:** Packaged Electron product UAT, long-timeline Rust fixture generation, canonical `.veproj` persistence, scheduler/preview/export guard evidence
**Confidence:** HIGH for repo surfaces and locked scope; MEDIUM for external Playwright/Electron documentation fallback

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
## Implementation Decisions

### Long Timeline Scale

- **D-01:** Use a hybrid baseline. Product E2E runs a stable medium-long timeline; Rust/testkit runs larger pressure cases.
- **D-02:** Product E2E baseline is `180 segments/track x 3 tracks` covering video, audio, and text, about `540` total segments.
- **D-03:** Product E2E segment grain is `1s/segment`, producing about a 3-minute timeline. Export should validate metadata and sampled semantic frames rather than relying on file existence.
- **D-04:** Rust/testkit blocking gate should cover `1000 segments/track`; `3000 segments/track` is a non-blocking diagnostic pressure run.

### Product UAT Path

- **D-05:** Generate the long project through Rust/testkit or an equivalent Rust-owned fixture path, then open the generated `.veproj` in the product. Do not build all 540 segments manually through the UI in the long-session UAT.
- **D-06:** After opening the generated project, the product UAT must perform the core edit set through normal UI paths: selection, scroll/zoom, scrub/play, move, trim, split, undo/redo, inspector visual edit, save/reopen, and export.
- **D-07:** Phase 19 retime/effect/filter/mask/blend parity controls are not part of Phase 20's main long-session path. Phase 20 may include already-stable baseline semantics if present in the fixture, but detailed Phase 19 parity belongs to Phase 23.
- **D-08:** The long-session product UAT must run two reopen cycles and two exports: open generated project, edit and save, reopen and verify, continue editing and save, reopen and verify, then export twice.
- **D-09:** Packaged Electron long-session UAT is the blocking product gate. A dev workflow may run the same flow or a shortened diagnostic flow for faster debugging, but packaged evidence is required for Phase 20 completion.

### Performance And Telemetry Budgets

- **D-10:** Use pragmatic product interaction budgets for the Phase 20 baseline: inspector edit `<= 2.5s`; selection, move, trim, split, undo, and redo single-step operations `<= 2s`; scroll, zoom, and scrub visible feedback `<= 1.5s`.
- **D-11:** Formalize the existing scheduler stress baseline: scheduler queue latency `p95 <= 2s`, normal product work rejected count `0`, fallback count `0`, stale generations may be rejected but must never present, and visible preview center must change under pressure.
- **D-12:** Export should complete within a reasonable test timeout, but fixed wall-clock export duration is not a hard success budget. Success requires `ffprobe` duration/fps/resolution/audio validation plus sampled semantic frames around start, middle, tail, or edit points.
- **D-13:** Rust/testkit large-scale gates should prioritize bounded graph diff, dirty range, and cache invalidation assertions over wall-clock timing. Wall-clock data should be captured as diagnostics, not used as a hard gate in Phase 20.

### Evidence And Failure Artifacts

- **D-14:** Phase 20 uses a structured evidence bundle. Successful runs keep lightweight JSON summaries; failed runs retain full Playwright trace, screenshots, video, telemetry, project semantic summaries, native preview evidence, `ffprobe` output, and sampled frame evidence.
- **D-15:** Save/reopen evidence uses semantic normalized comparison of canonical draft facts: materials, tracks, segments, timing, visual, audio, text, and revisions. Comparisons must explicitly ignore derived artifacts, absolute temp paths, runtime-only handles, and other non-canonical facts.
- **D-16:** No-fallback/source guards are blocking Phase 20 gates and must be extended to cover the generated long `.veproj`, packaged UAT, long-session preview evidence, and export evidence. Fallback, mock, artifact, CPU probe, DOM overlay, native single-video proof, first-frame snapshot, or file-exists-only export success must fail.
- **D-17:** Failure diagnostics should include a product-readable summary plus developer details. Product summary names the workflow, segment/time/export stage when possible; developer details include telemetry, native command observations, realtime host state, scheduler counters, and `ffprobe`/stderr summaries.

### the agent's Discretion

Planner and executor may choose exact helper names, test file names, JSON evidence schema details, export timeout values, sampled-frame extraction implementation, and non-blocking diagnostic command wiring, as long as the decisions above and the architecture boundary are preserved.

### Deferred Ideas (OUT OF SCOPE)
## Deferred Ideas

- Crop/export parity decisions are deferred to Phase 22.
- Existing Phase 19 effect/filter/transition/mask/blend parity and diagnostics taxonomy are deferred to Phase 23.
- Shortcut and high-frequency interaction hardening are deferred to Phase 21.
- UI polish and final acceptance sweep are deferred to Phase 24.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UAT11-01 | User can complete a packaged product E2E session that imports mixed media, edits a long timeline, previews through the production compositor, saves, reopens, exports, and continues editing. | Use existing packaged Electron launch plus new long-session UAT over a Rust-generated `.veproj`; preview proof must use `renderGraphGpuComposited` and visible pixel motion. [VERIFIED: .planning/REQUIREMENTS.md; apps/desktop-electron/tests/helpers/packagedApp.ts; apps/desktop-electron/tests/helpers/userJourney.ts] |
| UAT11-02 | User can repeat edit, save, reopen, and export cycles without semantic drift, stale preview/export state, or derived artifact pollution of `.veproj/project.json`. | `project_store` saves and opens canonical `project.json`, and existing tests reject unknown/derived fields; Phase 20 needs normalized semantic comparison around two reopen/export cycles. [VERIFIED: .planning/REQUIREMENTS.md; crates/project_store/src/bundle.rs; crates/project_store/tests/project_bundle.rs] |
| LONG11-01 | User can work on a long multi-track timeline with selection, scroll, zoom, scrub, move, trim, split, undo, redo, and preview within documented responsiveness budgets. | Existing Playwright helpers cover selection, seek/scrub, zoom, move, trim, split, undo, redo, inspector visual edit, and preview evidence; Phase 20 should add stopwatch budget gates around those helpers on the long fixture. [VERIFIED: .planning/REQUIREMENTS.md; apps/desktop-electron/tests/helpers/userJourney.ts] |
| LONG11-02 | Export, artifact generation, probing, and cache work do not block playhead scrub, inspector edits, preview delivery, or interaction-session commit and cancel paths. | Existing scheduler stress test already combines export/import pressure, inspector edit timing, telemetry, visible preview changes, rejected count `0`, and fallback count `0`; Phase 20 should run the same pattern against the long project and add commit/cancel evidence. [VERIFIED: .planning/REQUIREMENTS.md; apps/desktop-electron/tests/product-scheduler-stress.spec.ts; crates/realtime_preview_runtime/src/telemetry.rs] |
| GATE11-01 | Product success cannot be satisfied by fallback, mock, artifact, CPU probe, DOM overlay, native single-video proof, first-frame snapshot, or file-exists-only export evidence. | Existing no-fallback guard and product helpers already reject artifact preview loops, non-`renderGraphGpu` backends, diagnostic sources, and missing compositor success; Phase 20 must extend those guards to the new long UAT and export proof. [VERIFIED: .planning/REQUIREMENTS.md; docs/no-product-fallback-policy.md; scripts/no-product-fallback-guards.sh] |
</phase_requirements>

## Project Constraints (from AGENTS.md)

- UI emits commands; Rust core owns project and timeline semantics; UI code must not construct FFmpeg commands. [VERIFIED: AGENTS.md]
- Do not patch around known-wrong preview, edit, render, session, media, or native-surface boundaries; replace structurally wrong boundaries and delete legacy paths. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is canonical; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, preview caches, and exports are derived artifacts. [VERIFIED: AGENTS.md]
- Use Jianying-style vocabulary: draft/material/track/segment/keyframe/filter/transition. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates, not naked persisted floating-point time. [VERIFIED: AGENTS.md]
- Render Graph owns editing-to-render intent isolation; FFmpeg Runtime executes jobs and reports progress/errors. [VERIFIED: AGENTS.md]
- Kdenlive/MLT are conceptual references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md]
- External drafts go through adapters and compatibility reports; proprietary IDs stay external references. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution must be reviewed for LGPL/GPL/nonfree build options, notices, and commercial product obligations. [VERIFIED: AGENTS.md]
- Direct repo edits should happen through GSD workflows unless explicitly bypassed. [VERIFIED: AGENTS.md]
- The production architecture review skill requires current-code inspection before architecture judgment and rejects fallback-driven product proof. [VERIFIED: .agents/skills/production-architecture-review/SKILL.md]

## Summary

Phase 20 should be planned as a guard and evidence phase, not as new editor feature breadth. The locked path is to generate a deterministic 180 segments/track x 3-track `.veproj` through Rust/testkit, open it in packaged Electron, perform a normal-user edit/save/reopen/export/reopen/export loop, and record evidence that proves production compositor preview, canonical persistence, export validity, and scheduler responsiveness. [VERIFIED: .planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-CONTEXT.md; .planning/ROADMAP.md]

The repo already has most of the primitives the planner should reuse: `LargeTimelineConfig`/`build_large_timeline`, bounded dirty-range/cache tests, packaged Electron launch, product UI helpers, scheduler telemetry readers, preview evidence helpers, no-fallback source guards, and project-store canonical round-trip tests. The missing Phase 20 work is the connective baseline: materialize a real long product bundle, add semantic normalization/evidence bundle helpers, add a packaged long-session spec, extend guard scripts, and wire a `test:phase20` aggregate. [VERIFIED: crates/testkit/src/large_timeline.rs; crates/testkit/tests/large_timeline_incremental.rs; apps/desktop-electron/tests/helpers/packagedApp.ts; apps/desktop-electron/tests/helpers/userJourney.ts; scripts/no-product-fallback-guards.sh]

**Primary recommendation:** Use existing Rust/testkit and Playwright product helpers; add only Phase 20-specific fixture materialization, semantic comparison, evidence bundling, sampled export-frame validation, and source-guard wiring. Do not introduce new npm packages or alternate UAT frameworks. [VERIFIED: package.json; apps/desktop-electron/package.json; docs/product-e2e-acceptance-policy.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Long `.veproj` fixture generation | Rust testkit / project_store | Playwright reads generated path | Segment/material/track semantics and canonical save belong below the UI; Playwright should open the bundle, not synthesize its semantics. [VERIFIED: crates/testkit/src/large_timeline.rs; crates/project_store/src/bundle.rs] |
| Product long-session workflow | Electron product UI | Rust editor runtime | The product test must click/drag/type through visible controls or the same UI command bridge, while Rust owns command execution and saves. [VERIFIED: docs/product-e2e-acceptance-policy.md; apps/desktop-electron/tests/helpers/userJourney.ts] |
| Preview evidence | realtime_preview_runtime / native preview host | Electron captures telemetry and pixels | Product success requires `renderGraphGpuComposited`, `renderGraphGpu`, visible pixel change, and no artifact frame loop. [VERIFIED: docs/no-product-fallback-policy.md; apps/desktop-electron/tests/helpers/userJourney.ts] |
| Export evidence | editor_runtime / ffmpeg_compiler / media_runtime | Playwright uses bundled ffprobe and sampled frames | Export semantics and FFmpeg compilation stay Rust-owned; Playwright may validate output metadata and sample evidence. [VERIFIED: docs/runtime-boundaries.md; apps/desktop-electron/tests/helpers/realWorkflow.ts] |
| Save/reopen canonical comparison | project_store / draft_model | Playwright evidence helper reads `project.json` | `project_store` validates and serializes canonical draft facts; UI must not decide canonical equality. [VERIFIED: crates/project_store/src/bundle.rs; crates/project_store/tests/project_bundle.rs] |
| Scheduler budget gating | task_runtime / realtime_preview_runtime | Playwright reads product-safe telemetry | Queue latency, rejected/fallback/stale counters, pacing, and resource saturation are Rust telemetry surfaced through `getTaskRuntimeTelemetry`. [VERIFIED: crates/realtime_preview_runtime/src/telemetry.rs; apps/desktop-electron/src/main/nativeBinding.ts] |
| No-fallback/source guards | scripts + product tests | Rust/Electron source surfaces | Source guards fail old product-success paths before runtime tests can mask them. [VERIFIED: scripts/no-product-fallback-guards.sh; scripts/phase19-source-guards.sh] |

## Standard Stack

### Core

| Library / Component | Version | Purpose | Why Standard |
|---------------------|---------|---------|--------------|
| Rust workspace crates | local `0.1.0` crates | Draft model, commands, engine, render graph, runtime, project store, bindings, testkit | Existing architecture places semantics, persistence, preview/export ownership, and scheduler telemetry in Rust. [VERIFIED: cargo metadata --locked; docs/runtime-boundaries.md] |
| Electron | `42.4.1`, npm modified `2026-06-25T23:07:44.337Z` | Desktop product shell and packaged app target | Existing desktop app and packaged UAT helpers are Electron-based. [VERIFIED: apps/desktop-electron/package.json; npm registry] |
| React + TypeScript | React `19.2.7`, TypeScript `6.0.3` | Renderer UI and typed product test integration | Existing renderer and generated contracts are TypeScript/React. [VERIFIED: apps/desktop-electron/package.json; npm registry] |
| Node-API via `@napi-rs/cli` | `3.7.2`, npm modified `2026-06-14T02:55:57.636Z` | Build `bindings_node` for Electron | Existing `build:native` uses `napi build` over the Rust binding crate. [VERIFIED: apps/desktop-electron/package.json; npm registry] |
| Playwright Test | `@playwright/test` `1.61.0`, npm modified `2026-06-27T06:18:39.822Z` | Electron product UAT and trace retention | Existing tests use Playwright; official docs expose Electron automation through `_electron`, `ElectronApplication`, `firstWindow`, and `evaluate`. [VERIFIED: apps/desktop-electron/playwright.config.ts; CITED: https://playwright.dev/docs/api/class-electron] |
| electron-builder | `26.15.3`, npm modified `2026-06-26T15:05:31.059Z` | Packaged app directory builds | Existing packaged scripts run `electron-builder --dir`; Phase 20 completion requires packaged evidence. [VERIFIED: apps/desktop-electron/package.json; npm registry] |

### Supporting

| Component | Version | Purpose | When to Use |
|-----------|---------|---------|-------------|
| `project_store` | local `0.1.0` | Canonical `.veproj/project.json` save/open and material URI validation | Use for generated long bundle creation and reopen semantic comparisons. [VERIFIED: crates/project_store/src/lib.rs; crates/project_store/src/bundle.rs] |
| `testkit::large_timeline` | local `0.1.0` | Deterministic long draft builder and localized edit target | Extend for 180 x 3 product bundle and 1000/3000 pressure gates. [VERIFIED: crates/testkit/src/large_timeline.rs] |
| `task_runtime` telemetry | local `0.1.0` | Scheduler counters and latency summaries | Gate queue p95, rejected count, fallback count, stale rejection, and saturation under pressure. [VERIFIED: crates/task_runtime/tests/scheduler_telemetry.rs; crates/bindings_node/tests/scheduler_runtime.rs] |
| Bundled ffprobe | app-local runtime path | Export metadata validation | Use app runtime-discovered ffprobe, not `PATH`, for product proof. [VERIFIED: apps/desktop-electron/tests/helpers/realWorkflow.ts; docs/runtime-boundaries.md] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Rust-generated fixture | Building 540 segments manually through UI | Explicitly rejected by D-05; manual UI construction would test fixture creation latency more than long-session editing. [VERIFIED: 20-CONTEXT.md] |
| Packaged Electron UAT | Dev Electron-only UAT | Dev flow is useful for diagnosis, but D-09 requires packaged evidence for completion. [VERIFIED: 20-CONTEXT.md] |
| Playwright product proof | Cypress, unit tests, direct N-API calls | New framework/package is unnecessary; product policy requires Playwright/Electron normal-user evidence and existing helper reuse. [VERIFIED: docs/product-e2e-acceptance-policy.md; apps/desktop-electron/tests/helpers/userJourney.ts] |
| File-exists export proof | `fs.existsSync(output)` only | Rejected by GATE11-01 and D-12; export proof must include ffprobe metadata and sampled semantic frames. [VERIFIED: .planning/REQUIREMENTS.md; 20-CONTEXT.md] |

**Installation:**

```bash
# No new external packages are recommended for Phase 20.
pnpm install --frozen-lockfile
```

**Version verification:** `npm view` verified Electron `42.4.1`, `@playwright/test` `1.61.0`, `electron-builder` `26.15.3`, `@napi-rs/cli` `3.7.2`, React/React DOM `19.2.7`, Vite `8.0.16`, TypeScript `6.0.3`, and `@vitejs/plugin-react` `6.0.2`; `cargo metadata --locked` verified the local workspace crate versions. [VERIFIED: npm registry; cargo metadata --locked]

## Package Legitimacy Audit

Phase 20 should not install new external packages; it should use the existing repo stack and local Rust crates. [VERIFIED: package.json; apps/desktop-electron/package.json]

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| None new | npm/crates | N/A | N/A | N/A | N/A | No new install recommended. [VERIFIED: package.json] |

**Packages removed due to [SLOP] verdict:** none. [VERIFIED: package.json]
**Packages flagged as suspicious [SUS]:** none. [VERIFIED: package.json]

*The package-legitimacy gate is not required because this phase installs no new external packages; existing package versions were registry-checked for currency. [VERIFIED: npm registry]*

## Architecture Patterns

### System Architecture Diagram

```text
Rust/testkit fixture config
  -> build_large_timeline(180 x 3, 1s grain, real fixture URIs)
  -> project_store::save_project_bundle()
  -> generated .veproj/project.json
  -> packaged Electron open-project path
  -> normal UI actions: select/scroll/zoom/scrub/move/trim/split/undo/redo/inspector
  -> Rust project-session intents and interaction commits
  -> realtime preview host + renderGraphGpuComposited evidence
  -> save/reopen semantic normalizer
  -> export modal -> editor_runtime/export service -> ffmpeg_compiler/media_runtime
  -> bundled ffprobe + sampled frames/audio evidence
  -> structured evidence bundle
  -> phase20 source guards and aggregate gate
```

### Recommended Project Structure

```text
crates/testkit/
+-- src/large_timeline.rs                         # extend config/materialization helpers
+-- tests/large_timeline_incremental.rs           # add 1000 blocking and 3000 diagnostic coverage

apps/desktop-electron/tests/
+-- product-long-timeline-uat.spec.ts             # new packaged long-session UAT
+-- helpers/
    +-- longTimelineFixture.ts                    # invokes Rust-generated .veproj or consumes generated path
    +-- longTimelineEvidence.ts                   # evidence bundle, semantic summary, export samples
    +-- userJourney.ts                            # reuse/extend product UI actions and telemetry readers
    +-- realWorkflow.ts                           # reuse bundled ffprobe/export helpers where possible

scripts/
+-- phase20-source-guards.sh                      # long UAT/no-fallback/export/source guard wiring
```

### Pattern 1: Rust-Owned Long Product Fixture

**What:** Extend `testkit::large_timeline` with a Phase 20 materialization path that builds 180 segments/track x 3 tracks at `1_000_000` microseconds per segment, assigns repo-owned media fixture URIs or bundle-relative copied fixtures, saves via `project_store`, and emits fixture metadata for Playwright. [VERIFIED: crates/testkit/src/large_timeline.rs; crates/project_store/src/bundle.rs]

**When to use:** Use before launching packaged Electron so Playwright opens the canonical project rather than constructing 540 segments manually. [VERIFIED: 20-CONTEXT.md]

**Example:**

```rust
// Source: crates/testkit/src/large_timeline.rs and crates/project_store/src/bundle.rs
let fixture = build_large_timeline(
    LargeTimelineConfig::new(180)
        .with_track_mix(true, true, true)
        .with_segment_duration(Microseconds::new(1_000_000))
        .with_target_stride(Microseconds::new(1_000_000)),
)?;
save_project_bundle(&StdPlatformFileSystem, &bundle_path, &fixture.draft)?;
```

### Pattern 2: Packaged Product UAT Entry

**What:** Use `pnpm --filter @video-editor/desktop package:dir`, `launchPackagedApp`, real runtime env flags, and `VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE`/open-project UI path for blocking product evidence. [VERIFIED: apps/desktop-electron/package.json; apps/desktop-electron/tests/helpers/packagedApp.ts; apps/desktop-electron/tests/helpers/userJourney.ts]

**When to use:** Use for the blocking Phase 20 UAT; keep dev launch only for shorter diagnostics. [VERIFIED: 20-CONTEXT.md]

**Example:**

```typescript
// Source: apps/desktop-electron/tests/helpers/packagedApp.ts
const { app, page } = await launchPackagedApp({
  VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
  VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "0",
  VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE: bundlePath,
  VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([])
});
```

### Pattern 3: Product Preview Evidence Gate

**What:** Require `ok`, `productReady`, `fallbackActive === false`, backend `renderGraphGpu`, diagnostic source `none`, content evidence `renderGraphGpuComposited`, visible center hash change, and unchanged `requestProjectSessionPreviewFrame` count. [VERIFIED: apps/desktop-electron/tests/helpers/userJourney.ts; scripts/no-product-fallback-guards.sh]

**When to use:** Every scrub/play/preview assertion in the long UAT and scheduler-pressure path. [VERIFIED: docs/no-product-fallback-policy.md]

**Example:**

```typescript
// Source: apps/desktop-electron/tests/helpers/userJourney.ts
const before = await waitForCompositedPreviewEvidence(page, app, 15_000, -1);
const visibleBefore = await captureVisiblePreviewEvidence(page, app);
const frameRequestsBefore = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
const playback = await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBefore);
expect(playback.after.hostState?.contentEvidence?.source).toBe("renderGraphGpuComposited");
```

### Pattern 4: Canonical Save/Reopen Comparison

**What:** Compare normalized draft semantics from `.veproj/project.json`: materials, tracks, segments, target/source timing, visual/audio/text fields, and expected revision facts; assert forbidden derived/runtime fields are absent. [VERIFIED: 20-CONTEXT.md; crates/project_store/tests/project_bundle.rs; scripts/phase13-source-guards.sh]

**When to use:** After each save/reopen cycle and before/after exports. [VERIFIED: 20-CONTEXT.md]

**Example:**

```typescript
// Source pattern: crates/project_store/tests/project_bundle.rs
const canonical = normalizeProjectJson(await readProjectJson(bundlePath));
expect(canonical.derivedArtifacts).toEqual([]);
expect(canonical.materials.length).toBeGreaterThanOrEqual(3);
expect(canonical.tracks.map((track) => track.kind)).toEqual(expect.arrayContaining(["video", "audio", "text"]));
```

### Pattern 5: Scheduler Budget Gate Under Product Pressure

**What:** Measure UI operation elapsed time in Playwright and pair it with `getTaskRuntimeTelemetry` counters: `queueLatencyUs.p95 <= 2_000_000`, `rejectedCount === 0`, `fallbackCount === 0`, stale generations rejected but not presented, and visible preview center changes. [VERIFIED: apps/desktop-electron/tests/product-scheduler-stress.spec.ts; apps/desktop-electron/tests/helpers/userJourney.ts]

**When to use:** While export/probe/artifact/cache pressure is active in the long UAT. [VERIFIED: 20-CONTEXT.md]

**Example:**

```typescript
// Source: apps/desktop-electron/tests/product-scheduler-stress.spec.ts
const before = await readTaskRuntimeTelemetry(page);
const startedAt = Date.now();
await updateSelectedVisualThroughInspector(page, app);
expect(Date.now() - startedAt).toBeLessThanOrEqual(2_500);
const after = await readTaskRuntimeTelemetry(page);
expect(after.queueLatencyUs.p95 ?? 0).toBeLessThanOrEqual(2_000_000);
expect(after.rejectedCount).toBe(0);
expect(after.fallbackCount).toBe(0);
```

### Anti-Patterns to Avoid

- **UI-built long fixture:** Do not create all 540 segments through Playwright setup; D-05 locks Rust/testkit generation. [VERIFIED: 20-CONTEXT.md]
- **Direct Rust/N-API success proof:** Direct calls may prepare fixtures or inspect evidence, but product success must be driven through packaged UI or the same UI bridge. [VERIFIED: docs/product-e2e-acceptance-policy.md]
- **File-exists-only export proof:** Export success must validate metadata and sampled semantic frames/audio, not only output presence. [VERIFIED: 20-CONTEXT.md; docs/product-e2e-acceptance-policy.md]
- **Renderer canonical comparisons:** The renderer must not own save/reopen canonical equality, cache invalidation semantics, FFmpeg commands, or render graph decisions. [VERIFIED: docs/runtime-boundaries.md; scripts/phase19-source-guards.sh]
- **Fallback as diagnostic success:** Fallback/mock/artifact/CPU/DOM/native-video/first-frame evidence may be diagnostic only and must fail product success. [VERIFIED: docs/no-product-fallback-policy.md; scripts/no-product-fallback-guards.sh]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Long draft semantics | TypeScript JSON generator | `testkit::large_timeline` + `project_store::save_project_bundle` | Rust owns draft validation and canonical persistence. [VERIFIED: crates/testkit/src/large_timeline.rs; crates/project_store/src/bundle.rs] |
| Product app launch | Custom Electron process spawning | `launchPackagedApp` and existing Playwright config | Helper already finds packaged executable, poisons `PATH`, and launches through Playwright. [VERIFIED: apps/desktop-electron/tests/helpers/packagedApp.ts] |
| Preview proof | Screenshot-only or DOM overlay hash | `waitForProductPlaybackSuccess`, `captureVisiblePreviewEvidence`, host telemetry | Existing helper rejects playhead-only and artifact preview success. [VERIFIED: apps/desktop-electron/tests/helpers/userJourney.ts] |
| Export proof | `existsSync(outputPath)` | Bundled ffprobe plus sampled frames/audio evidence | Product policy requires output validation and preview/export semantic evidence. [VERIFIED: docs/product-e2e-acceptance-policy.md; apps/desktop-electron/tests/helpers/realWorkflow.ts] |
| Scheduler telemetry parsing | Raw scheduler internals in UI | `getTaskRuntimeTelemetry` product-safe response | Existing binding exposes product-safe counters and hides raw internals by default. [VERIFIED: crates/bindings_node/tests/scheduler_runtime.rs; apps/desktop-electron/src/main/nativeBinding.ts] |
| No-fallback enforcement | Ad hoc assertions per spec only | Extend `scripts/no-product-fallback-guards.sh` and add `scripts/phase20-source-guards.sh` | Source guards fail old success paths before UAT flakiness can hide them. [VERIFIED: scripts/no-product-fallback-guards.sh; scripts/phase19-source-guards.sh] |

**Key insight:** Phase 20 is a proof-quality baseline. The highest-risk failure is not missing a helper; it is accepting product success from a smaller fixture, fallback preview, direct runtime call, or file existence. [VERIFIED: .planning/research/SUMMARY.md; docs/no-product-fallback-policy.md]

## Common Pitfalls

### Pitfall 1: Product UAT Accidentally Tests Setup Work

**What goes wrong:** A Playwright spec creates 540 segments through the UI and times out or measures setup latency instead of long-session editing. [VERIFIED: 20-CONTEXT.md]
**Why it happens:** Fixture generation is placed in the browser tier rather than Rust/testkit. [VERIFIED: docs/runtime-boundaries.md]
**How to avoid:** Generate and save the long bundle before product launch, then use UI paths only for the core edit set. [VERIFIED: 20-CONTEXT.md]
**Warning signs:** The spec contains loops adding hundreds of segments through visible controls. [ASSUMED]

### Pitfall 2: Canonical Drift Hidden By Raw JSON Equality

**What goes wrong:** Tests either fail on harmless temp paths/revisions or miss derived artifact pollution because they compare the wrong fields. [VERIFIED: 20-CONTEXT.md]
**Why it happens:** `.veproj/project.json` contains canonical draft facts, while paths, runtime handles, render graphs, thumbnails, waveforms, preview caches, and exports are separate concerns. [VERIFIED: docs/runtime-boundaries.md; crates/project_store/src/lib.rs]
**How to avoid:** Build a semantic normalizer that includes materials/tracks/segments/timing/visual/audio/text and explicitly asserts forbidden derived/runtime fields are absent. [VERIFIED: 20-CONTEXT.md; crates/project_store/tests/project_bundle.rs]
**Warning signs:** Assertions only check `project.json` exists or compare raw strings without normalization. [VERIFIED: apps/desktop-electron/tests/helpers/realWorkflow.ts; 20-CONTEXT.md]

### Pitfall 3: Preview Evidence Rewards A Fallback Path

**What goes wrong:** A test passes because the playhead advanced, a first frame appeared, or a DOM/screenshot changed while the production compositor was unavailable. [VERIFIED: apps/desktop-electron/tests/product-user-journey.spec.ts; docs/no-product-fallback-policy.md]
**Why it happens:** Product proof omits host backend/source checks and frame-request counts. [VERIFIED: apps/desktop-electron/tests/helpers/userJourney.ts]
**How to avoid:** Require `renderGraphGpuComposited`, backend `renderGraphGpu`, diagnostic source `none`, visible center motion, and unchanged preview-frame artifact request count. [VERIFIED: apps/desktop-electron/tests/helpers/userJourney.ts]
**Warning signs:** Assertions mention `firstFrame`, `nativeVideoBridge`, `requestProjectSessionPreviewFrame`, `previewArtifact`, or file-exists-only success. [VERIFIED: scripts/no-product-fallback-guards.sh]

### Pitfall 4: Export Gate Uses Wall-Clock Duration As Success

**What goes wrong:** Slow machines fail valid exports, or invalid exports pass because they complete quickly. [VERIFIED: 20-CONTEXT.md]
**Why it happens:** Export duration is easier to measure than semantic media validity. [ASSUMED]
**How to avoid:** Use a generous test timeout; gate success on ffprobe duration/fps/resolution/audio plus sampled semantic frames at start/middle/tail/edit points. [VERIFIED: 20-CONTEXT.md]
**Warning signs:** The only export assertions are timeout and output file existence. [VERIFIED: docs/product-e2e-acceptance-policy.md]

### Pitfall 5: Scheduler Metrics Collected But Not Blocking

**What goes wrong:** Telemetry is logged but UAT still passes with rejected normal work, fallback count, or stale presentation. [VERIFIED: apps/desktop-electron/tests/product-scheduler-stress.spec.ts]
**Why it happens:** Diagnostics are treated as observation rather than acceptance criteria. [ASSUMED]
**How to avoid:** Promote queue p95, rejected count, fallback count, visible preview change, and stale-not-presented checks to blocking assertions. [VERIFIED: 20-CONTEXT.md; apps/desktop-electron/tests/product-scheduler-stress.spec.ts]
**Warning signs:** Console metrics are emitted without `expect(...)` budget assertions. [ASSUMED]

## Code Examples

### Long Fixture Materialization

```rust
// Source: crates/testkit/src/large_timeline.rs
pub fn phase20_product_fixture(
    bundle_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_large_timeline(
        LargeTimelineConfig::new(180)
            .with_track_mix(true, true, true)
            .with_segment_duration(Microseconds::new(1_000_000))
            .with_target_stride(Microseconds::new(1_000_000)),
    )?;
    save_project_bundle(&StdPlatformFileSystem, bundle_path, &fixture.draft)?;
    Ok(())
}
```

### Long UAT Product Loop

```typescript
// Source: apps/desktop-electron/tests/helpers/packagedApp.ts and helpers/userJourney.ts
await generatePhase20LongProject(bundlePath);
const { app, page } = await launchPackagedApp({
  VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
  VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "0",
  VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE: bundlePath,
  VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([])
});
await openProjectFromProductEntry(app, page);
await selectLongTimelineSegment(page, app);
await zoomTimelineIn(page);
await seekTimelinePlayhead(page, app, 90_000_000);
await moveSelectedSegmentBy(page, app, 500_000);
await trimSelectedSegmentRightEdgeLeft(page, app, 250_000);
await splitSelectedSegment(page, app, 91_000_000);
await undoTimelineEdit(page, app);
await redoTimelineEdit(page, app);
```

### Semantic Project Summary

```typescript
// Source pattern: crates/project_store/tests/project_bundle.rs
type CanonicalSummary = {
  materials: Array<{ id: string; kind: string; uri: string; durationUs: number | null }>;
  tracks: Array<{ id: string; kind: string; segmentCount: number }>;
  segments: Array<{ id: string; trackId: string; materialId: string; startUs: number; durationUs: number }>;
};
```

### Export Metadata And Sample Evidence

```typescript
// Source: apps/desktop-electron/tests/helpers/realWorkflow.ts
const ffprobePath = await readBundledFfprobePath(page);
const { stdout } = await execFileAsync(ffprobePath, [
  "-v", "error", "-print_format", "json", "-show_format", "-show_streams", outputPath
]);
const probe = JSON.parse(stdout);
expect(probe.streams.find((stream) => stream.codec_type === "video")?.avg_frame_rate).toBe("30/1");
expect(probe.streams.find((stream) => stream.codec_type === "audio")).toBeDefined();
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Unit-only or tiny-fixture feature proof | Packaged Electron product UAT with visible/exported evidence | v1.1 roadmap created 2026-06-27 | Phase 20 is the first v1.1 product-truth gate. [VERIFIED: .planning/ROADMAP.md; .planning/research/SUMMARY.md] |
| Preview PNG/artifact/native-video proof | `renderGraphGpuComposited` production compositor evidence plus visible pixel motion | v1.0 Phase 11+ and no-fallback hardening | Preview success fails closed without compositor evidence. [VERIFIED: docs/no-product-fallback-policy.md; apps/desktop-electron/tests/helpers/userJourney.ts] |
| File-exists export validation | ffprobe metadata plus sampled semantic frames/audio | Locked for Phase 20 | Export proof cannot be satisfied by an empty/wrong file. [VERIFIED: 20-CONTEXT.md] |
| Renderer/cache/FFmpeg ownership | Rust project sessions, render graph, compiler/runtime, task telemetry | v1.0 runtime boundaries | Phase 20 guards must preserve the boundary while adding UAT. [VERIFIED: docs/runtime-boundaries.md] |

**Deprecated/outdated:**
- `requestProjectSessionPreviewFrame` loops as playback proof are forbidden for product success. [VERIFIED: scripts/no-product-fallback-guards.sh; apps/desktop-electron/tests/helpers/userJourney.ts]
- Native single-video bridge evidence is diagnostic-only and cannot prove product realtime preview. [VERIFIED: docs/no-product-fallback-policy.md; apps/desktop-electron/tests/product-user-journey.spec.ts]
- Direct UI/renderer FFmpeg or render graph construction remains forbidden. [VERIFIED: docs/runtime-boundaries.md; scripts/phase19-source-guards.sh]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Warning signs in some pitfalls are inferred patterns rather than verified current violations. | Common Pitfalls | Planner may over-prioritize source scans that find no current issue. |
| A2 | Sampled export-frame validation can be implemented with the bundled FFmpeg/ffprobe runtime and Node `execFile` without adding packages. | Architecture Patterns / Code Examples | Planner may need a small Rust/testkit helper if Node-side sampling becomes too brittle. |
| A3 | Node `24.15.0` should be acceptable even though `package.json` pins engine text to `24.12.0`. | Environment Availability | If tooling enforces exact engine equality, packaged gates may need Node normalization first. |
| A4 | The 3000 segments/track diagnostic can be wired as non-blocking script output rather than part of the default blocking aggregate. | Validation Architecture | If CI treats all script failures as blocking, the planner must isolate diagnostic execution. |

## Open Questions

1. **Exact sampled-frame implementation**
   - What we know: D-12 requires ffprobe metadata plus sampled semantic frames near start/middle/tail/edit points. [VERIFIED: 20-CONTEXT.md]
   - What's unclear: Whether sampling should run via bundled `ffmpeg` from Playwright or a Rust/testkit helper.
   - Recommendation: Prefer existing bundled-runtime discovery from product tests first; move sampling into Rust/testkit only if Playwright sampling is flaky. [ASSUMED]

2. **Evidence bundle schema**
   - What we know: D-14 requires lightweight success JSON and full failure artifacts. [VERIFIED: 20-CONTEXT.md]
   - What's unclear: Exact JSON keys and retention directory names.
   - Recommendation: Use `test-results/phase20/<run-id>/summary.json` for success and nested `trace/`, `screenshots/`, `video/`, `telemetry/`, `project/`, `export/` folders on failure. [ASSUMED]

3. **Node engine exactness**
   - What we know: repo `engines.node` says `24.12.0`; current local Node is `24.15.0`. [VERIFIED: package.json; environment probe]
   - What's unclear: Whether package tooling enforces exact equality in this workspace.
   - Recommendation: Planner should add a Wave 0 environment checkpoint or use the repo's existing package manager path before declaring packaged UAT blocked. [ASSUMED]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | pnpm, Playwright, Electron build | Yes with exact-engine caveat | `v24.15.0` vs repo engine `24.12.0` | Use repo-supported Node if engine strictness blocks install/build. [VERIFIED: environment probe; package.json] |
| pnpm | package scripts | Yes | `10.32.1`, matches `packageManager` | None needed. [VERIFIED: environment probe; package.json] |
| corepack | pnpm management | Yes | `0.34.6` | Use installed pnpm directly. [VERIFIED: environment probe] |
| Cargo | Rust tests/builds | Yes | `1.95.0` | None needed. [VERIFIED: environment probe] |
| rustc | Rust crates | Yes | `1.95.0` | None needed. [VERIFIED: environment probe] |
| ffmpeg | local media sampling diagnostics | Yes on `PATH` | `8.1.2` | Product proof should use bundled runtime path, not `PATH`. [VERIFIED: environment probe; apps/desktop-electron/tests/helpers/realWorkflow.ts] |
| ffprobe | local media metadata diagnostics | Yes on `PATH` | `8.1.2` | Product proof should use bundled runtime path. [VERIFIED: environment probe; docs/runtime-boundaries.md] |
| ripgrep | source guards | Yes | `15.1.0` | Required by source guard scripts. [VERIFIED: environment probe; scripts/phase13-source-guards.sh] |
| git | commit and diff gates | Yes | `2.50.1` | None. [VERIFIED: environment probe] |

**Missing dependencies with no fallback:** none found. [VERIFIED: environment probe]

**Missing dependencies with fallback:** Node exact engine mismatch may require switching to `24.12.0` if tooling is strict. [VERIFIED: environment probe; package.json]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Cargo/Rust tests and Playwright Test `1.61.0` for Electron. [VERIFIED: Cargo.toml; apps/desktop-electron/package.json] |
| Config file | `apps/desktop-electron/playwright.config.ts` with `trace: "retain-on-failure"` and 30s default timeout; specs override to 90s/120s where needed. [VERIFIED: apps/desktop-electron/playwright.config.ts; apps/desktop-electron/tests/product-scheduler-stress.spec.ts] |
| Quick run command | `cargo test -p testkit large_timeline_incremental -- --nocapture` plus `pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts --grep @diagnostic --reporter=line --workers=1` after Wave 0 creates the spec. [VERIFIED: package.json; planned file] |
| Full suite command | `pnpm run test:phase20` after Wave 0 wires the aggregate. [VERIFIED: package.json pattern; planned script] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| UAT11-01 | Packaged long product session opens generated project, edits, previews, saves/reopens, exports, continues editing, exports again | Playwright packaged E2E | `pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts --reporter=line --workers=1` | No - Wave 0 |
| UAT11-02 | Two save/reopen/export cycles keep normalized canonical semantics stable and derived artifacts absent | Playwright E2E + Rust canonical helper | `pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts -g "canonical" --reporter=line --workers=1` | No - Wave 0 |
| LONG11-01 | Selection, scroll, zoom, scrub, move, trim, split, undo, redo, preview meet budgets on long timeline | Playwright E2E | `pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts -g "responsiveness" --reporter=line --workers=1` | No - Wave 0 |
| LONG11-02 | Export/probe/artifact/cache pressure does not block scrub, inspector edit, preview delivery, commit/cancel | Playwright stress + Rust scheduler telemetry | `pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts -g "pressure" --reporter=line --workers=1` | No - Wave 0 |
| GATE11-01 | No fallback/mock/artifact/CPU/DOM/native-video/first-frame/file-exists-only success | Source guard + Playwright negative checks | `pnpm run test:no-product-fallback && bash scripts/phase20-source-guards.sh` | Existing guard yes; Phase 20 guard no - Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p testkit large_timeline_incremental -- --nocapture` or the narrow Playwright grep for the touched helper. [VERIFIED: package.json; apps/desktop-electron/tests/helpers/userJourney.ts]
- **Per wave merge:** `pnpm run test:no-product-fallback && bash scripts/phase20-source-guards.sh && pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts --reporter=line --workers=1`. [VERIFIED: scripts/no-product-fallback-guards.sh; planned script]
- **Phase gate:** `pnpm run test:phase20`, expected to compose Rust large-timeline gates, no-fallback/source guards, packaged long UAT, `cargo check --workspace --locked`, and `pnpm run test:contracts`. [VERIFIED: package.json phase script pattern]

### Wave 0 Gaps

- [ ] `crates/testkit/tests/long_timeline_product_fixture.rs` or extension inside `large_timeline_incremental.rs` for 180 product fixture, 1000 blocking gate, and 3000 diagnostic gate. [VERIFIED: crates/testkit/tests/large_timeline_incremental.rs]
- [ ] `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts` for packaged long-session UAT. [VERIFIED: apps/desktop-electron/tests]
- [ ] `apps/desktop-electron/tests/helpers/longTimelineEvidence.ts` for semantic summaries, evidence bundle writing, ffprobe/sample collection, and budget assertions. [VERIFIED: apps/desktop-electron/tests/helpers/userJourney.ts; apps/desktop-electron/tests/helpers/realWorkflow.ts]
- [ ] `scripts/phase20-source-guards.sh` to require long UAT files and reject fallback/source-success patterns. [VERIFIED: scripts/no-product-fallback-guards.sh]
- [ ] `package.json` scripts: `test:phase20-rust`, `test:phase20-source-guards`, `test:phase20-desktop`, `test:phase20`. [VERIFIED: package.json]

## Security Domain

Security enforcement is enabled in `.planning/config.json`; Phase 20 touches local files, packaged app launch, media probing/export processes, and product evidence integrity. [VERIFIED: .planning/config.json; docs/runtime-boundaries.md]

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No authentication surface in this desktop UAT phase. [VERIFIED: .planning/REQUIREMENTS.md] |
| V3 Session Management | limited | Product project sessions are local editor runtime sessions, not user auth sessions; guard owner/revision/stale handling through existing project-session tests. [VERIFIED: crates/bindings_node/tests/project_session.rs] |
| V4 Access Control | limited | Restrict normal product proof to selected `.veproj` bundle and repo-owned media fixtures; do not expose raw backend selectors. [VERIFIED: docs/no-product-fallback-policy.md; apps/desktop-electron/tests/helpers/packagedApp.ts] |
| V5 Input Validation | yes | `project_store` rejects malformed JSON, unknown fields, unsupported schema versions, parent traversal, and derived artifact fields. [VERIFIED: crates/project_store/tests/project_bundle.rs; crates/project_store/src/paths.rs] |
| V6 Cryptography | no | No cryptographic feature is added; hashes are test evidence digests, not security controls. [ASSUMED] |

### Known Threat Patterns for Electron/Rust Local Media UAT

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal or absolute-path confusion in `.veproj` material URIs | Tampering / Information Disclosure | Use `project_store::classify_material_uri` and reject parent traversal for bundle-relative paths. [VERIFIED: crates/project_store/src/paths.rs] |
| PATH-based FFmpeg/ffprobe spoofing during packaged tests | Tampering | `launchPackagedApp` creates poison `PATH`; product runtime discovery must use bundled runtime. [VERIFIED: apps/desktop-electron/tests/helpers/packagedApp.ts; docs/runtime-boundaries.md] |
| Fallback evidence spoofing product success | Spoofing / Tampering | No-fallback guards and host-state assertions require `renderGraphGpuComposited`, `renderGraphGpu`, diagnostic source `none`, and no artifact frame loop. [VERIFIED: scripts/no-product-fallback-guards.sh; apps/desktop-electron/tests/helpers/userJourney.ts] |
| Derived artifact pollution of canonical project | Tampering | Project-store tests reject derived fields and Phase 20 should add long-project semantic normalizer checks. [VERIFIED: crates/project_store/tests/project_bundle.rs; 20-CONTEXT.md] |
| Raw scheduler/runtime internals leaking into product evidence | Information Disclosure | Product-safe telemetry hides raw scheduler details unless diagnostics are explicitly requested. [VERIFIED: crates/bindings_node/tests/scheduler_runtime.rs] |

## Sources

### Primary (HIGH confidence)

- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-CONTEXT.md` - locked Phase 20 decisions, boundaries, evidence requirements.
- `.planning/REQUIREMENTS.md` - UAT11/LONG11/GATE11 requirement text and traceability.
- `.planning/ROADMAP.md` and `.planning/STATE.md` - Phase 20 goal, success criteria, current state.
- `docs/product-e2e-acceptance-policy.md` - product evidence policy.
- `docs/no-product-fallback-policy.md` - no-fallback product success policy.
- `docs/runtime-boundaries.md` - Electron/Rust/runtime ownership boundaries.
- `crates/testkit/src/large_timeline.rs` and `crates/testkit/tests/large_timeline_incremental.rs` - existing large-timeline fixture and bounded assertions.
- `apps/desktop-electron/tests/helpers/userJourney.ts`, `realWorkflow.ts`, and `packagedApp.ts` - product helper surfaces.
- `scripts/no-product-fallback-guards.sh`, `scripts/phase13-source-guards.sh`, and `scripts/phase19-source-guards.sh` - guard patterns.

### Secondary (MEDIUM confidence)

- `https://playwright.dev/docs/api/class-electron` - official Playwright Electron automation API, fetched via web fallback after Context7/ctx7 were unavailable.
- `https://playwright.dev/docs/test-configuration` - official Playwright config concepts for timeouts/traces, fetched via web fallback.
- `https://www.electronjs.org/docs/latest/tutorial/automated-testing` - official Electron automated testing guidance, fetched via web fallback.

### Tertiary (LOW confidence)

- Assumptions A1-A4 above; each needs planner or executor validation before becoming a locked implementation decision.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - package manifests, npm registry checks, and cargo metadata verified existing versions; no new package install is recommended.
- Architecture: HIGH - project docs and current code agree on Rust-owned semantics, packaged product proof, no fallback success, and canonical `.veproj`.
- Pitfalls: MEDIUM - major risks are documented and code-backed, but exact Phase 20 long-session behavior has not been run yet.
- External docs: MEDIUM - Context7 and `ctx7` were unavailable; official web docs were used as fallback.

**Research date:** 2026-06-28 CST
**Valid until:** 2026-07-05 for package/tooling facts; codebase findings are valid until the referenced files change.
