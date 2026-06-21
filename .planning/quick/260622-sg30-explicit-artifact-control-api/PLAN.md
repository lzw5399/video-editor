# Quick Task: 260622-sg30 Explicit Artifact Control API

## Objective

Remove the renderer's generic `CommandEnvelope` construction from product/developer resource artifact status, task, quota, and cleanup controls. Artifact store operations are Rust-owned derived-resource controls and should cross Electron through explicit native APIs.

## Production Boundary

- Renderer must not construct artifact/resource command envelopes.
- Electron preload/main should expose explicit artifact status, generation action, quota, and garbage-collection APIs.
- Rust binding should expose explicit artifact entry points that call `artifact_store_service` directly.
- Generic `executeCommand` may remain temporarily for runtime capability compatibility, but product artifact controls must leave that path.

## Work Items

1. Add explicit Rust Node-API functions for artifact status, refresh, retry/resume/cancel, quota, and garbage collection.
2. Remove artifact commands from the public `executeCommand` allowlist.
3. Add typed nativeBinding/preload/main wrappers and test observation support.
4. Update renderer resource handlers to call explicit APIs instead of renderer-built command envelopes.
5. Remove artifact command builders/imports from renderer command helpers.
6. Add source guards/tests preventing renderer artifact control from returning to generic command construction.

## Verification

- `cargo test -p bindings_node artifact_store_commands`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase14-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- Focused workspace artifact/resource tests if available.
- `cargo fmt --all --check`
- `git diff --check`
