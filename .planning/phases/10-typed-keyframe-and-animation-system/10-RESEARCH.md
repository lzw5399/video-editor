# Phase 10: Typed Keyframe And Animation System - Research

**Researched:** 2026-06-18
**Status:** Ready for planning

## Summary

Phase 10 should replace the current `Keyframe { at, property: String, value: String }` placeholder with Rust-owned typed keyframe semantics. The most compatible model is still segment-attached keyframes: each keyframe is expressed at a time relative to the segment head, targets a constrained property, stores a typed value, and declares interpolation/easing. Static Phase 08/09 fields remain base values; keyframes override those fields at frame evaluation time.

The implementation should be a vertical semantic slice:

1. Draft schema and generated contracts.
2. Rust commands with undo/redo and validation.
3. Engine frame-time evaluation and render graph propagation.
4. Desktop inspector/timeline UI that only sends commands.
5. Source guards and executable gates.

## Existing Code Shape

### Draft Model

`crates/draft_model/src/timeline.rs` currently defines:

- `Segment.visual.transform.position/scale/rotation/opacity/crop/anchor`
- `Segment.text` with static text style, text box, layout region, wrapping, unsupported bubble/effect refs
- `Segment.volume.level_millis`
- `Segment.keyframes: Vec<Keyframe>`
- `Keyframe { at: Microseconds, property: String, value: String }`

The placeholder keyframe type is not enough for ANIM-01/02 because property and value are arbitrary strings and there is no interpolation/easing.

### Validation

`crates/draft_model/src/validation.rs` validates segment visual, text, and volume fields directly. It only checks keyframe string fields for non-empty `property` and `value`. Phase 10 needs validation for:

- keyframe time offset is within the segment target duration
- property is supported for the segment/track/material kind
- typed value variant matches property
- interpolation/easing are allowed
- duplicate keyframes for the same property/time are rejected or normalized deterministically
- keyframe values obey the same constraints as static visual/text/volume fields

### Commands

Existing command patterns:

- `draft_commands::visual::update_segment_visual`
- `draft_commands::text::edit_text_segment`
- `draft_commands::audio::set_segment_volume`

All clone the draft, locate a segment, validate track lock and draft rules, push undo snapshot, preserve or update selection, and emit one command event. Phase 10 should add a dedicated `keyframe` command module rather than overloading visual/text/audio commands.

Recommended commands:

- `setSegmentKeyframe`: add or replace a keyframe for a segment property at a segment-relative time.
- `removeSegmentKeyframe`: remove a keyframe for a segment property at a segment-relative time.
- `updateSegmentKeyframeTiming`: optional if UI needs drag/move keyframe markers in this phase.
- `updateSegmentKeyframeCurve`: optional if interpolation/easing can be changed separately from value.

For MVP execution, the first two commands plus curve fields in `setSegmentKeyframe` are enough.

### Engine Core

`engine_core::normalize` currently copies `segment.keyframes` into `NormalizedSegment`. `frame_state.rs` resolves active segments at timeline time and emits:

- `FrameVisualLayer.visual: SegmentVisual`
- `FrameAudioSegment.volume_level_millis`
- `ResolvedTextOverlay`

This is the correct insertion point for animation evaluation. `resolve_frame_state` should compute segment-relative time:

```text
relative = at - segment.target_timerange.start
```

Then it should resolve keyframes against the static base values and emit the resolved visual/text/volume state. Renderer and preview/export must consume those resolved outputs instead of interpolating locally.

### Render Graph And Compiler

`render_graph::graph` currently preserves `keyframes` on `RenderVideoLayer` and uses resolved `SegmentVisual`, text overlays, and volume values. Phase 10 can:

- preserve typed `keyframes` as render intent for diagnostics and future compiler support
- use engine-resolved sampled frame states for deterministic preview/export tests
- add diagnostics for keyframe domains not yet compiled into FFmpeg expressions

`ffmpeg_compiler` should not decide editing behavior. For Phase 10, compiler support can be conservative: compile or snapshot only a subset if already practical, and report unsupported animation intent deterministically for the rest.

## Jianying-Aligned Concepts

Local `reference/pyJianYingDraft` provides useful vocabulary evidence:

- Keyframes are segment-attached "时刻-数值" pairs.
- Time offset is relative to the segment head and measured in microseconds.
- Keyframes override overall/static clip settings.
- Visual segment keyframe properties include position X/Y, rotation, scale X/Y, uniform scale, alpha, saturation/contrast/brightness, and volume.
- Text and sticker keyframes use the same mechanism but are limited to position/size-related properties.
- Audio keyframes default to volume only.
- pyJianYingDraft notes that effect/filter parameter keyframes are not fully supported.

This supports the Phase 10 model:

- Use `关键帧` as the visible and internal concept.
- Use segment-relative `at` or `timeOffset` semantics.
- Keep static `画面调节 / clip_settings` as base values.
- Start with first-party typed keyframes for transform, text, and volume.
- Represent filter/sticker/effect parameter animation boundaries without pretending full support.

Conceptual Kdenlive/MLT references reinforce a separation of concerns: effect/filter metadata can declare whether a parameter is animatable, but timeline editing semantics should not be embedded in UI strings or FFmpeg syntax. MLT YAML marks parameters with `animation: yes`, which is useful conceptually for future Phase 12 capability reporting, not as code to copy.

## Recommended Model

### Types

Prefer typed property and value enums:

