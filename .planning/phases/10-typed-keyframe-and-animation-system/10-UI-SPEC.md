---
phase: 10
slug: typed-keyframe-and-animation-system
status: approved
shadcn_initialized: false
preset: none
created: 2026-06-18
---

# Phase 10 — UI Design Contract

> Visual and interaction contract for typed keyframe and animation controls. Generated for Phase 10 planning.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none; existing hand-authored React + CSS renderer |
| Preset | not applicable |
| Component library | none |
| Icon library | Existing compact text/CSS symbols are allowed; use generic icons only if an icon dependency already exists |
| Font | Inter, "PingFang SC", "Microsoft YaHei", ui-sans-serif, system-ui, sans-serif |

Sources: `AGENTS.md`, Phase 04.1 UI spec, Phase 08/09 UI specs, Phase 10 context, and existing `Inspector.tsx`, `Timeline.tsx`, `preview-inspector.css`, `timeline.css`.

Phase 10 must preserve the current desktop editor shell: top feature bar as primary navigation, left contextual panel, center preview, right inspector, and bottom timeline. Do not add a duplicate left primary menu. All user-visible copy is Simplified Chinese and Jianying-style, but the UI must remain original/open-source and must not copy Jianying/CapCut brand assets, icons, wording that implies official affiliation, or trade dress.

---

## Spacing Scale

Declared values (must be multiples of 4):

| Token | Value | Usage |
|-------|-------|-------|
| xs | 4px | Icon gaps, keyframe diamond offsets, marker strip gaps |
| sm | 8px | Inspector row gaps, timeline marker padding, section gaps |
| md | 16px | Inspector section padding ceiling, empty state padding |
| lg | 24px | Reserved for larger compact empty-state breathing room |
| xl | 32px | Inspector tab row height and icon button rhythm |
| 2xl | 48px | Top feature bar and timeline toolbar height |
| 3xl | 64px | Maximum top category item width at wider desktop sizes |

Exceptions:

- Region dividers remain 1px.
- Timeline playhead remains 2px wide with a 12px handle.
- Inline keyframe buttons are fixed 28x28px.
- Timeline keyframe markers are 6x6px diamonds with a minimum 16x16px pointer/focus target when interactive.
- Segment blocks must not grow taller when keyframe markers appear; marker strips are overlaid inside the existing segment box.

---

## Typography

| Role | Size | Weight | Line Height |
|------|------|--------|-------------|
| Marker metadata / chips | 12px | 400 | 1.4 |
| Body / controls | 13px | 400 | 1.5 |
| Panel heading / selected labels | 14px | 600 | 1.3 |
| Workspace title / monitor title | 18px | 600 | 1.2 |

Rules:

- Letter spacing is `0`.
- Do not scale font size with viewport width.
- Button labels and tab labels must stay single-line at 1120x720; truncate dynamic property names, not fixed control labels.
- No hero/display typography. This remains a dense editing workspace.

---

## Color

| Role | Value | Usage |
|------|-------|-------|
| Dominant (60%) | `#151515` | App background, timeline bed, inspector scroll surface |
| Secondary (30%) | `#20201e` | Top bar, left panel, inspector panel, track rows |
| Elevated surface | `#252522` | Inspector sections, compact controls, animation rows |
| Divider | `#343431` | Region dividers, inspector/timeline row borders |
| Primary text | `#f0eee8` | Headings, selected labels |
| Secondary text | `#9b9890` | Metadata, help copy, disabled-adjacent copy |
| Accent (10%) | `#20c7d9` | Active inspector tab, focused controls, selected segment outline, active keyframe at playhead, selected timeline marker |
| Keyframe inactive | `#aaa79f` | Property can keyframe but no keyframe at playhead |
| Keyframe has data | `#d6a23f` | Property has keyframes away from playhead, unsupported/degraded animation badges |
| Success | `#5f9f73` | Accepted command/state-ready indicators only |
| Destructive | `#e45a5a` | Remove keyframe, command errors, invalid values |

