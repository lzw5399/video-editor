# Phase 07: Project Canvas Space And Coordinate System - Research

**Researched:** 2026-06-18
**Domain:** Rust-owned draft canvas/profile schema, normalized visual coordinates, render/preview/export propagation, Electron inspector UI
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

All constraints in this section are copied from `.planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md`. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]

### Locked Decisions
## Implementation Decisions

### Jianying-Aligned Canvas Concepts
- Use Jianying-style vocabulary consistently across schema, Rust, IPC, docs, tests, and UI: `draft`, `canvas`, `canvasConfig`, `canvasBackground`, `aspectRatio`, `frameRate`, `background`, `backgroundFilling`, `material`, `track`, `segment`. Avoid introducing parallel names such as `Asset`, `Clip`, `Stage`, or `SceneSize`.
- Treat pyJianYingDraft's `canvas_config` width/height/fps pattern as the vocabulary reference for project canvas/profile. Our canonical `.veproj/project.json` should store the equivalent in a self-owned, typed shape such as draft-level canvas config/profile, not a Jianying JSON clone.
- Keep clip-level `BackgroundFilling` / `materials.canvases` from pyJianYingDraft as Phase 08 transform/compositing territory. Phase 07 may support draft-level background black/solid/blur/image options, but must not claim full per-segment background filling parity.
- UI visible copy should use 简体中文 terms: 草稿参数, 画布比例, 画布尺寸, 帧率, 画布背景, 黑色, 纯色, 模糊填充, 图片背景, 未接入, 不支持.

### Draft Schema And Validation
- Add a required draft-level canvas/profile model to `draft_model::Draft`. Defaults should preserve existing MVP behavior: 1920 x 1080, 30/1 fps, 16:9, black background.
- Store frame rate as existing `RationalFrameRate`, never floating-point fps. Store dimensions as positive integers. Store aspect ratio as a typed semantic value derived/validated against width and height or represented by stable presets plus custom width/height.
- Support semantic canvas background modes for Phase 07: black, solid color, blur fill, and image background. Black can be represented as a specific background mode instead of a magic color string.
- Classify background rendering capability explicitly. If blur fill or image background are not fully renderable in preview/export yet, engine/render graph should surface supported/degraded/unsupported diagnostics instead of silently faking support.

### Coordinate System
- Document one normalized visual coordinate system now so Phase 08-13 do not invent separate coordinate spaces. Use the Jianying/pyJianYingDraft-compatible model: origin at canvas center, x/y normalized to canvas half-width/half-height, with positive x right and positive y up; `0,0` is center, right/top edges are `1`, and left/bottom edges are `-1`.
- Also document how UI pixel space and render pixel space map into normalized coordinates. UI preview can display derived pixels, but persisted semantics should use normalized coordinates or typed integer/rational values, not renderer pixel offsets.
- Text safe area and future sticker/PIP/transform/keyframe values must share this coordinate definition. If a later phase needs local material-space coordinates for crop/mask, it must explicitly convert from this canvas-space contract.
- Keep coordinate documentation in project docs or phase docs and enforce with Rust tests that engine/profile conversion uses canvas width/height deterministically.

### Engine, Preview, Export, And Render Graph
- `engine_core::EngineProfile` currently supplies `canvas_width`, `canvas_height`, and `frame_rate` outside the draft. Phase 07 should resolve that profile from `Draft.canvasConfig` so preview/export stop relying on hard-coded 1920x1080/30fps defaults.
- `render_graph` and `ffmpeg_compiler` should consume canvas/profile from normalized engine state, not from Electron UI or ad hoc export presets. Renderer must not construct resolution, FPS, background, filter_complex, render graph, or FFmpeg output settings directly.
- Export validation should expect output dimensions and frame rate from draft canvas/profile. Preview frame/segment requests should use the same resolved profile so preview/export stay on one semantic path.
- Existing fixed-size test helpers can keep convenience defaults, but Phase 07 tests should prove changing canvas size/frame rate changes engine/render/export expectations through Rust-owned semantics.

### Desktop UI Placement And Behavior
- The main Phase 07 UI entry is the right inspector's no-selection `草稿参数` area. It should expose compact Jianying-style controls for 画布比例, 画布尺寸, 帧率, and 画布背景 without adding another left-side primary menu.
- The center preview monitor may show the current canvas ratio/size and use the canvas aspect ratio for the black canvas shell. The preview ratio/fit control can display the current ratio but must route semantic changes through Rust commands, not local draft mutation.
- Use compact controls: segmented ratio presets, numeric inputs for custom width/height, frame-rate select or rational input, color swatch for solid color, disabled/deferred image selector if image background is not implemented yet.
- Deferred states should remain visible in Chinese. For example, 图片背景 can show 未接入 or 需素材选择 rather than disappearing.

### Command Ownership
- Add Rust-owned commands for project canvas updates. Renderer may build generated command envelopes but must not mutate `draft.canvasConfig`, `draft.tracks`, segment timeranges, undo/redo, snapping, render graph, FFmpeg args, or export scripts directly.
- Canvas commands should be undoable if they mutate draft semantics, using the same session-only command history pattern as timeline/text/audio edits.
- Command responses should return `TimelineCommandResponse` or a canvas-specific response that includes updated `draft`, `commandState`, `selection`, and events. Favor reusing established command envelope patterns over inventing a renderer-owned state channel.
- Source guards should be extended so renderer direct mutation of `draft.canvasConfig`, canvas background, aspect ratio, dimensions, frame rate, and normalized coordinate semantics is blocked outside the generated command helper boundary.

