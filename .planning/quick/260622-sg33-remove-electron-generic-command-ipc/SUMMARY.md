# Summary: 260622-sg33 Remove Electron Generic Command IPC

## Status

Completed.

## Changes

- Removed Electron main `core:executeCommand` IPC registration.
- Removed the Electron `nativeBinding.executeCommand` wrapper, native binding type field, validation, and cached binding entry.
- Kept Rust `bindings_node::execute_command` for direct compatibility tests while removing it from the Electron shell boundary.
- Updated stale test names/messages that described product actions as routing through `executeCommand`.
- Extended source guards to reject Electron main/native generic `executeCommand` IPC or wrappers.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron test electron-smoke.spec.ts`
- `corepack pnpm --dir apps/desktop-electron run test:packaged-smoke`
