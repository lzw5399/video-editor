# Quick Task: 260622-sg32 Explicit Runtime Discovery API

## Objective

Remove the production preload bridge's generic `executeCommand` exposure by replacing the remaining renderer-visible runtime discovery envelope path with an explicit native API.

## Production Boundary

- Renderer/preload product API must not expose `executeCommand`.
- Runtime discovery should use `probeMediaRuntime()` instead of a renderer-built `probeMediaRuntime` command envelope.
- Main/native/Rust may keep compatibility `executeCommand` for direct tests and legacy generated command contracts, but it must no longer be reachable from the normal renderer bridge.
- Tests should use explicit product APIs or test observation bridges, not the product `videoEditorCore.executeCommand` escape hatch.

## Work Items

1. Add explicit Rust Node-API `probeMediaRuntime`.
2. Add typed nativeBinding/preload/main wrappers and preserve test mock/runtime behavior.
3. Update packaged smoke and real workflow helpers to call `probeMediaRuntime()`.
4. Remove `executeCommand` from the preload-exposed `videoEditorCore` product API and update smoke/setup tests.
5. Add source guards preventing preload from exposing generic `executeCommand` to renderer.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node binding_smoke`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
