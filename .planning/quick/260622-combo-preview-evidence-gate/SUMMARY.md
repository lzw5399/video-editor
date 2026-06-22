---
status: completed
completed: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Combo Preview Evidence Gate Summary

## Result

The product combo preview UAT now proves the video + external audio + title text + two-cue SRT flow through the native render-graph GPU path. The gate records full workbench screenshots and native host PNGs for both subtitle cues, rejects DOM text-overlay evidence, verifies native surface placement, checks text/subtitle presentation-space bounding boxes, and confirms real white text pixels exist inside those native host PNG boxes.

## Fixes

- Project-session default title text remains in the top safe area, while imported SRT subtitles now use a bottom subtitle-safe layout.
- Realtime scheduler evidence now exposes active text overlay presentation-space `x/y/width/height`.
- GPU text compositing now scales render-graph canvas coordinates into the native presentation target before drawing text layers. This fixes the low-resolution draft canvas case where 320x180 text coordinates were drawn directly onto a larger native surface and pinned text/subtitles to the upper-left.
- Product UAT deselects edit chrome before capturing native host PNGs so selection overlays cannot satisfy the evidence.

## Verification

- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `cargo fmt -p realtime_preview_runtime -p bindings_node --check`
- `cargo test -p realtime_preview_runtime --lib scheduler -- --nocapture`
- `cargo test -p bindings_node --test project_session project_session_add_text_audio_subtitle_use_session_playhead_and_core_timing -- --nocapture`
- `cargo test -p bindings_node --lib scheduler_ -- --nocapture`
- `VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime --test gpu_subset real_wgpu_compositor_renders_bundled_text_overlay_with_render_pass -- --ignored --nocapture`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "composites video external audio text and two-cue SRT" --workers=1`
- `VIDEO_EDITOR_P0_USER_MATERIAL=/Users/zhiwen/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4 corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "P0 user portrait" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "resizing larger and smaller" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`

## Evidence

- `test-results/phase15-3/combo-preview-first-subtitle.png`
- `test-results/phase15-3/combo-preview-second-subtitle.png`
- `test-results/phase15-3/combo-preview-first-subtitle-workspace.png`
- `test-results/phase15-3/combo-preview-second-subtitle-workspace.png`
- `test-results/phase15-3/native-surface-playing-expanded-host.png`
- `test-results/phase15-3/native-surface-playing-narrow-host.png`

Full-page Playwright screenshots still cannot capture the macOS child native surface pixels; the native host PNGs are the authoritative pixel evidence for compositor content.
