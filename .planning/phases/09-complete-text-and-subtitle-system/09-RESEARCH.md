# Phase 09: Complete Text And Subtitle System - Research

**Researched:** 2026-06-18
**Domain:** Jianying-style text/subtitle semantics, deterministic text layout, ASS rendering, Rust command ownership
**Confidence:** HIGH

## Summary

Phase 09 should extend the existing text model rather than replacing it. The repo already has `TextSegment`, `TextStyle`, `addTextSegment`, `editTextSegment`, deterministic `engine_core::text_layout`, render graph text overlays, and FFmpeg ASS sidecars. The missing pieces are the complete text/subtitle semantics required by templates: font refs, text box dimensions, line height, letter spacing, layout region/safe area, batch subtitle import, and explicit degraded/unsupported reports for proprietary text resources.

The recommended implementation is:

- Keep `Segment.text` as the canonical text/subtitle semantic container.
- Add typed integer fields with defaults so existing MVP text drafts migrate smoothly.
- Represent `字幕` as a text segment usage/source classification and batch command, not a separate render pipeline.
- Propagate new fields through engine frame state, render graph, ASS sidecars, preview/export, and desktop inspector.

## Reference Findings

### pyJianYingDraft

`reference/pyJianYingDraft/README.md` treats text like other timeline segments: create `TextSegment`, add it to a track, and control font, style, and image adjustment settings. It exposes `TextStyle` for size/color/underline/alignment, supports auto wrapping via `auto_wrapping` and `max_line_width`, and imports SRT by creating text segments on a subtitle track.

`reference/pyJianYingDraft/tests/test_import_srt.py` shows useful product behavior: import creates a missing text track, preserves subtitle timing, applies a time offset, and can inherit style/clip settings from a reference text segment. We should borrow the behavior concept, not code.

Useful vocabulary to align:

| Jianying / pyJianYingDraft | Internal Concept |
| --- | --- |
| 文本 / 文字 | `TextSegment` |
| 字幕 | text segment with `source=subtitle` / subtitle import command |
| 文本轨道 / 字幕轨道 | `TrackKind::Text` with Chinese track name |
| 字体 | `TextFont` / font reference |
| 文字样式 | `TextStyle` |
| 自动换行 | wrapping/layout policy |
| 最大行宽 | text box width / layout region width |
| 文字气泡 | external text bubble capability ref |
| 花字效果 | external text effect capability ref |
| 图像调节 / clip_settings | `Segment.visual` transform from Phase 08 |

### Kdenlive / MLT

Kdenlive and MLT should remain conceptual references only. Their subtitle code demonstrates useful boundaries:

- subtitle data is separate from text rendering mechanics;
- subtitle items have start/end/text and are edited as timeline items;
- SRT parsing/import is model/service work, not UI drawing logic;
- the rendering layer applies a text filter after resolving active subtitle text.

The relevant design lesson is separation: draft/timeline semantics decide what text exists and when; engine/render graph decide deterministic resolved overlay state; compiler/runtime decide how to render the supported subset.

## Codebase Map

| Area | Current State | Phase 09 Change |
| --- | --- | --- |
| `crates/draft_model/src/timeline.rs` | `TextSegment { content, style }`; `TextStyle` has font size, color, alignment, stroke, shadow, background. | Add font reference, segment source/type, text box, layout region, line height, letter spacing, wrapping, bubble/effect capability refs. |
| `crates/draft_model/src/validation.rs` | Validates non-empty content, font size, color strings, stroke width. | Validate font refs, integer layout ranges, hex colors, style refs, unsupported refs, line height/letter spacing limits. |
| `crates/draft_commands/src/text.rs` | Add/edit text segment and undo/redo. | Add complete text update command shape or extend edit command tests; add subtitle SRT import command in Rust. |
| `crates/engine_core/src/text_layout.rs` | Uses profile-level safe area and line height; layout width is canvas minus safe area. | Resolve segment-level text box/layout region, line height, letter spacing, wrapping, font ref, and unsupported diagnostics. |
| `crates/render_graph/src/graph.rs` | Preserves `FrameTextOverlay`. | Preserve new text layout/style intent and diagnostics without FFmpeg syntax. |
| `crates/ffmpeg_compiler/src/ass.rs` | Generates ASS sidecars with font, color, stroke, shadow, background, alignment, margins. | Add letter spacing, text box margins/regions, layout-derived margins, and explicit diagnostics for unsupported bubbles/effects/fonts. |
| `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` | Compact text controls for content, font size, color, alignment, stroke, shadow, background. | Add font, text box, line height, letter spacing, safe area/layout controls, unsupported 花字/气泡 rows, subtitle source display. |
| `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx` | `文字` page has add-text controls; deferred categories visible. | Add `导入字幕` visible shell/command route and compact text/subtitle cards while preserving no duplicate primary left menu. |
| `apps/desktop-electron/tests/workspace.spec.ts` | Tests existing text commands and layout. | Add command-only complete text edit, SRT import, text inspector UI, and viewport layout checks. |

## Recommended Phase Split

1. Text schema and Rust command contracts.
2. Engine/render graph/ASS propagation.
3. Subtitle SRT import and multi-segment render parity.
4. Desktop text/subtitle inspector and feature panel UI.
5. Phase 09 source guards, public gates, verification, and docs.

## Risks

- Adding required fields without defaults would break existing fixtures and demo draft. Use defaults or update migration/fixtures atomically.
- Text wrapping can be platform-font dependent. Keep deterministic layout metadata and use pinned font paths/candidates.
- ASS supports many text features but not all Jianying proprietary bubbles/effects. Unsupported must be explicit.
- Renderer-side SRT parsing or text box layout would violate project architecture. Keep parsing/layout in Rust.
- Adding too many style features can spill into Phase 10 animation/effects. Keep Phase 09 static.

## Required Gates

- `cargo test -p draft_model text -- --nocapture`
- `cargo test -p draft_commands text -- --nocapture`
- `cargo test -p bindings_node text_commands -- --nocapture`
- `cargo test -p engine_core text -- --nocapture`
- `cargo test -p render_graph text -- --nocapture`
- `cargo test -p ffmpeg_compiler text -- --nocapture`
- `pnpm --filter @video-editor/desktop test:workspace -g "文字|字幕|command-only text|五大区域"`
- `pnpm run test:phase9`
- `pnpm run test`
- `/Users/zhiwen/.cargo/bin/just test`
- `/Users/zhiwen/.cargo/bin/just build`

## RESEARCH COMPLETE
