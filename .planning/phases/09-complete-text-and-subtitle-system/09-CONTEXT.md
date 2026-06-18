# Phase 9: Complete Text And Subtitle System - Context

**Gathered:** 2026-06-18
**Status:** Ready for research, UI design contract, and planning
**Mode:** Autonomous continuation to Phase 13

<domain>
## Phase Boundary

Phase 09 upgrades MVP text into a complete Jianying-style `文字 / 字幕` semantic model suitable for real templates. It builds on Phase 08 segment visual semantics, so text/subtitle placement and layer ordering should reuse `Segment.visual` for canvas transform while `Segment.text` owns text content, font, style, text box, line metrics, layout region, and unsupported text bubble/effect references.

This phase owns static text/subtitle semantics: font reference, font size, color, stroke, shadow, background, alignment, text box width/height, line height, letter spacing, safe-area/layout region, multiple text/subtitle segment rendering, and capability reports for proprietary text bubbles, 花字, and font resources.

This phase does not implement animated text/keyframes, typewriter animations, text entrance/outro/loop animation, speed/retiming, filters/effects, or transitions. Those remain Phases 10-13.

</domain>

<decisions>
## Implementation Decisions

### Jianying-Aligned Terms
- Product copy, Rust domain types, IPC commands, schema, docs, and tests should use Jianying-aligned concepts: `文字`, `字幕`, `文本轨道`, `文字片段`, `字体`, `样式`, `描边`, `阴影`, `背景`, `文本框`, `自动换行`, `安全区域`, `花字`, `气泡`.
- Do not introduce a separate internal product vocabulary such as `caption asset` or `text layer item`. Internal code may use Rust/TS-friendly English names, but they should map directly to Jianying concepts such as `TextSegment`, `Subtitle`, `TextStyle`, `TextFont`, `TextBox`, and `TextEffectRef`.
- `text` and `subtitle` share the same core segment semantics. `subtitle` is a usage/source classification over text segments, not a separate render pipeline.

### Text Schema
- Extend `Segment.text` instead of creating a parallel draft object. Text content remains on the segment; material entries may still use internal `text://` material refs.
- Add typed fields for font reference, text box, line height, letter spacing, wrapping, and layout region using integer persisted units. Do not persist naked floating-point time or renderer pixels as canonical semantics.
- Keep existing fields backward-compatible with current fixtures where possible by adding defaults. If a required semantic field is introduced, update fixtures and migration tests in the same plan.
- Proprietary `花字` / `气泡` / font IDs should be stored as external capability references only. They must not be treated as internally renderable unless the compiler actually supports them.

### Layout And Rendering
- `engine_core` owns deterministic text layout resolution. The renderer can display form state, but it must not compute canonical wrapping, text box layout, safe area, or subtitle timing semantics.
- `render_graph` should preserve text/subtitle intent and diagnostics without FFmpeg syntax.
- `ffmpeg_compiler` should compile the supported static subset through ASS sidecars: font, font size, primary color, stroke, shadow, background, alignment, margins, line breaks, and letter spacing where supported.
- Unsupported text effects, proprietary bubbles, unsupported font refs, or unsupported layout variants should produce explicit degraded/unsupported diagnostics rather than silent approximation.

### Subtitle Semantics
- Subtitle import should parse SRT into multiple text segments in Rust, preserving integer microsecond timing and applying a shared style/layout template.
- The desktop UI may pass selected file content or a path to a command boundary, but parsing and segment creation belong to Rust commands.
- Imported subtitles should use a text track named `字幕` or a user-selected text track, with command events identifying the batch import.

### Desktop UI
- The top feature bar remains the primary navigation. The left panel under `文字` can show compact secondary cards for `默认文字`, `花字`, `气泡`, and `导入字幕`, but it must not add another left-side primary menu.
- The right inspector should expose Jianying-style text tabs/sections with compact rows for content, font,字号,颜色,描边,阴影,背景,对齐,文本框,行高,字间距,安全区域/位置.
- Deferred `花字` and `气泡` controls remain visible with Chinese unsupported/degraded states.
- UI proportions, compact dark scrollbars, and no-overlap layout at 1280x800 and 1120x720 remain mandatory.

### Command Ownership
- Add or extend Rust-owned commands for text style/layout edits and subtitle SRT import. Renderer code may build generated envelopes only.
- Successful text/subtitle semantic edits should clear stale preview/export display state after Rust accepts the command.
- Source guards must block renderer-owned mutation of `segment.text`, text style/layout fields, subtitle parsing, render graph, FFmpeg/ASS sidecar generation, preview cache invalidation, and undo/redo semantics.

</decisions>

<canonical_refs>
## Canonical References

Downstream agents MUST read these before planning or implementing.

### Project And Requirements
- `.planning/PROJECT.md` — product objective, architecture constraints, terminology rules.
- `.planning/REQUIREMENTS.md` — TEXT2-01, TEXT2-02, TEXT2-03 requirements.
- `.planning/ROADMAP.md` — Phase 09 goal, dependencies, success criteria.
- `.planning/STATE.md` — current phase and prior decisions.

### Prior Phase Foundation
- `.planning/phases/08-segment-transform-and-visual-compositing/08-CONTEXT.md` — Segment.visual ownership and layer composition.
- `.planning/phases/08-segment-transform-and-visual-compositing/08-VERIFICATION.md` — verified transform/compositing gates and residual risks.
- `crates/draft_model/src/timeline.rs` — `TextSegment`, `TextStyle`, `Segment.visual`, and track/segment schema.
- `crates/draft_commands/src/text.rs` — existing Rust-owned text command pattern.
- `crates/engine_core/src/text_layout.rs` — current deterministic text layout profile and resolved overlay.
- `crates/ffmpeg_compiler/src/ass.rs` — current ASS sidecar generation path.
- `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` — existing selected text inspector surface.
- `apps/desktop-electron/tests/workspace.spec.ts` — command-only and layout regression patterns.

### Reference Projects
- `reference/pyJianYingDraft/README.md` — conceptual evidence for `TextSegment`, `TextStyle`, font/style/clip settings, auto wrapping, SRT import, and style references.
- `reference/pyJianYingDraft/tests/test_import_srt.py` — conceptual evidence for SRT import creating text tracks/segments and applying style references.
- `reference/mlt/src/modules/plus/subtitles` — conceptual evidence for separating subtitle parsing from text drawing; do not copy code.
- `reference/kdenlive/src/timeline2` — conceptual evidence for subtitle timeline items, editing, move/resize, and model separation; do not copy GPL code or UI.

</canonical_refs>

<specifics>
## Specific Ideas

- User explicitly wants internal and external terminology to stay aligned with Jianying, not a translation table into invented internal names.
- User emphasized desktop language is Chinese.
- User emphasized visual similarity to Jianying Pro should preserve original resources and open-source originality.
- User objected to duplicate left primary menus; keep top feature navigation as the primary level.
- User called out scrollbar and proportion issues; every UI change should keep compact dark scrollbars and desktop viewport checks.

</specifics>

<deferred>
## Deferred Ideas

- Text/keyframe animation, entrance/outro/loop effects, typewriter effects, and animated text parameters belong to Phase 10.
- Speed/retiming impacts on subtitles belong to Phase 11.
- First-party filter/effect semantics belong to Phase 12.
- Transitions belong to Phase 13.
- Full proprietary Jianying text bubble/花字 parity belongs to compatibility/adapters later; Phase 09 only records capability-aware references.

</deferred>
