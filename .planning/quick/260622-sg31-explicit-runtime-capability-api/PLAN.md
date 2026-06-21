# Quick Task: 260622-sg31 Explicit Runtime Capability API

## Objective

Remove the renderer's final product-facing generic `CommandEnvelope` construction for runtime capability probing. Runtime readiness should cross Electron through an explicit native API.

## Production Boundary

- Renderer must not construct `probeRuntimeCapabilities` command envelopes.
- Electron preload/main/nativeBinding should expose a zero-argument `probeRuntimeCapabilities` API.
- Rust binding should expose explicit `probeRuntimeCapabilities()` and continue using bundled-only runtime discovery.
- Generic `executeCommand` may remain temporarily for compatibility/testing, but product renderer should no longer use it.

## Work Items

1. Add explicit Rust Node-API `probeRuntimeCapabilities`.
2. Add typed nativeBinding/preload/main wrappers and test mock support.
3. Update renderer runtime diagnostics to call the explicit API.
4. Remove the runtime capability command builder from renderer command helpers.
5. Add source guards preventing renderer `executeCommand`/runtime command builder from returning.

## Verification

- `cargo test -p bindings_node runtime_capabilities`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `cargo fmt --all --check`
- `git diff --check`
