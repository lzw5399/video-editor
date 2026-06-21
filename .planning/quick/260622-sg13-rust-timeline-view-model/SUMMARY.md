# 260622-sg13 Rust Timeline View Model Summary

## Completed

- Added Rust session `viewModel` responses for project open/create and timeline intents.
- Moved timeline rows, selection handles, selected segment/track views, display labels, visual kind, and ruler ticks into `bindings_node`.
- Switched renderer timeline, inspector, feature panel, workspace shell, delete selection, and preview seek follow-up to consume `workspace.viewModel`.
- Preserved manual create/open paths by passing Rust `viewModel` through `openWorkspaceFromDraft`.
- Removed renderer-owned timeline row/selection projection helpers and handle encoders.
- Added encoded handle and view-model assertions to `project_session` tests, including product time-label format.
- Extended `phase3-source-guards` to reject renderer timeline projection and handle encoding reintroduction.

## Verification

- `cargo fmt --all --check` passed.
- `git diff --check` passed.
- `corepack pnpm run test:phase3-source-guards` passed.
- `cargo test -p bindings_node --test project_session -- --nocapture` passed: 24/24.
- `corepack pnpm --dir apps/desktop-electron run build:native` passed.
- `corepack pnpm --dir apps/desktop-electron run build:electron` passed.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline" --reporter=line` passed: 3/3.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line` passed: 10/10.
