# Phase 19: Production Effects, Retiming, And Transition Semantics - Research

**Researched:** 2026-06-25  
**Domain:** Rust-owned video editor semantics, realtime GPU preview, FFmpeg export compilation, capability reporting  
**Confidence:** HIGH for repository architecture and current code anchors; MEDIUM for external FFmpeg/wgpu/Serde/Schemars API facts

<user_constraints>
## User Constraints (from CONTEXT.md)

Source: `.planning/phases/19-production-effects-retiming-and-transition-semantics/19-CONTEXT.md` [VERIFIED: codebase grep]

### Locked Decisions

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

### Deferred Ideas (OUT OF SCOPE)

- Full proprietary Jianying/CapCut/Kaipai effect parity is deferred.
- Large preset/effect marketplace and cloud effect resource distribution are
  deferred.
- Mobile UI and store-ready mobile apps are deferred.
- Server multi-tenant rendering, auth, billing, and remote storage are deferred.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PRODFX-01 | Retiming/speed curves are typed draft semantics evaluated by engine_core and represented in render graph/audio graph without renderer-owned time math. | Current `engine_core` source mapping is linear and must be replaced/extended at `crates/engine_core/src/frame_state.rs:82` and `:264`; persisted time primitives are integer microseconds at `crates/draft_model/src/time.rs:5`. [VERIFIED: codebase grep] |
| PRODFX-02 | Transitions between adjacent visual segments have typed semantics, preview/export implementations or explicit degraded diagnostics, and undoable commands. | Current `Transition` is only `name + duration` at `crates/draft_model/src/timeline.rs:186`, render graph carries a diagnostic intent at `crates/render_graph/src/graph.rs:285`, and realtime preview marks it unsupported at `crates/realtime_preview_runtime/src/capabilities.rs:288`. [VERIFIED: codebase grep] |
| PRODFX-03 | Filters/effects use a capability registry that maps semantic effect intent to GPU preview and export/compiler implementations where supported, before implementation expands. | Current filters are string maps at `crates/draft_model/src/timeline.rs:179` and graph intent support is tri-state at `crates/render_graph/src/graph.rs:275`; this is the seam to replace with typed registry-backed semantics. [VERIFIED: codebase grep] |
| PRODFX-04 | Masks, blend modes, blur, and complex effects use production GPU preview for realtime interaction and classify unsupported export paths. | Existing mask/blend placeholders are present in `SegmentVisual`, and realtime preview already reports unsupported mask/blend diagnostics at `crates/realtime_preview_runtime/src/capabilities.rs:246`. [VERIFIED: codebase grep] |
| PRODFX-05 | Complex Jianying/Kaipai-like template fixtures verify preview/export parity, fallback reports, and performance budgets for production editing scenarios. | Template import preview/export tests already exercise reports and no-fallback evidence at `crates/testkit/tests/template_import_preview.rs:71` and `crates/testkit/tests/template_import_exports.rs:134`; Phase 19 should extend these fixtures with supported first-party effects and retiming. [VERIFIED: codebase grep] |
</phase_requirements>

## Summary

Phase 19 should be planned as a destructive contract upgrade, not a UI enablement pass. The current system already has placeholders for filters, transitions, masks, blends, keyframes, graph fingerprints, realtime capability diagnostics, adapter reports, and Rust-owned interaction sessions, but the filter/transition contracts are still stringly typed and realtime preview currently classifies the Phase 19 categories as unsupported. [VERIFIED: codebase grep]

The recommended sequence is: capability registry and typed contracts first; retiming/source-time mapping second; transitions as adjacency/overlap relationships third; narrow first-party effects/filters fourth; masks/blends after the registry and GPU/export diagnostics are real; then UI controls, Kaipai/template fixture expansion, source guards, and aggregate validation. This ordering follows the locked Phase 19 discretion and avoids exposing controls before Rust semantics, preview, export, dirty-range, and report behavior are coherent. [VERIFIED: 19-CONTEXT.md]

**Primary recommendation:** Plan Phase 19 around a small end-to-end supported slice that proves schema -> commands -> engine evaluation -> render graph -> realtime GPU preview -> FFmpeg export -> UI -> fixture parity, while all unsupported/proprietary paths remain explicit diagnostics. [VERIFIED: 19-CONTEXT.md]

## Project Constraints (from AGENTS.md)

