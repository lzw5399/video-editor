---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Jianying Top Row Alignment

## Goal

Align the product workbench top row with the Jianying reference: feature tabs belong to the left material column, while the preview monitor titlebar and draft-parameter inspector start on the same row.

## Scope

- Move `.top-feature-bar` from full-width row chrome to the left column only.
- Let `.preview-monitor` and `.inspector-panel` span the feature-tab row plus editor body row.
- Add UI reference assertions that the top feature row, preview monitor, and inspector are row-aligned while the material library stays below the feature tabs.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1`
- Screenshot review for `workspace-1280x800.png`, `workspace-1120x720.png`, `preview-monitor-*`, and material-library crops.