Accent reserved for: active `动画` tab, active/selected keyframe buttons, selected keyframe marker, focus ring, selected segment outline, playhead, and primary command buttons. Do not use accent as a broad animation panel background.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Primary CTA | `添加关键帧` |
| Empty state heading | `还没有关键帧` |
| Empty state body | `在画面、文本或音频参数行点击◇，可在播放头位置添加关键帧。` |
| No segment heading | `未选择片段` |
| No segment body | `选择时间线片段后，可查看动画参数和关键帧。` |
| Selected property heading | `属性关键帧` |
| Selected property body | `当前属性已有关键帧，可调整插值和缓动。` |
| Pending command | `关键帧命令处理中` |
| Invalid command | `关键帧操作失败：{错误信息}。请检查播放头位置或参数值后重试。` |
| Unsupported/deferred | `暂不支持该参数动画` |
| Deferred effect body | `当前阶段仅显示特效动画能力边界，不会创建关键帧。` |
| Destructive confirmation | `删除关键帧`: no modal required for this phase; the button must be disabled without an active selected keyframe and use `aria-label="删除所选关键帧"` |

Required visible Chinese labels:

- Tabs/sections: `动画`, `关键帧`, `属性关键帧`, `常用动画`, `入场`, `出场`, `循环`, `插值`, `缓动`, `时间`, `属性`, `数值`.
- Property groups: `画面`, `基础`, `文本`, `音频`, `特效`.
- Property labels: `位置 X`, `位置 Y`, `缩放 X`, `缩放 Y`, `旋转`, `不透明度`, `字号`, `颜色`, `行高`, `字间距`, `布局 X`, `布局 Y`, `布局宽`, `布局高`, `音量`.
- State badges: `当前帧`, `已有关键帧`, `未接入`, `命令中`, `失败`.

Accessibility labels and titles:

- Inline inactive button: `添加{属性}关键帧`.
- Inline active-at-playhead button: `移除{属性}关键帧`.
- Property has keyframes away from playhead: `查看{属性}关键帧`.
- Pending button: `{属性}关键帧命令处理中`.
- Unsupported button: `{属性}关键帧暂不支持`.
- Timeline marker: `{片段名} {属性}关键帧 {时间码}`.
- Animation tab list: `动画参数`.
- Keyframe list: `{属性}关键帧列表`.

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none | not applicable; `components.json` absent and existing manual CSS is the approved design system — 2026-06-18 |
| third-party | none | not applicable; no third-party registry blocks declared — 2026-06-18 |

If implementation adds shadcn or any third-party registry during this phase, this section must be updated with `npx shadcn view` evidence before use.

---

## Phase Scope

Phase 10 UI turns disabled keyframe placeholders into command-only keyframe controls and adds an animation overview shell. It must not implement animation semantics in the renderer.

Renderer may:

- Hold local form input state before a keyframe command is submitted.
- Hold hover/focus state for inline keyframe buttons and timeline markers.
- Read accepted keyframe data from the current Rust-returned draft for display.
- Format integer microseconds as timecode.
- Construct generated command envelopes and call `window.videoEditorCore.executeCommand`.

Renderer must not:

- Mutate `draft.tracks`, `track.segments`, `segment.keyframes`, `segment.visual`, `segment.text`, volume semantics, timeranges, undo/redo, or snapping directly.
- Interpolate animation values, evaluate easing, sample frame-time animation, or calculate persisted animated results.
- Own render graph, FFmpeg, preview/export cache invalidation, preview artifacts, export validation, waveform, or thumbnail semantics.
- Persist unsupported effect/filter/sticker animation as fake supported keyframes.

---

## Inspector Contract

Primary inspector tabs remain:

`画面`, `音频`, `变速`, `动画`, `调节`, `AI效果`

### Inline Keyframe Buttons

Replace current disabled `KeyframeButton` placeholders with command-only keyframe buttons on rows and section headers where Phase 10 supports typed values.

Button contract:

- Fixed 28x28px; symbol `◇+` for inactive, `◆` for active at playhead, `◇` with warning tone for property has keyframes elsewhere.
- Buttons never change row height, section width, or label wrapping.
- Buttons are disabled during pending commands and for unsupported/deferred parameters.
- Clicking a supported inactive button submits a generated keyframe add/update command for the selected segment, current playhead microseconds, property id, current typed value, interpolation, and easing.
- Clicking an active-at-playhead button submits a generated keyframe remove command.
- Clicking a property-with-keyframes-away button selects/opens that property in the `动画` tab through command-owned selection if available; otherwise it may only update local inspector focus, not draft state.

Inline supported properties:

