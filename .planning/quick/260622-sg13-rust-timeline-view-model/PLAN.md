# 260622-sg13 Rust Timeline View Model

## Decision

Use the `production-architecture-review` standard and continue the destructive session-ownership refactor. This slice moves timeline display projection from the renderer into Rust project session responses.

## Scope

- Add a Rust-owned project session `viewModel` to open/create and timeline intent responses.
- Include timeline rows, ruler ticks, track labels/status, segment labels/time labels/visual kind, selection handles, and selected track/segment views.
- Update Electron renderer to consume `workspace.viewModel` instead of deriving rows or selected views from `draft`.
- Keep raw draft fields only as transitional compatibility data; do not add new renderer-owned timeline semantics.
- Strengthen source guards so renderer timeline projection and handle construction cannot return.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
