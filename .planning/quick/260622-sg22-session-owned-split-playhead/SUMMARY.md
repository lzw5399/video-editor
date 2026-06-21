# Summary: 260622-sg22 Session-Owned Split Playhead

## Status

Completed.

## Changes

- Removed renderer-supplied `splitAt` from `splitSelectedSegmentIntent` in the Electron/Rust project-session contract.
- Updated the timeline split button so it emits only user intent; App syncs session playhead first, then sends split intent without timing payload.
- Updated Rust project session split conversion to use the runtime session playhead when building the canonical draft command.
- Added project-session tests proving split uses session playhead and rejects legacy `splitAt`.
- Added source guards blocking renderer/native binding reintroduction of selected-segment split timestamps.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session_split`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
