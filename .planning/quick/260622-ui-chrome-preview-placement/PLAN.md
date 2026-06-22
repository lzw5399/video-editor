---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# UI Chrome And Preview Placement Pass

## Goal

Move the product workbench closer to the Jianying Mac reference screenshots while preserving the real native render-graph preview path. This pass focuses on visible product chrome and preview placement evidence, not debug UI or artifact fallback.

## Scope

- Compare current product captures against `docs/ui-reference/jianying-pro/screenshots`.
- Audit top feature tabs, preview controls, timeline toolbar/track headers, product diagnostics, and native preview placement.
- Implement one coherent high-impact UI/preview slice with icon-backed controls from `/icons`.
- Keep product preview evidence native: no DOM overlay or artifact fallback may satisfy product preview tests.

## Verification

- Product screenshots for static workbench, playback, and narrow window.
- Product Playwright gates for native preview placement and fallback exclusion.
- UI/source guards that prevent diagnostic/product-boundary regressions.

## Result

- Completed top feature navigation parity slice: primary tabs stay unclipped at 1280x800 and 1120x720, with secondary features reachable through a product overflow menu.
- Replaced dead placeholder panels for sticker/effect/transition/filter/adjust/template/digital-human categories with product-like showcase rails and cards.
- Removed product-facing "暂未开放/暂不可用/暂未接入" copy from the top feature surfaces covered by this slice.
