# Summary: 260622-sg40 Remove Generic Material Draft Commands

## Status

Completed.

## Changes

- Removed generic `CommandEnvelope` support for `importMaterial`, `listMaterials`, and `listMissingMaterials`.
- Deleted the draft-bearing material payload/response contracts from `draft_model` and regenerated schema/TypeScript contracts.
- Removed generic `bindings_node::execute_command` material routes and converted binding smoke coverage to assert these commands are rejected.
- Preserved project-session `importMaterial` intent plus `listProjectSessionMaterials` and `listProjectSessionMissingMaterials`.
- Added Phase 3 source guards preventing generic material draft commands from returning.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture`
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture`
- `cargo test -p draft_model schema_fixtures_validate_command_contracts -- --nocapture`
- `cargo test -p draft_model contract_rejects_mismatched_command_and_payload_kind -- --nocapture`
- `cargo test -p bindings_node --test binding_smoke -- --nocapture`
- `cargo test -p bindings_node project_session_material_reads_use_canonical_session_draft -- --nocapture`
- `cargo test -p bindings_node project_session_missing_material_reads_use_canonical_session_bundle_path -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/real-workflow.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --workers=1`
