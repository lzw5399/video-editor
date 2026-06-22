---
status: complete
created: 2026-06-23
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Clean UI Reference Media

## Goal

Make product UI reference screenshots represent a healthy editing workspace instead of a demo error-state workspace with missing or failed materials.

## Production Decision

Partially correct current chain: product UI correctly exposes missing/failed materials when they are real project facts, but UI reference screenshots should use real available fixture media. Hiding error labels globally would be wrong; changing the screenshot fixture setup is the correct boundary.

## Scope

- Inspect how `ui-reference-regression.spec.ts` launches the workspace and imports materials.
- Replace or augment the UI reference screenshot fixture with real local test media from `apps/desktop-electron/tests/fixtures/media`.
- Keep tests that intentionally cover missing/failed demo material states unchanged.
- Add/adjust UI reference assertions so default product screenshots do not contain missing/failed material copy.

## Verification

- `build:electron`
- UI reference regression and refreshed static/narrow screenshots.
- Product playback native-surface smoke if screenshot setup touches product workspace startup.
- `test:phase3-source-guards`
- `git diff --check`
