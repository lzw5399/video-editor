# Summary: 260622-sg41 Remove Generic Preview Commands

## Status

Completed.

## Changes

- Removed generic public `CommandEnvelope` support for `requestPreviewDecode`, `releasePreviewFrame`, `requestPreviewFrame`, `requestPreviewSegment`, and `invalidatePreviewCache`.
- Deleted the draft-bearing preview decode/frame/segment/cache payload contracts from `draft_model` and regenerated schema/TypeScript contracts.
- Removed generic `bindings_node::execute_command` preview routes while preserving explicit project-session preview APIs and realtime preview scheduler/session APIs.
- Kept preview/export artifact adapter internals as binding-local service helpers, no longer exposed as public generic command envelope variants.
- Updated Electron tests and helpers to observe `requestProjectSessionPreviewFrame` / `requestProjectSessionPreviewSegment` instead of the removed generic preview commands.
- Fixed test observation recording for explicit audio preview calls after removing generic preview timerange fields, and disabled the audio retry control while an audio command is in flight.
- Hardened source guards so the removed generic preview commands and cache invalidation payloads cannot return to public generated contracts.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `cargo test -p draft_model --test schema_exports -- --nocapture`
- `cargo test -p bindings_node --test preview_commands -- --nocapture`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `cargo test -p realtime_preview_runtime -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase5-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`

## Notes

- `corepack pnpm run test:phase13-source-guards` currently reaches its final generated-contract `git diff --exit-code` check. It should pass after this intended generated/schema update is committed.
