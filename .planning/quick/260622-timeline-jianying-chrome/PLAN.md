---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Timeline Jianying Chrome Slice

## Goal

Move the product timeline/track area closer to the Jianying Mac reference screenshots while preserving Rust-owned editing semantics, drag-to-timeline behavior, native preview evidence, and existing product mode boundaries.

## Scope

- Tighten bottom timeline toolbar grouping, spacing, icon treatment, track header width, row density, and segment visuals.
- Use existing `/icons` through the app icon pipeline where possible; do not introduce text-only controls where an icon button already exists.
- Keep all timeline editing actions routed through existing session intent handlers.
- Update product UI screenshot/layout gates so the timeline chrome cannot regress to bulky/debug-like rows.

## Verification

- Product workspace screenshots at 1280x800 and 1120x720.
- Timeline layout Playwright assertions for compact toolbar, row density, icon buttons, drag target, and no product diagnostics.
- Existing product/user journey checks touched by timeline interactions.
- `build:electron`, source guards, and `git diff --check`.
