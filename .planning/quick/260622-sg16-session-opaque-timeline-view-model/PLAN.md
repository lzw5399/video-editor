# 260622-sg16 Session Opaque Timeline View Model

## Decision

Continue the destructive session ownership refactor under `production-architecture-review`. `ProjectSessionViewModel` must stop exposing raw `Track` and `Segment` objects to product renderer code; Rust should emit explicit display, capability, and edit-state fields.

## Scope

- Remove `track: Track` and `segment: Segment` from timeline row, timeline segment, and selected segment view models.
- Add Rust-owned timeline capabilities, track row action state, segment keys, waveform handles, keyframe marker display rows, selected segment visual/audio/text/keyframe fields, and source/target labels.
- Switch Timeline, FeaturePanel, Inspector, PreviewMonitor, and playback end detection to the opaque Rust-owned fields.
- Guard session view models and product workspace code against reintroducing raw `Track` / `Segment` VM exposure or consumption.
- Keep top-level session `draft` response payloads unchanged for this slice; removing them is a separate canonical-session boundary change.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline|草稿参数" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
