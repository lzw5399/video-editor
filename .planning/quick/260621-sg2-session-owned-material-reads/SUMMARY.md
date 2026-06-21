# Session-Owned Material Reads Summary

## Result

Product material listing and missing-material diagnostics now read from the Rust project session instead of sending renderer-held draft and bundle path payloads back through legacy commands.

## Changes

- Added `listProjectSessionMaterials` and `listProjectSessionMissingMaterials` binding APIs using `sessionId` plus `expectedRevision`.
- Routed Electron main/preload/native binding types through the new session read APIs.
- Updated product renderer material refresh/open/missing-material paths to use session reads.
- Removed renderer builders for legacy `listMaterials` and `listMissingMaterials`.
- Added source guard coverage blocking legacy material-read builders and command payloads in product renderer code.

## Verification

- `cargo fmt --all`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `cargo test -p bindings_node material_service -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback rejects missing render-graph GPU compositor evidence" --reporter=line`
