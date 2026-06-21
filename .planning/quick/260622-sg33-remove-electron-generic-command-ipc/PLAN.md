# Quick Task: 260622-sg33 Remove Electron Generic Command IPC

## Objective

Delete the remaining Electron-side generic `executeCommand` IPC/wrapper so the desktop shell exposes only explicit product APIs to renderer-facing code. Keep the Rust `bindings_node::execute_command` compatibility function for direct Rust contract tests until the generated legacy command contract is retired.

## Production Boundary

- Electron main must not register `core:executeCommand`.
- Electron nativeBinding must not expose an `executeCommand` wrapper or require the native addon to provide it.
- Renderer/preload already do not expose generic command dispatch and should stay that way.
- Test observations should continue through explicit observation bridges and project-session call recording, not through product generic command dispatch.

## Work Items

1. Remove `executeCommand` import, wrapper, type field, native binding validation, and cached binding entry from `apps/desktop-electron/src/main/nativeBinding.ts`.
2. Remove `core:executeCommand` IPC handler and generic command test mock helpers from `apps/desktop-electron/src/main/index.ts` if no longer referenced.
3. Update source guards to reject Electron main/native generic command IPC/wrapper reintroduction while allowing Rust direct compatibility tests.
4. Rename stale tests/messages that say product actions route through `executeCommand`.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron test electron-smoke.spec.ts`
