---
phase: 01
slug: foundation-and-golden-harness
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-17
updated: 2026-06-17
---

# Phase 01 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`, Playwright Electron via `@playwright/test`, and pnpm script tests |
| **Config file** | Plan 01-01 creates root workspace config; Plan 01-08 creates Electron Playwright config |
| **Quick run command** | `just test` after Plan 01-09 |
| **Full suite command** | `just test` in Phase 1; later phases may split quick/full once suites grow |
| **Estimated runtime** | ~120 seconds after scaffold |

---

## Sampling Rate

- **After every task commit:** Run the task-specific automated verification command listed below.
- **After every plan:** Run the plan-level `<verification>` block from the completed `PLAN.md`.
- **After Plan 01-09:** Run full `just build` and `just test`.
- **Before `$gsd-verify-work`:** `just build` and `just test` must be green, plus `git diff --exit-code schemas apps/desktop-electron/src/generated`.
- **Max feedback latency:** 120 seconds after Plan 01-09 finalizes the full gate.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-W0-01 | 01-01 | 0 | FOUND-01 | T-01-SC | Lockfile and toolchain setup uses pinned package manager/toolchain | integration | `corepack enable; pnpm install --frozen-lockfile` and `cargo metadata --format-version 1 --locked` | No - execution pending | pending |
| 01-W0-02 | 01-01 | 0 | FOUND-01 | T-01-03 | Unified command surface is auditable through `just` | tooling | `command -v just >/dev/null || cargo install just --locked; just --list` | No - execution pending | pending |
| 01-W1-01 | 01-02 | 1 | FOUND-01 | T-01-03 | Pure semantic crates avoid platform/runtime dependencies | Rust check | `cargo check -p draft_commands -p engine_core -p render_graph -p ffmpeg_compiler --locked` | No - execution pending | pending |
| 01-W1-02 | 01-02 | 1 | FOUND-02, TEST-01 | T-01-01, T-01-02 | Rust contracts reject unknown fields and own Jianying-aligned envelope types | Rust unit | `cargo test -p draft_model contract -- --nocapture` | No - execution pending | pending |
| 01-W1-03 | 01-03 | 1 | FOUND-03, FOUND-04 | T-01-01, T-01-02 | Platform traits live only at service boundaries | Rust check | `cargo check -p media_runtime -p media_runtime_desktop -p project_store -p preview_service -p testkit --locked` | No - execution pending | pending |
| 01-W1-04 | 01-03 | 1 | FOUND-03 | T-01-03 | FFmpeg distribution and hardware encoder scope are documented as deferred | doc grep | `grep -n "does not download\\|does not.*bundle\\|HardwareEncoder" docs/runtime-boundaries.md` | No - execution pending | pending |
| 01-W2-01 | 01-04 | 2 | FOUND-02 | T-01-01 | Binding crate exposes only planned native functions | Rust check | `cargo check -p bindings_node --locked` | No - execution pending | pending |
| 01-W2-02 | 01-04 | 2 | FOUND-02 | T-01-02, T-01-03 | Unsupported commands return structured envelope errors | Rust unit | `cargo test -p bindings_node -- --nocapture` | No - execution pending | pending |
| 01-W2-03 | 01-06 | 2 | FOUND-02, TEST-01 | T-01-01 | Schema and TS contracts regenerate from Rust without drift | Rust integration | `cargo test -p draft_model schema -- --nocapture` and `git diff --exit-code schemas apps/desktop-electron/src/generated` | No - execution pending | pending |
| 01-W2-04 | 01-06 | 2 | FOUND-04, TEST-01 | T-01-02, T-01-03 | Fixtures are classified and unknown fields fail validation | Rust integration | `cargo test -p draft_model schema -- --nocapture` | No - execution pending | pending |
| 01-W3-01 | 01-05 | 3 | FOUND-03 | T-01-01, T-01-02, T-01-03 | FFmpeg paths are probed with argument arrays and bounded errors | Rust integration | `cargo test -p media_runtime discovery -- --nocapture` | No - execution pending | pending |
| 01-W3-02 | 01-05 | 3 | FOUND-02, FOUND-03 | T-01-04 | Runtime discovery failures map into stable command envelope errors | Rust unit | `cargo test -p bindings_node execute_command -- --nocapture` | No - execution pending | pending |
| 01-W4-01 | 01-07 | 4 | FOUND-04 | T-01-01, T-01-03 | Render media is generated in temp dirs and not committed | Rust integration | `cargo test -p testkit generate_tiny -- --nocapture` plus binary-fixture find gate | No - execution pending | pending |
| 01-W4-02 | 01-07 | 4 | FOUND-04 | T-01-01, T-01-02 | Render smoke uses ffprobe metadata only and fails when tools are missing | Rust integration | `cargo test -p testkit render_smoke -- --nocapture` | No - execution pending | pending |
| 01-W4-03 | 01-08 | 4 | FOUND-01, FOUND-02 | T-01-SC | Electron package builds with approved dependencies | Node build | `pnpm --filter @video-editor/desktop build` | No - execution pending | pending |
| 01-W4-04 | 01-08 | 4 | FOUND-02 | T-01-01, T-01-04 | Renderer uses narrow preload API and does not construct FFmpeg commands | Electron smoke | `pnpm --filter @video-editor/desktop test` plus raw-IPC/FFmpeg grep gates | No - execution pending | pending |
| 01-W5-01 | 01-09 | 5 | FOUND-01, FOUND-02, FOUND-03, FOUND-04, TEST-01 | T-01-01, T-01-03 | Full local gates cover generated contracts, runtime discovery, and render smoke | full gate | `just build` and `just test` | No - execution pending | pending |
| 01-W5-02 | 01-09 | 5 | FOUND-01, FOUND-02, FOUND-03, FOUND-04, TEST-01 | T-01-02, T-01-04 | CI runs local gates without adding app distribution or FFmpeg bundling | CI config grep | `grep -n "just build\\|just test\\|24.12.0\\|ffmpeg" .github/workflows/ci.yml` | No - execution pending | pending |

