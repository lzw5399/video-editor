# Quick Task: 260622-sg24 Session-Owned Move Target

## Objective

Move selected-segment move range derivation out of the renderer product path. The renderer may send the user-selected target start time, but it must not send a derived move delta or construct target/source timeranges.

## Production Boundary

- Rust project session owns selection, segment lookup, timerange math, snapping, overlap validation, magnet behavior, and canonical draft mutation.
- Renderer sends user intent only: move selected segment to an absolute timeline start.
- Renderer must not send `targetTimerange`, `sourceTimerange`, `segmentId`, `trackId`, or renderer-derived move `delta` for the product session path.
- Rust derives the target move command from selected segment + target start.

## Work Items

1. Replace `moveSelectedSegmentIntent` project-session payload `delta` with `startAt`.
2. Update timeline drag completion to compute the target segment start and send `startAt`, not delta.
3. Update Rust project session to call the canonical move command using selected segment and target start.
4. Add tests rejecting legacy renderer `delta` and proving move uses selected segment + `startAt`.
5. Strengthen source guards so renderer/native binding cannot reintroduce product move `delta`.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session_move`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
