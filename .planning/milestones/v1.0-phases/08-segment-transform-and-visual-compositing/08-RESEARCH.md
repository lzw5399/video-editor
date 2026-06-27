# Phase 08: Segment Transform And Visual Compositing - Research

**Researched:** 2026-06-18
**Domain:** Jianying-style segment `画面 / 基础 / 变换`, background filling, layer composition, Rust command ownership
**Confidence:** HIGH

## Summary

Phase 08 should add static, typed segment visual semantics to `draft_model::Segment`, route all edits through Rust commands, propagate transform/layer data through `engine_core::FrameState`, preserve composition intent in `render_graph`, compile a conservative FFmpeg subset, and expose compact Chinese controls in the right inspector. This phase should not build the full keyframe system; it should make the fields keyframe-ready for Phase 10.

The safest first implementation is:

- Use integer typed units for persisted visual math so schema/contracts remain `Eq`-friendly and time rules stay clean.
- Store normalized segment position in canvas-half units as millis: `x = 1000` is right edge, `y = 1000` is top edge.
- Store scale/opacity/crop/anchor as millis/permille-style typed values.
- Store fit/fill/stretch/background filling/blend/mask as enums with explicit supported/degraded/unsupported capability boundaries.

## Reference Findings

### pyJianYingDraft

`reference/pyJianYingDraft/pyJianYingDraft/segment.py` defines `ClipSettings` with `alpha`, `flip`, `rotation`, `scale`, and `transform`. Its comments say `transform_x` is horizontal displacement in half canvas width and `transform_y` is vertical displacement in half canvas height. This directly matches Phase 07's normalized canvas coordinate contract.

`reference/pyJianYingDraft/pyJianYingDraft/segment.py` also treats visual segments as the shared base for video, sticker, and text and records `uniform_scale`. Keyframes are attached later through `KeyframeProperty` values such as rotation, scale, and alpha; Phase 08 should not keep using the current string keyframe placeholder as the real future model.

`reference/pyJianYingDraft/pyJianYingDraft/video_segment.py` defines `BackgroundFilling` with `canvas_blur` and `canvas_color`, plus `MixMode` and `Mask`. `add_background_filling()` is segment-level and documents that background filling applies to lower video-track clips. This supports Phase 08 owning per-segment `backgroundFilling` separately from Phase 07 draft-level canvas background.

### Kdenlive / MLT

Kdenlive and MLT are useful only as conceptual references. Their local sources show a separation between timeline/project model, effects/composition services, and render/runtime execution. MLT examples and module metadata show transform/composite/affine/opacity concepts, but this project must not copy GPL implementation, XML, presets, or UI.

The relevant design lesson is the boundary: editing semantics describe visual intent, render graph preserves typed intent, and the FFmpeg/runtime layer compiles the supported subset. Unsupported compositor details remain diagnostics/capability reports.

## Codebase Map

| Area | Current State | Phase 08 Change |
| --- | --- | --- |
| `crates/draft_model/src/timeline.rs` | `Segment` has timeranges, string `Keyframe`, `Filter`, `Transition`, optional `TextSegment`, and volume. | Add `SegmentTransform`, `SegmentFitMode`, `SegmentBackgroundFilling`, `SegmentBlendMode`, `SegmentMask`, and visibility semantics. |
| `crates/draft_model/src/validation.rs` | Validates timers, canvas, IDs, text, volume. | Validate transform ranges, crop bounds, opacity/scale limits, blend/mask unsupported metadata, background filling color/material references. |
| `crates/draft_commands/src/timeline.rs` | Routes timeline/text/audio/canvas commands. | Route transform/composition commands through the same `TimelineCommandResponse` pattern. |
| `crates/draft_commands/src/history.rs` | Owns undo/redo snapshots. | Reuse for transform edit undo/redo. |
| `crates/engine_core/src/normalize.rs` | Normalized segments omit transform and visual visibility. Visual stack is track-order only. | Copy validated segment transform/composition into `NormalizedSegment` and add diagnostics for degraded/unsupported modes. |
| `crates/engine_core/src/frame_state.rs` | `FrameVisualLayer` has stack/source/timerange only. | Include transform, fit mode, background filling, blend/mask support, and visibility-derived omission. |
| `crates/render_graph/src/graph.rs` | `RenderVideoLayer` preserves filters/transitions, no transform. | Preserve visual transform/composition intent without FFmpeg syntax. |
| `crates/ffmpeg_compiler/src/filters.rs` | Scales every video to output dimensions, overlays at `0:0`, and supports canvas background color. | Compile the supported subset: fit/fill/stretch sizing, crop, scale, normalized position overlay, opacity, and deterministic layer order. Rotation may be supported conservatively or diagnosed if incomplete. |
| `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` | Selected `画面` tab shows static shell values. | Add compact editable controls for `位置`, `缩放`, `旋转`, `不透明度`, `适应方式`, `背景填充`; route via generated command helper. |
| `apps/desktop-electron/tests/workspace.spec.ts` | Tests command bridge, layout, canvas settings. | Add transform command regression and 1280x800/1120x720 no-overlap checks. |
| `scripts/phase7-source-guards.sh` | Guards canvas/render ownership. | Add Phase 08 guard for renderer-owned segment transform/composition and FFmpeg/render semantics. |

## Recommended Phase Split

1. Model and command contracts: add typed segment visual semantics, generated schema/TS, command payloads, validation, undo/redo tests.
2. Engine and render graph: propagate transform/layer semantics into frame state and graph, add diagnostics and snapshots.
3. FFmpeg compiler and preview/export invalidation: compile conservative placement/scale/crop/opacity subset and invalidate derived preview/export state after transform edits.
4. Desktop inspector UI: replace selected segment shell values with Chinese editable compact controls that call generated commands.
5. Guards and gates: add source guard, Playwright coverage, package scripts, and final verification docs.

## Risks

- `f64` persisted values would force broad removal of `Eq` derives from `Draft`, command payloads, and generated responses. Use integer millis for Phase 08.
- Rotation around arbitrary anchors can make FFmpeg filters complex. It is acceptable to support simple rotation or classify advanced anchor/rotation cases as degraded if diagnostics are explicit.
- Current compiler scales all visual layers to output dimensions. Introducing fit/fill defaults can change existing snapshots. To preserve MVP behavior, default `fitMode` should be `stretch`, while UI can expose `适应`/`填充`/`拉伸`.
- Text overlays currently render through ASS sidecars separately from video layers. Phase 08 should carry transform semantics for text in the model/render graph but may leave full text transform rendering to Phase 09 if clearly diagnosed.

## Required Gates

- `cargo test -p draft_model transform -- --nocapture`
- `cargo test -p draft_commands transform -- --nocapture`
- `cargo test -p bindings_node transform_commands -- --nocapture`
- `cargo test -p engine_core transform -- --nocapture`
- `cargo test -p render_graph transform -- --nocapture`
- `cargo test -p ffmpeg_compiler transform -- --nocapture`
- `pnpm --filter @video-editor/desktop test:workspace -g "画面变换|command-only transform|五大区域"`
- `pnpm run test:phase8`
- `pnpm run test`
- `/Users/zhiwen/.cargo/bin/just test`
- `/Users/zhiwen/.cargo/bin/just build`

## RESEARCH COMPLETE
