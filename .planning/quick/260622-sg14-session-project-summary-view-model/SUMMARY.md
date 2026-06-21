# 260622-sg14 Session Project Summary View Model Summary

## Completed

- Added Rust session `viewModel.project` with draft name, canvas config, real sequence duration, frame duration, track count, and material count.
- Switched preview monitor inputs, canvas inspector readouts, and playback end detection to the Rust-owned project summary.
- Removed renderer project-summary helpers that scanned `draft.tracks` / `draft.materials`.
- Made the preview titlebar show the full product canvas readout, not just a hidden title attribute.
- Updated the standalone inspector modal test helper to observe project session intents.
- Extended `phase3-source-guards` so product renderer code cannot read `workspace.draft.metadata`, `workspace.draft.canvasConfig`, `workspace.draft.tracks`, or `workspace.draft.materials`.

## Verification

- `cargo fmt --all --check` passed.
- `git diff --check` passed.
- `corepack pnpm run test:phase3-source-guards` passed.
- `cargo test -p bindings_node --test project_session -- --nocapture` passed: 24/24.
- `corepack pnpm --dir apps/desktop-electron run build:native` passed.
- `corepack pnpm --dir apps/desktop-electron run build:electron` passed.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline|草稿参数" --reporter=line` passed: 4/4.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/inspector-modal.spec.ts --reporter=line` passed: 1/1.
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line` passed: 10/10.