- UI emits commands; Rust core owns project and timeline semantics, and UI code must not construct FFmpeg commands. [VERIFIED: AGENTS.md]
- Known-wrong preview/edit/render/session/media/native-surface boundaries should be replaced with production architecture instead of patched with temporary compatibility paths. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is canonical; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts. [VERIFIED: AGENTS.md]
- Product, desktop code, Rust domain types, IPC, docs, schema, and tests should prefer Jianying vocabulary such as draft/material/track/segment/keyframe/filter/transition. [VERIFIED: AGENTS.md]
- Persisted semantic time math must use integer microseconds, frame indices, or rational frame rates, not naked floating-point time. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are conceptual references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md]
- External drafts go through adapters and compatibility reports; proprietary IDs are external references, not internal render semantics. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution must be reviewed for LGPL/GPL/nonfree build options, notices, and commercial product obligations before external distribution. [VERIFIED: AGENTS.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Capability registry and support matrix | Rust semantic/runtime core | Electron display only | Registry must map semantic intent to GPU/export/report support below adapters; UI only renders statuses. [VERIFIED: 19-CONTEXT.md] |
| Retiming and source-to-target mapping | `draft_model` + `engine_core` | `render_graph`, audio graph, compiler | Current source mapping is computed in engine core and must remain Rust-owned for preview/export parity. [VERIFIED: codebase grep] |
| Transition adjacency/overlap validation | `draft_commands` | `render_graph`, preview/export | Timeline commands already centralize validation and undo; renderer-generated transition deltas are disallowed. [VERIFIED: 19-CONTEXT.md] |
| GPU effect/mask/blend preview | `realtime_preview_runtime` | `preview_service` scheduling/cache | Preview support is classified in Rust and product success requires realtime GPU compositor evidence. [VERIFIED: docs/no-product-fallback-policy.md] |
| FFmpeg effect/retime/transition export | `ffmpeg_compiler` | `media_runtime` process execution | FFmpeg filter docs provide backend primitives, but compiler owns filtergraph generation; runtime only executes. [CITED: https://ffmpeg.org/ffmpeg-filters.html] |
| High-frequency effect/speed/transition manipulation | `editor_runtime` interaction sessions | Renderer ghost/proxy only | Existing sessions track generation, accepted sequence, provisional drafts, commit/cancel. [VERIFIED: codebase grep] |
| Kaipai/Jianying compatibility mapping | `adapter_kaipai` + `draft_import` | `testkit` fixtures | Adapter already maps supported generic concepts and reports native effects as needsNativeEffect/dropped. [VERIFIED: codebase grep] |

## Standard Stack

### Core

| Library / Crate | Version | Purpose | Why Standard |
|-----------------|---------|---------|--------------|
| Workspace crates `draft_model`, `draft_commands`, `engine_core`, `render_graph`, `ffmpeg_compiler`, `realtime_preview_runtime`, `preview_service`, `editor_runtime`, `adapter_kaipai`, `testkit` | `0.1.0` workspace crates | Own persisted semantics, commands, frame evaluation, graph intent, compiler, GPU preview, sessions, adapters, and fixtures | These crates are the locked ownership boundaries for Phase 19. [VERIFIED: 19-CONTEXT.md] |
| `serde` | 1.0.228; created 2014-12-05; current crate OK | Serialize persisted draft/contracts | Existing crate is pinned across semantic crates and official docs support tagged enum representations. [VERIFIED: crates.io] [CITED: https://serde.rs/enum-representations.html] |
| `schemars` | 1.2.1; created 2019-08-08; current crate OK | JSON schema generation for `.veproj` and command contracts | Existing schema export tests use generated schemas; docs support `JsonSchema` derive. [VERIFIED: crates.io] [CITED: https://docs.rs/schemars/1.2.1/schemars/] |
| `ts-rs` | 12.0.1; created 2020-12-15; current crate OK | Generated TypeScript contracts from Rust types | Existing `draft_model` schema export tests generate desktop TS contracts. [VERIFIED: crates.io] |
| `wgpu` | 29.0.3; created 2019-01-24; current crate OK | Realtime GPU preview compositor/effect pipelines | Existing runtime depends on `wgpu`; docs define render pipeline, bind group, texture, and sampler APIs needed for effect passes. [VERIFIED: crates.io] [CITED: https://docs.rs/wgpu/29.0.3/wgpu/] |
| FFmpeg/ffprobe | Local PATH 8.1.2; bundled runtime also provisioned by desktop build | Export compiler target and fixture media generation | Official filter docs cover `setpts`, `asetpts`, `atempo`, `xfade`, `gblur`, `overlay`, and `blend` primitives for compiler backends only. [VERIFIED: local command] [CITED: https://ffmpeg.org/ffmpeg-filters.html] |

### Supporting

| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| Electron | Repo pinned 42.4.1; latest 42.5.0 published 2026-06-23 | Desktop shell and native preview host | Keep pinned for Phase 19 unless a separate dependency plan handles upgrade risk. [VERIFIED: npm registry] |
| React | Repo pinned 19.2.7; latest 19.2.7 published 2026-06-01 | Renderer UI controls | Use existing UI patterns for FeaturePanel, Inspector, Timeline, and PreviewMonitor. [VERIFIED: npm registry] |
| Playwright | Repo pinned `@playwright/test` 1.61.0; latest 1.61.1 published 2026-06-23 | Product E2E and UI regression gates | Extend existing desktop tests for visible Phase 19 controls and no-fallback evidence. [VERIFIED: npm registry] |
| `@napi-rs/cli` | Repo pinned 3.7.2; latest 3.7.2 published 2026-06-14 | Build Node-API binding | Keep `bindings_node` thin over `editor_runtime`. [VERIFIED: npm registry] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Existing Rust capability registry inside workspace | New external effects package | Do not add external packages; registry must encode first-party semantics and adapter reports. [VERIFIED: 19-CONTEXT.md] |
| `wgpu` compositor passes | DOM/CSS/canvas overlay effects | Product success cannot be DOM/mock/artifact evidence and UI must not own render semantics. [VERIFIED: docs/no-product-fallback-policy.md] |
| `ffmpeg_compiler` filtergraph output | Renderer-built FFmpeg strings | UI-owned FFmpeg construction violates AGENTS.md and source guards. [VERIFIED: AGENTS.md] |

**Installation:** No new external package installation is recommended for Phase 19. [VERIFIED: codebase grep]

```bash
# Keep existing lockfiles. Do not add packages for Phase 19 unless a plan explicitly changes scope.
pnpm install --frozen-lockfile
cargo check --workspace --locked
```

**Version verification:** Ran `npm view`, `cargo search`, `cargo info`, `cargo metadata`, and local `node/pnpm/cargo/rustc/ffmpeg/ffprobe --version` probes on 2026-06-25. [VERIFIED: npm registry] [VERIFIED: crates.io] [VERIFIED: local command]

## Package Legitimacy Audit

> Phase 19 research recommends no new external package installation. The table below records informational checks for the existing pinned stack so planners do not upgrade accidentally. [VERIFIED: codebase grep]

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `wgpu` | crates | since 2019-01-24 | 544,810/wk | `github.com/gfx-rs/wgpu` | OK | Approved as existing pinned dependency. [VERIFIED: crates.io] |
| `serde` | crates | since 2014-12-05 | 16,761,904/wk | `github.com/serde-rs/serde` | OK | Approved as existing pinned dependency. [VERIFIED: crates.io] |
| `schemars` | crates | since 2019-08-08 | 8,457,436/wk | `github.com/GREsau/schemars` | OK | Approved as existing pinned dependency. [VERIFIED: crates.io] |
| `ts-rs` | crates | since 2020-12-15 | 285,591/wk | `github.com/Aleph-Alpha/ts-rs` | OK | Approved as existing pinned dependency. [VERIFIED: crates.io] |
| `electron` | npm | since 2012-05-18; pinned 42.4.1 published 2026-06-16 | 4,746,073/wk | `github.com/electron/electron` | SUS on latest due `too-new` | Do not upgrade/install new Electron in Phase 19; use lockfile. [VERIFIED: npm registry] |
| `@playwright/test` | npm | since 2020-09-24; pinned 1.61.0 published 2026-06-15 | 42,384,285/wk | `github.com/microsoft/playwright` | SUS on latest due `too-new` | Do not upgrade/install new Playwright in Phase 19; use lockfile. [VERIFIED: npm registry] |
| `react` | npm | since 2011-10-26; pinned/latest 19.2.7 published 2026-06-01 | 150,302,852/wk | `github.com/facebook/react` | SUS on latest due `too-new` | Existing dependency only; no Phase 19 install. [VERIFIED: npm registry] |
| `@napi-rs/cli` | npm | since 2020-11-09; pinned/latest 3.7.2 published 2026-06-14 | 1,115,825/wk | `github.com/napi-rs/napi-rs` | SUS on latest due `too-new` | Existing dependency only; no Phase 19 install. [VERIFIED: npm registry] |

**Packages removed due to [SLOP] verdict:** none. [VERIFIED: package-legitimacy seam]  
**Packages flagged as suspicious [SUS]:** latest npm releases above; planner should add a human verification checkpoint only if it changes package versions. [VERIFIED: package-legitimacy seam]

## Architecture Patterns

### System Architecture Diagram

```text
User command / interaction
  -> Electron renderer (visible controls, ghost/proxy only)
  -> preload/main IPC validation
  -> bindings_node thin JSON transport
  -> editor_runtime project session
      -> draft_commands validate/commit undoable semantics
      -> draft_model persisted typed contract in .veproj/project.json
      -> engine_core normalize + source-time/effect/keyframe evaluation
      -> render_graph typed intents + fingerprints + dirty ranges
          -> realtime_preview_runtime capability classifier + GPU compositor
          -> ffmpeg_compiler export filtergraph/job generation
          -> preview_service / task_runtime scheduling, cache, telemetry
      -> media_runtime executes FFmpeg/export jobs
  -> Electron renderer displays state/diagnostics, never semantic success fallback

External Kaipai/Jianying-like fixture
  -> adapter_kaipai maps supported concepts to first-party semantics
  -> draft_import applies plan and report
  -> unsupported/private IDs remain external refs in report, not render semantics
```

The current Node adapter already delegates project-session calls to `editor_runtime` at `crates/bindings_node/src/project_session_service.rs:7`, so Phase 19 should add semantic services below this adapter rather than duplicating logic in N-API or Electron. [VERIFIED: codebase grep]

### Recommended Project Structure

```text
crates/
├── draft_model/src/effects.rs              # new typed capability/effect/filter/transition/retime contracts
├── draft_commands/src/effects.rs           # effect/filter/mask/blend commands
├── draft_commands/src/retiming.rs          # speed/curve commands and validation
├── draft_commands/src/transition.rs        # adjacency/overlap transition commands
├── engine_core/src/time_mapping.rs         # deterministic source<->target mapping
├── render_graph/src/effects.rs             # registry-backed render intents and fingerprints
├── realtime_preview_runtime/src/effects.rs # GPU support classification and passes
├── ffmpeg_compiler/src/effects.rs          # export compiler support/degraded diagnostics
├── editor_runtime/src/project_session_node.rs # project intents and interaction payloads
├── adapter_kaipai/src/mapper.rs            # map only supported external refs into first-party semantics
└── testkit/tests/production_effects_*.rs   # preview/export/template parity fixtures
```

Exact filenames are planner discretion, but these module boundaries preserve the locked crate ownership map. [VERIFIED: 19-CONTEXT.md]

### Pattern 1: Typed Semantic Contract Before Backend Implementation

**What:** Replace `Filter { name, parameters: BTreeMap<String, String> }` and `Transition { name, duration }` with tagged first-party variants plus external-reference/report-only variants. [VERIFIED: codebase grep]

**When to use:** Use this before adding any UI controls, GPU shader pass, or FFmpeg filter compiler branch. [VERIFIED: 19-CONTEXT.md]

**Example:**
```rust
// Source pattern: existing tagged keyframe values in crates/draft_model/src/timeline.rs:155.
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VisualEffectKind {
    GaussianBlur { radius_millis: u32 },
    ColorAdjustment { brightness_millis: i32, contrast_millis: i32 },
    ExternalReference { provider: String, external_id: String },
}
```

Serde tagged enums are an official supported enum representation and Schemars can derive schemas from Rust types while respecting Serde attributes. [CITED: https://serde.rs/enum-representations.html] [CITED: https://docs.rs/schemars/1.2.1/schemars/]

### Pattern 2: Engine-Owned Time Mapping

**What:** Add a retiming/time-map abstraction that `engine_core` uses for frame state and render ranges before render graph or compiler layers consume source positions. [VERIFIED: codebase grep]

**When to use:** Use for constant speed, reverse/curve diagnostics, split/trim/move validation, transition overlap windows, and audio follow-speed policy. [VERIFIED: 19-CONTEXT.md]

**Example:**
```rust
// Source anchor: current linear mapping lives in crates/engine_core/src/frame_state.rs:264.
pub struct SegmentTimeMap {
    pub target_range: TargetTimerange,
    pub source_range: SourceTimerange,
    pub speed: SegmentSpeed,
}

impl SegmentTimeMap {
    pub fn source_at(&self, target_at: Microseconds) -> Result<Microseconds, EngineError> {
        // Use integer/rational math only; no persisted floating-point seconds.
        todo!("planner should task constant-speed first, curve/reverse as diagnostics if unsupported")
    }
}
```

### Pattern 3: Capability Registry Drives Both Preview And Export

**What:** A first-party registry should answer whether a semantic intent is supported, degraded, unsupported, or external/proprietary for GPU preview and export/compiler separately. [VERIFIED: 19-CONTEXT.md]

**When to use:** Use every time a new effect, filter, transition, mask, blend, or retime mode is introduced. [VERIFIED: 19-CONTEXT.md]

**Example:**
```rust
// Source anchor: RenderIntentSupport already has Supported/Degraded/Unsupported at
// crates/render_graph/src/graph.rs:295.
pub struct CapabilityDecision {
    pub semantic_id: String,
    pub preview_support: RenderIntentSupport,
    pub export_support: RenderIntentSupport,
    pub reason: String,
}
```

### Pattern 4: High-Frequency Interaction Sessions

**What:** Use `ProjectInteractionSession` for speed handles, effect sliders, mask handles, and transition duration changes; updates produce provisional drafts and commit/cancel determines persistence. [VERIFIED: codebase grep]

**When to use:** Use for drag/slider/scrub controls that would otherwise save or push undo on every pointer move. [VERIFIED: 19-CONTEXT.md]

**Example:**
```rust
// Source anchor: existing interaction session sequence gate at crates/draft_model/src/interaction.rs:46.
session.accept_sequence(sequence)?;
active.latest_payload = Some(payload);
active.provisional_draft = Some(provisional.draft);
```

### Anti-Patterns to Avoid

- **Stringly typed permanent effects:** Keeping generic `name` and `BTreeMap<String, String>` as first-party semantics prevents schema validation, capability reporting, and typed UI. [VERIFIED: codebase grep]
- **FFmpeg-first retiming:** Implementing `setpts`/`atempo` only in `ffmpeg_compiler` would leave preview, engine frame state, dirty ranges, and audio graph wrong. [VERIFIED: codebase grep] [CITED: https://ffmpeg.org/ffmpeg-filters.html]
- **Transition as a single-segment decoration:** Phase 19 requires adjacent/overlap relationships with edit validation, not only `segment.transition`. [VERIFIED: 19-CONTEXT.md]
- **Renderer-visible unsupported controls:** Existing UI tests expect unsupported effect/transition/filter/adjustment categories to stay unavailable until production backing exists. [VERIFIED: codebase grep]
- **Adapter-native IDs as semantics:** `adapter_kaipai` tests already reject provider-native effects entering canonical filters. [VERIFIED: codebase grep]

## Recommended Plan Sequence

| Wave | Scope | Main Files / Crates | Gate Focus |
|------|-------|---------------------|------------|
| 0 | Validation/source-guard scaffolding and UI-SPEC | `package.json`, `scripts/phase19-source-guards.sh`, `19-UI-SPEC.md`, test placeholders | Fail renderer-owned time/effect/FFmpeg/cache logic and package `test:phase19`. [VERIFIED: codebase grep] |
| 1 | Capability registry + typed contracts | `draft_model`, schema exports, generated TS, `render_graph` intent support | Replace string placeholders with typed support matrix and migration tests. [VERIFIED: codebase grep] |
| 2 | Retiming/speed semantics | `draft_model`, `draft_commands`, `engine_core`, audio graph/render graph/compiler | Constant speed first; curve/reverse/pitch combos diagnostic unless implemented. [VERIFIED: 19-CONTEXT.md] |
| 3 | Transition semantics | `draft_commands`, `render_graph`, `realtime_preview_runtime`, `ffmpeg_compiler` | Adjacency/overlap validation, undoable commands, dissolve/crossfade preview/export parity. [VERIFIED: 19-CONTEXT.md] |
| 4 | First-party effects/filters | `render_graph`, `realtime_preview_runtime`, `ffmpeg_compiler`, `testkit` | Narrow supported slice such as opacity/blur/color adjustment with supported/degraded/unsupported reports. [VERIFIED: 19-CONTEXT.md] |
| 5 | Masks/blends and complex diagnostics | `draft_model`, `realtime_preview_runtime`, `ffmpeg_compiler` | Supported simple mask/blend only if GPU/export gates exist; otherwise explicit unsupported reports. [VERIFIED: 19-CONTEXT.md] |
| 6 | UI integration | `FeaturePanel.tsx`, `Inspector.tsx`, `Timeline.tsx`, `PreviewMonitor.tsx` | Visible controls use project interactions and product preview/export evidence. [VERIFIED: codebase grep] |
| 7 | Template fidelity and aggregate validation | `adapter_kaipai`, `testkit`, Playwright, docs | Kaipai-like fixtures prove local semantics without proprietary parity. [VERIFIED: codebase grep] |

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Persisted effect schemas | Ad hoc strings/maps | Serde tagged enums + Schemars + ts-rs exports | Existing contract tests already enforce schema/TS drift. [VERIFIED: codebase grep] |
| GPU preview effects | DOM overlays or CSS filters as product evidence | `wgpu` compositor passes in `realtime_preview_runtime` | Product success must be GPU composited, not DOM/mock/artifact evidence. [VERIFIED: docs/no-product-fallback-policy.md] |
| Export filtergraphs | UI or adapter-built FFmpeg strings | `ffmpeg_compiler` | AGENTS.md and source guards require Rust compiler ownership. [VERIFIED: AGENTS.md] |
| Time mapping | Renderer math or floating seconds | `engine_core` integer/rational time-map APIs | Persisted semantic time must be integer microseconds/frame/rational. [VERIFIED: AGENTS.md] |
| Cache invalidation | Renderer cache keys or effect heuristics | `render_graph` fingerprints and dirty ranges | Existing render graph snapshots fingerprint filters/transitions and dirty domains. [VERIFIED: codebase grep] |
| Provider-native effect support | Direct Kaipai/Jianying IDs in render graph | Adapter reports plus first-party capability registry | Existing adapter tests require native effects to stay report-only. [VERIFIED: codebase grep] |
| High-frequency updates | Mousemove -> save/revision/undo loop | Rust project interaction sessions | Existing sessions coalesce provisional updates and commit/cancel deterministically. [VERIFIED: codebase grep] |

**Key insight:** Phase 19 complexity is not the first blur shader or dissolve filter; it is keeping one semantic model coherent across persisted drafts, engine evaluation, preview, export, cache, adapters, UI, and tests. [VERIFIED: 19-CONTEXT.md]

## Runtime State Inventory

> Included because Phase 19 may perform destructive semantic/schema refactors. [VERIFIED: 19-CONTEXT.md]

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `.veproj/project.json` persists current `Filter`, `Transition`, `SegmentMask`, `SegmentBlendMode`, `Keyframe`, and audio effect slots through `draft_model` schema. [VERIFIED: codebase grep] | Add schema migration/compat tests for old placeholder filters/transitions or explicitly reject unsupported old semantics with diagnostics. |
| Live service config | No external live service config is in scope; adapters operate from offline bundles and reports. [VERIFIED: 19-CONTEXT.md] | None, but fixture reports must remain deterministic. |
| OS-registered state | No OS-registered Phase 19 state was found in project docs or phase context. [VERIFIED: codebase grep] | None. |
| Secrets/env vars | No Phase 19 secret/env-var dependency was found; tests use command/runtime env flags for diagnostics and fixtures. [VERIFIED: codebase grep] | Do not add secrets; keep env flags diagnostic/test-only. |
| Build artifacts | Generated schemas and TS contracts live under `schemas` and `apps/desktop-electron/src/generated`; native binding artifacts live in `apps/desktop-electron/native`; derived artifacts/cache are not canonical. [VERIFIED: codebase grep] | Regenerate contracts after schema changes; `pnpm run test:contracts` must be clean; rebuild native binding for desktop E2E. |

**Nothing found in category:** OS-registered state and secrets/env vars have no Phase 19 runtime migration item. [VERIFIED: codebase grep]

## Common Pitfalls

### Pitfall 1: Treating Retiming As Export-Only

**What goes wrong:** FFmpeg export duration changes while preview, engine frame state, keyframes, audio graph, and dirty ranges still use linear source offsets. [VERIFIED: codebase grep]  
**Why it happens:** Current `source_position_at` is `source.start + target offset` in `engine_core`. [VERIFIED: codebase grep]  
**How to avoid:** Add a time-map contract first, then feed render graph/audio/compiler from it. [VERIFIED: 19-CONTEXT.md]  
**Warning signs:** `setpts`, `atempo`, or `durationMsWithSpeed` appears in renderer, adapter semantics, or compiler before engine tests exist. [VERIFIED: codebase grep]

### Pitfall 2: Capability Registry Arrives After Effects

**What goes wrong:** Each effect adds bespoke preview/export logic and diagnostics drift. [ASSUMED]  
**Why it happens:** Current filter/transition support is spread across draft placeholders, render graph intents, and realtime classifier diagnostics. [VERIFIED: codebase grep]  
**How to avoid:** Implement registry contracts and tests before enabling the first supported effect. [VERIFIED: 19-CONTEXT.md]  
**Warning signs:** New effect kinds bypass `RenderIntentSupport`, realtime diagnostics, or export diagnostics. [VERIFIED: codebase grep]

### Pitfall 3: Transition Validation Ignores Editing Commands

**What goes wrong:** Split/trim/move/snapping/main-track magnet can create impossible transition windows. [VERIFIED: 19-CONTEXT.md]  
**Why it happens:** A segment-local `Transition` cannot fully model adjacency/overlap relationships. [VERIFIED: codebase grep]  
**How to avoid:** Add transition relationship validation inside `draft_commands` and update command deltas/undo tests. [VERIFIED: 19-CONTEXT.md]  
**Warning signs:** UI creates transition deltas directly or transition duration changes do not run `validate_timeline_rules`. [VERIFIED: codebase grep]

### Pitfall 4: Cache Fingerprints Miss New Semantics

**What goes wrong:** Preview/export cache may reuse stale frames after effect/retime/transition changes. [VERIFIED: 19-CONTEXT.md]  
**Why it happens:** Fingerprints already include current filters/transitions, but new typed semantics must be included in all semantic/input/runtime capability fingerprints. [VERIFIED: codebase grep]  
**How to avoid:** Extend `RenderGraphSnapshot` and dirty domain tests with each new semantic field. [VERIFIED: codebase grep]  
**Warning signs:** Effect or speed fields change without `render_graph` fingerprint snapshot changes. [VERIFIED: codebase grep]

### Pitfall 5: Product UI Exposes Unsupported Controls

**What goes wrong:** Users can click effect/transition/filter/mask controls that only produce diagnostics or fallback. [VERIFIED: docs/product-e2e-acceptance-policy.md]  
**Why it happens:** UI categories are already visible/reserved, while unsupported controls are hidden/gated. [VERIFIED: codebase grep]  
**How to avoid:** Add UI controls only after Rust preview/export support and product E2E evidence. [VERIFIED: docs/product-e2e-acceptance-policy.md]  
**Warning signs:** FeaturePanel/Inspector text says an effect is usable before no-fallback preview/export tests exist. [VERIFIED: codebase grep]

## Code Examples

Verified patterns from existing sources:

### Existing Typed Keyframe Contract

```rust
// Source: crates/draft_model/src/timeline.rs:120
pub struct Keyframe {
    pub at: Microseconds,
    pub property: KeyframeProperty,
    pub value: KeyframeValue,
    pub interpolation: KeyframeInterpolation,
    pub easing: KeyframeEasing,
}
```

Use this style for retime/effect/transition parameters: typed fields, integer microseconds/millis, generated schema/TS, and command tests. [VERIFIED: codebase grep]

### Existing Realtime Unsupported Diagnostic

```rust
// Source: crates/realtime_preview_runtime/src/capabilities.rs:276
for filter in &layer.filters {
    diagnostics.push(RealtimePreviewDiagnostic::new(
        Some(layer.segment_id.as_str().to_owned()),
        RealtimePreviewDiagnosticDomain::Effect,
        RealtimePreviewSupport::Unsupported {
            reason: format!("filter {} is unsupported in realtime preview", filter.name),
        },
        format!("filter {} is unsupported in realtime preview", filter.name),
        None,
        true,
    ));
}
```

Phase 19 should convert specific first-party filter/effect variants from unsupported to supported only when registry, GPU preview, export, and tests exist. [VERIFIED: codebase grep]

### Existing Render Graph Fingerprint Inclusion

```rust
// Source: crates/render_graph/src/fingerprint.rs:222
fn video_layer_fingerprint(
    layer: &RenderVideoLayer,
    output_profile_fingerprint: &str,
    runtime_capability_fingerprint: &str,
) -> RenderGraphNodeFingerprint {
    fingerprint_parts(
        layer.node_id.clone(),
        &VideoLayerSemanticInput {
            stack_index: layer.stack_index,
            source_timerange: &layer.source_timerange,
            target_timerange: &layer.target_timerange,
            keyframes: &layer.keyframes,
            filters: &layer.filters,
            transition: layer.transition.as_ref(),
            visual: &layer.visual,
        },
```

Add retiming, effect parameters, masks, blends, and transition windows to the correct semantic fingerprint inputs when their contracts change. [VERIFIED: codebase grep]

### Existing Project Interaction Route

```rust
// Source: crates/editor_runtime/src/project_session_node.rs:1010
pub fn begin_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<BeginProjectInteractionRequest>(request) {
        Ok(request) => request,
        Err(error) => { /* error envelope */ }
    };
    with_project_session_registry(|registry| registry.begin_interaction(request))
}
```

Use this route for speed handles, effect sliders, mask handles, and transition duration controls. [VERIFIED: codebase grep]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Renderer or UI-owned edit math | Rust-owned draft commands and project sessions | Existing by Phase 17.1/18 | Phase 19 UI must use intents/interactions, not direct state mutation. [VERIFIED: codebase grep] |
| Preview artifact/CPU/DOM evidence | `renderGraphGpuComposited` product evidence only | Existing by no-product-fallback policy and Phase 15.2+ gates | Effects/transitions cannot pass through artifact fallback. [VERIFIED: docs/no-product-fallback-policy.md] |
| Placeholder effect/transition strings | Typed first-party capability registry required | Locked for Phase 19 | Plan must migrate/replace placeholders before widening support. [VERIFIED: 19-CONTEXT.md] |
| Provider-native effect passthrough | Adapter report-only `NeedsNativeEffect`/`Dropped` diagnostics | Existing by Phase 17 adapter tests | Kaipai private IDs stay external references. [VERIFIED: codebase grep] |
| Full graph rebuild acceptance | Stable node IDs, fingerprints, dirty ranges | Existing by Phase 13 | New semantics must participate in dirty facts/cache keys. [VERIFIED: codebase grep] |

**Deprecated/outdated:**
- Generic `Filter { name, parameters }` as first-party production semantics is inadequate for Phase 19 planning. [VERIFIED: codebase grep]
- Segment-local `Transition { name, duration }` is inadequate for adjacent/overlap transition semantics. [VERIFIED: codebase grep] [VERIFIED: 19-CONTEXT.md]
- FFmpeg artifact or preview artifact evidence is not product success. [VERIFIED: docs/no-product-fallback-policy.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | If exact first supported effects are not locked by the user, the planner should choose a narrow slice such as constant speed, dissolve/crossfade, blur, and basic color/opacity primitives. | Recommended Plan Sequence | Scope may need user confirmation before UI/API names are locked. |
| A2 | A capability registry can live inside existing workspace crates instead of a new crate. | Recommended Project Structure | If compile boundaries become cyclic, planner may need a small dedicated internal crate. |
| A3 | The exact performance budgets for Phase 19 should be derived from Phase 16/17 telemetry rather than invented in research. | Validation Architecture | Planner must set measurable thresholds during planning. |

## Open Questions

1. **Exact supported slice**
   - What we know: context recommends narrow real first-party support such as opacity/blur/basic color/filter primitives and one or more simple transitions. [VERIFIED: 19-CONTEXT.md]
   - What's unclear: exact product-visible effect/transition names and parameter ranges are not locked. [VERIFIED: 19-CONTEXT.md]
   - Recommendation: planner should pick the smallest complete slice and record deferred variants explicitly.

2. **Retiming breadth**
   - What we know: constant speed, reverse playback, curve speed, and audio follow-speed boundaries were in archived scope. [VERIFIED: ROADMAP_PHASES_11_13_ARCHIVE.md]
   - What's unclear: whether Phase 19 must support reverse/curve/pitch correction or only report them as degraded/unsupported. [VERIFIED: ROADMAP_PHASES_11_13_ARCHIVE.md]
   - Recommendation: implement constant-speed source mapping first and classify reverse/curve/pitch combinations unless scoped in a later wave.

3. **UI-SPEC granularity**
   - What we know: visible control changes require UI-SPEC and independent UI audit. [VERIFIED: 19-CONTEXT.md]
   - What's unclear: whether the first UI wave should expose resource panel presets, inspector controls, timeline handles, or only diagnostics. [VERIFIED: 19-CONTEXT.md]
   - Recommendation: gate each visible control behind a completed Rust semantic and product E2E case.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | Electron/Playwright/build scripts | yes | v24.15.0 | Project engine is 24.12.0; current is newer patch. [VERIFIED: local command] |
| pnpm | Workspace scripts | yes | 10.32.1 | None needed. [VERIFIED: local command] |
| Corepack | Package manager mediation | yes | 0.34.6 | Direct pnpm available. [VERIFIED: local command] |
| Cargo | Rust tests/builds | yes | 1.95.0 | None needed. [VERIFIED: local command] |
| rustc | Rust tests/builds | yes | 1.95.0 | None needed. [VERIFIED: local command] |
| FFmpeg | export tests and fixture generation | yes | 8.1.2 on PATH | Desktop build also provisions bundled runtime. [VERIFIED: local command] |
| ffprobe | output validation and material probes | yes | 8.1.2 on PATH | Desktop bundled runtime path for packaged app. [VERIFIED: local command] |
| ripgrep | source guards | yes | 15.1.0 | `grep` if unavailable, but current machine has `rg`. [VERIFIED: local command] |
| Docker | not required by Phase 19 | no | unavailable | Not needed. [VERIFIED: local command] |
| ctx7 | optional docs lookup | no | unavailable | Used websearch/official URLs plus repo docs. [VERIFIED: local command] |

**Missing dependencies with no fallback:** none for planned research and local validation. [VERIFIED: local command]  
**Missing dependencies with fallback:** ctx7 is absent; official docs were checked through websearch/URLs. [VERIFIED: local command]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` 1.95.0, Playwright `@playwright/test` 1.61.0, bash source guards, schema/TS contract diff. [VERIFIED: local command] |
| Config file | Root `package.json`, `Cargo.toml`, `apps/desktop-electron/playwright.config.ts`. [VERIFIED: codebase grep] |
| Quick run command | `pnpm run test:phase19-rust` after Wave 0 creates it. [ASSUMED] |
| Full suite command | `pnpm run test:phase19` after Wave 0 creates it. [ASSUMED] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| PRODFX-01 | Retiming source mapping, split/trim/move validation, audio follow-speed diagnostics, render graph/audio graph representation | Rust unit/integration + export parity | `cargo test -p draft_model retiming && cargo test -p draft_commands retiming && cargo test -p engine_core retiming && cargo test -p render_graph retiming && cargo test -p ffmpeg_compiler retiming` | No; Wave 0/1 gap. [VERIFIED: codebase grep] |
| PRODFX-02 | Transition adjacency/overlap semantics, undoable commands, preview/export supported or degraded diagnostics | Rust + Playwright if visible handles | `cargo test -p draft_commands transition && cargo test -p render_graph transition && cargo test -p realtime_preview_runtime transition && cargo test -p ffmpeg_compiler transition` | Partial placeholders only; Wave gap. [VERIFIED: codebase grep] |
| PRODFX-03 | Capability registry maps semantic intent to preview/export support | Rust unit + schema/contract | `cargo test -p draft_model capability && cargo test -p render_graph capability && cargo test -p realtime_preview_runtime capability_matrix` | Partial realtime matrix exists. [VERIFIED: codebase grep] |
| PRODFX-04 | Masks/blends/blur/complex effects use GPU preview where supported and classify export unsupported paths | Rust GPU/offscreen + compiler diagnostics + Playwright if visible | `cargo test -p realtime_preview_runtime effects && cargo test -p ffmpeg_compiler effects && pnpm --filter @video-editor/desktop test:workspace -g "特效|滤镜|转场|蒙版|混合"` | No Phase 19 files yet. [VERIFIED: codebase grep] |
| PRODFX-05 | Complex Kaipai-like fixtures verify preview/export parity, fallback reports, performance budgets | `testkit` + Playwright template import | `cargo test -p testkit template_import_preview template_import_exports && pnpm --filter @video-editor/desktop exec playwright test tests/template-import.spec.ts --reporter=line` | Existing files need Phase 19 cases. [VERIFIED: codebase grep] |

### Sampling Rate

- **Per task commit:** narrow crate test plus `pnpm run test:contracts` when schemas or generated TS change. [VERIFIED: codebase grep]
- **Per wave merge:** `pnpm run test:phase19-rust`, `pnpm run test:phase19-source-guards`, `pnpm run test:no-product-fallback`, and relevant Playwright tests. [ASSUMED]
- **Phase gate:** `pnpm run test:phase19 && pnpm run test:no-product-fallback && cargo check --workspace --locked && pnpm run test:contracts`. [ASSUMED]

### Wave 0 Gaps

- [ ] `scripts/phase19-source-guards.sh` - block renderer-owned retiming/effect evaluation, FFmpeg filter construction, cache/fingerprint/dirty logic, fallback success, provider-native ID semantics, and mousemove save/revision loops. [ASSUMED]
- [ ] Root `package.json` scripts: `test:phase19-rust`, `test:phase19-source-guards`, `test:phase19-desktop`, `test:phase19`. [ASSUMED]
- [ ] `19-UI-SPEC.md` before visible controls change. [VERIFIED: 19-CONTEXT.md]
- [ ] Rust tests for `retiming`, `effect_capability_registry`, `transition_semantics`, `production_effects_preview_export`, and `template_import_production_effects`. [ASSUMED]
- [ ] Playwright E2E for visible speed/effect/transition controls once enabled. [VERIFIED: docs/product-e2e-acceptance-policy.md]
- [ ] Independent UI audit after UI implementation. [VERIFIED: 19-CONTEXT.md]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Desktop/offline Phase 19 does not introduce auth. [VERIFIED: 19-CONTEXT.md] |
| V3 Session Management | partial | Runtime/project sessions are local Rust-owned IDs with revision/generation checks; do not expose semantic state ownership to UI. [VERIFIED: codebase grep] |
| V4 Access Control | partial | File/import/export paths must remain under project/session/runtime services and not be built by renderer. [VERIFIED: docs/runtime-boundaries.md] |
| V5 Input Validation | yes | Use Serde `deny_unknown_fields`, Schemars schema tests, command validation, and adapter report validation. [VERIFIED: codebase grep] |
| V6 Cryptography | no new crypto | Resource SHA checks already exist in template localization options; Phase 19 should not add crypto. [VERIFIED: codebase grep] |

### Known Threat Patterns for Video Editor Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| FFmpeg command injection through effect parameters | Tampering | Typed compiler-owned filter generation; no UI strings. [VERIFIED: AGENTS.md] |
| Path traversal in imported template resources | Tampering | Keep adapter/resource localization and project-store path boundaries; do not let renderer resolve resources. [VERIFIED: codebase grep] |
| Proprietary/native IDs treated as supported semantics | Tampering/Repudiation | Keep external refs in adapter reports and classify unsupported/degraded explicitly. [VERIFIED: codebase grep] |
| Fallback/mock/artifact evidence counted as success | Repudiation | `pnpm run test:no-product-fallback` and product E2E visible/export assertions. [VERIFIED: docs/no-product-fallback-policy.md] |
| High-frequency drag causing save/undo denial of service | Denial of Service | Rust interaction sessions with coalesced provisional updates and commit/cancel. [VERIFIED: codebase grep] |

## Sources

### Primary (HIGH confidence)

- `AGENTS.md` - project architecture, no fallback, canonical project format, time model, render boundary. [VERIFIED: codebase grep]
- `.planning/phases/19-production-effects-retiming-and-transition-semantics/19-CONTEXT.md` - locked Phase 19 decisions, sequence, constraints. [VERIFIED: codebase grep]
- `.planning/ROADMAP.md` and `.planning/REQUIREMENTS.md` - Phase 19 scope and PRODFX-01..PRODFX-05. [VERIFIED: codebase grep]
- `docs/no-product-fallback-policy.md`, `docs/product-e2e-acceptance-policy.md`, `docs/refactor-and-legacy-cleanup-policy.md`, `docs/runtime-boundaries.md` - mandatory gates and runtime ownership. [VERIFIED: codebase grep]
- Current code anchors in `crates/draft_model`, `draft_commands`, `engine_core`, `render_graph`, `ffmpeg_compiler`, `realtime_preview_runtime`, `editor_runtime`, `bindings_node`, `adapter_kaipai`, and `testkit`. [VERIFIED: codebase grep]

### Secondary (MEDIUM confidence)

- FFmpeg official filter documentation - filtergraph, timestamp, transition, blur, overlay, and blend primitives for compiler backend planning. [CITED: https://ffmpeg.org/ffmpeg-filters.html]
- wgpu docs.rs 29.0.3 - render pipeline, bind group, texture/sampler APIs for compositor passes. [CITED: https://docs.rs/wgpu/29.0.3/wgpu/]
- Serde enum representation docs - tagged enum contract patterns. [CITED: https://serde.rs/enum-representations.html]
- Schemars docs.rs 1.2.1 - `JsonSchema` derive and schema generation. [CITED: https://docs.rs/schemars/1.2.1/schemars/]
- npm registry and crates.io checks for existing pinned stack versions and legitimacy. [VERIFIED: npm registry] [VERIFIED: crates.io]

### Tertiary (LOW confidence)

- No external production editor architecture source was adopted; repo architecture is authoritative for this phase. [VERIFIED: 19-CONTEXT.md]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH for existing repo dependencies and local versions; MEDIUM for current registry recency because npm latest releases were flagged too-new. [VERIFIED: npm registry]
- Architecture: HIGH because locked context, AGENTS.md, runtime-boundary docs, and code anchors agree. [VERIFIED: codebase grep]
- Pitfalls: HIGH for repo-specific wrong states; MEDIUM for external FFmpeg backend constraints. [VERIFIED: codebase grep] [CITED: https://ffmpeg.org/ffmpeg-filters.html]

**Research date:** 2026-06-25  
**Valid until:** 2026-07-25 for repo architecture; 2026-07-02 for npm/wgpu/FFmpeg version recency.
