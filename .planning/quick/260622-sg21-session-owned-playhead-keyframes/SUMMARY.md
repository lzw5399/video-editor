# Summary: 260622-sg21 Session-Owned Playhead Keyframes

## Status

Completed.

## Changes

- Added runtime-only `setSessionPlayhead` to the Rust project session and stored playhead state outside persisted draft/undo semantics.
- Removed renderer-supplied `at` from selected-segment keyframe set/remove project intents.
- Updated renderer keyframe commands to sync session playhead first, then send property-only keyframe intents.
- Updated Inspector deletion semantics so row times are display-only; only the active playhead keyframe can be removed.
- Strengthened source guards and Rust tests to reject legacy keyframe `at` payloads.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session_keyframe`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
