# 260622-sg17 Session No Renderer Draft State

## Decision

Continue the destructive Rust-owned project session boundary under `production-architecture-review`. Product renderer state must no longer hold the canonical draft, command state, or timeline selection. Session open/edit responses should expose view models and metadata only; preview still/segment requests should read the canonical draft from the Rust session.

## Scope

- Remove `draft` from project session open and timeline intent response contracts.
- Remove `draft`, `commandState`, and `selection` from renderer `WorkspaceState` and product response handling.
- Add session-owned preview frame and preview segment N-API commands that accept `{ sessionId, expectedRevision, ... }`, resolve the canonical draft through `project_session_snapshot`, and reject stale/unknown sessions.
- Route Electron main/preload/renderer preview requests through the session-owned preview APIs instead of renderer-built `requestPreviewFrame` / `requestPreviewSegment` draft payloads.
- Extend tests and source guards so product session code cannot reintroduce raw draft/edit-state response fields or renderer-owned still preview draft payloads.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline|草稿参数" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
