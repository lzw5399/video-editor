# Session-Owned Export Boundary

## Goal

Remove the product export path's renderer-supplied full `Draft` payload. Starting an export from the desktop product UI should use the Rust project session as canonical source: Electron sends only `sessionId`, `expectedRevision`, output path, and preset; Rust reads the session draft and starts the existing export registry from that canonical snapshot.

## Scope

- Add a Rust N-API binding for `startProjectSessionExport`.
- Reuse existing export registry/compiler/runtime after constructing the payload from a project-session snapshot inside Rust.
- Add Electron main/preload/native binding bridge for session-owned export.
- Change `App.tsx` start-export flow to call the new bridge and reject missing sessions instead of falling back to renderer draft export.
- Keep `getExportJobStatus` and `cancelExport` on the existing job-id command path for this slice.
- Add guards/tests so product renderer cannot call `buildStartExportCommand` or send a full draft for start export.

## Verification

- `cargo fmt --all --check`
- `cargo test -p bindings_node --test project_session project_session_export -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/export-modal.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --grep "导出" --reporter=line`