### Testing And Gates
- Add Rust tests for draft schema defaults/validation/migration, invalid dimensions/frame rates/background references, generated schema/TypeScript drift, command routing, undo/redo for canvas edits, and engine profile resolution from draft canvas.
- Add render/preview/export tests that prove non-1920x1080 or vertical canvas profiles flow through normalized engine state and output validation.
- Add Playwright Electron tests at 1280x800 and 1120x720 verifying the Chinese canvas settings are visible, compact, and command-driven without overlapping existing workspace regions.
- Add a Phase 07 source guard and wire `pnpm run test:phase7` plus `just test` or the equivalent public gate.

### the agent's Discretion
- The exact Rust type names may be code-friendly as long as they preserve Jianying concepts and do not introduce a second product vocabulary.
- The planner may split Phase 07 into schema, command, engine/render, UI, and verification plans, or combine nearby tasks if tests stay focused.
- The planner may defer actual image background material selection to Phase 08 or a later material-picker plan, but the unsupported/deferred state must be explicit.

### Deferred Ideas (OUT OF SCOPE)
## Deferred Ideas

- Segment transform controls such as position, scale, rotation, opacity, crop, anchor, fit, fill, stretch, and background filling belong to Phase 08.
- Complete text box layout, line height, letter spacing, safe areas beyond existing MVP text layout, and subtitle parity belong to Phase 09.
- Typed keyframe editing and animated values belong to Phase 10.
- Retiming/speed behavior belongs to Phase 11.
- Filter/adjustment/effect parameter schemas belong to Phase 12.
- Transition attachment and overlap behavior belongs to Phase 13.
- Jianying/CapCut/Kaipai import/export adapters remain post-MVP and should consume these semantics later rather than defining them now.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CANVAS-01 | Draft has a canonical project canvas/profile model for aspect ratio, canvas width/height, and rational frame rate. [VERIFIED: .planning/REQUIREMENTS.md] | Add `Draft.canvas_config`/`canvasConfig` with typed aspect ratio, dimensions, and existing `RationalFrameRate`; migrate fixtures/schema/generated TS and resolve `EngineProfile` from it. [VERIFIED: crates/draft_model/src/draft.rs, crates/draft_model/src/material.rs, crates/engine_core/src/normalize.rs] |
| CANVAS-02 | Draft supports semantic canvas background modes: black, solid color, blur fill, and image background. [VERIFIED: .planning/REQUIREMENTS.md] | Add typed background enum and capability diagnostics; keep clip-level `BackgroundFilling` out of scope because pyJianYingDraft models blur/color filling on video segments. [CITED: https://github.com/GuanYixuan/pyJianYingDraft/blob/245f5d3f2cbbd512d0ab6026f0dd9ef918780458/pyJianYingDraft/video_segment.py#L247-L272, https://github.com/GuanYixuan/pyJianYingDraft/blob/245f5d3f2cbbd512d0ab6026f0dd9ef918780458/pyJianYingDraft/video_segment.py#L556-L580] |
| CANVAS-03 | Visual coordinate semantics are documented and shared by transform, sticker, text, PIP, keyframe, preview, and export paths. [VERIFIED: .planning/REQUIREMENTS.md] | Document center-origin normalized canvas coordinates and add conversion tests; pyJianYingDraft describes `transform_x`/`transform_y` in half-canvas-width/height units. [CITED: https://github.com/GuanYixuan/pyJianYingDraft/blob/245f5d3f2cbbd512d0ab6026f0dd9ef918780458/pyJianYingDraft/segment.py#L142-L177] |
| CANVAS-04 | Desktop UI exposes project canvas settings with Simplified Chinese Jianying-style terminology and Rust command ownership. [VERIFIED: .planning/REQUIREMENTS.md] | Upgrade `Inspector.tsx` no-selection `草稿参数`, update `PreviewMonitor.tsx` readout/aspect ratio, add generated command helpers, and extend source guards/test recorder to block renderer-owned semantics. [VERIFIED: apps/desktop-electron/src/renderer/workspace/Inspector.tsx, apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx, apps/desktop-electron/src/renderer/commandHelpers.ts, scripts/phase4-source-guards.sh] |
</phase_requirements>

## Project Constraints (from AGENTS.md)

- UI emits commands and Rust core owns project/timeline semantics; UI must not construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is the canonical semantic source; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts. [VERIFIED: AGENTS.md]
- Product language, Rust domain types, IPC commands, docs, schema, and tests should use Jianying concepts; prefer draft/material/track/segment/keyframe/filter/transition terms. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates; persisted semantics must avoid naked floating-point time. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors but does not decide editing behavior. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are conceptual references only; do not copy GPL code/assets/XML/presets/UI implementation. [VERIFIED: AGENTS.md]
- External drafts go through adapters and compatibility reports; proprietary IDs remain external references. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution must be reviewed for LGPL/GPL/nonfree options, notices, and commercial obligations. [VERIFIED: AGENTS.md]
- File changes should occur through GSD workflow entrypoints; this artifact is part of the active GSD phase-research workflow. [VERIFIED: AGENTS.md, .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]

## Summary

Phase 07 should be planned as a vertical semantic propagation phase: add a required draft-level `canvasConfig`, validate it in `draft_model`, mutate it only through Rust commands, resolve `engine_core::EngineProfile` from it, and let render graph, compiler, preview, export, and UI consume the resulting Rust-owned state. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md, crates/draft_model/src/draft.rs, crates/engine_core/src/normalize.rs, crates/render_graph/src/graph.rs]

The highest-risk implementation gap is that preview/export still use `EngineProfile::mvp_default()` and fixed output dimensions in service layers, while the UI hard-codes `16:9` and `1920 x 1080`. [VERIFIED: crates/preview_service/src/service.rs, crates/bindings_node/src/preview_export_service.rs, apps/desktop-electron/src/renderer/workspace/Inspector.tsx, apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx] The plan should eliminate those hard-coded semantic defaults from production paths while preserving convenience helpers for tests. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]

