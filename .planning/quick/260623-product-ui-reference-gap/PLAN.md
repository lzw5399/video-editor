---
status: complete
created: 2026-06-23
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Product UI Reference Gap

## Goal

Continue product-mode UI convergence against the Jianying Pro reference screenshots after native preview placement is stable.

## Scope

- Regenerate current product static and narrow screenshots.
- Compare the current first viewport against `docs/ui-reference/jianying-pro/screenshots`.
- Fix the highest-signal product chrome/layout gap that does not require changing Rust-owned editing semantics.
- Preserve native render-graph preview evidence, drag-to-timeline flow, top-right export, and hidden diagnostics.

## Verification

- UI reference regression at required desktop sizes.
- Product static/narrow screenshots in `test-results/phase15-3`.
- Relevant product journey playback smoke if touched layout can affect preview or export.
- `build:electron`, `test:phase3-source-guards`, and `git diff --check`.