| Section | Fields |
|---------|--------|
| `画面` / `基础` | `位置 X`, `位置 Y`, `缩放 X`, `缩放 Y`, `旋转`, `不透明度` |
| `文本` | `字号`, `颜色` |
| `样式` | `描边宽度`, `背景颜色` only if the typed model supports them; otherwise show unsupported |
| `文本框` | `行高`, `字间距` |
| `布局` | `布局 X`, `布局 Y`, `布局宽`, `布局高` |
| `音频` | `音量` |

Deferred inline properties:

- `裁剪`, `背景填充`, `混合模式`, `蒙版`, `花字`, `气泡`, `特效`, `滤镜`, `贴纸` animation controls remain visible as `暂不支持该参数动画` unless the Phase 10 typed model explicitly supports them.
- Deferred buttons keep the same 28x28px footprint and use `aria-label="{属性}关键帧暂不支持"`.

### `动画` Tab

The `动画` tab is the selected segment's keyframe overview. It is not a separate primary navigation area.

Layout:

- Use the existing inspector scroll container.
- Top summary section: selected segment name, keyframe count, playhead time, current selected property.
- Compact property list grouped as `画面`, `文本`, `音频`, `特效`.
- Selected property detail section with keyframe rows: `时间`, `数值`, `插值`, `缓动`, delete action.
- Optional presets area: `入场`, `出场`, `循环` as visible deferred shells only. Do not create template animation semantics unless Rust commands exist.

Controls:

- Interpolation segmented control: `保持`, `线性`.
- Easing segmented control: `无`, `缓入`, `缓出`, `缓入缓出`.
- Each change submits through generated keyframe update commands; the renderer must not locally rewrite accepted keyframe arrays.
- The tab must show command errors below the relevant property/detail section using destructive text.

State contracts:

| State | Inspector Result |
|-------|------------------|
| No segment | Show `未选择片段` / `选择时间线片段后，可查看动画参数和关键帧。`; no mutating controls enabled |
| Selected segment without keyframes | Show `还没有关键帧`; supported inline row buttons are enabled; `动画` tab shows supported property groups with zero counts |
| Selected property with keyframes | Inline button shows `已有关键帧` state; `动画` tab lists keyframes for that property, highlights the keyframe at playhead if present |
| Invalid command/value | Keep local input value, do not add/remove markers, show `关键帧操作失败：{错误信息}。请检查播放头位置或参数值后重试。` |
| Pending command | Disable supported keyframe buttons and property controls; show `关键帧命令处理中` without layout shift |
| Unsupported/deferred effect animation | Show `特效动画暂未接入` and `当前阶段仅显示特效动画能力边界，不会创建关键帧。` |

---

## Timeline Keyframe Contract

Timeline keyframes are markers over accepted segment data only. They do not own timing, interpolation, easing, or selection semantics.

Visual shell:

- Each segment block may render an internal `.segment-keyframe-strip` along the lower edge.
- Markers are 6x6px diamonds, positioned from accepted keyframe `at` times relative to the segment target time range.
- Markers are clipped within the segment block and must not overflow into adjacent segments or track headers.
- Selected marker uses cyan; unselected existing markers use warning/gold; unsupported/deferred markers, if shown, use muted gray.
- Marker strip must be hidden when a segment is narrower than 24px unless the selected segment is focused; the segment label still has priority.

Interaction:

- Marker hover may show a native `title`/tooltip with property, time, and easing.
- Marker click/focus may select a keyframe through a generated command if Phase 10 defines command-owned keyframe selection. If no command exists, marker click may only focus/open the corresponding property in the `动画` tab locally.
- Timeline marker display must update only from Rust-returned draft/selection state after commands.
- The playhead remains the user's current insertion time for inline keyframe add/remove commands.

Required labels:

- Track/segment marker strip: `关键帧标记`.
- Marker `aria-label`: `{片段名} {属性}关键帧 {时间码}`.
- Selected marker `aria-current="true"` when it corresponds to the active command-owned selected keyframe.

---

## Interaction States

| State | Contract |
|-------|----------|
| Default | Compact dark UI, stable rows, no decorative gradients or floating panels |
| Hover | Border/background changes only; no size or text reflow |
| Focus | Visible cyan outline on button/marker/control |
| Active at playhead | Inline keyframe button uses cyan `◆`; animation tab row receives subtle selected outline |
| Has keyframes elsewhere | Inline button uses gold/muted `◇`; property row shows compact count chip such as `3 个` |
| Disabled unsupported | Same dimensions, 48-55% opacity, `aria-disabled`/`disabled`, Chinese unsupported label |
| Pending | Disable mutating keyframe controls; show `关键帧命令处理中` near the active section |
| Error | Destructive text near active section; markers and accepted values remain unchanged |

