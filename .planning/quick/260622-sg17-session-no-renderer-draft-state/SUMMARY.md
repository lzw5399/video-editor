# 260622-sg17 Session No Renderer Draft State Summary

## Completed

- Removed `draft`, `commandState`, and `selection` from renderer workspace state and project session open/edit response handling.
- Removed renderer-owned blank/demo Draft fixtures and legacy draft-bearing open/save/import/preview command helpers from product command helpers.
- Added session-owned still preview APIs: `requestProjectSessionPreviewFrame` and `requestProjectSessionPreviewSegment`.
- Routed Electron main/preload/renderer preview artifact requests through `{ sessionId, expectedRevision, ... }` session APIs instead of renderer draft payloads.
- Removed `draft` from Rust `ProjectSessionOpenResponse` and `ProjectSessionIntentResponse`.
- Extended project session tests to assert open/edit responses do not expose `draft`, `commandState`, or `selection`, and that session preview APIs reject stale, unknown, and draft-bearing payloads.
- Extended source guards to block session response state fields, renderer `WorkspaceState` canonical project state, renderer response reads, and old draft-bearing preview helpers.

## Architecture Review

- Decision: confirmed for this slice. The renderer no longer owns canonical project data for session open/edit/preview still commands; Rust session remains the canonical draft owner and emits view models/material reads.
- Remaining gap: import material responses still return a material-only response rather than a refreshed full view model. This is acceptable for this cut but should be folded into the broader Rust-owned delta/view-model response contract.

## Verification

- `cargo fmt --all --check` passed.
- `git diff --check` passed.
- `corepack pnpm run test:phase3-source-guards` passed.
- `cargo test -p bindings_node --test project_session -- --nocapture` passed: 25/25.
- `corepack pnpm --dir apps/desktop-electron run build:native` passed.
- `corepack pnpm --dir apps/desktop-electron run build:electron` passed.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline|草稿参数" --reporter=line` passed: 4/4.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line` passed: 10/10.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line` passed: 2/2, both 3s cadence windows reported 90 presented frames, 0 dropped frames, and 0 artifact frame requests.
