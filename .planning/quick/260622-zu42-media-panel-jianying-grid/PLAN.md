---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Media Panel Jianying Grid

## Goal

Move the product media panel closer to the Jianying reference by replacing the diagnostic-looking material list with a thumbnail-grid material library while preserving Rust-owned drag/add semantics.

## Scope

- Keep imported materials draggable with the existing material-id-only drag payload.
- Keep the accessible add-to-timeline button, but make drag/grid browsing the primary visual workflow.
- Reduce diagnostic/resource chip dominance in product mode.
- Preserve existing product E2E selectors and drag-to-timeline behavior.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- Targeted product drag/add Playwright gate
- 1280x800 and 1120x720 screenshots after change
- `git diff --check`
