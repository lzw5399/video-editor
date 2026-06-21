# 260622-sg15 Session Edit Controls View Model

## Decision

Continue the destructive session ownership refactor under `production-architecture-review`. Product edit controls must stop reading renderer-held `commandState` and `selection`; Rust project sessions should emit the actionable control state as part of `ProjectSessionViewModel`.

## Scope

- Add Rust-owned `viewModel.editControls` to project session responses.
- Include undo/redo availability, snapping enabled/label state, selected segment availability, and selected track availability.
- Switch timeline transport controls and canvas inspector snapping status to consume `workspace.viewModel.editControls`.
- Reject renderer-supplied project session state fields such as `draft`, `commandState`, and `selection` at the session intent boundary.
- Guard product workspace components against reintroducing direct `workspace.commandState` / `workspace.selection` reads for edit controls.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline|草稿参数" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
