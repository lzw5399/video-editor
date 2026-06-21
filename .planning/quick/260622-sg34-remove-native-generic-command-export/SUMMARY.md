# Summary: 260622-sg34 Remove Native Generic Command Export

## Status

Completed.

## Changes

- Removed the `#[napi]` export from Rust `execute_command` while keeping the public Rust function available for direct compatibility tests.
- Rebuilt the native addon wrapper locally and confirmed it no longer exposes JS `executeCommand`.
- Added a phase3 source guard rejecting generated native JS/d.ts `executeCommand` exports.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node --test binding_smoke`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