**Primary recommendation:** Implement `canvasConfig` as the canonical draft profile, route one undoable `updateDraftCanvasConfig` command through the existing generated command envelope and `TimelineCommandResponse` shape, and add `EngineProfile::from_draft_canvas(&Draft)` so preview/export/render graph use one canvas source. [VERIFIED: crates/draft_model/src/lib.rs, crates/draft_commands/src/history.rs, crates/engine_core/src/normalize.rs]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Draft canvas/profile schema | Rust `draft_model` | JSON Schema / generated TS | `Draft` and generated contracts are current source of truth for persisted `.veproj/project.json` semantics. [VERIFIED: crates/draft_model/src/draft.rs, crates/draft_model/tests/schema_exports.rs] |
| Canvas validation and migration | Rust `draft_model::validation` | Fixture/schema tests | Current migration validates required fields, schema version, derived artifact leakage, rational frame rates, IDs, timeranges, and text. [VERIFIED: crates/draft_model/src/validation.rs, crates/draft_model/tests/draft_fixtures.rs] |
| Canvas edit command and undo/redo | Rust `draft_commands` | `bindings_node` route | Existing timeline/text/audio commands clone, validate, and return updated `Draft`, `CommandState`, `TimelineSelection`, and events. [VERIFIED: crates/draft_commands/src/timeline.rs, crates/draft_commands/src/history.rs, crates/bindings_node/src/lib.rs] |
| Normalized profile resolution | Rust `engine_core` | Render graph | `NormalizedDraft.profile` currently stores `frame_rate`, `canvas_width`, and `canvas_height`; render graph reads canvas from that profile. [VERIFIED: crates/engine_core/src/normalize.rs, crates/render_graph/src/graph.rs] |
| Preview/export dimensions and validation | Rust service/graph/compiler/runtime | Electron only displays status | `ffmpeg_compiler` derives validation expectations from `RenderOutputProfile`, and runtime validates rendered output metadata. [VERIFIED: crates/ffmpeg_compiler/src/job.rs, crates/media_runtime/src/validate.rs] |
| Canvas settings UI | Electron renderer | Rust generated contracts | Current no-selection inspector shows hard-coded canvas readouts and renderer helpers build command envelopes through generated types. [VERIFIED: apps/desktop-electron/src/renderer/workspace/Inspector.tsx, apps/desktop-electron/src/renderer/commandHelpers.ts] |
| Source boundary enforcement | Scripts/tests | Playwright | Existing guards block renderer-owned draft mutation, FFmpeg/render/cache ownership, English workspace copy, and contract drift. [VERIFIED: scripts/phase4-source-guards.sh, scripts/phase5-source-guards.sh, apps/desktop-electron/tests/workspace.spec.ts] |

## Standard Stack

### Core
| Library / Crate | Version | Purpose | Why Standard |
|-----------------|---------|---------|--------------|
| Rust workspace crates (`draft_model`, `draft_commands`, `engine_core`, `render_graph`, `ffmpeg_compiler`, `preview_service`, `bindings_node`) | Rust 1.95.0 / edition 2024 | Own draft semantics, commands, normalization, render intent graph, compiler, services, and Node-API route. | Existing architecture assigns semantics to Rust and keeps renderer command-only. [VERIFIED: Cargo.toml, rust-toolchain.toml, AGENTS.md] |
| `serde` + `schemars` + `ts-rs` derives | Existing crate dependencies | Generate JSON schema and TypeScript contracts from Rust types. | Current schema export test regenerates `schemas/*.json` and `apps/desktop-electron/src/generated/*.ts`. [VERIFIED: crates/draft_model/tests/schema_exports.rs] |
| React + TypeScript renderer | React 19.2.7 / TypeScript 6.0.3 | Inspector and preview monitor UI. | Existing Electron renderer uses React/TS and generated contracts. [VERIFIED: apps/desktop-electron/package.json, apps/desktop-electron/src/renderer/App.tsx] |
| Playwright Electron tests | `@playwright/test` 1.61.0 | Workspace interaction, screenshots, command routing. | Existing workspace tests launch Electron, record command calls, and verify 1280x800 / 1120x720 layout. [VERIFIED: apps/desktop-electron/package.json, apps/desktop-electron/tests/workspace.spec.ts] |

### Supporting
| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| FFmpeg / ffprobe | 8.1 available locally | Preview/export generation and output validation. | Needed for render smoke, preview/export parity, and real workflow gates. [VERIFIED: local `ffmpeg -version`, local `ffprobe -version`, crates/media_runtime/src/validate.rs] |
| `rg` | Available through scripts | Source guard pattern matching. | Extend Phase 4/5 guards with Phase 7 canvas ownership checks. [VERIFIED: scripts/phase4-source-guards.sh, scripts/phase5-source-guards.sh] |
| `slopcheck` | 0.6.1 available | Package legitimacy audit if new packages are introduced. | No new package is recommended for Phase 07, but the tool is available if the plan changes. [VERIFIED: local `slopcheck --version`] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Draft-owned `canvasConfig` | Renderer-local preview/export settings | Reject: violates command-only renderer and splits preview/export semantics from persisted draft. [VERIFIED: AGENTS.md, .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md] |
| One `updateDraftCanvasConfig` command | Separate UI-only mutations for width, height, fps, background | Reject: undo/redo and validation are already Rust-owned command responsibilities. [VERIFIED: crates/draft_commands/src/history.rs, apps/desktop-electron/src/renderer/commandHelpers.ts] |
| pyJianYingDraft JSON clone | Self-owned typed schema with Jianying vocabulary | Reject: context locks self-owned `.veproj/project.json`; pyJianYingDraft is reference evidence, not internal schema. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md; CITED: https://github.com/GuanYixuan/pyJianYingDraft/blob/245f5d3f2cbbd512d0ab6026f0dd9ef918780458/pyJianYingDraft/script_file.py#L791-L797] |

