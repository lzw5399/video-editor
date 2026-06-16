---
phase: 01
slug: foundation-and-golden-harness
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-17
---

# Phase 01 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`, Playwright Electron via `@playwright/test`, and pnpm script tests |
| **Config file** | None yet - Wave 0 must create `Cargo.toml`, `pnpm-workspace.yaml`, package scripts, and optional `playwright.config.ts` |
| **Quick run command** | `just test` |
| **Full suite command** | `just test` in Phase 1; later phases may split quick/full once suites grow |
| **Estimated runtime** | ~120 seconds after scaffold |

---

## Sampling Rate

- **After every task commit:** Run `just test`
- **After every plan wave:** Run `just test` plus `git diff --exit-code schemas apps/desktop-electron/src/generated` after schema/type generation exists
- **Before `$gsd-verify-work`:** `just build` and `just test` must be green
- **Max feedback latency:** 120 seconds after Wave 0 test infrastructure exists

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-W0-01 | TBD | 0 | FOUND-01 | - | N/A | integration | `just build` | No - Wave 0 | pending |
| 01-W0-02 | TBD | 0 | FOUND-02 | T-01 / T-02 | Renderer uses narrow preload IPC and Rust rejects malformed command envelopes | Electron smoke + Rust unit | `pnpm --filter @video-editor/desktop test` and `cargo test -p bindings_node` | No - Wave 0 | pending |
| 01-W0-03 | TBD | 0 | FOUND-03 | T-03 / T-04 / T-05 | FFmpeg paths are probed without shell concatenation and failures bound stderr | Rust unit/integration | `cargo test -p media_runtime discovery` | No - Wave 0 | pending |
| 01-W0-04 | TBD | 0 | FOUND-04 | T-05 | Render smoke uses generated temp media and no UI-authored FFmpeg commands | Rust integration | `cargo test -p testkit render_smoke` | No - Wave 0 | pending |
| 01-W0-05 | TBD | 0 | TEST-01 | T-02 | Schema validation rejects unknown command fields and validates fixtures | Rust unit/integration | `cargo test -p draft_model schema` | No - Wave 0 | pending |

---

## Threat References

| Ref | Threat | Required Mitigation |
|-----|--------|---------------------|
| T-01 | Renderer invokes arbitrary IPC/native methods | Expose only typed preload methods; do not expose raw `ipcRenderer` to renderer code |
| T-02 | Command envelope accepts unknown fields | Use serde `deny_unknown_fields`, JSON Schema validation, and negative fixtures |
| T-03 | User-controlled FFmpeg path executes unintended binary | Probe explicit env var paths with `-version` and surface checked path in structured errors |
| T-04 | Unbounded process stderr floods logs/UI | Bound stderr summaries in probe/render errors |
| T-05 | Shell injection through FFmpeg command strings | Use process argument arrays; UI must not construct FFmpeg commands |

---

## Wave 0 Requirements

- [ ] `Cargo.toml` workspace and compile-safe crate shells for all planned crates
- [ ] `package.json`, `pnpm-workspace.yaml`, `apps/desktop-electron/package.json`, and Corepack `packageManager`
- [ ] `justfile` with `dev`, `build`, and `test`
- [ ] `crates/draft_model` contract types and schema/type generation test
- [ ] `crates/media_runtime` discovery tests for env var, PATH, missing binary, and bad binary
- [ ] `crates/testkit` render smoke using lavfi plus ffprobe metadata
- [ ] Minimal Playwright Electron binding smoke

---

## Manual-Only Verifications

All Phase 1 behaviors have automated verification. Manual review may inspect generated files and CLI output, but no requirement is manual-only.

---

## Validation Sign-Off

- [ ] All tasks have automated verify commands or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all missing test infrastructure references
- [ ] No watch-mode flags in verification commands
- [ ] Feedback latency under 120 seconds after Wave 0
- [ ] Set `nyquist_compliant: true` in frontmatter after plans assign concrete task IDs and commands

**Approval:** pending
