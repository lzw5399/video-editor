# Summary: 260622-sg24 Session-Owned Move Target

## Status

Completed.

## Changes

- Replaced product `moveSelectedSegmentIntent` payload `delta` with `startAt`.
- Updated timeline move drag completion to send the target segment start, not raw drag delta.
- Updated Rust project session to derive canonical move command from selected segment + target start.
- Added project-session tests proving move behavior and rejecting legacy `delta`.
- Added source guards blocking renderer/native binding reintroduction of move `delta` payloads.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session_move`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
