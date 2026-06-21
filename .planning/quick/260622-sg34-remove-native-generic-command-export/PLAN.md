# Quick Task: 260622-sg34 Remove Native Generic Command Export

## Objective

Remove the generic `executeCommand` export from the JavaScript-facing native addon while preserving the Rust `bindings_node::execute_command` function for direct Rust compatibility and negative tests.

## Production Boundary

- Packaged Electron native module must not export `executeCommand` to JavaScript.
- Rust direct tests may still call `bindings_node::execute_command` until legacy generated command contracts are retired.
- Explicit N-API functions remain the only JavaScript/native product control surface.

## Work Items

1. Remove the `#[napi]` export from `execute_command` while leaving the Rust function public.
2. Regenerate `apps/desktop-electron/native/index.cjs` and `index.d.ts`.
3. Add a source guard rejecting native JS `executeCommand` exports.
4. Verify Rust direct tests still pass and Electron build still loads the native addon.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node --test binding_smoke`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
