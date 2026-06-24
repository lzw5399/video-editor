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

- **After every task commit:** Run `cargo check --workspace --locked` plus the affected crate test or guard.
- **After every plan wave:** Run `pnpm run test:phase18`.
- **Before `$gsd-verify-work`:** Run `pnpm run test:phase18 && pnpm run test:no-product-fallback && pnpm run test:contracts`.
- **Max feedback latency:** 240 seconds for the aggregate gate, 90 seconds for targeted crate/source-guard checks.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 18-01-01 | 01 | 1 | BIND-01, BIND-02 | T18-01 | Runtime/session/handle authority lives below adapters | unit + source guard | `cargo test -p editor_runtime handle_registry -- --nocapture && bash scripts/phase18-source-guards.sh` | W0 | pending |
| 18-01-02 | 01 | 1 | BIND-02, BIND-03 | T18-02 | Stale/wrong-owner/wrong-device/double-release handles fail closed | unit | `cargo test -p editor_runtime handle_registry low_copy_handles -- --nocapture` | W0 | pending |
| 18-02-01 | 02 | 2 | BIND-01, BIND-05 | T18-03 | Node adapter delegates to shared runtime without duplicated semantics | smoke + contract | `cargo test -p bindings_node binding_smoke -- --nocapture && pnpm run test:contracts` | W0 | pending |
| 18-03-01 | 03 | 2 | PLAT-01, PLAT-03, BIND-05 | T18-04 | C ABI has bounded buffers, stable error codes, explicit release | ABI smoke + drift | `cargo test -p bindings_c abi_smoke mobile_contract_handles -- --nocapture && bash scripts/phase18-abi-drift.sh` | W0 | pending |
| 18-04-01 | 04 | 2 | PLAT-02, BIND-04 | T18-05 | Server opens `.veproj`, resolves materials, exports, reports progress without Electron | integration | `cargo test -p server_runtime server_export_smoke server_export_progress_cancel -- --nocapture` | W0 | pending |
| 18-05-01 | 05 | 3 | PLAT-03, BIND-03, BIND-05 | T18-06 | Mobile lifecycle/permission/texture/session contracts are executable enough for future adapters | docs + guard | `bash scripts/phase18-mobile-contract-guards.sh && cargo test -p bindings_c mobile_contract_handles -- --nocapture` | W0 | pending |
| 18-06-01 | 06 | 3 | PLAT-01, PLAT-02, PLAT-03, BIND-01, BIND-02, BIND-03, BIND-04, BIND-05 | T18-07 | No adapter fallback/mock/artifact path can satisfy product success | aggregate | `pnpm run test:phase18 && pnpm run test:no-product-fallback && pnpm run test:contracts` | W0 | pending |

---

## Wave 0 Requirements

- [ ] `crates/editor_runtime/` exists with shared runtime/session/export/handle test scaffolding.
- [ ] `crates/bindings_c/` exists with `cdylib`/`staticlib` crate metadata, C ABI tests, and generated-header contract.
- [ ] `crates/server_runtime/` exists with Electron-free `.veproj` export smoke fixture tests.
- [ ] `docs/mobile-runtime-contracts.md` exists and covers JNI/Swift lifecycle, sandboxed file, texture, cancellation, and session-close contracts.
- [ ] `scripts/phase18-source-guards.sh` fails semantic duplication, adapter-owned lifetime, renderer/main render behavior, and fallback-success paths.
- [ ] `scripts/phase18-abi-drift.sh` regenerates the C header through pinned `cbindgen` and fails on dirty diffs.
- [ ] `scripts/phase18-mobile-contract-guards.sh` checks the mobile contract doc and ABI handle smoke expectations.
- [ ] `package.json` contains `test:phase18-rust`, `test:phase18-source-guards`, `test:phase18-abi`, `test:phase18-server`, `test:phase18-mobile-contracts`, and `test:phase18`.
- [ ] Pinned `cbindgen` invocation is documented or vendored through a reproducible project script; do not upgrade `@napi-rs/cli` without a checkpoint.

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
