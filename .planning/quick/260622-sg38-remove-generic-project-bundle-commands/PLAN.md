# Quick Task: 260622-sg38 Remove Generic Project Bundle Commands

## Objective

Remove legacy generic `openProjectBundle` / `saveProjectBundle` command-envelope support that still exposes full `Draft` payloads outside the Rust project session.

## Production Boundary

- Product project open/save must be project-session owned.
- Generic `CommandEnvelope` must not accept project bundle open/save commands that return or accept full `Draft`.
- `.veproj/project.json` remains canonical, but renderer/native generic command compatibility must not become a second project database path.

## Work Items

1. Remove `OpenProjectBundle` / `SaveProjectBundle` command names, payloads, responses, binding routes, and smoke coverage.
2. Regenerate/update TypeScript generated command contracts and JSON schema.
3. Add source guards blocking generic project bundle command reintroduction.
4. Verify Rust and Electron contract tests.

## Verification

- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust`
- `cargo test -p bindings_node binding_smoke`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
