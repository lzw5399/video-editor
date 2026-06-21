# Quick Task: 260622-sg22 Session-Owned Split Playhead

## Objective

Move selected-segment split timing out of the renderer product path. The renderer may synchronize playhead control state to the Rust project session, but split-selected-segment intent must not carry renderer-built `splitAt` timestamps.

## Production Boundary

- Rust project session owns selection, timeline semantics, and session playhead state.
- Renderer sends user/control intent only: playhead changed, split selected segment.
- Session playhead is runtime state: it must not persist to `.veproj/project.json`, must not increment draft revision, and must not enter undo/redo.
- Split position is resolved by Rust from selected segment + session playhead.

## Work Items

1. Remove `splitAt` from `splitSelectedSegmentIntent` in TS and Rust project-session intent contracts.
2. Make Rust session convert split intent to command payload using `self.playhead`.
3. Update renderer Timeline/App callbacks so split button sends no timeline timestamp.
4. Strengthen source guards and tests to reject renderer-supplied split `splitAt`.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
