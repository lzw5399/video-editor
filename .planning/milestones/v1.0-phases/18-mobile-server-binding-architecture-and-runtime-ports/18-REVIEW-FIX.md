---
phase: 18-mobile-server-binding-architecture-and-runtime-ports
fixed: 2026-06-25T04:35:00Z
status: fixed-reviewed
findings_fixed:
  - "2026-06-25T04:00:18Z CR-01 material import reported failure after committing project"
  - "2026-06-25T04:00:18Z WR-01 diagnostic buffer docs mismatched side-effecting ABI calls"
  - "2026-06-25T04:20:13Z CR-01 C ABI fabricated lifecycle handles through generic acquire"
  - "2026-06-25T04:20:13Z WR-01 material probe worker spawn failure stranded scheduler jobs"
---

# Phase 18 Review Fix Notes

## Fixes

- Material import persistence and async probe scheduling now have coherent command semantics. If `.veproj/project.json` is saved successfully but probe scheduling fails, the command returns `ok: true` with `probeStatus: "failed"`, no `probeJobId`, and a probe-failed diagnostic. It no longer reports a failed command after committing the project.
- `ProjectSessionImportMaterialResponse.probeJobId` is optional in Rust serialization and the Electron TypeScript adapter type.
- A `test-hooks` feature on `editor_runtime` provides deterministic test-only failure injection for material-probe enqueue and worker-spawn failures. Product builds do not enable this feature.
- Material-probe worker spawn failure now completes the already-started scheduler job as failed, releases scheduler capacity, and starts any newly ready pending jobs. Worker-chain spawn failures also write probe-failure metadata when safe.
- The C ABI generic `ve_handle_acquire` now rejects `VE_HANDLE_KIND_RUNTIME_SESSION` and `VE_HANDLE_KIND_PROJECT_SESSION`. Runtime sessions and project-session handles can only come from lifecycle APIs.
- Mobile runtime contracts now distinguish diagnostic-only buffer calls from side-effecting ABI calls, and explicitly state that lifecycle handles are not created by generic acquire.

## Regression Coverage

- `project_session_import_material_reports_probe_schedule_failure_after_commit_as_success`
- `project_session_material_probe_worker_spawn_failure_releases_scheduler_capacity`
- `abi_smoke_rejects_invalid_inputs_without_panicking` now asserts runtime/project-session generic acquire returns `VE_STATUS_INVALID_ARGUMENT` and does not fabricate handles.

## Verification

- `cargo test -p bindings_node --test project_session project_session_import_material_reports_probe_schedule_failure_after_commit_as_success -- --nocapture` - passed.
- `cargo test -p bindings_node --test project_session project_session_material_probe_worker_spawn_failure_releases_scheduler_capacity -- --nocapture` - passed.
- `cargo test -p bindings_node --test project_session -- --nocapture` - passed, 43 tests.
- `cargo test -p bindings_c --test abi_smoke -- --nocapture` - passed, 6 tests.
- `cargo test -p bindings_c --test mobile_contract_handles -- --nocapture` - passed, 2 tests.
- `bash scripts/phase18-abi-drift.sh` - passed.
- `bash scripts/phase18-source-guards.sh` - passed.
- `bash scripts/phase18-mobile-contract-guards.sh` - passed.
- `pnpm run test:phase18` - passed.
- `pnpm --filter @video-editor/desktop build` - passed after the TypeScript response contract update.

## Non-Blocking Warnings

- Existing local Node engine warning remains: package expects Node `24.12.0`, current runtime is `24.15.0`.
- Existing `media_runtime_desktop` macOS AVFoundation deprecation warning remains.
- Existing unused-helper warnings in `bindings_node` remain.
