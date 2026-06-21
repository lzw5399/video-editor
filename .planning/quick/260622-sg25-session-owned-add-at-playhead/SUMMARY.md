# Summary: 260622-sg25 Session-Owned Add At Playhead

## Status

Completed.

## Changes

- Added an internal `targetStart` field to the Rust add-timeline intent payload so the project session can supply placement without exposing placement fields to the renderer product path.
- Updated project-session add intent execution to place new material segments at the session playhead.
- Updated renderer add flow to sync the Rust project-session playhead first, then send a material-only `addTimelineSegmentIntent`.
- Added Rust tests proving add uses the session playhead and rejects renderer placement fields.
- Strengthened Phase 3 source guards so renderer/native binding code cannot pass `targetStart`, `targetTimerange`, `sourceTimerange`, `trackId`, or `segmentId` for product add intents.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
