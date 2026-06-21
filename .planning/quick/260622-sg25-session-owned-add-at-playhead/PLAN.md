# Quick Task: 260622-sg25 Session-Owned Add At Playhead

## Objective

Move add-material placement into the Rust project session. The renderer may choose a material and control the playhead, but it must not decide the target timerange or rely on default append behavior for the product path.

## Production Boundary

- Rust project session owns track selection, segment ID allocation, source/target timeranges, snapping/overlap validation, and canonical draft mutation.
- Renderer sends user intent only: add selected material at the current playhead.
- Session playhead is runtime state and must be synchronized before the add command.
- Renderer must not pass `targetStart`, `targetTimerange`, `sourceTimerange`, `trackId`, or `segmentId` for product add.

## Work Items

1. Extend Rust add-timeline intent internals so the project session can provide a target start.
2. Make product `addTimelineSegmentIntent` use session playhead as the target start.
3. Update renderer add flow to sync session playhead before sending the material-only add intent.
4. Add tests proving add uses session playhead and rejecting renderer placement fields.
5. Strengthen source guards against renderer add placement fields.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session_add`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
