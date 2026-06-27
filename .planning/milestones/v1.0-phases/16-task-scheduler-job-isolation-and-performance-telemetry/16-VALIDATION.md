---
phase: 16
slug: task-scheduler-job-isolation-and-performance-telemetry
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-23
---

# Phase 16 - Validation Strategy

Per-phase validation contract for the Rust-owned task scheduler, job isolation,
backpressure, cancellation, and performance telemetry work.

## Test Infrastructure

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test`; Playwright Electron product E2E; shell source guards with `rg` |
| Config file | `Cargo.toml`, `package.json`, `scripts/phase16-source-guards.sh` after Wave 0 |
| Quick run command | `cargo test -p task_runtime -- --nocapture` after crate creation |
| Full suite command | `pnpm run test:phase16 && pnpm run test:no-product-fallback` after scripts are added |
| Estimated runtime | ~8-20 minutes for full product stress gate, depending on package/build state |

## Sampling Rate

- After every task commit: run the focused Rust or source-guard command for the touched scheduler boundary.
- After every plan wave: run `pnpm run test:phase16-rust && pnpm run test:phase16-source-guards`.
- Before phase closeout: run `pnpm run test:phase16`, `pnpm run test:no-product-fallback`, `cargo check --workspace --locked`, and `git diff --check -- . ':!reference'`.
- Max feedback latency: one plan wave for required automated gates.

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 16-W0-01 | 16-01 | 1 | SCHED-01/SCHED-03 | T-16-01 | Workspace and canonical freshness migration establish one Rust-owned scheduler boundary and one `PlaybackGeneration` source. | unit | `cargo test -p task_runtime scheduler_contracts -- --nocapture` | no W0 | pending |
| 16-W0-02 | 16-01B | 2 | SCHED-01/SCHED-03/SCHED-04 | T-16-02 | Scheduler core, config, cancellation, stale generation, queue depth, latency, saturation, and telemetry are deterministic. | unit | `cargo test -p task_runtime -- --nocapture` | no W0 | pending |
| 16-W0-03 | 16-02 | 3 | SCHED-01/SCHED-02 | T-16-03 | Preview and audio interactive jobs cannot be starved by background jobs. | unit/integration | `cargo test -p task_runtime starvation -- --nocapture && cargo test -p bindings_node scheduler_preview_audio -- --nocapture` | no W0 | pending |
| 16-W0-04 | 16-03 | 3 | SCHED-02/SCHED-04 | T-16-04 | Export jobs use scheduler admission/status/cancel instead of binding-owned unbounded threads. | unit/integration | `cargo test -p bindings_node scheduler_export -- --nocapture` | no W0 | pending |
| 16-W0-05 | 16-04 | 3 | SCHED-01/SCHED-02 | T-16-05 | Artifact generation, media probe, and IO work enqueue through scheduler and reject stale commits. | unit/integration | `cargo test -p bindings_node scheduler_artifact_probe -- --nocapture` | no W0 | pending |
| 16-W0-06 | 16-05 | 4 | SCHED-01/SCHED-04 | T-16-06 | Product-safe scheduler status/telemetry is exposed without renderer-owned policy mutation. | binding/e2e | `cargo test -p bindings_node scheduler_runtime -- --nocapture` | no W0 | pending |
| 16-W0-07 | 16-06 | 5 | SCHED-02/SCHED-04 | T-16-07 | Source guards, no-product-fallback checks, package scripts, and product E2E prove preview, scrub, inspector edits, and audio stay responsive under export/artifact/probe pressure. | guard/e2e | `pnpm run test:phase16-source-guards && pnpm run test:no-product-fallback && pnpm run test:phase16-desktop` | no W0 | pending |
| 16-W0-08 | 16-07 | 6 | all SCHED | T-16-08 | Aggregate gate proves scheduler closeout and workspace health. | aggregate | `pnpm run test:phase16` | no W0 | pending |

Status: pending, green, red, flaky.

## Wave 0 Requirements

- [ ] `crates/task_runtime/Cargo.toml` and `crates/task_runtime/src/lib.rs` - scheduler contracts, fake-clock/fake-executor harness, cancellation, backpressure, telemetry, and resource budget types.
- [ ] `crates/task_runtime/tests/scheduler_contracts.rs` - typed domain/priority/resource/freshness/config coverage.
- [ ] `crates/task_runtime/tests/starvation.rs` - interactive preview/audio admission under export/artifact/probe saturation.
- [ ] `crates/task_runtime/tests/scheduler_telemetry.rs` - queue latency, duration, cancellation, stale rejection, cache hit, dropped/repeated frames, queue depth, and saturation snapshots.
- [ ] `crates/bindings_node/tests/scheduler_runtime.rs` - narrow scheduler config/status/telemetry APIs and preview/audio/export/artifact/probe integration contracts.
- [ ] `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` - normal user workflow that starts preview, export, artifact/probe pressure, timeline scrub, and verifies visible preview motion with scheduler telemetry.
- [ ] `scripts/phase16-source-guards.sh` - direct binding-owned thread/FFmpeg/artifact/probe bypass guard plus product fallback success guard extensions.
- [ ] `package.json` scripts - `test:phase16-rust`, `test:phase16-source-guards`, `test:phase16-desktop`, and `test:phase16`.

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Native hardware/device variability under sustained export pressure | SCHED-02/SCHED-04 | CI may not cover every GPU/audio/storage combination. | On macOS and Windows development machines, run `pnpm run test:phase16-desktop` while collecting scheduler telemetry snapshots and confirm no hidden fallback success. |

## Validation Sign-Off

- [x] All planned behaviors have automated verification or Wave 0 dependencies.
- [x] Sampling continuity: no three consecutive tasks lack automated verification.
- [x] Wave 0 covers all missing test and guard infrastructure.
- [x] No watch-mode flags.
- [x] Feedback latency is one plan wave or less for required gates.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** approved 2026-06-23
