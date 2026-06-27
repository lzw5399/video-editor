---
phase: 20-long-timeline-product-uat-and-guard-baseline
artifact: ui-review
audited: 2026-06-28
baseline: abstract 6-pillar standards
ui_spec: none
screenshots: not captured - no dev server on localhost:3000, 5173, or 8080
overall_score: 16/24
blocking_ui_changes_required: false
---

# Phase 20 - UI Review

**Audited:** 2026-06-28 CST  
**Baseline:** abstract 6-pillar standards, production-grade desktop video editor expectations  
**Screenshots:** not captured. No dev server responded on `localhost:3000`, `5173`, or `8080`; this is a code-only audit.  
**Blocking UI changes needed for Phase 20:** No. The implemented app supports the Phase 20 long-timeline product UAT path without a blocking visual or interaction defect.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Product copy is specific and Chinese editor-oriented, but disabled/future feature copy is visible in core panels. |
| 2. Visuals | 3/4 | The four-pane editor layout is appropriate, dense, and CapCut-like; several disabled controls still read as unfinished surface area. |
| 3. Color | 2/4 | Palette is coherent at runtime, but color is hardcoded heavily: 161 unique hex values and `#20c7d9` appears 67 times. |
| 4. Typography | 2/4 | Dense editor typography works, but CSS uses 10 font sizes and 5 font weights without a clear token scale. |
| 5. Spacing | 3/4 | Spacing mostly follows a compact 4/6/8/12px rhythm, with several one-off gaps/paddings and pill radii. |
| 6. Experience Design | 3/4 | Phase 20 UAT covers real product flows and states well; no React error boundary was found for renderer crash recovery. |

**Overall: 16/24**

---

## Top 3 Priority Fixes

1. **Tokenize and reduce color usage** - Hardcoded colors make the editor harder to polish consistently across preview, timeline, inspector, and export surfaces. Introduce CSS variables for neutral surfaces, text hierarchy, cyan accent, warning, danger, and success, then replace direct values in `styles.css`, `timeline.css`, and `preview-inspector.css`.
2. **Normalize the typography scale** - The current mix of 10/11/12/13/14/15/16/17/18/20px and five weights creates subtle hierarchy drift. Define a compact editor type scale and migrate headings, chips, buttons, inspector labels, timeline labels, and status rows to it.
3. **De-emphasize unavailable controls in production mode** - Disabled media sources, preview view controls, feature cards, and the preview title menu expose incomplete affordances in the primary workspace. Hide them outside developer/roadmap modes or replace them with one clear gated message per panel.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

**WARNING:** Product copy is mostly domain-specific and task-oriented. The entry screen uses clear project-start language in `apps/desktop-electron/src/renderer/App.tsx:3300` to `:3307`, timeline controls use editor verbs such as "分割所选片段", "删除所选片段", "添加视频轨道" in `apps/desktop-electron/src/renderer/workspace/Timeline.tsx:660` to `:679`, and export status copy maps internal phases to product-readable strings in `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx:693` to `:724`.

**WARNING:** The production workspace still surfaces repeated "暂不可用" / "暂不支持" copy for controls that are not part of the completed product path. Examples include disabled media rail entries in `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:143` to `:148`, disabled preview controls in `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx:1304` to `:1321`, and unavailable feature gates in `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:1219` to `:1229`. This does not block Phase 20, but it makes the editor feel less finished.

**Audit note:** Generic English labels such as `Submit`, `Click Here`, `OK`, `Cancel`, `Save`, `No data`, and `went wrong` were not found in the audited renderer sources.

### Pillar 2: Visuals (3/4)

**WARNING:** The main workspace structure matches a desktop editor: titlebar, category rail, media panel, preview monitor, inspector, and timeline are explicitly laid out in `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx:230` to `:429`; the CSS grid uses a dense three-column, four-row workspace in `apps/desktop-electron/src/renderer/styles.css:146` to `:157`.

**WARNING:** The central focal point is correct. The preview monitor owns the middle pane, presents the draft title and canvas, and keeps transport/status controls local to the monitor in `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx:1120` to `:1391`.

**WARNING:** Several disabled controls are visually present in primary surfaces: the disabled preview menu in `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx:1124`, disabled view controls in `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx:1304` to `:1321`, disabled media sources in `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:143` to `:148`, and disabled advanced filter in `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:708` to `:713`. These are quality issues, not task blockers.

### Pillar 3: Color (2/4)

**WARNING:** The runtime palette reads as a dark operational editor with cyan active states and restrained status colors, which fits the product category. Examples: active categories and export action use cyan in `apps/desktop-electron/src/renderer/styles.css:294` to `:306` and `:338` to `:355`; timeline active/accent states use the same cyan in `apps/desktop-electron/src/renderer/workspace/timeline.css:104` to `:108`.

**WARNING:** Color implementation is not design-system safe. The scan found **161 unique hex values** across renderer CSS/TSX, and the accent `#20c7d9` appears **67 times**. The same palette is repeated directly across `apps/desktop-electron/src/renderer/styles.css`, `apps/desktop-electron/src/renderer/workspace/timeline.css`, and `apps/desktop-electron/src/renderer/workspace/preview-inspector.css` rather than through shared tokens.