---

## Threat References

| Ref | Threat | Required Mitigation |
|-----|--------|---------------------|
| T-01-01 | Renderer or runtime invokes more privilege than intended | Expose only typed preload methods, keep service traits at consuming boundaries, and use process argument arrays |
| T-01-02 | Command envelope accepts unknown fields or generated contracts drift | Use serde `deny_unknown_fields`, JSON Schema validation, generated TypeScript, and drift gates |
| T-01-03 | User-controlled FFmpeg path executes unintended binary | Probe explicit env var paths with `-version`, record checked paths, and surface structured errors |
| T-01-04 | Runtime/native errors leak unbounded or unsafe output | Bound stderr summaries and return standardized `ok/error/events` envelopes |
| T-01-SC | Package install tampering | Use only approved packages from 01-RESEARCH.md and lockfile-based installs |

---

## Wave 0 Requirements

- [ ] 01-01 creates `Cargo.toml`, `rust-toolchain.toml`, `package.json`, `pnpm-workspace.yaml`, `.nvmrc`, `.gitignore`, and initial `justfile`
- [ ] 01-01 verifies Corepack/pnpm lockfile install and Cargo metadata
- [ ] 01-01 exposes `just dev`, `just build`, and `just test` as the root command surface

---

## Manual-Only Verifications

All Phase 1 behaviors have automated verification. Manual review may inspect generated files and CLI output, but no requirement is manual-only.

---

## Validation Sign-Off

- [x] All tasks have automated verify commands or explicit dependency on an earlier plan that creates the command
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers missing root test infrastructure references
- [x] No watch-mode flags in verification commands
- [x] Feedback latency target remains under 120 seconds after Plan 01-09
- [x] `nyquist_compliant: true` set in frontmatter after assigning concrete task IDs and commands

**Approval:** pending execution
