# Quick Task: 260622-sg37 Remove Test Legacy Intent Aliases

## Objective

Remove remaining test-side project-session intent aliases that translate Rust-owned intent names back to legacy low-level command names.

## Production Boundary

- Product and workspace E2E must assert raw project-session intent names such as `deleteSelectedSegment`, `editSelectedText`, and `updateSelectedSegmentVisual`.
- Test helpers must not normalize current Rust session behavior into old `deleteSegment`, `editTextSegment`, `updateSegmentVisual`, or similar structural command names.
- Low-level command names may remain only as explicit forbidden-name guards proving they were not used.

## Work Items

1. Inspect test helpers and focused specs that still map `intentKind` to low-level aliases.
2. Remove alias mapping from helpers and specs; expose raw `intentKind` observations.
3. Update product journey assertions and waits to use raw session intent names.
4. Keep legacy command names only in negative assertions/source guards.

## Verification

- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/inspector-modal.spec.ts tests/product-user-journey.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`
