# 260622-sg18 Session Import View Model Response

## Decision

Continue the destructive Rust-owned project session refactor under `production-architecture-review`. `importMaterial` is a project-session intent and must return Rust-owned session state evidence, not leave renderer code to reconcile project/material view state locally.

## Scope

- Add Rust-owned `viewModel`, events, delta, and complete material list fields to `ProjectSessionImportMaterialResponse`.
- Update renderer import handling to atomically consume the returned view model and material list without local reconciliation or a second material-read IPC.
- Keep response contracts free of `draft`, `commandState`, and `selection`.
- Extend project session tests and source guards so import responses cannot regress to material-only renderer reconciliation or full draft exposure.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline|草稿参数" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
