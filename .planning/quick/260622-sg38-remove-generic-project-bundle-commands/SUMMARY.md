# Summary: 260622-sg38 Remove Generic Project Bundle Commands

## Result

Completed. Generic `CommandEnvelope` no longer exposes `openProjectBundle` or `saveProjectBundle`, removing the remaining project bundle path that accepted/returned full `Draft` outside Rust project sessions.

- Removed `OpenProjectBundle` / `SaveProjectBundle` command names, payloads, responses, schema entries, and TS generated types.
- Removed `bindings_node::execute_command` routes and smoke coverage for generic bundle open/save.
- Added a Phase 3 source guard blocking generic project bundle open/save command reintroduction.
- Updated the real no-mock workflow to require project-session creation and to construct a valid 6s timeline through Rust-owned intents: two sequential main-video segments plus separate text/audio/overlay tracks.

## Verification

- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust`
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust`
- `cargo test -p bindings_node --test binding_smoke`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/real-workflow.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`

`product-preview-cadence.spec.ts` reported 90/90 accounted frames in both single-video and video+external-audio+text+two-cue-SRT scenarios.
