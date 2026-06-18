# Jianying Pro UI Reference

Reference source: Jianying Pro / 剪映专业版 10.8.12793 on macOS.

These files are visual and interaction references for building a reduced-feature editor that still follows Jianying-style layout, density, and interaction patterns. They are not assets to copy. Do not copy proprietary images, icons, text art, effect presets, or implementation details.

## Scope

- In scope: workspace layout, top feature tabs, resource panel, preview monitor, inspector, timeline, export modal patterns, dropdown/modal behavior, and compact desktop density.
- Out of scope: 数字人.
- Product direction: reduce feature count, not interaction structure. The application should feel like a production desktop editor, not a debug workbench.

## Screenshot Set

Current screenshots live in `docs/ui-reference/jianying-pro/screenshots/`.

| File | Intended Reference |
|------|--------------------|
| `02-workspace-media-window.png` | Main workspace on media/material tab |
| `03-top-feature-tabs.png` | Compact top feature tab row |
| `04-left-material-library.png` | Left material/resource library |
| `05-center-preview-monitor.png` | Center preview monitor |
| `06-right-draft-parameters.png` | Right draft parameter inspector |
| `07-bottom-timeline.png` | Bottom timeline and track area |
| `08-audio-panel-window.png` | Audio feature panel |
| `09-text-panel-window.png` | Text feature panel |
| `10-sticker-panel-window.png` | Sticker feature panel |
| `11-effects-panel-window.png` | Effects feature panel |
| `12-transition-panel-window.png` | Transition feature panel |
| `13-captions-panel-window.png` | Captions/subtitle feature panel |
| `14-smart-package-panel-window.png` | Smart package panel |
| `15-filter-panel-window.png` | Filter panel |
| `16-adjustment-panel-window.png` | Adjustment panel |
| `17-top-tabs-overflow-window.png` | Top tab overflow/switched set |
| `18-template-panel-window.png` | Template panel |
| `28-export-modal-advanced-expanded-fullscreen.png` | Export modal with advanced section expanded |
| `42-export-audio-samplerate-dropdown-fullscreen.png` | Export audio sample-rate dropdown state |

## Quality Note

The current screenshot batch is provisional. Some titles from the capture session did not match the apparent UI state, and only 19 files are present in the working tree. Treat this set as a rough layout reference, not as a locked golden set.

Before a full UI-alignment pass, recapture the missing states with a manifest that records:

- app version, date, window size, and display scale;
- active top tab and selected secondary category;
- modal/dropdown/open-section state;
- screenshot filename and expected visible state;
- whether the state is production-relevant or excluded.

Known missing or incomplete states:

- export base modal, resolution/format/framerate/bitrate/codec dropdowns, and audio enablement states need a verified complete recapture;
- draft parameter edit modal needs capture;
- preview monitor menus and ratio/fit/fullscreen controls need capture;
- timeline toolbar, track header controls, zoom, snapping/link/more menus need capture;
- material import dropdown, view/sort/filter menus, search states, and empty states need capture;
- per-feature panel internal tabs/filters/dropdowns need capture beyond the first visible screen.

## Alignment Rules

- Preserve the five-zone editor structure: top feature bar, left resource panel, center preview, right inspector, bottom timeline.
- Keep primary category navigation in the top bar. Do not duplicate primary feature tabs in the left panel.
- Use compact icon buttons for editor tools; text buttons are reserved for clear commands such as import/export/apply.
- Keep debug/runtime/internal information out of the default production UI. FFmpeg paths, probe details, artifact paths, cache paths, and raw diagnostics may only appear behind an explicit developer diagnostics mode.
- Use Jianying-style Chinese terminology where the product already has an equivalent: 草稿, 素材, 轨道, 片段, 关键帧, 滤镜, 转场, 导出.
- Keep renderer behavior command-driven. UI controls may collect intent, but Rust command responses remain the accepted source for draft/timeline changes.