**Installation:**
```bash
# No new runtime or npm packages are recommended for Phase 07. [VERIFIED: package.json, apps/desktop-electron/package.json]
```

**Version verification:** Existing versions were verified from repo manifests and local probes; no package install is required. [VERIFIED: package.json, apps/desktop-electron/package.json, Cargo.toml, rust-toolchain.toml, local CLI probes]

## Package Legitimacy Audit

Phase 07 does not require installing external packages. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md, package.json, apps/desktop-electron/package.json]

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| none | — | — | — | — | not run for packages | No new packages recommended. [VERIFIED: package.json, apps/desktop-electron/package.json] |

**Packages removed due to slopcheck [SLOP] verdict:** none. [VERIFIED: no new packages recommended]
**Packages flagged as suspicious [SUS]:** none. [VERIFIED: no new packages recommended]

## Architecture Patterns

### System Architecture Diagram

```text
Inspector 草稿参数 form
  -> generated command helper buildUpdateDraftCanvasConfigCommand
  -> window.videoEditorCore.executeCommand
  -> Electron main IPC allowlist / test recorder
  -> bindings_node command envelope deserialize + route
  -> draft_commands canvas command
     -> clone Draft
     -> patch canvasConfig
     -> validate_draft + canvas capability classification
     -> push undo snapshot
     -> TimelineCommandResponse { draft, commandState, selection, events }
  -> renderer applies Rust response only

Preview/export request
  -> Draft.canvasConfig
  -> engine_core::EngineProfile::from_draft_canvas
  -> normalize_draft
  -> resolve_render_range
  -> build_render_graph with RenderCanvas
  -> RenderOutputProfile dimensions/frameRate from draft profile
  -> ffmpeg_compiler validation expectations
  -> media_runtime output validation
```

This diagram reflects existing command, normalization, render graph, compiler, and runtime boundaries. [VERIFIED: crates/draft_model/src/lib.rs, crates/draft_commands/src/timeline.rs, crates/engine_core/src/normalize.rs, crates/render_graph/src/graph.rs, crates/ffmpeg_compiler/src/job.rs, crates/media_runtime/src/validate.rs]

### Recommended Project Structure

```text
crates/draft_model/src/
├── canvas.rs          # DraftCanvasConfig, CanvasAspectRatio, CanvasBackground, normalized coordinate docs/tests
├── draft.rs           # Draft gains required canvas_config field and default
├── validation.rs      # canvas dimension/fps/background validation
└── lib.rs             # re-export canvas types and command payload types

crates/draft_commands/src/
├── canvas.rs          # updateDraftCanvasConfig command, clone/patch/validate/commit
└── lib.rs             # export canvas module

crates/engine_core/src/
└── normalize.rs       # EngineProfile::from_draft_canvas / profile resolution from Draft

apps/desktop-electron/src/renderer/
├── commandHelpers.ts  # build/apply canvas command helper using generated contracts
├── viewModel.ts       # format canvas ratio, size, fps, background status
└── workspace/
    ├── Inspector.tsx      # 草稿参数 editable controls
    └── PreviewMonitor.tsx # draft-aware canvas aspect/readout/status

scripts/
└── phase7-source-guards.sh
```

The locations above match existing ownership boundaries and generated-contract patterns. [VERIFIED: repo file tree, crates/draft_model/tests/schema_exports.rs, apps/desktop-electron/src/renderer/commandHelpers.ts]

### Pattern 1: Draft-Owned Canvas Schema

**What:** Add required `canvas_config` Rust field serialized as `canvasConfig`, with defaults in `Draft::new`. [VERIFIED: crates/draft_model/src/draft.rs]
**When to use:** Use for persisted project-wide width/height/aspect/fps/background semantics. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
**Example:**
```rust
// Pattern source: crates/draft_model/src/draft.rs and crates/draft_model/src/material.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftCanvasConfig {
    pub aspect_ratio: CanvasAspectRatio,
    pub width: u32,
    pub height: u32,
    pub frame_rate: RationalFrameRate,
    pub background: CanvasBackground,
}

impl DraftCanvasConfig {
    pub fn mvp_default() -> Self {
        Self {
            aspect_ratio: CanvasAspectRatio::Preset(CanvasAspectRatioPreset::Ratio16x9),
            width: 1920,
            height: 1080,
            frame_rate: RationalFrameRate::new(30, 1),
            background: CanvasBackground::Black,
        }
    }
}
```

### Pattern 2: Clone/Patch/Validate/Commit Command

**What:** Canvas edits should clone the draft, update `canvasConfig`, validate, push undo snapshot, and return `TimelineCommandResponse`. [VERIFIED: crates/draft_commands/src/timeline.rs, crates/draft_commands/src/history.rs]
**When to use:** Use for every draft-semantic canvas mutation, including ratio, dimensions, frame rate, and background. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
**Example:**
```rust
// Pattern source: crates/draft_commands/src/timeline.rs and crates/draft_commands/src/history.rs
pub fn update_draft_canvas_config(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    canvas_config: DraftCanvasConfig,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    next_draft.canvas_config = canvas_config;
    validate_draft(&next_draft)?;

    Ok(TimelineCommandResponse {
        draft: next_draft,
        command_state: command_state_after_commit(command_state, draft, selection, "updateDraftCanvasConfig"),
        selection: selection.clone(),
        events: vec![CommandEvent { kind: "draftCanvasConfigUpdated".to_owned(), message: None }],
    })
}
```

### Pattern 3: Resolve Engine Profile From Draft