**WARNING:** Status and accent colors are functionally useful but fragmented: warning gold, danger red, success green, purple, orange, neutral surface, and text colors all appear as direct values. This raises the risk that later Phase 21-24 polish work changes one surface without matching timeline/preview/inspector behavior.

### Pillar 4: Typography (2/4)

**WARNING:** The base font choice is appropriate for a Chinese desktop editor: `Inter`, `PingFang SC`, `Microsoft YaHei`, and system fallbacks are set in `apps/desktop-electron/src/renderer/styles.css:1` to `:8`.

**WARNING:** The CSS scan found **10 distinct font sizes**: 10, 11, 12, 13, 14, 15, 16, 17, 18, and 20px. It also found **5 font weights**: 400, 500, 600, 700, and 800. This exceeds the abstract audit threshold of 4 sizes / 2 weights and makes hierarchy harder to reason about.

**WARNING:** The most important surfaces are reasonably compact, but not tokenized: project title text is defined in `apps/desktop-electron/src/renderer/styles.css:238` to `:248`, category labels in `apps/desktop-electron/src/renderer/styles.css:272` to `:327`, timeline labels/status in `apps/desktop-electron/src/renderer/workspace/timeline.css:199` to `:207`, and preview title/status text in `apps/desktop-electron/src/renderer/workspace/preview-inspector.css:37` to `:80`.

### Pillar 5: Spacing (3/4)

**WARNING:** The core editor uses stable dimensions that are appropriate for desktop use: `body` and `.workspace` enforce `1120x720` minimums in `apps/desktop-electron/src/renderer/styles.css:48` to `:54` and `:146` to `:157`; the timeline transport fixes control dimensions in `apps/desktop-electron/src/renderer/workspace/timeline.css:61` to `:75`; the preview canvas uses aspect ratio and max constraints in `apps/desktop-electron/src/renderer/workspace/preview-inspector.css:93` to `:108`.

**WARNING:** Most spacing follows compact editor values such as 4, 6, 8, 10, and 12px, but the scan also found one-off values including `gap: 3px`, `gap: 5px`, `gap: 7px`, `gap: 14px`, `padding: 14px 16px`, `padding: 3px 7px`, and `padding: 4px 1px 3px`. These are not blockers, but they should be normalized during UI polish.

**WARNING:** Border radius is mostly 2/4/5/6/8px, which fits the desktop-tool contract, but `border-radius: 999px` appears 22 times for chips, dots, sliders, and status pills. Keep it for true circular/pill controls only; avoid expanding that pattern into panels or cards.

### Pillar 6: Experience Design (3/4)

**WARNING:** Phase 20's product path is well covered. The packaged responsiveness UAT writes success/failure evidence in `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts:77` to `:112`, and it measures zoom, selection, scroll, scrub, play, trim, move, split, undo, redo, and inspector edit budgets in `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts:274` to `:360`.

**WARNING:** The long-session export and pressure paths are product-realistic. Export is opened through the product UI and validated through media evidence in `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts:510` to `:524`; scheduler pressure keeps playback, scrub, inspector edit, commit, cancel, export, telemetry, fallback, and stale-generation assertions in `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts:377` to `:486`.

**WARNING:** The UI exposes useful pending/error/empty/destructive states: export progress/log/validation in `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx:639` to `:653`, command errors in `apps/desktop-electron/src/renderer/workspace/Inspector.tsx:934` and `:1470`, project entry `role="alert"` in `apps/desktop-electron/src/renderer/App.tsx:3311` to `:3313`, destructive delete confirmation in `apps/desktop-electron/src/renderer/App.tsx:2470`, and inline effect/mask confirmations in `apps/desktop-electron/src/renderer/workspace/Inspector.tsx:1986` to `:2039` and `:2105` to `:2187`.

**WARNING:** No `ErrorBoundary` was found under `apps/desktop-electron/src/renderer`. Command-level errors are handled, but a React render exception could still blank the app without a product-readable recovery surface. This is not a Phase 20 blocker, but it should be added before a production acceptance sweep.

---

## Files Audited

- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-01-SUMMARY.md`
- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-02-SUMMARY.md`
- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-03-SUMMARY.md`
- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-04-SUMMARY.md`
- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-01-PLAN.md`
- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-02-PLAN.md`
- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-03-PLAN.md`
- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-04-PLAN.md`
- `.planning/phases/20-long-timeline-product-uat-and-guard-baseline/20-CONTEXT.md`
- `apps/desktop-electron/src/renderer/App.tsx`
- `apps/desktop-electron/src/renderer/styles.css`
- `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx`
- `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx`
- `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx`
- `apps/desktop-electron/src/renderer/workspace/Inspector.tsx`
- `apps/desktop-electron/src/renderer/workspace/Timeline.tsx`
- `apps/desktop-electron/src/renderer/workspace/preview-inspector.css`
- `apps/desktop-electron/src/renderer/workspace/timeline.css`
- `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts`
