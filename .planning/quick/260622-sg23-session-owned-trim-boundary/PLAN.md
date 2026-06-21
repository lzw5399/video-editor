# Quick Task: 260622-sg23 Session-Owned Trim Boundary

## Objective

Move selected-segment trim range derivation out of the renderer product path. The renderer may send a user-selected trim boundary time, but it must not send a derived trim delta or construct target/source timeranges.

## Production Boundary

- Rust project session owns selection, segment lookup, timerange math, snapping, overlap validation, and canonical draft mutation.
- Renderer sends user intent only: trim selected segment edge to an absolute timeline boundary.
- Renderer must not send `targetTimerange`, `sourceTimerange`, `segmentId`, `trackId`, or renderer-derived trim `delta` for the product session path.
- Rust derives the target timerange from selected segment + direction + trim boundary.

## Work Items

1. Replace `trimSelectedSegmentIntent` project-session payload `delta` with `trimAt`.
2. Update timeline drag completion to compute the user boundary time and send `trimAt`, not a delta.
3. Update Rust project session to derive the target trim range from selected segment + trim boundary.
4. Add tests rejecting legacy renderer `delta` and proving trim uses selected segment + `trimAt`.
5. Strengthen source guards so renderer/native binding cannot reintroduce product trim `delta`.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session_trim`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
