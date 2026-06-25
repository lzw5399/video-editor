---
phase: 19
slug: production-effects-retiming-and-transition-semantics
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-25
---

# Phase 19 - Validation Strategy

> Per-phase validation contract for production effects, retiming, transition semantics, masks, blends, preview/export parity, and high-frequency interaction safety.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` 1.95.0, Playwright `@playwright/test` 1.61.0, bash source guards, schema/TypeScript contract checks |
| Config file | `Cargo.toml`, root `package.json`, `apps/desktop-electron/playwright.config.ts` |
| Quick run command | `pnpm run test:phase19-rust` after Wave 0 creates the script |
| Full suite command | `pnpm run test:phase19` after Wave 0 creates the script |
| Estimated runtime | Quick: under 180 seconds after Wave 0; full phase gate: under 30 minutes on local desktop test media |

---

## Sampling Rate

- After every task commit: run the narrow Rust crate or Playwright test named in that task.
- After any draft schema or generated TypeScript change: run `cargo test -p draft_model schema_exports -- --nocapture` and `pnpm run test:contracts`.
- After every plan wave: run `pnpm run test:phase19-rust`, `pnpm run test:phase19-source-guards`, and the wave-specific desktop/testkit gate.
- Before final verification: run `pnpm run test:phase19`, `pnpm run test:no-product-fallback`, `cargo check --workspace --locked`, and `pnpm run test:contracts`.
- Max feedback latency: no three consecutive task commits may lack an automated gate.

---

## Requirement Verification Map

| Req ID | Behavior | Test Type | Automated Command | File Exists | Status |
|--------|----------|-----------|-------------------|-------------|--------|
| PRODFX-01 | Constant retiming source mapping, split/trim/move validation, audio follow-speed diagnostics, render graph/export representation | Rust unit/integration plus export parity | `cargo test -p draft_model retiming && cargo test -p draft_commands retiming && cargo test -p engine_core retiming && cargo test -p render_graph retiming && cargo test -p ffmpeg_compiler retiming` | No; Wave 0/1 must add tests | pending |
| PRODFX-02 | Transition adjacency, overlap, undoable commands, preview/export support or explicit degraded diagnostics | Rust unit/integration plus visible timeline E2E when controls are enabled | `cargo test -p draft_commands transition && cargo test -p render_graph transition && cargo test -p realtime_preview_runtime transition && cargo test -p ffmpeg_compiler transition` | Partial placeholders only | pending |
| PRODFX-03 | Capability registry maps semantic effect/filter/transition intent to preview/export support states | Rust capability matrix plus schema/contract checks | `cargo test -p draft_model capability && cargo test -p render_graph capability && cargo test -p realtime_preview_runtime capability_matrix` | Partial realtime matrix exists | pending |
| PRODFX-04 | Masks, blends, blur, filters, and complex effects use GPU preview where supported and classify unsupported export paths | Rust GPU/offscreen tests, compiler diagnostics, desktop E2E | `cargo test -p realtime_preview_runtime effects && cargo test -p ffmpeg_compiler effects && pnpm --filter @video-editor/desktop exec playwright test tests/production-effects.spec.ts --reporter=line` | No Phase 19 file yet | pending |
| PRODFX-05 | Kaipai-like fixtures verify template import preview/export parity, compatibility reports, and performance budgets | `testkit` plus desktop template import/product E2E | `cargo test -p testkit production_effects -- --nocapture && pnpm --filter @video-editor/desktop exec playwright test tests/template-import.spec.ts --reporter=line` | Existing template tests need Phase 19 cases | pending |

---

## Wave 0 Requirements

- [ ] `scripts/phase19-source-guards.sh` blocks renderer-owned FFmpeg strings, retime source mapping, transition overlap validation, effect evaluation, dirty-range/cache semantics, fallback success, provider-native IDs as internal semantics, and per-mousemove save/revision/undo loops.
- [ ] Root `package.json` contains `test:phase19-rust`, `test:phase19-source-guards`, `test:phase19-desktop`, and `test:phase19`.
- [ ] Rust RED tests exist before implementation for retiming, transition semantics, effect capability registry, GPU preview/export classification, and template fixture parity.
- [ ] Playwright E2E is added before visible controls are treated as product-complete.
- [ ] UI implementation is followed by an independent `gsd-ui-auditor` review artifact.

---

## Manual-Only Verifications

All Phase 19 product behavior must have automated Rust, Playwright, source guard, or testkit coverage. Manual inspection may supplement UI polish, but cannot replace product success evidence.

---

## Validation Sign-Off

- [ ] All tasks have an automated verify command or a Wave 0 dependency.
- [ ] Sampling continuity: no three consecutive tasks without automated verify.
- [ ] Wave 0 covers all missing validation references.
- [ ] No watch-mode flags in verification commands.
- [ ] Feedback latency targets are listed per plan wave.
- [ ] `nyquist_compliant: true` is set after the final plan/checker gate proves complete validation coverage.

**Approval:** pending
