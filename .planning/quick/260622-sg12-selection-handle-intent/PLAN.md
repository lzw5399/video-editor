# Selection Handle Intent Boundary

## Goal

Remove the renderer-facing `selectTimelineSegments { segmentIds, trackIds }` project intent and route timeline selection through a single session-resolved item handle. This is a destructive boundary cleanup toward Rust-owned timeline semantics: Electron may choose a visible timeline item, but Rust validates and resolves the actual `TimelineSelection`.

## Scope

- Replace Electron `selectTimelineSegments` project-session calls with `selectTimelineItemIntent`.
- Propagate selection handles through timeline rows, inspector, and feature-panel track mute controls.
- Keep `TimelineEditPayload::SelectTimelineSegments` Rust-internal only.
- Add Rust tests rejecting old/malformed/stale selection handles and extra legacy selection fields.
- Tighten Phase 3 source guards so renderer/main/preload cannot reintroduce selection ID arrays as project intent payloads.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
