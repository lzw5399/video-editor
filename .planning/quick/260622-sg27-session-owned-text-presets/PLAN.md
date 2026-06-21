# Quick Task: 260622-sg27 Session-Owned Text Presets

## Objective

Move product add-time text and subtitle default style/layout construction from the renderer into the Rust project session. The renderer should provide user content only; Rust session should build the default `TextSegment` preset used for new text and subtitle imports.

## Production Boundary

- Renderer add text intent sends text content only.
- Renderer subtitle import intent sends SRT content only.
- Rust project session owns add-time `TextSegmentSource`, default font, style, text box, layout region, wrapping, and empty effect/bubble fields.
- Existing selected text editing may continue to send a full `TextSegment` because it edits an existing segment's properties.
- Low-level draft commands remain capable of accepting full text/style payloads as internal/compatibility APIs.

## Work Items

1. Thin product `ProjectIntent` and native binding types for add text and import subtitle.
2. Add Rust session helper for default text/subtitle `TextSegment` presets.
3. Update renderer feature-panel callbacks and handlers to stop constructing add-time `TextSegment` templates.
4. Update tests proving Rust-owned defaults and rejection of renderer style/layout fields.
5. Strengthen source guards against renderer add-time text preset construction.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node project_session`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
