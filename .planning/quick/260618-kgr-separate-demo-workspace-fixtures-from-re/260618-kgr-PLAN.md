---
quick_id: 260618-kgr
description: Separate demo workspace fixtures from real app startup while preserving tests
status: complete
date: 2026-06-18
---

# Quick Task 260618-kgr: Separate demo workspace fixtures from real app startup

## Goal

Make the desktop app boot as a real blank editor by default while keeping the existing demo/material fixture state available only to tests that explicitly request it.

## Tasks

1. Split renderer workspace draft setup into blank production state and demo test fixture state.
   - Files: `apps/desktop-electron/src/renderer/viewModel.ts`, `apps/desktop-electron/src/renderer/App.tsx`
   - Verify: production app uses the blank draft unless a test-only flag requests the demo fixture.

2. Update tests so demo-dependent tests opt into the demo fixture, while the real no-mock workflow runs against the blank startup state.
   - Files: `apps/desktop-electron/tests/*.spec.ts`, `apps/desktop-electron/tests/helpers/*.ts`
   - Verify: workspace/smoke/runtime tests still pass, and real workflow no longer depends on deleting fake segments.

3. Run focused gates for the changed behavior.
   - Verify: `pnpm --filter @video-editor/desktop test:real-workflow` and relevant desktop Playwright tests pass.
