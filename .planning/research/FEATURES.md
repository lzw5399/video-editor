# Feature Research

## MVP Table Stakes

The MVP must feel like a real desktop editor even if it has fewer effects than Jianying.

### Draft And Material

- Create/open/save a `.veproj` draft.
- Import video, image, and audio materials.
- Probe duration, fps, size, streams, and audio properties.
- Show a material bin with thumbnails and basic search/filter.
- Detect missing materials and provide a clear recovery flow.

### Timeline

- One sequence in MVP, with model support for multiple tracks.
- Track types: video, audio, text at minimum; sticker/filter/effect can be schema-ready and UI-deferred.
- Segment operations: add, select, move, split, trim, delete.
- Source time range and target time range are first-class.
- Main-track magnet/snapping support.
- Timeline zoom and playhead.
- Undo/redo for every committed edit.

### Preview

- Center preview/player matching Jianying workspace layout.
- Source preview and draft preview can share the same player implementation.
- Single-frame preview for edits.
- Short-range preview cache for playback.
- Preview and export must use the same resolved draft semantics.

### Text And Audio

- Text/subtitle segments.
- Basic text style: font, size, color, align, stroke, shadow, background.
- SRT import can be part of MVP if low-cost; otherwise first post-MVP.
- Add BGM/audio segment, volume, mute.

### Export

- H.264 MP4 export preset.
- Output path selection.
- Progress, cancel, logs, and classified errors.
- Export full draft or selected range if range support is ready.

### UI

- Jianying-like layout: top feature categories, left material/function panel, center preview, right inspector, bottom timeline.
- MVP can implement only media/text/audio panels, but reserve slots for sticker, effect, transition, filter, adjustment.
- Dense editor workspace, not a SaaS dashboard.

## Post-MVP Features

- Multi-sequence/nested drafts.
- Stickers, GIF/WebP/video overlays.
- Transform keyframes for position, scale, rotation, opacity.
- Filters/effects on segments and tracks.
- Transitions between adjacent segments.
- Masks, blend modes, text bubbles, text effects.
- Proxy generation and waveform/thumbnail caching at scale.
- Jianying/CapCut adapter L0-L2 with compatibility reports.
- Mobile/server runtime bindings.

## Explicit Anti-Features For MVP

- AI oral-video workflow.
- Proprietary Jianying/Kaipai effect reproduction.
- Jianying draft as primary storage format.
- GPU real-time effect engine.
- Large preset marketplace.

