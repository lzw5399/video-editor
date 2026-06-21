# 260622-sg14 Session Project Summary View Model

## Decision

Continue the destructive session ownership refactor under `production-architecture-review`. The next renderer-owned draft derivation to remove is project summary state used by preview, canvas inspector, and playback end detection.

## Scope

- Add a Rust-owned `viewModel.project` summary to project session responses.
- Include draft name, canvas config, real sequence duration, frame duration, track count, and material count.
- Switch preview title, canvas inspector readouts, and playback end alignment to consume `workspace.viewModel.project`.
- Remove renderer helpers that scan `workspace.draft.tracks` or `workspace.draft.materials` for project summary decisions.
- Guard against renderer product views reintroducing project summary derivation from `workspace.draft`.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls|professional timeline|草稿参数" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/inspector-modal.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