```rust
pub struct Keyframe {
    pub at: Microseconds,
    pub property: KeyframeProperty,
    pub value: KeyframeValue,
    pub interpolation: KeyframeInterpolation,
    pub easing: KeyframeEasing,
}

pub enum KeyframeProperty {
    VisualPositionX,
    VisualPositionY,
    VisualScaleX,
    VisualScaleY,
    VisualRotation,
    VisualOpacity,
    TextFontSize,
    TextColor,
    TextLineHeight,
    TextLetterSpacing,
    TextLayoutX,
    TextLayoutY,
    TextLayoutWidth,
    TextLayoutHeight,
    Volume,
    StickerPositionX,
    StickerPositionY,
    StickerScaleX,
    StickerScaleY,
    FilterParameterUnsupported { name: String },
}

pub enum KeyframeValue {
    Int(i32),
    UInt(u32),
    Color(String),
}

pub enum KeyframeInterpolation {
    Hold,
    Linear,
}

pub enum KeyframeEasing {
    None,
    EaseIn,
    EaseOut,
    EaseInOut,
}
```

Notes:

- Do not use `f32`/`f64` in persisted semantics. If easing needs fractional math internally, keep it runtime-local and return integer-rounded values deterministically.
- Color interpolation can be `Hold` initially unless a deterministic channel interpolation is explicitly implemented and tested.
- For scale/opacity/volume use existing millis units.
- For visual position, use existing integer position units from Phase 08.
- For text layout use existing millesimal layout units.

### Property Applicability

Initial support matrix:

| Property Group | Phase 10 Support |
|----------------|------------------|
| Visual position X/Y | Supported |
| Visual scale X/Y | Supported |
| Visual rotation | Supported for engine/render graph; compiler may diagnose if render support is incomplete |
| Visual opacity | Supported |
| Text font size | Supported |
| Text color | Supported, hold or deterministic channel interpolation |
| Text line height / letter spacing | Supported |
| Text layout X/Y/width/height | Supported |
| Audio volume | Supported |
| Sticker position/scale | Schema/capability shell if sticker static semantics are not complete |
| Filter/effect parameters | Unsupported/deferred capability refs until Phase 12 |
| Transitions | Deferred to Phase 13 |
| Speed/retiming | Deferred to Phase 11 |

### Evaluation

Evaluation should:

1. Group keyframes by property.
2. For each supported property, find previous and next keyframe around segment-relative time.
3. Before first keyframe, use the first keyframe value or static base value. Pick one behavior explicitly and test it. Recommendation: use static base until first keyframe; after last keyframe, hold last keyframe.
4. If exact keyframe time, use that value.
5. For `Hold`, use previous value.
6. For `Linear`, interpolate numeric values using integer rational math where possible.
7. Apply easing to progress before interpolation. Internally use deterministic integer or rational helper; do not persist floats.
8. Validate the resolved value with the same range rules as static fields.

The base-before-first behavior must be documented because many editors treat animation as holding nearest keyframe; this project should choose the easier-to-explain behavior for templates and verify snapshots.

## UI Implications

`Inspector.tsx` already has disabled `KeyframeButton` placeholders and an `动画` tab. Phase 10 should:

- turn placeholders into 28px command-owned buttons
- show active-at-playhead, has-keyframes-elsewhere, pending, error, and unsupported states
- expose selected segment keyframes in the `动画` tab
- render accepted keyframe markers in timeline segment blocks
- keep all copy Simplified Chinese
- keep no duplicate left primary menu
- preserve compact dark scrollbars and viewport stability at 1280x800 and 1120x720

Renderer can derive display-only counts and marker positions from accepted draft data, but it must not interpolate values or mutate accepted keyframe arrays.

## Tests And Gates

### Rust

- `cargo test -p draft_model keyframe -- --nocapture`
- `cargo test -p draft_commands keyframe -- --nocapture`
- `cargo test -p bindings_node keyframe_commands -- --nocapture`
- `cargo test -p engine_core keyframe -- --nocapture`
- `cargo test -p render_graph keyframe -- --nocapture`
- targeted `ffmpeg_compiler` tests for diagnostics or supported subset

### Contracts

- `cargo test -p draft_model schema_exports -- --nocapture`
- `git diff --exit-code schemas apps/desktop-electron/src/generated`

### Desktop

- `pnpm --filter @video-editor/desktop build`
- `pnpm --filter @video-editor/desktop test:workspace -g "关键帧|动画|command-only keyframe|五大区域"`

### Source Guard

Create `scripts/phase10-source-guards.sh` extending Phase 09:

- require keyframe types and commands in generated contracts
- require Chinese keyframe/animation copy in UI/tests
- reject renderer direct mutation of `segment.keyframes`
- reject renderer interpolation/easing/frame-time animation evaluation
- reject renderer-owned render graph/FFmpeg/cache/export semantics
- reject naked persisted float time in Rust/schema/generated contracts

### Final Gates

- `pnpm run test:phase10`
- `pnpm run test`
- `/Users/zhiwen/.cargo/bin/just test`
- `/Users/zhiwen/.cargo/bin/just build`
- generated contract drift check

## Risks

- Migrating the old string keyframe schema can break fixtures. Mitigate with defaults/migration tests or update fixtures intentionally.
- If color interpolation is over-scoped, UI/render parity can slip. Hold interpolation for color is acceptable if documented.
- Rotation compiler support is already limited in Phase 08. Engine/render graph can still resolve rotation while compiler emits an unsupported/degraded diagnostic until a later render plan.
- Sticker/filter parameter animation overlaps Phase 12. Phase 10 should define typed boundaries and avoid full filter semantics.
- Timeline marker UI can easily cause layout instability. Markers should be overlaid inside existing segment blocks and hidden for very narrow segments.

## Planning Recommendation

Use five plans:

1. Typed keyframe schema, validation, generated contracts.
2. Rust command and binding route.
3. Engine/render graph/compiler evaluation and diagnostics.
4. Desktop keyframe inspector/timeline UI.
5. Source guards, package scripts, full verification, summaries.

Wave order should keep schema first, then commands, then engine/render, then UI, with verification closure last.
