# Phase 09: Complete Text And Subtitle System - UI Spec

**Created:** 2026-06-18
**Status:** Approved for planning

## UI Boundary

Phase 09 updates the existing professional desktop workspace only. It must keep the top feature bar as the primary navigation, the left resource/function panel as contextual content, the center preview shell, the right inspector, and the bottom timeline. Do not add a second left primary menu.

## Left Panel Contract

When the top feature is `文字`, the left panel shows compact Jianying-style secondary content:

- `默认文字` for adding a normal text segment.
- `字幕` for importing SRT and showing subtitle-oriented empty/ready states.
- `花字` and `气泡` visible as deferred/unsupported capability cards.
- Search/filter/sort controls may remain display-only if not backed by commands.

Deferred `花字` and `气泡` must remain visible with Chinese copy such as `暂未接入` / `导入后将以不支持能力报告显示`.

## Inspector Contract

When a text/subtitle segment is selected, the right inspector uses compact sections:

- `文本`: 文字内容, 字幕来源, 字体, 字号, 颜色.
- `样式`: 描边, 阴影, 背景, 对齐.
- `文本框`: 宽度, 高度, 自动换行, 行高, 字间距.
- `布局`: 安全区域 / 布局区域 controls using the existing canvas coordinate language.
- `花字/气泡`: visible unsupported/degraded rows.

Controls should use sliders plus numeric inputs, color swatches, segmented alignment controls, toggles, and disabled keyframe placeholders. Labels must remain legible at 1120x720.

## Interaction Contract

- Every semantic edit calls `window.videoEditorCore.executeCommand` through generated command helpers.
- Renderer may keep local form state only; accepted `draft` updates must come back from Rust command responses.
- SRT parsing and multi-segment creation are Rust-owned. Renderer does not parse SRT into segments.
- Successful text/subtitle edits clear stale preview/export display state after Rust accepts the command.
- Invalid text/layout values show a Chinese error and must not partially mutate draft state.

## Visual Style

- Continue the compact dark Jianying-style workspace.
- Keep restrained cyan active state; avoid large cyan fills.
- Use compact dark scrollbars, not default light scrollbars.
- Avoid nested cards, decorative gradients, oversized headings, and marketing copy.
- All user-visible copy is Simplified Chinese.

## Playwright Contract

At 1280x800 and 1120x720:

- Top feature bar, left panel, preview, inspector, and timeline are visible and non-overlapping.
- Selecting a text/subtitle segment reveals `文本`, `样式`, `文本框`, and `布局` controls.
- At least one text style/layout edit is observed through `window.videoEditorCore.executeCommand`.
- SRT import command path can be exercised through a test fixture or test-mode mock without renderer parsing.
- The left panel has no duplicate primary menu.

## UI-SPEC VERIFIED
