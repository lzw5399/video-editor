---
status: complete
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Drag Primary Material Cards Summary

## Result

Material cards now present drag-to-timeline as the default product workflow. The persistent thumbnail add button is hidden in the resting card state and revealed on hover or keyboard focus, preserving the fallback command path for accessibility and tests.

## Changes

- Hid `.material-add-icon-button` by default with a short opacity/position transition.
- Revealed the add affordance on draggable card hover and focus-within.
- Extended UI reference regression checks to prove the first material card is draggable, the add affordance is hidden by default, keyboard focus reveals it, and the affordance is hidden again before screenshots.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/ui-reference-regression.spec.ts --workers=1 --reporter=line`
- Refreshed and inspected:
  - `test-results/phase15-3/workspace-1280x800.png`
  - `test-results/phase15-3/workspace-1120x720.png`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "native surface aligned" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep "product user can import a repo video" --workers=1 --reporter=line`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`
