---
status: complete
completed_at: "2026-06-21T14:53:01Z"
---

# Preview Session Snapshot Boundary Summary

Removed the renderer-owned realtime preview draft snapshot boundary. Electron now asks the realtime preview host to sync from a Rust project session by `projectSessionId` and `expectedRevision`; Rust clones the canonical session draft and bundle path internally before updating the preview service.

## Changes

- Added project-session snapshot reads for realtime preview in `bindings_node`.
- Replaced renderer/main/preload `updateDraftSnapshot(draft, bundlePath)` with `updateProjectSessionSnapshot(projectSessionId, expectedRevision)`.
- Removed the public native `updateRealtimePreviewDraftSnapshot` API and its request payload.
- Added stale, unknown-session, and renderer-draft-payload rejection tests.
- Added source guards against reintroducing realtime preview full-draft snapshot sync.

## Verification

- `cargo fmt --all --check`
- `cargo test -p bindings_node realtime_preview -- --nocapture`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase11-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --grep "native preview host bridge|fallback source guard" --reporter=line`
- `corepack pnpm run test:no-product-fallback`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line`

Cadence evidence: both single-video and video + external-audio + text + two-cue-SRT product preview tests presented 90/90 accounted frames with real `renderGraphGpu` evidence and no artifact fallback.
