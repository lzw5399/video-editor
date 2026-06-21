# Summary: 260622-sg27 Session-Owned Text Presets

## Status

Completed.

## Changes

- Thinned product `addTextSegmentIntent` to `content` only.
- Thinned product `importSubtitleSrtIntent` to `srtContent` only.
- Added a Rust project-session default text segment helper that owns add-time text/subtitle source, bundled font, style, stroke, shadow, text box, layout region, wrapping, and empty bubble/effect fields.
- Removed renderer add-time `TextSegment` construction and the `createDefaultTextSegment` helper from `FeaturePanel`.
- Updated project-session tests to prove Rust-owned text/subtitle defaults and reject legacy full text/style/layout payloads from renderer intents.
- Strengthened source guards against renderer/native binding add-time text preset fields and callback signatures.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
