# Selection Handle Intent Boundary Summary

## Outcome

Completed. Renderer-facing project selection no longer uses `selectTimelineSegments { segmentIds, trackIds }`. Timeline clicks and track controls now send `selectTimelineItemIntent { itemHandle }`; the Rust project session decodes, validates, and resolves that handle against the canonical draft before updating selection.

## Changes

- Replaced Electron `ProjectIntent` selection payload with `selectTimelineItemIntent`.
- Propagated encoded selection handles through timeline rows, selected-track views, feature-panel mute, and inspector mute controls.
- Removed renderer-side segment-to-track lookup from selection dispatch.
- Added strict Rust handle parsing with percent-decoded components and stale/malformed/legacy payload rejection tests.
- Updated Playwright command observation for project-session selection and track/segment command aliases.
- Tightened Phase 3 guards against legacy selection intents and `segmentIds`/`trackIds` on the new selection intent.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `cargo test -p bindings_node --test binding_smoke execute_command_rejects_public_timeline_edit_commands -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "command-only timeline edit|multitrack controls" --reporter=line`

## Remaining Gap

Selection handles are still derived from renderer view-model rows. This removes raw ID arrays from the IPC selection boundary, but the next destructive slice should move timeline view-model/handle generation itself into Rust so Electron consumes opaque accepted item handles rather than formatting them locally.
