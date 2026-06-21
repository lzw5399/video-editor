# 260622-sg15 Session Edit Controls View Model Summary

## Completed

- Added Rust session `viewModel.editControls` with undo/redo availability, snapping enabled/label state, and selected segment/track availability.
- Switched timeline transport controls and inspector snapping status away from `workspace.commandState` / `workspace.selection` to the Rust-owned edit controls view model.
- Extended project session tests to assert edit controls after create, add, undo, redo, and selection-only intents.
- Extended session payload rejection coverage so renderer-supplied `draft`, `commandState`, and `selection` fields are rejected before execution.
- Extended `phase3-source-guards` so product workspace code cannot reintroduce direct legacy edit-state reads.

## Architecture Review

- Subagent architecture check returned `partially correct`: this is a valid intermediate containment step toward Rust-owned session UI state.
- Remaining gap: `ProjectSessionViewModel` still exposes raw `Track` and `Segment` objects. The next destructive slice should replace raw semantic exposure with Rust-owned track display state, segment display state, keyframe rows, inspector capabilities, and command enablement fields.

## Verification

- `cargo fmt --all --check` passed.
- `git diff --check` passed.
- `corepack pnpm run test:phase3-source-guards` passed.
- `cargo test -p bindings_node --test project_session -- --nocapture` passed: 24/24.
- `corepack pnpm --dir apps/desktop-electron run build:native` passed.
- `corepack pnpm --dir apps/desktop-electron run build:electron` passed.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline|草稿参数" --reporter=line` passed: 4/4 when run sequentially.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line` passed: 10/10 when run sequentially.
- Note: an initial attempt ran workspace and product Playwright suites concurrently; native Electron/window state interference caused transient startup/surface failures. Sequential reruns passed.
