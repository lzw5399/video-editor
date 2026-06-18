# Phase 8: Segment Transform And Visual Compositing - Context

**Gathered:** 2026-06-18
**Status:** Ready for research, UI design contract, and planning
**Mode:** Autonomous continuation to Phase 13

<domain>
## Phase Boundary

Phase 08 implements Jianying-style segment-level `画面 / 基础 / 变换` semantics and deterministic visual compositing. It builds on Phase 07's draft-level canvas and normalized coordinate contract, then applies typed visual properties to each visual segment so preview, export, render graph, and desktop inspector all use the same Rust-owned meaning.

This phase owns static segment transform and composition semantics: position x/y, scale, rotation, opacity, crop, anchor, fit/fill/stretch, per-segment background filling, explicit visual layer ordering, visibility, blend-mode capability boundaries, and mask capability boundaries.

This phase does not implement typed keyframe animation, complete text/subtitle semantics, retiming/speed, first-party effect parameter systems, or transition relationships. Those remain Phases 09-13.

</domain>

<decisions>
## Implementation Decisions

### Jianying-Aligned Terms
- Product copy, Rust domain types, IPC commands, schema, docs, and tests should keep Jianying terms aligned: `draft`, `material`, `track`, `segment`, `canvas`, `transform`, `crop`, `anchor`, `fit`, `fill`, `stretch`, `backgroundFilling`, `blendMode`, `mask`, `keyframe`.
- UI visible labels must be Simplified Chinese and use Jianying-style placement: `画面`, `基础`, `变换`, `位置`, `缩放`, `旋转`, `不透明度`, `裁剪`, `锚点`, `适应`, `填充`, `拉伸`, `背景填充`, `混合模式`, `蒙版`, `未接入`, `不支持`.
- Do not introduce an internal `Asset`/`Clip`/`LayerItem` vocabulary when `material`/`segment`/`layer` is enough. English code names may be Rust/TS-friendly, but they should still map one-to-one to Jianying concepts.

### Transform Semantics
- Add typed segment-level transform semantics to visual segments. Defaults preserve existing behavior: centered, scale 100%, rotation 0 degrees, opacity 100%, no crop, center anchor, normal fit behavior.
- Use Phase 07's normalized canvas coordinate system for persisted position: origin at canvas center, positive x right, positive y up, `-1..1` at canvas edges. UI may show derived values, but persisted segment position should not be renderer pixel offsets.
- Persist scale and opacity as typed percentages with validation, rotation as typed degrees with validation, crop as normalized edge insets, and anchor as normalized local material coordinates where center is the default.
- Phase 08 values are static typed values. Phase 10 will wrap these same fields in typed animated values and keyframes; Phase 08 should not create ad hoc animation placeholders.

### Fit, Fill, Stretch, And Background Filling
- `fit`, `fill`, and `stretch` are first-class segment semantics, not preview-only layout math. They define how the source material rectangle maps into the project canvas before user transform is applied.
- Per-segment `backgroundFilling` handles aspect-ratio mismatch such as vertical media on a horizontal canvas. It should support at least black/solid/blur/image semantic variants with capability reporting.
- If blur or image background filling cannot be faithfully rendered in Phase 08, represent it as `degraded` or `unsupported` in engine/render diagnostics instead of silently faking support.

### Visual Layer Composition
- Engine and render graph must evaluate deterministic visual composition order for video, image, text, and future sticker segments.
- Track stacking remains Rust-owned. Visual composition should sort by explicit track/segment order and visibility semantics, not by renderer DOM order.
- Add visibility semantics where needed so hidden visual tracks or segments are omitted from frame state and render graph. Existing audio mute/volume semantics remain separate.
- Text segments should participate in layer ordering and transform defaults in Phase 08, while complete text box/layout expansion remains Phase 09.

### Blend Mode And Mask Boundaries
- Represent blend mode and mask as capability-aware semantic boundaries now, even if only `normal` blend mode and `none` mask render fully in Phase 08.
- Unsupported proprietary blend/mask modes should produce explicit capability reports or diagnostics. Do not store private Jianying IDs as internal render semantics.

### Rust Command Ownership
- Transform and visual composition mutations must be Rust-owned commands with validation and undo/redo. Renderer may only build generated command envelopes and call `window.videoEditorCore.executeCommand`.
- Required command coverage includes updating segment transform, fit/fill/stretch/background filling, visibility/layer settings where applicable, and rejecting invalid values atomically.
- Existing selection, timeline history, and command response patterns should be reused. Avoid a renderer-owned transform state channel.
- Source guards must block renderer direct mutation of `draft.tracks`, `track.segments`, `segment.transform`, segment timeranges, undo/redo, render graph, FFmpeg commands, preview cache semantics, and export scripts outside generated helpers.

### Engine, Render Graph, Preview, And Export
- `engine_core` should emit transform-aware frame state so preview/export share the same segment placement and layer order.
- `render_graph` should carry typed composition operations from normalized draft state to compiler inputs. It should not decide editing behavior.
- `ffmpeg_compiler` should compile the first supported transform/composition subset deterministically: placement, scale, rotation, opacity, crop, layer ordering, and supported background fill modes. Unsupported modes should be surfaced through diagnostics.
- Preview frame and preview segment caches must invalidate when transform/composition semantics change.