Keyboard/accessibility:

- Every icon-only keyframe button has Chinese `aria-label` and `title`.
- Markers that are focusable have Chinese `aria-label`.
- The `动画` tab uses `role="tabpanel"` and its property groups use labelled sections.
- Inputs keep visible labels; placeholders are not labels.

---

## Component Inventory

| Component/File | Phase 10 Contract |
|----------------|-------------------|
| `Inspector.tsx` | Replace disabled keyframe placeholders with command-only inline controls; implement selected-segment `动画` tab states and property keyframe overview |
| `Timeline.tsx` | Add segment keyframe marker strip and marker focus/selection shell without renderer-owned semantics |
| `preview-inspector.css` | Style keyframe buttons, animation tab rows, property counts, pending/error states, and deferred animation shells |
| `timeline.css` | Style marker strip/diamonds inside existing segment blocks without row height or segment width changes |
| `workspace.spec.ts` | Add Phase 10 workspace coverage for inspector keyframe controls, animation tab states, marker shell, accessibility labels, and command-only execution |
| `commandHelpers.ts` / `App.tsx` | Route generated keyframe commands through `window.videoEditorCore.executeCommand`; accepted draft state only comes from command responses |
| `viewModel.ts` | May derive display-only keyframe counts and marker positions from accepted draft data; must not mutate or interpolate |

---

## Viewport And Density Verification

Required viewports:

| Viewport | Required Result |
|----------|-----------------|
| 1280x800 | Five workspace regions visible; inspector animation tab scrolls internally; timeline markers visible on selected segment without overlap |
| 1120x720 | Top categories readable; no duplicate left primary menu; keyframe buttons and marker strip remain accessible without toolbar wrapping |

Geometry checks:

- Top feature bar, left panel, preview, inspector, and timeline have non-zero boxes and do not overlap except intentional 1px dividers.
- No left duplicate primary menu or secondary category rail appears.
- Inspector row labels, keyframe buttons, count chips, and tab labels do not overlap or clip.
- Timeline marker strip remains inside segment blocks; markers do not cover track headers, ruler, or toolbar.
- Hover/selection/pending/error states do not resize segment blocks, inspector rows, timeline toolbar, or workspace regions.
- Compact dark scrollbars remain active; no large light native scrollbars.

---

## Verification Contract

Required gates:

- `just build`
- `just test`
- `pnpm --filter @video-editor/desktop test:workspace`
- Phase 10 source guard extending Phase 09 guards.

Playwright Electron coverage:

- At 1280x800 and 1120x720, verify five-region layout, no clipping/overlap, compact dark scrollbars, and no duplicate left primary menu.
- Verify inspector tabs include `动画`.
- With no selected segment, `动画` tab shows `未选择片段` copy and no enabled mutating controls.
- With a selected segment and no keyframes, supported inline row buttons show `添加{属性}关键帧` labels and empty state `还没有关键帧`.
- Adding a supported keyframe is observed through `window.videoEditorCore.executeCommand`; accepted UI updates only after the command response.
- With keyframes present, the selected property shows `属性关键帧`, keyframe list rows, interpolation/easing controls, and timeline markers.
- Removing a keyframe is observed through `window.videoEditorCore.executeCommand`; markers are removed only after the command response.
- Invalid keyframe values or command failures show Chinese error copy and do not mutate marker display or accepted draft state.
- Unsupported/deferred effect animation shows `特效动画暂未接入` and does not call a mutating keyframe command.

Source guard coverage:

- Reject renderer direct mutation of `draft.tracks`, `track.segments`, `segment.keyframes`, timeranges, text/visual/audio semantics, undo/redo, and snapping.
- Reject renderer-owned interpolation/easing/frame-time animation evaluation, including ad hoc animation sampling helpers.
- Reject renderer ownership of render graph, FFmpeg, preview/export cache semantics, export validation, thumbnails, waveforms, and proxy generation.
- Require generated command/type drift check with `git diff --exit-code schemas apps/desktop-electron/src/generated`.

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** approved for Phase 10 planning and execution
