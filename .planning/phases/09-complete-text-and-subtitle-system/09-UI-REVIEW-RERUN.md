# Phase 09 - UI Review Rerun

**Audited:** 2026-06-18
**Baseline:** `.planning/phases/09-complete-text-and-subtitle-system/09-UI-SPEC.md`
**Compared Against:** commit `cabdf15 fix(09): tighten text panel UI density`
**Screenshots:** not captured (localhost ports 3000, 5173, and 8080 returned 502)
**Executable Gate:** `bash scripts/phase9-source-guards.sh` passed

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 4/4 | Prior `Rust 解析 SRT` leak is gone; text/subtitle UI copy is Simplified Chinese and user-facing. |
| 2. Visuals | 4/4 | Prior left-panel duplication is resolved; `默认文字` is now contextual add controls only. |
| 3. Color | 3/4 | Dark Jianying-style baseline remains, but cyan is still used as filled active/primary color in several surfaces. |
| 4. Typography | 3/4 | Compact editor typography remains legible, but the workspace still uses six distinct CSS font sizes. |
| 5. Spacing | 3/4 | Text panel density is materially improved, but CSS still relies on many one-off pixel values instead of a documented scale. |
| 6. Experience Design | 4/4 | Viewport/content clipping coverage was strengthened for the text panel and selected-text inspector; command ownership remains guarded. |

**Overall: 21/24**

---

## Priority Findings From Previous Review

1. **Resolved:** The left `默认文字` card no longer duplicates inspector controls. [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:204) through [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:228) now contains only text content, duration, and `添加文字`.
2. **Resolved:** Visible implementation copy `Rust 解析 SRT` is removed. The subtitle card now uses `字幕 / 导入字幕` and `自动生成字幕片段` in [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:233).
3. **Resolved:** Viewport/content clipping tests were strengthened. [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:283) adds `expectLocatorInsideHorizontalContainer`, and the text panel plus selected-text inspector use it at [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:466) and [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:493).

## Remaining Priority Blockers

None.

## Remaining Recommendations

1. **WARNING:** Reduce filled cyan active states where practical. Keep cyan for focus, outlines, active underlines, and primary commitments, but avoid making secondary active segmented controls large cyan fills.
2. **WARNING:** Normalize text/subtitle spacing tokens around the existing 4/8/12px rhythm so future phases do not reintroduce panel overload.
3. **WARNING:** If a dev server or Electron screenshot harness is available in the next pass, capture screenshot evidence at 1280x800 and 1120x720 to validate the strengthened clipping assertions visually.

---

## Detailed Findings

### Pillar 1: Copywriting (4/4)

- **WARNING:** Previous implementation-facing copy is fixed. `rg "Rust 解析 SRT" apps/desktop-electron/src/renderer apps/desktop-electron/tests/workspace.spec.ts scripts/phase9-source-guards.sh` finds only the forbidden guard pattern in [phase9-source-guards.sh](/Users/zhiwen/code/video-editor/scripts/phase9-source-guards.sh:113), not renderer UI.
- **WARNING:** [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:233) uses `字幕 / 导入字幕` and `自动生成字幕片段`, matching the UI-SPEC's Jianying-style Simplified Chinese requirement.
- **WARNING:** Deferred capability copy remains visible and Chinese: [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:267) keeps `花字` / `气泡` with `暂未接入，导入后将以不支持能力报告显示。`.

### Pillar 2: Visuals (4/4)

- **WARNING:** Previous duplicate-inspector defect is fixed. [FeaturePanel.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:204) now renders `默认文字` as a compact card with only `文字内容`, `时长（微秒）`, and `添加文字`.
- **WARNING:** Style controls now live in the selected-text inspector, as intended by the UI-SPEC: [Inspector.tsx](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/Inspector.tsx:357) exposes `文本`, `样式`, `文本框`, `布局`, and `花字 / 气泡`.
- **WARNING:** [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:426) asserts `默认文字` does not contain `字号`, and [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:427) asserts it does not contain `描边`.

