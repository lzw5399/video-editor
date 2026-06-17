---
phase: 06
slug: mvp-hardening-and-packaging
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-18
---

# Phase 06 - Validation Strategy

> Per-phase validation contract for MVP hardening, packaged desktop smoke, real no-mock workflow, and release readiness.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`; Playwright Electron via `@playwright/test` 1.61.0; shell source guards |
| **Config file** | `apps/desktop-electron/playwright.config.ts` |
| **Quick run command** | `pnpm run test:phase6-packaging && pnpm run test:phase6-runtime` |
| **Full suite command** | `pnpm run test && /Users/zhiwen/.cargo/bin/just test` |
| **Estimated runtime** | ~3-10 minutes depending on packaging and FFmpeg availability |

---

## Sampling Rate

- **After every task commit:** Run the focused command named in the task acceptance criteria.
- **After every plan wave:** Run the phase-level gate introduced or touched by that wave.
- **Before `$gsd-verify-work`:** Run `pnpm run test` and `/Users/zhiwen/.cargo/bin/just test`.
- **Max feedback latency:** 10 minutes for packaged smoke, 3 minutes for non-packaging Rust/renderer/source-guard tasks.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | TEST-07 | T-06-01 | Packaged app loads only trusted file renderer and preload bridge | package smoke | `pnpm --filter @video-editor/desktop test:packaged-smoke` | ❌ W0 | ⬜ pending |
| 06-01-02 | 01 | 1 | TEST-07 | T-06-02 | Native `.node` binding loads from packaged/unpacked resource path, not renderer | package smoke | `pnpm --filter @video-editor/desktop test:packaged-smoke` | ❌ W0 | ⬜ pending |
| 06-02-01 | 02 | 2 | TEST-06 | T-06-03 | Runtime capability report is Rust-owned | rust | `cargo test -p media_runtime runtime_capability -- --nocapture` | ❌ W0 | ⬜ pending |
| 06-02-02 | 02 | 2 | TEST-06 | T-06-04 | Runtime capability command is generated from Rust and routed through binding | rust + contract | `cargo test -p bindings_node runtime_capabilities -- --nocapture` | ❌ W0 | ⬜ pending |
| 06-03-01 | 03 | 3 | TEST-06 | T-06-05 | Runtime diagnostics UI uses generated command bridge only | e2e + source guard | `pnpm --filter @video-editor/desktop test:runtime-diagnostics` | ❌ W0 | ⬜ pending |
| 06-03-02 | 03 | 3 | TEST-06 | T-06-06 | Diagnostics panel fits the compact preview shell and uses locked Chinese copy | e2e | `pnpm --filter @video-editor/desktop test:runtime-diagnostics` | ❌ W0 | ⬜ pending |
| 06-04-01 | 04 | 4 | TEST-06, TEST-07 | T-06-07 | No-mock UI workflow uses deterministic media and generated command bridge only | e2e helper | `rg -n "runRealImportPreviewExportWorkflow|generatePhase6MediaFixtures" apps/desktop-electron/tests/helpers` | ❌ W0 | ⬜ pending |
| 06-04-02 | 04 | 4 | TEST-06, TEST-07 | T-06-08 | Dev and packaged no-mock import/preview/export validate output | e2e | `pnpm --filter @video-editor/desktop test:real-workflow && pnpm --filter @video-editor/desktop test:packaged-real-workflow` | ❌ W0 | ⬜ pending |
| 06-05-01 | 05 | 5 | TEST-07 | T-06-09 | Release docs do not imply bundled GPL/nonfree FFmpeg | source guard | `pnpm run test:phase6-release-gates` | ❌ W0 | ⬜ pending |
| 06-05-02 | 05 | 5 | TEST-06, TEST-07 | T-06-10 | Root gates include Phase 6 checks or document slower package gates explicitly | root gate | `pnpm run test && /Users/zhiwen/.cargo/bin/just test` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `apps/desktop-electron/tests/packaged-smoke.spec.ts` - packaged launch, file renderer, preload, native binding, runtime probe.
- [ ] `apps/desktop-electron/tests/real-workflow.spec.ts` - no-mock generated media import/edit/preview/export workflow.
- [ ] Runtime capability report tests in the Rust crate chosen by the planner.
- [ ] `scripts/phase6-release-guards.sh` - release docs, package scripts, source ownership, and contract drift checks.
- [ ] `docs/release-ffmpeg-manifest.md`, `docs/third-party-notices.md`, `docs/mvp-known-limits.md` - release readiness docs checked by source guard.

Existing infrastructure covers Rust, generated contracts, Phase 4/5 workspace guards, and root `pnpm` / `just` command surfaces.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| macOS signing/notarization | TEST-07 | Certificates and Apple account are not assumed in local MVP | Confirm `docs/mvp-known-limits.md` states signing/notarization status and that packaged smoke is unsigned/local-only if no signing config exists. |
| Bundled FFmpeg redistribution | TEST-07 | Phase 6 defaults to external FFmpeg; actual redistribution requires selected binary/build source | Confirm docs state FFmpeg is external/user-provided unless implementation explicitly bundles it with manifest/notices. |

---

## Validation Sign-Off

- [x] All tasks have an automated verify command or Wave 0 dependency.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all missing references.
- [x] No watch-mode flags.
- [x] Feedback latency < 10 minutes.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** approved 2026-06-18
