# Phase 10: Typed Keyframe And Animation System - Context

**Gathered:** 2026-06-18
**Status:** Ready for research and planning
**Source:** GSD continuation after Phase 09 verification

<domain>
## Phase Boundary

Phase 10 upgrades the existing segment `keyframes` placeholder into a typed Jianying-style `关键帧 / 动画` system. The phase must support deterministic animated values for segment 画面 transform, opacity, text parameters, sticker/filter parameter placeholders where applicable, and audio volume. It builds on Phase 08 static `Segment.visual` and Phase 09 static `Segment.text`; it does not replace those static defaults.

The editor remains a general desktop video editor. This phase is not about oral-video automation, AI talking-head workflows, or proprietary Jianying effect parity.

</domain>

<decisions>
## Locked Decisions

### Terminology
- Product copy, Rust domain types, IPC commands, schemas, tests, and docs should use Jianying-aligned terms: `draft`, `material`, `track`, `segment`, `keyframe`, `animation`, `filter`, `transition`, `画面`, `基础`, `动画`, `关键帧`, `文本`, `音频`.
- Avoid inventing parallel internal names such as `asset`, `clip`, `layer item`, `motion point`, or UI-only animation vocabulary when Jianying terms already cover the concept.

### Ownership
- Rust owns keyframe storage, validation, interpolation/easing semantics, undo/redo, and timeline command behavior.
- Renderer may only construct generated command envelopes and call `window.videoEditorCore.executeCommand`.
- Renderer must not mutate `draft.tracks`, `track.segments`, `segment.keyframes`, timeranges, undo/redo stacks, render graphs, FFmpeg commands, preview/export cache semantics, or calculate persisted animation results.

### Data Model
- Keep static Phase 08/09 fields as default/base values. Keyframes should express animated overrides for those fields rather than replacing the static model.
- Persisted time must use integer microseconds, frame indices, or rational frame rates. Do not persist naked floating-point time.
- Animated values must be typed. Property references must be constrained to supported Jianying-style segment properties instead of arbitrary strings.
- Keyframes must include time, typed value, interpolation policy, and easing curve.
- Unsupported/deferred animated domains, including proprietary 花字/气泡/effect animation and future filter/sticker details, must be represented explicitly as unsupported/deferred capability boundaries rather than faked support.

### Evaluation And Rendering
- `engine_core` must evaluate animated values at frame time and emit resolved frame state for preview/export.
- `render_graph` must preserve animation intent and/or resolved sampled frame intent without exposing FFmpeg syntax as editing semantics.
- `ffmpeg_compiler` may implement a small supported subset first, but unsupported animation intent must be diagnostic and deterministic.
- Preview and export must share the same normalized draft -> frame state -> render graph path.

### UI
- Desktop UI remains Simplified Chinese and Jianying-style.
- The top feature bar remains the primary navigation. Do not reintroduce a duplicate left-side primary menu.
- Existing disabled keyframe placeholders in the inspector should become compact command-owned controls where Phase 10 scope allows.
- The right inspector should expose `动画` and inline keyframe controls for selected segment properties while keeping stable 1120x720 and 1280x800 layouts, compact scrollbars, and no overlap/cutoff.

### Testing
- Phase 10 is not complete without executable gates across schema, command, engine, render graph/compiler, generated contract drift, Electron UI, and source guards.
- Source guards must extend Phase 09 guards to block renderer-owned keyframe mutation and animation interpolation.

</decisions>

<canonical_refs>
## Canonical References

Downstream agents MUST read these before planning or implementation.

### Roadmap And Requirements
- `.planning/ROADMAP.md` - Phase 10 goal, dependency, requirements, and success criteria.
- `.planning/REQUIREMENTS.md` - ANIM-01, ANIM-02, ANIM-03 and cross-phase constraints.
- `AGENTS.md` - project architecture, terminology, time model, testing, and GSD workflow constraints.