**What:** Add an engine helper that transforms `Draft.canvasConfig` into `EngineProfile`; keep `mvp_default()` as a test convenience, not the production preview/export path. [VERIFIED: crates/engine_core/src/normalize.rs, crates/preview_service/src/service.rs, crates/bindings_node/src/preview_export_service.rs]
**When to use:** Use before `normalize_draft` in preview/export/render tests and services. [VERIFIED: crates/preview_service/src/service.rs, crates/bindings_node/src/preview_export_service.rs]
**Example:**
```rust
// Pattern source: crates/engine_core/src/normalize.rs
impl EngineProfile {
    pub fn from_draft_canvas(draft: &Draft) -> Result<Self, EngineError> {
        let canvas = &draft.canvas_config;
        let profile = Self {
            frame_rate: canvas.frame_rate.clone(),
            canvas_width: canvas.width,
            canvas_height: canvas.height,
            text_layout: Some(TextLayoutProfile::for_canvas(canvas.width, canvas.height)),
        };
        profile.validate()?;
        Ok(profile)
    }
}
```

### Pattern 4: Renderer Form State Is Temporary Only

**What:** The renderer may keep local form input while editing, but committed state must come from Rust command response. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md, apps/desktop-electron/src/renderer/App.tsx]
**When to use:** Use in `Inspector.tsx` for width/height/fps/color fields and disabled/degraded background mode rows. [VERIFIED: apps/desktop-electron/src/renderer/workspace/Inspector.tsx]

### Anti-Patterns to Avoid

- **Hard-coded canvas in production paths:** Do not leave `EngineProfile::mvp_default()`, `1920 x 1080`, `16:9`, `960 x 540`, or export preset dimensions as production semantic sources after Phase 07. [VERIFIED: rg hard-code audit, crates/preview_service/src/service.rs, crates/bindings_node/src/preview_export_service.rs, apps/desktop-electron/src/renderer/workspace/Inspector.tsx]
- **Renderer-owned semantic conversion:** Do not let React decide canonical aspect ratio, output validation dimensions, persisted coordinate values, FFmpeg args, or render graph data. [VERIFIED: AGENTS.md, scripts/phase4-source-guards.sh, scripts/phase5-source-guards.sh]
- **Cloning pyJianYingDraft schema:** Use pyJianYingDraft as vocabulary/coordinate evidence, not as `.veproj` JSON structure. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md; CITED: https://github.com/GuanYixuan/pyJianYingDraft/blob/245f5d3f2cbbd512d0ab6026f0dd9ef918780458/pyJianYingDraft/assets/draft_content_template.json#L1-L39]
- **Silent background fake support:** If blur/image backgrounds are not renderable yet, report degraded/unsupported and show Chinese status. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md, .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Persisted frame rate | Float fps fields or renderer-derived fps strings | Existing `RationalFrameRate` | Current validation already rejects zero numerator/denominator and generated schema patches rational fps. [VERIFIED: crates/draft_model/src/material.rs, crates/draft_model/src/validation.rs, crates/draft_model/tests/schema_exports.rs] |
| Canvas command state | UI-local undo stack or direct draft mutation | `CommandState` + `TimelineCommandResponse` | Existing command history is session-only and includes draft/selection snapshots. [VERIFIED: crates/draft_model/src/lib.rs, crates/draft_commands/src/history.rs] |
| Preview/export output profile | Renderer-built dimensions/fps/validation | `EngineProfile` -> `RenderOutputProfile` -> compiler validation | Compiler output validation derives expected width/height/fps from render profile. [VERIFIED: crates/ffmpeg_compiler/src/job.rs] |
| Coordinate systems | Separate text/sticker/PIP/keyframe coordinate models | One normalized canvas-space contract | Phase context and UI spec require shared center-origin normalized coordinates. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md, .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md] |
| Project profile vocabulary | New `Stage`, `SceneSize`, `Asset`, `Clip` names | Jianying-aligned `draft`, `canvas`, `material`, `track`, `segment` | Project constraints forbid parallel terminology. [VERIFIED: AGENTS.md, .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md] |

**Key insight:** This phase is less about adding UI controls than about eliminating multiple existing hidden canvas sources before transform/text/keyframe phases depend on them. [VERIFIED: rg hard-code audit, .planning/ROADMAP.md]

## Common Pitfalls

### Pitfall 1: Canvas Field Added But Fixtures Not Migrated
**What goes wrong:** `migrate_draft_json` rejects existing positive fixtures because required fields remain `schemaVersion`, `draftId`, `metadata`, `materials`, and `tracks` only. [VERIFIED: crates/draft_model/src/validation.rs, fixtures/draft/positive/minimal-draft/project.json]
**Why it happens:** The current migration explicitly checks required top-level fields before deserializing. [VERIFIED: crates/draft_model/src/validation.rs]
**How to avoid:** Add `canvasConfig` to required-field checks, update all positive fixtures, add negative missing/invalid canvas fixtures, and regenerate draft schema. [VERIFIED: crates/draft_model/tests/draft_fixtures.rs, crates/draft_model/tests/schema_exports.rs]
**Warning signs:** `schema_fixtures` fails with unknown/missing field messages, or generated `Draft.ts` lacks `canvasConfig`. [VERIFIED: crates/draft_model/tests/draft_fixtures.rs, apps/desktop-electron/src/generated/Draft.ts]

### Pitfall 2: Preview Export Still Use MVP Defaults
**What goes wrong:** UI displays vertical/custom canvas but preview/export still render/validate 1920x1080 at 30 fps. [VERIFIED: crates/preview_service/src/service.rs, crates/bindings_node/src/preview_export_service.rs]
**Why it happens:** `preview_service` and export preparation call `normalize_draft(&draft, &EngineProfile::mvp_default())`, and export dimensions are selected by preset. [VERIFIED: crates/preview_service/src/service.rs, crates/bindings_node/src/preview_export_service.rs]
**How to avoid:** Replace production calls with `EngineProfile::from_draft_canvas(&draft)` and derive output dimensions from normalized profile unless an explicit future export resize feature exists. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
**Warning signs:** Tests still assert 960x540 previews or 1920x1080 export validation for a vertical/custom draft. [VERIFIED: crates/preview_service/src/service.rs, crates/render_graph/tests/render_graph_snapshots.rs, apps/desktop-electron/tests/workspace.spec.ts]