### Pillar 3: Color (3/4)

- **WARNING:** Compact dark baseline remains intact in [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:2) and global dark scrollbars remain at [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:13).
- **WARNING:** Cyan `#20c7d9` is still used as filled color for primary actions and active segmented states: examples include [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:241), [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:483), [preview-inspector.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/preview-inspector.css:591), and [preview-inspector.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/preview-inspector.css:843). This is acceptable but still slightly heavier than the UI-SPEC's restrained cyan direction.

### Pillar 4: Typography (3/4)

- **WARNING:** Typography remains compact and desktop-appropriate: root is 13px in [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:5), section headings are 14px at [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:108), and labels are mostly 12px.
- **WARNING:** The CSS scan still finds six distinct sizes across the renderer: 11px, 12px, 13px, 14px, 16px, and 18px. This is controlled enough for the current editor, but not yet a tight declared type scale.
- **WARNING:** Font weights remain constrained to 400, 500, and 600, which is appropriate for dense editor hierarchy.

### Pillar 5: Spacing (3/4)

- **WARNING:** The previous text-panel overload is resolved by removing the left-panel style/layout form. The remaining `默认文字` card is short enough to fit the contextual resource-panel role.
- **WARNING:** The text panel and inspector use compact spacing (`gap: 8px`, `gap: 10px`, `padding: 10px`, `padding: 12px`) in [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:214), [styles.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/styles.css:426), and [preview-inspector.css](/Users/zhiwen/code/video-editor/apps/desktop-electron/src/renderer/workspace/preview-inspector.css:600).
- **WARNING:** The broader workspace still contains many one-off pixel values such as 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 18, 20, 24, 26, 28, and 32px. This is no longer blocking Phase 09, but a declared spacing scale would reduce future drift.

### Pillar 6: Experience Design (4/4)

- **WARNING:** Test coverage now checks content-level horizontal clipping, not just region boxes. [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:283) defines `expectLocatorInsideHorizontalContainer`.
- **WARNING:** [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:462) through [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:466) verifies `默认文字`, `字幕 导入字幕`, `花字`, and `气泡` stay inside the left panel.
- **WARNING:** [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:491) through [workspace.spec.ts](/Users/zhiwen/code/video-editor/apps/desktop-electron/tests/workspace.spec.ts:493) verifies selected-text inspector sections stay inside the inspector.
- **WARNING:** Source guards now fail on the previous copy leak and require the new Chinese text/subtitle copy. [phase9-source-guards.sh](/Users/zhiwen/code/video-editor/scripts/phase9-source-guards.sh:91) through [phase9-source-guards.sh](/Users/zhiwen/code/video-editor/scripts/phase9-source-guards.sh:114) enforce this contract.

---

## Registry Safety

Skipped: `components.json` is not present, so shadcn/third-party registry audit does not apply.

---

## Files Audited

- `.planning/phases/09-complete-text-and-subtitle-system/09-UI-REVIEW.md`
- `.planning/phases/09-complete-text-and-subtitle-system/09-UI-SPEC.md`
- `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx`
- `apps/desktop-electron/src/renderer/workspace/Inspector.tsx`
- `apps/desktop-electron/src/renderer/styles.css`
- `apps/desktop-electron/src/renderer/workspace/preview-inspector.css`
- `apps/desktop-electron/src/renderer/workspace/timeline.css`
- `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx`
- `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx`
- `apps/desktop-electron/src/renderer/workspace/Timeline.tsx`
- `apps/desktop-electron/src/renderer/App.tsx`
- `apps/desktop-electron/src/renderer/commandHelpers.ts`
- `apps/desktop-electron/src/renderer/viewModel.ts`
- `apps/desktop-electron/tests/workspace.spec.ts`
- `scripts/phase9-source-guards.sh`
