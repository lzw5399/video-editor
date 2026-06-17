# Phase 7: Project Canvas Space And Coordinate System - Context

**Gathered:** 2026-06-18
**Status:** Ready for UI design contract and planning
**Mode:** Autonomous smart discuss, user-approved goal is to continue through Phase 13

<domain>
## Phase Boundary

Phase 07 establishes the draft-level project canvas space used by preview, export, text layout, later transform, sticker, PIP, keyframes, effects, and transitions. It should add a canonical canvas/profile model to `.veproj/project.json`, Rust domain types, validation, generated contracts, command routes, normalized engine profile resolution, and a Chinese desktop UI surface for project canvas settings.

This phase owns project-level canvas/profile semantics: 画布比例, 画布尺寸, 帧率, and 画布背景. It does not implement segment transform, fit/fill/stretch, crop, layer compositing, blend modes, masks, or clip-level 背景填充 behavior beyond representing clear deferred/degraded boundaries for later phases.

</domain>

<decisions>
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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/draft_model/src/draft.rs` owns `Draft`, `DraftMetadata`, and `DraftSchemaVersion`; this is the correct place for required draft-level canvas/profile semantics.
- `crates/draft_model/src/material.rs` already defines `RationalFrameRate`; reuse it for draft canvas frame rate.
- `crates/draft_model/src/validation.rs` already validates rational frame rates, timeranges, text fields, duplicate IDs, and derived-artifact leakage; extend this for canvas dimensions/background semantics.
- `crates/draft_commands` already owns pure Rust edit commands, undo/redo history, selection, snapping, text, audio, and timeline edits; add canvas command semantics here or in a sibling pure module.
- `crates/engine_core/src/normalize.rs` already has `EngineProfile` with `frame_rate`, `canvas_width`, `canvas_height`, and `text_layout`; Phase 07 should resolve it from draft canvas config.
- `crates/render_graph`, `crates/ffmpeg_compiler`, `preview_service`, and `bindings_node::preview_export_service` already form the preview/export path that must consume the draft canvas/profile.
- `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` already shows no-selection `草稿参数` with hard-coded `画布比例` and `画布尺寸`; this is the direct UI upgrade point.
- `apps/desktop-electron/src/renderer/commandHelpers.ts`, `App.tsx`, `viewModel.ts`, and generated contracts already centralize renderer command envelopes and state display.

### Established Patterns
- Rust serde/ts-rs/schemars types are the source of truth for schema and TypeScript contracts.
- `schemas/draft.schema.json`, `schemas/command.schema.json`, and `apps/desktop-electron/src/generated/*.ts` are regenerated by `crates/draft_model/tests/schema_exports.rs`.
- Visible desktop copy is Simplified Chinese and should stay Jianying-style.
- Renderer display state is allowed, but draft semantics must come from Rust command responses.
- Public gates are package scripts plus `just build`/`just test`; phase source guards are shell scripts using `rg` and contract drift checks.

### Integration Points
- `crates/draft_model/src/draft.rs`
- `crates/draft_model/src/validation.rs`
- `crates/draft_model/tests/schema_exports.rs`
- `crates/draft_model/tests/draft_fixtures.rs` and `fixtures/draft/positive|negative`
- `crates/draft_commands/src/`
- `crates/bindings_node/src/lib.rs`
- `crates/engine_core/src/normalize.rs`, `frame_state.rs`, and tests
- `crates/render_graph/src/lib.rs` and tests
- `crates/ffmpeg_compiler/src/lib.rs` and tests
- `crates/preview_service/src/lib.rs`
- `apps/desktop-electron/src/renderer/commandHelpers.ts`
- `apps/desktop-electron/src/renderer/App.tsx`
- `apps/desktop-electron/src/renderer/viewModel.ts`
- `apps/desktop-electron/src/renderer/workspace/Inspector.tsx`
- `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx`
- `apps/desktop-electron/tests/workspace.spec.ts`
- `scripts/phase4-source-guards.sh`, `scripts/phase5-source-guards.sh`, and a new Phase 07 guard

</code_context>

<specifics>
## Specific Ideas

- User explicitly wants internal and external terminology to align with Jianying terms, not a user-facing translation layer over invented internal names.
- User emphasized the left panel should not add duplicate primary menus; Phase 07 UI should preserve the top feature bar as primary navigation and use the inspector/preview controls for canvas settings.
- User emphasized scrollbars/proportions should stay close to Jianying Pro; any Phase 07 UI must retain compact dark scrollbars and 1120x720/1280x800 layout checks.
- Kdenlive/MLT are references only. MLT profiles show a useful structure: width, height, frame_rate_num, frame_rate_den, display_aspect_num, display_aspect_den. Do not copy code or profile files.
- pyJianYingDraft evidence: `DraftFolder.create_draft(name, width, height, fps=30)` creates draft dimensions/fps; `ScriptFile.dumps()` writes `canvas_config` and `fps`; `VideoSegment.add_background_filling()` creates per-segment canvas blur/color background filling, which is not the same as draft-level canvas config.

</specifics>

<deferred>
## Deferred Ideas

- Segment transform controls such as position, scale, rotation, opacity, crop, anchor, fit, fill, stretch, and background filling belong to Phase 08.
- Complete text box layout, line height, letter spacing, safe areas beyond existing MVP text layout, and subtitle parity belong to Phase 09.
- Typed keyframe editing and animated values belong to Phase 10.
- Retiming/speed behavior belongs to Phase 11.
- Filter/adjustment/effect parameter schemas belong to Phase 12.
- Transition attachment and overlap behavior belongs to Phase 13.
- Jianying/CapCut/Kaipai import/export adapters remain post-MVP and should consume these semantics later rather than defining them now.

</deferred>
