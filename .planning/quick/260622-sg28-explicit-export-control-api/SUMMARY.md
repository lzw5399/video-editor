# Summary: 260622-sg28 Explicit Export Control API

## Status

Completed.

## Changes

- Added explicit Rust Node-API entry points for `getExportJobStatus` and `cancelExport`.
- Added typed Electron main/preload/nativeBinding wrappers for export job control.
- Updated product export status refresh/cancel flows to call explicit APIs instead of renderer-built command envelopes.
- Removed renderer helper builders for export status/cancel command envelopes.
- Added source guards and binding tests to prevent product export control from returning to generic command construction.

## Verification

- `cargo test -p bindings_node export_commands`
- `cargo test -p bindings_node explicit_export_control_apis_query_and_cancel_jobs_without_command_envelopes`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `cargo fmt --all --check && git diff --check`
