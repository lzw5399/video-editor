---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Combo Preview Evidence Gate

## Goal

Tighten the main product playback UAT for the required video + external audio + text + two-cue SRT flow so the test proves real native render-graph preview evidence, not just command success. Fix the discovered production semantic issue where default title text and SRT subtitles shared the same top safe-area layout.

## Scope

- Keep the existing Rust-owned intent path for importing video/audio/text/SRT.
- Move project-session default SRT subtitle layout to a bottom subtitle-safe region while leaving title text in the top safe area.
- Save full workbench screenshots for the first and second subtitle playback states.
- Assert each subtitle state has renderGraphGpu composited evidence, native surface placement aligned to the DOM host, and a centered/non-empty native host PNG.
- Expose active text overlay presentation-space bounding boxes and assert native PNG text pixels exist inside those boxes.
- Scale realtime GPU text overlays from render-graph canvas coordinates into the native presentation target so low-resolution draft canvases do not pin text to the surface's upper-left corner.
- Keep DOM text overlays banned during product native preview; DOM overlays remain editing controls only.

## Verification

- `corepack pnpm --dir apps/desktop-electron run package:dir`
- Focused `bindings_node` project-session test for text/subtitle default layout.
- Focused combo product journey test.
- Product P0 portrait and resize gates if native surface behavior is touched.
