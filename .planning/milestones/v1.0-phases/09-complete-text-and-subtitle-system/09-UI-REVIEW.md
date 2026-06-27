# Phase 09 - UI Review

**Audited:** 2026-06-18
**Baseline:** `.planning/phases/09-complete-text-and-subtitle-system/09-UI-SPEC.md`
**Screenshots:** not captured (localhost ports 3000, 5173, and 8080 returned 502)

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Chinese terminology is mostly correct, but visible `Rust 解析 SRT` leaks implementation language into the editor UI. |
| 2. Visuals | 3/4 | The five-area shell and top feature bar are preserved, but the left `默认文字` card duplicates inspector-style controls instead of staying contextual. |
| 3. Color | 3/4 | Dark Jianying-style baseline and compact scrollbars are preserved, with cyan somewhat over-applied to filled primary and active states. |
| 4. Typography | 3/4 | Type is compact and legible, but six distinct sizes are used across the workspace instead of a tighter desktop-editor scale. |
| 5. Spacing | 2/4 | Layout uses dense spacing, but the left text panel is vertically overloaded and many pixel values are ad hoc rather than a declared scale. |
| 6. Experience Design | 3/4 | Command ownership, invalid text guards, disabled states, and viewport tests exist, but code-only audit could not verify actual screenshots and tests miss content-level clipping in the overloaded text panel. |

**Overall: 17/24**

---

## Top 3 Priority Fixes

1. **Shrink the left `默认文字` card to contextual add controls only** - users get a crowded duplicate inspector in the resource panel - keep content/duration/add action there and move style, stroke, shadow, background, line height, and letter spacing to the right inspector.
2. **Replace implementation-facing copy `Rust 解析 SRT`** - visible UI should feel Jianying-style, not engineering-oriented - use copy like `导入后自动生成字幕片段` or `字幕文件`.
3. **Strengthen viewport regression checks for text-panel content** - current tests prove region boxes, not that controls remain usable at 1120x720 - assert key left-panel and inspector controls are inside their scroll containers and not horizontally clipped.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

- **WARNING:** [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:354) renders `Rust 解析 SRT` in the subtitle card. The UI-SPEC requires Simplified Chinese Jianying-style copy; this is implementation-facing language. Replace with user-facing copy such as `字幕文件` or `导入后自动生成字幕片段`.
- **WARNING:** [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:388) and [Inspector.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/Inspector.tsx:583) correctly keep `花字` / `气泡` visible with `暂未接入`, satisfying the deferred-capability contract.
- **WARNING:** Generic English UI labels were not found in the targeted text/subtitle surfaces, but `SRT` is necessarily technical and should be surrounded by Chinese user-facing context.

### Pillar 2: Visuals (3/4)

- **WARNING:** [WorkspaceShell.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx:88) keeps the top feature bar as the primary navigation, and [tests/workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:291) asserts no left secondary menu classes or per-category secondary nav appear.
- **WARNING:** [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:231) implements the required `默认文字`, `字幕 / 导入字幕`, `花字`, and `气泡` cards, but the `默认文字` card also contains font, size, color, alignment, stroke, shadow, background, line height, and letter spacing controls. That makes the left resource panel feel like a second inspector instead of compact contextual content.
- **WARNING:** [Inspector.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/Inspector.tsx:357) exposes the required compact sections `文本`, `样式`, `文本框`, `布局`, and `花字 / 气泡` for selected text segments.

### Pillar 3: Color (3/4)

- **WARNING:** Compact dark scrollbars are explicitly preserved in [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:10), with 4px WebKit scrollbars and dark thumbs.
- **WARNING:** The workspace uses a consistent dark neutral base in [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:60), matching the desktop editor direction.
- **WARNING:** Cyan `#20c7d9` appears on active nav underlines, focus borders, primary filled actions, active segmented controls, selected timeline outline, and playhead. The spec asks for restrained cyan and avoiding large cyan fills; [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:237) and [preview-inspector.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/preview-inspector.css:590) use filled cyan for primary/active states. Consider using outline or text-only cyan for secondary active states.

### Pillar 4: Typography (3/4)

- **WARNING:** Typography is compact and desktop-appropriate: root is 13px, labels are mostly 12px, section headings are 14px, and product/preview titles are 18px.
- **WARNING:** The scan found six distinct font sizes across the renderer CSS: 11px, 12px, 13px, 14px, 16px, and 18px. This is still controlled, but a tighter editor scale would better match the UI-SPEC’s compact requirement.
- **WARNING:** Font weights are limited mostly to 400, 500, and 600, which is acceptable for dense hierarchy.

### Pillar 5: Spacing (2/4)

- **WARNING:** [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:60) defines stable shell columns and rows, and [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:978) provides a narrower column set below 1200px.
- **WARNING:** The text panel is too tall for the left resource role: [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:237) through [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:347) creates a long form before the subtitle, 花字, and 气泡 cards. At 1120x720 this depends heavily on scrolling, which weakens the “compact contextual cards” contract.
- **WARNING:** Spacing is mostly small, but the CSS uses many one-off pixel values: 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, and 20px patterns across `styles.css`, `preview-inspector.css`, and `timeline.css`. Consolidate text/subtitle additions around the existing 4/8/12px rhythm.

### Pillar 6: Experience Design (3/4)

- **WARNING:** Text edits route through command handlers and clear stale preview/export state after accepted command responses; the Playwright test verifies `editTextSegment` and derived-state copy in [tests/workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:457).
- **WARNING:** SRT import sends raw SRT once and asserts no renderer-created `addTextSegment` cue path in [tests/workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:498), matching the Rust-owned parsing contract.
- **WARNING:** Invalid text values are blocked with Chinese validation messages in [Inspector.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/Inspector.tsx:1010), and the apply button is disabled when validation fails in [Inspector.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/Inspector.tsx:577).
- **WARNING:** The viewport tests in [tests/workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:106) assert region boxes are visible and non-overlapping at 1280x800 and 1120x720, but this audit could not capture screenshots because no dev server was available. Add screenshot-backed assertions for the text panel and selected-text inspector after Phase 09 UI changes.

---

## Files Audited

- `.planning/phases/09-complete-text-and-subtitle-system/09-UI-SPEC.md`
- `.planning/phases/09-complete-text-and-subtitle-system/09-CONTEXT.md`
- `.planning/phases/09-complete-text-and-subtitle-system/09-04-PLAN.md`
- `.planning/phases/09-complete-text-and-subtitle-system/09-04-SUMMARY.md`
- `.planning/phases/09-complete-text-and-subtitle-system/09-05-SUMMARY.md`
- `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx`
- `apps/desktop-electron/src/renderer/workspace/Inspector.tsx`
- `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx`
- `apps/desktop-electron/src/renderer/styles.css`
- `apps/desktop-electron/src/renderer/workspace/preview-inspector.css`
- `apps/desktop-electron/src/renderer/workspace/timeline.css`
- `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx`
- `apps/desktop-electron/src/renderer/viewModel.ts`
- `apps/desktop-electron/src/renderer/App.tsx`
- `apps/desktop-electron/src/renderer/commandHelpers.ts`
- `apps/desktop-electron/tests/workspace.spec.ts`
