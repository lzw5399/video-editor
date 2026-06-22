---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Jianying Preview Alignment

## Goal

Move the product workbench closer to the Jianying Pro reference and fix the P0 native preview placement/size contract so playback frames are visually centered in the preview canvas, not offset toward the lower-left corner.

## Scope

- Compare current product screenshots against `docs/ui-reference/jianying-pro/screenshots`.
- Preserve the real `renderGraphGpu` native surface path; no DOM/artifact preview fallback may count as success.
- Tighten native surface/canvas screenshot gates for static, playing, and narrow-window states.
- Keep product mode free of engineering diagnostics in the first visual layer.

## Verification

- Current app screenshots for static workbench, playback, and narrow-window views.
- Native surface bounds and visible-content coverage assertions against `.preview-canvas`.
- Existing product preview cadence and P0 user journey gates remain green.
