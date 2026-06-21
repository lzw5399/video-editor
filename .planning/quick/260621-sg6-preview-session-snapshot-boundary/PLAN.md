# Preview Session Snapshot Boundary

## Goal

Remove the renderer-owned realtime preview draft snapshot path. Product preview should sync from the Rust-owned project session by session ID and revision; renderer/preload must not pass a full `Draft` into the realtime preview host.

## Scope

- Add a Rust binding path that updates realtime preview from a canonical project session snapshot.
- Add an Electron main/preload bridge method that accepts only `projectSessionId` and `expectedRevision`.
- Change `App.tsx` playback preparation to call the project-session snapshot bridge instead of `updateDraftSnapshot(workspace.draft, bundlePath)`.
- Remove renderer-visible `updateDraftSnapshot` from the realtime preview host API.
- Add source guards so renderer/preload cannot reintroduce realtime preview full-draft snapshot sync.

## Verification

- `cargo fmt --all --check`
- `cargo test -p bindings_node realtime_preview -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase11-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --grep "native preview host bridge|fallback source guard" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line`