### Pitfall 3: Aspect Ratio Validation Is Too Loose
**What goes wrong:** `aspectRatio` says `16:9` while width/height are `1080x1920`, causing future transform mapping drift. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
**Why it happens:** Preset aspect ratio and dimensions can be stored independently without validation. [ASSUMED]
**How to avoid:** Either derive ratio from width/height for persistence, or validate preset ratios against reduced integer width/height with custom as explicit escape hatch. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
**Warning signs:** UI active ratio preset differs from generated preview monitor ratio. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md]

### Pitfall 4: Coordinate Y Direction Drifts Between UI And Semantics
**What goes wrong:** UI top-left pixel coordinates become persisted semantics while Rust/render expects center-origin Y-up coordinates. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md]
**Why it happens:** DOM pixel coordinates are top-left origin and Y-down; the locked semantic coordinate system is center-origin and Y-up. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md]
**How to avoid:** Document mapping formulas and test round trips for 16:9, 9:16, and square canvases. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
**Warning signs:** Transform/keyframe code later stores pixel offsets or separate text/sticker units. [VERIFIED: .planning/ROADMAP.md]

### Pitfall 5: Background Modes Are Represented But Not Classified
**What goes wrong:** Blur or image background is accepted, but preview/export silently render black without warning. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
**Why it happens:** Current `RenderIntentSupport` only has `Supported` and `Degraded`, and canvas has no background/capability diagnostics. [VERIFIED: crates/render_graph/src/graph.rs]
**How to avoid:** Add background capability state at engine/render graph level and UI status chips (`降级`, `未接入`, `不支持`). [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md]
**Warning signs:** No test asserts blur/image unsupported diagnostics. [VERIFIED: current test inventory via rg]

## Code Examples

### Normalized Coordinate Mapping
```rust
// Source: Phase 07 UI spec + pyJianYingDraft ClipSettings documentation.
// Normalized semantics: center origin, +X right, +Y up, 1.0 is half canvas width/height.
pub fn normalized_to_canvas_pixel(x: f64, y: f64, width: u32, height: u32) -> (f64, f64) {
    let px = (x + 1.0) * f64::from(width) / 2.0;
    let py = (1.0 - y) * f64::from(height) / 2.0;
    (px, py)
}

pub fn canvas_pixel_to_normalized(px: f64, py: f64, width: u32, height: u32) -> (f64, f64) {
    let x = (px / f64::from(width)) * 2.0 - 1.0;
    let y = 1.0 - (py / f64::from(height)) * 2.0;
    (x, y)
}
```
The formulas implement the locked coordinate contract; use tests to keep this as documentation, and avoid persisting pixel offsets. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md; CITED: https://github.com/GuanYixuan/pyJianYingDraft/blob/245f5d3f2cbbd512d0ab6026f0dd9ef918780458/pyJianYingDraft/segment.py#L142-L177]

