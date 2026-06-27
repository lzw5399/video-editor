# Phase 08: Segment Transform And Visual Compositing - UI Spec

**Created:** 2026-06-18
**Status:** Approved for planning

## UI Boundary

Phase 08 updates only the existing professional desktop workspace. The first screen remains a Jianying-style editor with top feature entries, left resource/function panel, center preview, right inspector, and bottom timeline. Do not add a second left-side primary menu.

## Inspector Contract

When a segment is selected and the right inspector is on `画面`, show:

- `片段参数`: read-only segment/material/track/source/target information.
- `基础`: compact editable rows for `位置`, `缩放`, `旋转`, `不透明度`.
- `适应方式`: segmented control with `适应`, `填充`, `拉伸`.
- `裁剪`: compact numeric rows for `上`, `下`, `左`, `右`; can be collapsed if height is tight.
- `背景填充`: visible controls for `无`, `纯色`, `模糊`; unsupported image background remains visible as `未接入`.
- `混合模式` and `蒙版`: visible deferred rows with `正常`/`无` and `未接入` capability state.
- Keyframe icon placeholders remain visible but disabled/deferred until Phase 10.

## Visual Style

- Dark compact workspace, Chinese copy, restrained cyan active state.
- Sliders and numeric inputs are fixed-height and do not cause layout shifts on hover/selection.
- Scrollbars use the existing compact dark scrollbar baseline. No large light native-looking scrollbar.
- Labels must fit at 1120x720 and 1280x800.
- Avoid hero, cards-inside-cards, decorative gradients, and marketing layout. This is an editor workspace.

## Interaction Contract

- Every semantic edit calls `window.videoEditorCore.executeCommand` through generated command helpers.
- Renderer can keep form input state, but only Rust command responses update persisted draft semantics.
- Successful transform edits clear stale preview frame, preview segment, export progress/result, and validation artifacts, same as canvas edits.
- Invalid input stays local or produces command error; it must not partially mutate `draft.tracks`.

## Playwright Contract

At both 1280x800 and 1120x720:

- Top feature bar, left panel, preview, inspector, and timeline are visible and not overlapping.
- Selecting a video/image/text segment reveals `基础`, `位置`, `缩放`, `旋转`, `不透明度`, `适应方式`.
- At least one transform edit is observed through `window.videoEditorCore.executeCommand`.
- The left panel has no duplicate primary menu.

## UI-SPEC VERIFIED
