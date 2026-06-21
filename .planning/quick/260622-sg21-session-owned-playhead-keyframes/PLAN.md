# Quick Task: 260622-sg21 Session-Owned Playhead Keyframes

## Objective

Move selected-segment keyframe timing out of the renderer product path. The renderer may synchronize playhead control state to the Rust project session, but keyframe set/remove intents must not carry renderer-built `at` timestamps.

## Production Boundary

- Rust project session owns draft, selection, timeline semantics, and session playhead state.
- Renderer sends user/control intent only: playhead changed, set keyframe for property, remove active keyframe for property.
- Session playhead is runtime state: it must not persist to `.veproj/project.json`, must not increment draft revision, and must not enter undo/redo.
- Keyframe value and relative timestamp are resolved by Rust from the selected segment and session playhead.

## Work Items

1. Add session playhead state to `ProjectSession`.
2. Add a non-mutating `setSessionPlayhead` project intent.
3. Remove `at` from `setSelectedSegmentKeyframe` and `removeSelectedSegmentKeyframe` project intents.
4. Update renderer App/Inspector callbacks so keyframe commands no longer send absolute timeline timestamps.
5. Strengthen source guards and tests to reject renderer-supplied keyframe `at`.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session_keyframe`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`

