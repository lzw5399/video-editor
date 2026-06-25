---
phase: 18
slug: mobile-server-binding-architecture-and-runtime-ports
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-25
---

# Phase 18 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` 1.95.0, Playwright 1.61.0, shell source guards |
| **Config file** | `Cargo.toml`, `package.json`, Phase 18 scripts created in Wave 0 |
| **Quick run command** | `cargo check --workspace --locked` plus the affected crate test |
| **Full suite command** | `pnpm run test:phase18` |
| **Estimated runtime** | ~240 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo check --workspace --locked` plus the affected crate test or staged guard.
- **After Wave 1:** Run guard/script self-tests only; full aggregate is not valid until later artifacts exist.
- **After Wave 2:** Run the affected crate integration targets plus staged `phase18-source-guards.sh --plan 03`, `--plan 04`, and `--plan 05`.
- **Before `$gsd-verify-work`:** Run `pnpm run test:phase18 && pnpm run test:no-product-fallback && pnpm run test:contracts`.
- **Max feedback latency:** 240 seconds for the aggregate gate, 90 seconds for targeted crate/source-guard checks.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Artifact Owner | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|----------------|--------|
| 18-01-01 | 01 | 1 | BIND-01, BIND-02, BIND-03, BIND-04 | T-18-01, T-18-03 | Runtime/project/export contracts live in `editor_runtime`, not adapters | integration + check | `cargo test -p editor_runtime --test project_session_runtime -- --nocapture && cargo check -p editor_runtime --locked` | Plan 01 | pending |
| 18-01-02 | 01 | 1 | BIND-02, BIND-03 | T-18-01, T-18-02, T-18-04 | Stale/wrong-owner/wrong-device/double-release handles fail closed | integration + check | `cargo test -p editor_runtime --test handle_registry -- --nocapture && cargo check --workspace --locked` | Plan 01 | pending |
| 18-02-01 | 02 | 1 | BIND-01, BIND-05 | T-18-05, T-18-08 | Source guards self-test duplicated semantics, Electron render behavior, and fallback success | guard self-test | `bash scripts/phase18-source-guards.sh --self-test` | Plan 02 | pending |
| 18-02-02 | 02 | 1 | PLAT-01, PLAT-03, BIND-05 | T-18-07, T-18-SC | ABI and mobile contract guards self-test pinned cbindgen and mobile contract expectations | guard self-test | `bash scripts/phase18-abi-drift.sh --self-test && bash scripts/phase18-mobile-contract-guards.sh --self-test` | Plan 02 | pending |
| 18-02-03 | 02 | 1 | PLAT-01, PLAT-02, PLAT-03, BIND-01, BIND-02, BIND-03, BIND-04, BIND-05 | T-18-06 | Root scripts expose per-wave and aggregate gates without watch mode | package script self-test | `pnpm run test:phase18-source-guards -- --self-test && pnpm run test:phase18-abi -- --self-test && pnpm run test:phase18-mobile-contracts -- --self-test` | Plan 02 | pending |
| 18-03-01 | 03 | 2 | BIND-01, BIND-02, BIND-03 | T-18-10 | Node project sessions delegate to shared runtime while preserving explicit transport envelopes | integration | `cargo test -p bindings_node --test project_session -- --nocapture && cargo test -p bindings_node --test binding_smoke -- --nocapture` | Plan 03 | pending |
| 18-03-02 | 03 | 2 | BIND-01, BIND-05 | T-18-11, T-18-12 | Desktop export adapter delegates lifecycle to `editor_runtime` and rejects binding-owned export policy | integration + staged guard | `cargo test -p bindings_node --test export_commands -- --nocapture && cargo test -p bindings_node --test scheduler_export -- --nocapture && bash scripts/phase18-source-guards.sh --plan 03` | Plan 03 | pending |
| 18-03-03 | 03 | 2 | BIND-01, BIND-05 | T-18-09, T-18-SC | Electron main/preload remain explicit IPC adapters and do not own render/export/fallback semantics | desktop build + contracts + staged guard | `pnpm --filter @video-editor/desktop build && pnpm run test:contracts && bash scripts/phase18-source-guards.sh --plan 03` | Plan 03 | pending |
| 18-04-01 | 04 | 2 | PLAT-01, BIND-02, BIND-03 | T-18-13, T-18-14, T-18-15 | C ABI validates pointers, buffers, handles, and errors without depending on Node | ABI smoke + staged guard | `cargo test -p bindings_c --test abi_smoke -- --nocapture && bash scripts/phase18-source-guards.sh --plan 04` | Plan 04 | pending |
| 18-04-02 | 04 | 2 | PLAT-01, BIND-05 | T-18-16, T-18-SC | Generated header is reproducible through project-local pinned cbindgen 0.29.4 | ABI drift | `bash scripts/phase18-abi-drift.sh` | Plan 04 | pending |
| 18-04-03 | 04 | 2 | PLAT-03, BIND-02, BIND-03 | T-18-14 | Mobile-held opaque handles require explicit release and Rust owner/generation/device validation | integration + mobile guard | `cargo test -p bindings_c --test abi_smoke -- --nocapture && cargo test -p bindings_c --test mobile_contract_handles -- --nocapture && bash scripts/phase18-mobile-contract-guards.sh --smoke-only` | Plan 04 | pending |
| 18-05-01 | 05 | 2 | PLAT-02, BIND-04 | T-18-17, T-18-18, T-18-20 | Server library/bin opens `.veproj` and exports through shared Rust runtime without Electron | check + staged guard | `cargo check -p server_runtime --locked && bash scripts/phase18-source-guards.sh --plan 05` | Plan 05 | pending |
| 18-05-02 | 05 | 2 | PLAT-02, BIND-04, BIND-05 | T-18-19 | Server export smoke proves output, metadata, progress, and cancellation without Electron | integration + package gate | `cargo test -p server_runtime --test server_export_smoke -- --nocapture && pnpm run test:phase18-server` | Plan 05 | pending |
| 18-06-01 | 06 | 3 | PLAT-03, BIND-03, BIND-05 | T-18-21, T-18-23, T-18-24 | Mobile lifecycle and ownership contracts are documented and guarded | docs + guard + integration | `bash scripts/phase18-mobile-contract-guards.sh && bash scripts/phase18-source-guards.sh --mobile-contracts && cargo test -p bindings_c --test mobile_contract_handles -- --nocapture` | Plan 06 | pending |
| 18-06-02 | 06 | 3 | PLAT-01, PLAT-02, PLAT-03, BIND-01, BIND-02, BIND-03, BIND-04, BIND-05 | T-18-22 | Runtime boundaries and source audit cover every GOAL/REQ/RESEARCH/CONTEXT item | full guard + mobile guard | `bash scripts/phase18-source-guards.sh && bash scripts/phase18-mobile-contract-guards.sh` | Plan 06 | pending |
| 18-06-03 | 06 | 3 | PLAT-01, PLAT-02, PLAT-03, BIND-01, BIND-02, BIND-03, BIND-04, BIND-05 | T-18-22 | Aggregate gates reject fallback/mock/artifact paths and contract drift | aggregate | `pnpm run test:phase18 && pnpm run test:no-product-fallback && pnpm run test:contracts` | Plan 06 | pending |

---

## Planned Artifact Requirements

- [ ] `crates/editor_runtime/` exists with shared runtime/session/export/handle test scaffolding.
- [ ] `crates/bindings_c/` exists with `cdylib`/`staticlib` crate metadata, C ABI tests, and generated-header contract.
- [ ] `crates/server_runtime/` exists with Electron-free `.veproj` export smoke fixture tests.
- [ ] `docs/mobile-runtime-contracts.md` exists and covers JNI/Swift lifecycle, sandboxed file, texture, cancellation, and session-close contracts.
- [ ] `scripts/phase18-source-guards.sh` fails semantic duplication, adapter-owned lifetime, renderer/main render behavior, and fallback-success paths.
- [ ] `scripts/phase18-abi-drift.sh` regenerates the C header through pinned `cbindgen` and fails on dirty diffs.
- [ ] `scripts/phase18-mobile-contract-guards.sh` checks the mobile contract doc and ABI handle smoke expectations.
- [ ] `package.json` contains `test:phase18-rust`, `test:phase18-source-guards`, `test:phase18-abi`, `test:phase18-server`, `test:phase18-mobile-contracts`, and `test:phase18`.
- [ ] `scripts/phase18-abi-drift.sh` bootstraps a project-local pinned `cbindgen` 0.29.4 binary and fails if the resolved version differs; do not upgrade `@napi-rs/cli` without a checkpoint.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `@napi-rs/cli` upgrade/reinstall decision | BIND-05 | Research flagged the currently installed version as suspicious only because it is very new; no change is required unless implementation needs it. | If a plan proposes changing `@napi-rs/cli`, pause and verify the package metadata before editing lockfiles. |
| FFmpeg distribution/license posture | PLAT-02, BIND-04 | Phase 18 can reuse the existing desktop FFmpeg runtime, but shipping/server distribution obligations remain a product/legal review. | Confirm no new FFmpeg binary distribution or nonfree/GPL mode is introduced by this phase. |

---

## Validation Sign-Off

- [x] All phase requirements have automated verification or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 lists every missing verification file/script/crate.
- [x] No watch-mode flags are required.
- [x] Feedback latency target is below 240 seconds for aggregate gates.
- [x] `nyquist_compliant: true` is set in frontmatter.

**Approval:** approved 2026-06-25
