# Summary: 260622-sg37 Remove Test Legacy Intent Aliases

## Result

Completed. Remaining product E2E helpers now expose raw Rust project-session intent names instead of translating them back to legacy structural command names.

- Removed `legacyCommandNameForProjectIntent` from `tests/helpers/userJourney.ts`.
- Removed `commandNameForProjectIntent` from `inspector-modal.spec.ts`.
- Updated product journey waits/assertions to use `deleteSelectedSegment` and `updateSelectedSegmentVisual`.
- Removed stale product UAT assumptions that renderer-selected text/audio duration overrides should control Rust-owned add intents.
- Added a Phase 3 source guard and self-test to block future test-side project-session alias mappers.

Legacy names such as `deleteSegment` and `updateSegmentVisual` remain only in negative direct-native-command guards.

## Verification

- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/inspector-modal.spec.ts tests/product-user-journey.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`

`product-preview-cadence.spec.ts` reported 90/90 accounted frames in both single-video and video+external-audio+text+two-cue-SRT scenarios.
