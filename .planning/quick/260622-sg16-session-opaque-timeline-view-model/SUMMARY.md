# 260622-sg16 Session Opaque Timeline View Model Summary

## Completed

- Removed raw `track: Track` and `segment: Segment` from project session timeline row, timeline segment, and selected segment view models.
- Added Rust-owned timeline capabilities, track row display/action state, segment keys, waveform material handles, keyframe marker display fields, selected segment timerange labels, and selected segment visual/audio/text/keyframe fields.
- Updated Timeline, FeaturePanel, Inspector, PreviewMonitor, and playback end detection to consume the opaque Rust-owned view model fields.
- Extended project session tests to assert timeline rows/segments and selected segment views do not expose raw `Track` / `Segment` payloads.
- Extended `phase3-source-guards` to reject raw `Track` / `Segment` view-model declarations and product workspace reads such as `row.track`, `segment.segment`, and `selected.segment`.

## Architecture Review

- Subagent architecture check returned `partially correct`: the required next destructive slice is to delete raw `Track` / `Segment` VM exposure while keeping top-level session `draft` response removal as a later, separate boundary.
- Remaining gap: session responses still include top-level `draft`, `commandState`, and `selection` compatibility fields. The next canonical-session slice should remove or further isolate those from product renderer state.

## Verification

- `cargo fmt --all --check` passed.
- `git diff --check` passed.
- `corepack pnpm run test:phase3-source-guards` passed.
- `cargo test -p bindings_node --test project_session -- --nocapture` passed: 24/24.
- `corepack pnpm --dir apps/desktop-electron run build:native` passed.
- `corepack pnpm --dir apps/desktop-electron run build:electron` passed.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline|草稿参数" --reporter=line` passed: 4/4.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line` passed: 10/10.
