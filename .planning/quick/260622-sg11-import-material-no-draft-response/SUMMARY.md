# Import Material Response Draft Boundary Summary

## Outcome

Completed. `ProjectSessionImportMaterialResponse` no longer returns a full `Draft` payload to Electron. The product import path updates the material panel from the single Rust-returned `material` record, leaving the Rust project session as the canonical draft and persistence owner.

## Changes

- Removed `draft` from Rust `ProjectSessionImportMaterialResponse`.
- Removed `draft` from Electron `ProjectSessionImportMaterialResponse` type.
- Updated renderer import handling to merge the returned material into `workspace.materials`.
- Updated project-session tests to assert import-material session responses do not expose `draft`.
- Added phase3 source guards rejecting reintroduction of full-draft import-material responses.

## Verification

- `cargo fmt --all --check`
- `git diff --check`
- `corepack pnpm run test:phase3-source-guards`
- `cargo test -p bindings_node --test project_session project_session_imports_material_then_adds_segment_without_renderer_draft -- --nocapture`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`

## Remaining Gap

This slice does not complete the full Rust-owned view model migration. Open/create/timeline responses still return full draft data, and renderer selection still passes visible `segmentId`/`trackId` handles for selection-mediated flows. The next destructive slice should replace renderer ID selection with Rust-owned selection/hit-test or opaque item handles.
