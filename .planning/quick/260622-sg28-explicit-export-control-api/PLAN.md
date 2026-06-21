# Quick Task: 260622-sg28 Explicit Export Control API

## Objective

Remove the renderer's generic `CommandEnvelope` construction from the product export control path. Starting export already uses the Rust project-session API; querying and cancelling export should also use explicit native APIs instead of `executeCommand`.

## Production Boundary

- Renderer should not construct product export command envelopes.
- Electron preload/main should expose explicit export status/cancel APIs.
- Rust binding should expose explicit `get_export_job_status` and `cancel_export` entry points that internally reuse the existing export registry.
- Generic `executeCommand` may remain temporarily for diagnostics/audio/artifact containment, but product export control must leave that path.

## Work Items

1. Add explicit Rust Node-API functions for export status and cancellation.
2. Add typed nativeBinding/preload/main request and response wrappers.
3. Update renderer export status/cancel handlers to call explicit APIs.
4. Remove now-unused renderer export command builders/imports.
5. Add source guards/tests preventing renderer export control from reintroducing `buildGetExportJobStatusCommand`, `buildCancelExportCommand`, or generic export `executeCommand`.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node export_commands`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
