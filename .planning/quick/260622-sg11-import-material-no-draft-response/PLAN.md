# Import Material Response Draft Boundary

## Goal

Reduce renderer-owned project-state surface by removing the full `Draft` payload from `ProjectSessionImportMaterialResponse`. The import material product path should update the material panel from Rust-returned material/view data while Rust session remains the canonical draft owner and persists `.veproj/project.json`.

## Scope

- Remove `draft` from the Rust import-material session response.
- Remove `draft` from the Electron native binding type for import-material responses.
- Update renderer import handling to use the returned material record instead of `result.data.draft`.
- Add a source guard preventing import-material session responses from re-exposing full draft payloads.
- Verify product import/add/play paths still pass.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