### Command Helper Shape
```typescript
// Source: apps/desktop-electron/src/renderer/commandHelpers.ts
export function buildUpdateDraftCanvasConfigCommand(
  context: CommandContext,
  canvasConfig: DraftCanvasConfig
): CommandEnvelope {
  const payload = {
    kind: "updateDraftCanvasConfig",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    canvasConfig
  } satisfies UpdateDraftCanvasConfigCommandPayload & { kind: "updateDraftCanvasConfig" };

  return envelope("updateDraftCanvasConfig", payload);
}
```
This follows existing generated-contract command helper style. [VERIFIED: apps/desktop-electron/src/renderer/commandHelpers.ts]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| MLT-style global profile fields (`width`, `height`, `frame_rate_num`, `frame_rate_den`, display aspect fields) | Self-owned `.veproj` draft `canvasConfig` using existing Rust schema/generation | Phase 07 planning | MLT validates the concept of project profile fields and rational frame rates, but it remains conceptual only. [CITED: https://www.mltframework.org/docs/profiles/; VERIFIED: AGENTS.md] |
| Kdenlive project profile preset drives compositing/transform/keyframes | Rust-owned draft canvas/profile drives preview/export/transform foundations | Phase 07 planning | Kdenlive documentation reinforces that project profiles define dimensions, aspect ratio, fps, and downstream operations; do not copy Kdenlive implementation. [CITED: https://docs.kdenlive.org/en/project_and_asset_management/project_settings/general_settings.html#project-profile-preset; VERIFIED: AGENTS.md] |
| Existing MVP default profile outside draft | Required draft-level canvas profile | Phase 07 target | Removes hard-coded production defaults and prepares Phase 08-13 semantics. [VERIFIED: crates/engine_core/src/normalize.rs, .planning/ROADMAP.md] |

**Deprecated/outdated:**
- `EngineProfile::mvp_default()` as a production preview/export source should be retired after Phase 07; keep it only for tests/helpers that intentionally use MVP defaults. [VERIFIED: crates/engine_core/src/normalize.rs, crates/preview_service/src/service.rs, crates/bindings_node/src/preview_export_service.rs]
- UI hard-coded `16:9` / `1920 x 1080` should be replaced with draft-driven values. [VERIFIED: apps/desktop-electron/src/renderer/workspace/Inspector.tsx, apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Aspect ratio validation can either derive ratio from dimensions or validate preset ratio against reduced integer dimensions. | Common Pitfalls | Planner may choose a schema shape that is too strict or too loose. |
| A2 | A single `updateDraftCanvasConfig` command is preferable to multiple partial canvas commands. | Summary / Architecture Patterns | Fine-grained commands could be desired for UI responsiveness, but existing command history works best with atomic semantic edits. |
| A3 | `TimelineCommandResponse` can be reused for canvas edits instead of introducing a canvas-specific response. | Summary / Architecture Patterns | If planner chooses a new response type, generated contracts and UI appliers need extra work. |

## Open Questions (RESOLVED)

1. **Should `aspectRatio` be stored as preset/custom or derived from dimensions only?**
   - What we know: Phase context allows derived/validated ratio or stable presets plus custom width/height. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
   - RESOLVED: Store a typed preset/custom semantic plus width/height, and validate preset dimensions by reduced ratio. This preserves Jianying-style preset behavior without making renderer-derived strings the canonical source.

2. **Should preview output be full canvas dimensions or scaled preview cache dimensions?**
   - What we know: Preview currently uses fixed 960x540 while export uses preset dimensions; Phase 07 says preview requests should use the same resolved profile so preview/export stay semantic. [VERIFIED: crates/preview_service/src/service.rs, crates/bindings_node/src/preview_export_service.rs, .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
   - RESOLVED: Preview semantics derive from draft canvas width/height/fps, while preview cache artifacts may be downscaled with a documented max size that preserves draft ratio. Export validation uses exact draft canvas dimensions and rational frame rate.

3. **Should image background accept a material ID now or remain disabled UI-only?**
   - What we know: UI spec allows disabled/deferred image selector if not implemented, but semantic mode list includes image background. [VERIFIED: .planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md]
   - RESOLVED: Add the semantic image-background variant with a validation boundary. If a material reference is present, it must resolve to an image material; if material selection is not implemented in UI, the UI keeps `图片背景` visible as disabled/`未接入` and does not fabricate a local material reference.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | Electron build/tests | ✓ | v24.12.0 | — [VERIFIED: local `node --version`, package.json] |
| pnpm | Workspace scripts | ✓ | 10.32.1 | — [VERIFIED: local `pnpm --version`, package.json] |
| Rust / Cargo | Rust crates/tests | ✓ | rustc 1.95.0 / cargo 1.95.0 | — [VERIFIED: local probes, rust-toolchain.toml] |
| just | Public gate `just test` | ✗ | — | Use `pnpm test` temporarily, or install `just` before final public gate. [VERIFIED: local `just --version`, justfile] |
| FFmpeg | Preview/export gates | ✓ | 8.1 | — [VERIFIED: local `ffmpeg -version`] |
| ffprobe | Output validation gates | ✓ | 8.1 | — [VERIFIED: local `ffprobe -version`] |
| ctx7 | Documentation lookup fallback | ✗ | — | Used official docs and upstream source via curl/git clone. [VERIFIED: local `command -v ctx7`; CITED: https://www.mltframework.org/docs/profiles/] |
| slopcheck | Package audit if packages added | ✓ | 0.6.1 | No packages recommended. [VERIFIED: local `slopcheck --version`] |

**Missing dependencies with no fallback:**
- None for research. [VERIFIED: environment probes]

**Missing dependencies with fallback:**
- `just` is missing; planner should either add an install/setup task or use `pnpm run test:phase7` plus `pnpm test` until `just` is available. [VERIFIED: local `just --version`, justfile]

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test`, schema drift tests, Playwright Electron, shell source guards. [VERIFIED: package.json, apps/desktop-electron/package.json] |
| Config file | `apps/desktop-electron/playwright.config.ts`; Rust workspace via `Cargo.toml`; scripts via `package.json` and `justfile`. [VERIFIED: repo files] |
| Quick run command | `pnpm run test:phase7` after adding it. [ASSUMED] |
| Full suite command | `pnpm test`; `just test` once `just` is installed. [VERIFIED: package.json, justfile, environment probes] |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| CANVAS-01 | `Draft::new` serializes default canvas config and fixtures validate with canvasConfig. | unit/schema | `cargo test -p draft_model canvas -- --nocapture` | ❌ Wave 0 [VERIFIED: crates/draft_model/tests/draft_schema.rs] |
| CANVAS-01 | Engine profile resolves width/height/fps from draft canvas. | unit | `cargo test -p engine_core canvas_profile -- --nocapture` | ❌ Wave 0 [VERIFIED: crates/engine_core/tests/normalization.rs] |
| CANVAS-02 | Background enum validates black/solid/blur/image and reports degraded/unsupported render modes. | unit/snapshot | `cargo test -p render_graph canvas_background -- --nocapture` | ❌ Wave 0 [VERIFIED: crates/render_graph/tests/render_graph_snapshots.rs] |
| CANVAS-03 | Normalized coordinate conversions are documented and deterministic for horizontal, vertical, square canvases. | unit/docs | `cargo test -p draft_model normalized_coordinates -- --nocapture` | ❌ Wave 0 [VERIFIED: no current canvas coordinate tests via rg] |
| CANVAS-04 | Canvas settings UI visible in Chinese and routes through command helper. | e2e | `pnpm --filter @video-editor/desktop test:workspace -g "画布|草稿参数"` | ❌ Wave 0 [VERIFIED: apps/desktop-electron/tests/workspace.spec.ts] |
| CANVAS-04 | Renderer source cannot directly mutate canvas semantics or build output dimensions/validation. | source guard | `pnpm run test:phase7-source-guards` | ❌ Wave 0 [VERIFIED: scripts/phase4-source-guards.sh, scripts/phase5-source-guards.sh] |

### Sampling Rate
- **Per task commit:** Run focused crate/test command for touched subsystem plus `pnpm run test:contracts` when generated contracts change. [VERIFIED: package.json]
- **Per wave merge:** Run `pnpm run test:phase7` after the script exists. [ASSUMED]
- **Phase gate:** Run `pnpm run test:phase7`, `pnpm test`, and `just test` after installing `just` or documenting equivalent. [VERIFIED: package.json, justfile, environment probes]

### Wave 0 Gaps
- [ ] `crates/draft_model/src/canvas.rs` and tests for schema/defaults/validation/coordinates. [VERIFIED: crates/draft_model/src/draft.rs]
- [ ] `crates/draft_commands/src/canvas.rs` and command tests for apply/undo/redo/invalid cases. [VERIFIED: crates/draft_commands/src/timeline.rs]
- [ ] `crates/engine_core/tests/canvas_profile.rs` or additions to normalization tests for non-default canvas profile. [VERIFIED: crates/engine_core/tests/normalization.rs]
- [ ] `crates/render_graph/tests/canvas_background_snapshots.rs` or additions to existing snapshots. [VERIFIED: crates/render_graph/tests/render_graph_snapshots.rs]
- [ ] `crates/preview_service/tests/preview_canvas_profile.rs` and `crates/bindings_node/tests/export_commands.rs` coverage for draft-driven dimensions/fps. [VERIFIED: crates/preview_service/tests/preview_generation.rs, crates/bindings_node/tests/export_commands.rs]
- [ ] `apps/desktop-electron/tests/workspace.spec.ts` canvas UI command-routing and layout coverage. [VERIFIED: apps/desktop-electron/tests/workspace.spec.ts]
- [ ] `scripts/phase7-source-guards.sh` plus `package.json` / `justfile` wiring. [VERIFIED: package.json, justfile]

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No authentication surface in Phase 07. [VERIFIED: phase scope and code paths] |
| V3 Session Management | no | Command history is local editor session state, not auth/session management. [VERIFIED: crates/draft_commands/src/history.rs] |
| V4 Access Control | limited | IPC sender allowlist remains in Electron main process. [VERIFIED: apps/desktop-electron/src/main/index.ts] |
| V5 Input Validation | yes | Validate dimensions, rational frame rate, aspect ratio consistency, color strings, and background references in Rust before committing draft changes. [VERIFIED: crates/draft_model/src/validation.rs, .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md] |
| V6 Cryptography | no | No cryptographic behavior in Phase 07. [VERIFIED: phase scope] |

### Known Threat Patterns for This Stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Renderer tampering with persisted draft canvas | Tampering | Source guards plus Rust command validation and generated contracts. [VERIFIED: scripts/phase4-source-guards.sh, crates/draft_model/tests/schema_exports.rs] |
| Malformed canvas dimensions/fps causing render/runtime failures | Denial of Service | Reject zero/invalid dimensions and rational frame rates before normalization/render graph compilation. [VERIFIED: crates/draft_model/src/validation.rs, crates/engine_core/src/normalize.rs, crates/render_graph/src/profile.rs] |
| Background image path/reference misuse | Tampering / Information Disclosure | If image background is represented, validate it as a material reference and keep file access in material/runtime boundaries, not renderer. [VERIFIED: AGENTS.md, crates/draft_model/src/material.rs, crates/bindings_node/src/material_service.rs] |
| IPC from untrusted renderer | Spoofing / Tampering | Preserve existing `assertAllowedIpcSender` before `executeCommand`. [VERIFIED: apps/desktop-electron/src/main/index.ts] |

## Sources

### Primary (HIGH Confidence)
- `.planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md` - locked Phase 07 decisions, scope, and integration points. [VERIFIED: local file]
- `.planning/phases/07-project-canvas-space-and-coordinate-system/07-UI-SPEC.md` - UI contract, copy, layout, coordinate wording, deferred/degraded states. [VERIFIED: local file]
- `AGENTS.md` - project architecture, terminology, render, project format, and testing constraints. [VERIFIED: local file]
- `crates/draft_model`, `crates/draft_commands`, `crates/engine_core`, `crates/render_graph`, `crates/ffmpeg_compiler`, `crates/preview_service`, `crates/bindings_node`, `apps/desktop-electron/src/renderer`, and scripts/tests named above - current implementation patterns. [VERIFIED: codebase grep/read]
- pyJianYingDraft upstream source at commit `245f5d3f2cbbd512d0ab6026f0dd9ef918780458` - `create_draft(width,height,fps)`, `canvas_config`, `transform_x/y`, and clip-level `BackgroundFilling`. [CITED: https://github.com/GuanYixuan/pyJianYingDraft/tree/245f5d3f2cbbd512d0ab6026f0dd9ef918780458]
- MLT Profiles documentation - profile fields and rational frame-rate rationale. [CITED: https://www.mltframework.org/docs/profiles/]
- Kdenlive Project Settings documentation - project profile/preset affects dimensions, aspect ratio, fps, compositing, transformations, and keyframes. [CITED: https://docs.kdenlive.org/en/project_and_asset_management/project_settings/general_settings.html#project-profile-preset]

### Secondary (MEDIUM Confidence)
- None. [VERIFIED: research process]

### Tertiary (LOW Confidence)
- Assumptions A1-A3 in the Assumptions Log. [ASSUMED]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Phase uses existing repo stack and no new packages. [VERIFIED: package.json, Cargo.toml, apps/desktop-electron/package.json]
- Architecture: HIGH - Ownership boundaries are explicit in AGENTS, CONTEXT, and current code. [VERIFIED: AGENTS.md, .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md, codebase files]
- Pitfalls: HIGH - Major pitfalls are directly visible in current hard-coded defaults and guard patterns. [VERIFIED: rg hard-code audit, scripts/phase4-source-guards.sh, scripts/phase5-source-guards.sh]
- External reference findings: HIGH for MLT/Kdenlive docs and pyJianYingDraft source; LOW for any inferred product-design preference not explicitly locked. [CITED: sources above]

**Research date:** 2026-06-18
**Valid until:** 2026-07-18 for codebase-local guidance; re-check external package/tool versions if adding dependencies. [ASSUMED]

**What might I have missed:** Existing untracked `reference/` content may contain additional local reverse-engineering data, but Phase 07 already has locked pyJianYingDraft findings and does not need proprietary adapter research. [VERIFIED: git status --short, .planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md]
