# Summary: 260622-sg26 Session-Owned Add Media Timing

## Status

Completed.

## Changes

- Removed renderer/native-binding product timing fields from `addTextSegmentIntent`, `addAudioSegmentIntent`, and `importSubtitleSrtIntent`.
- Updated text/audio/subtitle add handlers to sync the Rust project-session playhead before sending timing-free product intents.
- Moved add text/audio/subtitle placement into the Rust project session by passing session playhead into internal command payloads.
- Kept low-level command compatibility with optional internal duration/target fields while product session supplies defaults: text defaults to the core intent duration, audio defaults to material duration, and subtitles offset from the session playhead.
- Removed product UI controls that previously supplied add-time text/audio/subtitle timing values.
- Added binding tests for session-owned text/audio/subtitle add timing and rejection of legacy renderer timing fields.
- Strengthened Phase 3 source guards against renderer/native binding timing or placement fields on text/audio/subtitle add intents.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session`
- `cargo test -p draft_commands`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
