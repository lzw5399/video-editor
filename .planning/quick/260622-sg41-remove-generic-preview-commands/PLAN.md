# 260622-sg41 Remove Generic Preview Commands

## Decision

Confirmed under production-architecture-review: generic `CommandEnvelope` preview commands are the wrong public boundary. Product preview must use project-session preview APIs and realtime scheduler/session controls; Electron must not send full drafts or cache-invalidation semantics through a generic command envelope.

## Scope

- Remove `requestPreviewDecode`, `releasePreviewFrame`, `requestPreviewFrame`, `requestPreviewSegment`, and `invalidatePreviewCache` from the generic `CommandEnvelope` contract.
- Remove generic `bindings_node::execute_command` routes for these commands.
- Preserve explicit project-session preview APIs and realtime preview scheduler/session APIs.
- Keep internal preview/export cache machinery only where it is no longer public generic command surface.
- Regenerate schema and TypeScript contracts from Rust.
- Rewrite tests and source guards so old generic preview commands are rejected or absent from public contracts.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust`
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust`
- `cargo test -p bindings_node --test preview_commands`
- `cargo test -p bindings_node --test project_session`
- `cargo test -p realtime_preview_runtime`
- `corepack pnpm run test:contracts`
- Relevant product preview Playwright gates as needed after compile fixes.