### Desktop UI Placement And Behavior
- The right inspector's selected segment state should expose `画面` tab content with a compact `基础 / 变换` section. It should not add another left-side primary menu.
- Controls should match the Jianying-style dense desktop workspace: compact rows, sliders plus numeric inputs, color swatches, toggles, icon keyframe placeholders, and restrained cyan active/focus state.
- When no segment is selected, the inspector continues to show `草稿参数`. When a video/image/text segment is selected, it shows the relevant transform controls. Deferred fields such as mask can stay visible with `未接入` or `不支持`.
- Timeline command behavior must remain unchanged: selected segment transforms update via Rust commands, not local draft mutation.
- Desktop UI proportions and scrollbars must keep the Phase 04.1/07 compact dark baseline and pass 1280x800 plus 1120x720 visibility checks.

### Testing And Gates
- Add Rust model/schema tests for defaults, validation, generated schema/TS drift, and invalid transform/composition values.
- Add command tests for Rust-owned transform edits, undo/redo, and atomic rejection.
- Add engine/render graph/compiler tests proving transform and layer composition affect deterministic frame/render outputs.
- Add Electron Playwright tests verifying Chinese inspector controls, command-only updates via `executeCommand`, stable 1280x800 and 1120x720 layout, and no duplicate left primary menu.
- Add Phase 08 source guards and wire them into the project test gate.

### the agent's Discretion
- The exact first renderable FFmpeg subset can be conservative if unsupported/degraded diagnostics are explicit.
- The planner may split the phase across schema/commands, engine/render graph/compiler, UI, and guard/gate plans.
- The planner may treat future sticker segments as schema/render boundaries if no sticker UI exists yet.

</decisions>

<canonical_refs>
## Canonical References

Downstream agents MUST read these before planning or implementing.

### Project And Requirements
- `.planning/PROJECT.md` — project objective, architecture constraints, terminology rules.
- `.planning/REQUIREMENTS.md` — XFORM-01/02/03 and LAYER-01/02/03 requirements.
- `.planning/ROADMAP.md` — Phase 08 goal, dependencies, success criteria.
- `.planning/STATE.md` — current phase and prior decisions.

### Prior Phase Foundation
- `.planning/phases/07-project-canvas-space-and-coordinate-system/07-CONTEXT.md` — normalized canvas coordinate contract and deferred Phase 08 scope.
- `.planning/phases/07-project-canvas-space-and-coordinate-system/07-RESEARCH.md` — canvas/profile implementation map and reference evidence.
- `.planning/phases/07-project-canvas-space-and-coordinate-system/07-VERIFICATION.md` — verified Phase 07 gates and residual deferred risks.

### Local Code And References
- `crates/draft_model/src/timeline.rs` — segment/track schema ownership point.
- `crates/draft_commands/src/timeline.rs` and `crates/draft_commands/src/history.rs` — command and undo/redo patterns.
- `crates/engine_core/src/frame_state.rs` and `crates/engine_core/src/normalize.rs` — normalized draft/frame state path.
- `crates/render_graph/src/graph.rs` and `crates/ffmpeg_compiler/src/filters.rs` — render intent and FFmpeg filter compilation path.
- `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` — selected segment inspector integration point.
- `apps/desktop-electron/tests/workspace.spec.ts` — desktop UI and command-only regression pattern.
- `reference/pyJianYingDraft/pyJianYingDraft/segment.py` — Jianying transform vocabulary evidence.
- `reference/pyJianYingDraft/pyJianYingDraft/video_segment.py` — background filling and video segment concept evidence.
- `reference/kdenlive` and `reference/mlt` — conceptual reference only for effects/compositing models; do not copy code, XML, assets, presets, or UI implementation.

</canonical_refs>

<specifics>
## Specific Ideas

- User explicitly corrected terminology: internal and external language should use Jianying concepts consistently, not map Jianying terms into invented project terms.
- User emphasized the project is a general video editor, not an oral-video product.
- User emphasized desktop UI should visually approach Jianying Pro but with original assets and no trade-dress copy.
- User specifically objected to extra left-side primary menus; top feature bar remains the primary navigation.
- User specifically called out scrollbars and proportions; Phase 08 UI must keep compact dark scrollbars and verify 1120x720/1280x800.

</specifics>

<deferred>
## Deferred Ideas

- Complete text/subtitle semantics, safe area, full font resources, text effects, and subtitle parity belong to Phase 09.
- Typed keyframes and animated values belong to Phase 10.
- Retiming/speed and reverse/curve speed belong to Phase 11.
- First-party filter/adjustment/effect parameter systems belong to Phase 12.
- Transition attachment, overlap, and render windows belong to Phase 13.
- Jianying/CapCut/Kaipai import/export adapters should consume these semantics later; they do not define internal render behavior in Phase 08.

</deferred>
