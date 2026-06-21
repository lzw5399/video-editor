# Quick Task: 260622-sg26 Session-Owned Add Media Timing

## Objective

Move product text, audio, and subtitle add timing into the Rust project session. The renderer may provide user-facing content/style/material choices and sync the playhead, but it must not decide placement offsets or default clip durations for product add flows.

## Production Boundary

- Rust project session owns add placement through its session playhead.
- Rust command logic owns default text duration and audio material duration fallback.
- SRT import timing is relative to the Rust session playhead, not a renderer-supplied offset.
- Renderer sends product intent only: add this text, import this SRT style, or add this audio material at the current playhead.
- Renderer/native binding must not pass `duration`, `timeOffset`, `targetStart`, `targetTimerange`, `sourceTimerange`, `trackId`, or `segmentId` for product add text/audio/subtitle intents.

## Work Items

1. Extend Rust add text/audio/subtitle intent internals so project session can supply target start/time offset and omit renderer timing fields.
2. Update project-session intent parsing and payload construction for text/audio/subtitle add flows.
3. Update renderer feature-panel callbacks and handlers to sync playhead then send timing-free add intents.
4. Add Rust tests proving session playhead/default timing behavior and rejecting renderer timing fields.
5. Strengthen Phase 3 source guards against renderer timing fields on these product add intents.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
