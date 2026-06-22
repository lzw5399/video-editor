# 260622-sg40 Remove Generic Material Draft Commands

## Decision

Confirmed under production-architecture-review: generic `CommandEnvelope` material commands that carry a full `Draft` conflict with Rust session ownership. Product material import/read/missing diagnostics must go through project-session APIs and Rust-owned view models/deltas.

## Scope

- Remove `importMaterial`, `listMaterials`, and `listMissingMaterials` from the generic `CommandEnvelope` contract.
- Remove generic `bindings_node::execute_command` routes for these commands.
- Preserve project-session `importMaterial` intent and explicit session material read APIs.
- Regenerate schema and TypeScript contracts from Rust.
- Add source guards preventing the generic draft-bearing material commands from returning.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust`
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust`
- `cargo test -p bindings_node --test binding_smoke`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- Product/session flow smoke as needed after compile fixes.
