# Summary: 260622-sg36 Align Workspace Product Contract

## Result

Completed. `workspace.spec.ts` now asserts the current production contract:

- Project-session observations expose raw Rust intent names instead of legacy low-level aliases.
- Text/audio/subtitle/timeline tests assert Rust-owned intent results and returned UI state, not renderer-authored timeline payload fields.
- Product realtime preview tests no longer require hidden developer diagnostic labels.
- Ambiguous add-at-occupied-playhead cases now seek to a free playhead for accepted-path coverage.
- Export and preview locators use exact labels where accessible names overlap.

## Verification

- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`

`product-preview-cadence.spec.ts` reported 90/90 accounted frames in both single-video and video+external-audio+text+two-cue-SRT scenarios.