### Prior Phase Context
- `.planning/phases/08-segment-transform-and-visual-compositing/08-CONTEXT.md` - static visual transform ownership and Phase 10 deferral.
- `.planning/phases/08-segment-transform-and-visual-compositing/08-VERIFICATION.md` - verified transform/render baseline and residual risks.
- `.planning/phases/09-complete-text-and-subtitle-system/09-CONTEXT.md` - text/static semantics and Phase 10 text animation deferral.
- `.planning/phases/09-complete-text-and-subtitle-system/09-VERIFICATION.md` - verified text/render/binding baseline.
- `.planning/phases/09-complete-text-and-subtitle-system/09-UI-SPEC.md` - current inspector density and keyframe placeholder expectations.

### Rust Core And Contracts
- `crates/draft_model/src/timeline.rs` - current `Keyframe` placeholder, segment visual/text/volume schemas.
- `crates/draft_model/src/validation.rs` - draft validation patterns for timeranges, visual, text, and volume constraints.
- `crates/draft_model/src/lib.rs` - command envelope and generated contract surface.
- `crates/draft_commands/src/visual.rs` - update visual command pattern and undo snapshot behavior.
- `crates/draft_commands/src/text.rs` - edit text command pattern and Rust-owned import behavior.
- `crates/draft_commands/src/audio.rs` - segment volume command pattern.
- `crates/engine_core/src/normalize.rs` - normalized segment fields that currently pass keyframes through.
- `crates/engine_core/src/frame_state.rs` - frame-time evaluation point for visual/audio/text state.
- `crates/render_graph/src/graph.rs` - render intent graph that currently preserves keyframes and resolved visual/text/audio state.
- `crates/ffmpeg_compiler/src/filters.rs` and `crates/ffmpeg_compiler/src/job.rs` - compiler diagnostic/supported/degraded output patterns.

### Desktop UI
- `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` - inspector tabs, disabled keyframe placeholders, visual/text/audio controls.
- `apps/desktop-electron/src/renderer/workspace/Timeline.tsx` - track/segment display and command-only timeline integration.
- `apps/desktop-electron/src/renderer/commandHelpers.ts` - generated command envelope helper pattern.
- `apps/desktop-electron/tests/workspace.spec.ts` - Playwright command-spy and layout guard patterns.
- `scripts/phase9-source-guards.sh` - current renderer semantic/source guard baseline.

### Conceptual References
- `reference/pyJianYingDraft` - local reference for Jianying naming and concepts only.
- `reference/kdenlive` and `reference/mlt` - conceptual editing/rendering references only; do not copy GPL code, UI implementation, XML definitions, assets, presets, or effect code.

</canonical_refs>

<specifics>
## Implementation Shape To Explore

- Replace or migrate current `Keyframe { at, property: String, value: String }` into typed keyframe structures.
- Candidate model: `SegmentKeyframes` or a typed vector where each keyframe has `at`, `property`, `value`, `interpolation`, and `easing`.
- Candidate properties include visual transform position x/y, scale x/y, rotation, opacity, text font size/color/layout values, and volume. Sticker/filter parameter animation can start as typed capability shells if the underlying static parameter systems are deferred to Phase 12.
- Commands should include adding/updating/removing a keyframe and selecting/toggling keyframe state if needed. Undo/redo must follow existing command patterns.
- Engine evaluation should produce resolved `SegmentVisual`, resolved text style/layout, and resolved volume for a given timeline frame.
- UI should show keyframe buttons as active/inactive controls, a compact `动画` tab, and timeline marker shell. It should not persist animation locally.

</specifics>

<deferred>
## Deferred Ideas

- Curved speed/retiming belongs to Phase 11.
- First-party filter/adjustment/effect parameter systems belong to Phase 12, except for typed placeholder capability boundaries needed by ANIM-01.
- Transition relationships and transition animation windows belong to Phase 13.
- Full proprietary Jianying effect/keyframe parity, 花字/气泡 animation, mobile clients, and cloud/server rendering are out of this phase.
- GPU real-time animation preview is deferred; Phase 10 should prioritize deterministic core semantics and shared preview/export evaluation.

</deferred>

---

*Phase: 10-typed-keyframe-and-animation-system*
*Context gathered: 2026-06-18 via GSD continuation*
