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
| PRODFX-01 | Constant retiming and speed curve source mapping, split/trim/move validation, audio follow-speed diagnostics, render graph/audio graph/export representation | Rust unit/integration plus audio and export parity | `cargo test -p draft_commands retiming_commands -- --nocapture && cargo test -p engine_core retiming -- --nocapture && cargo test -p render_graph production_effects -- --nocapture && cargo test -p audio_engine dsp_timeline -- --nocapture && cargo test -p testkit audio_preview_export_parity -- --nocapture && cargo test -p ffmpeg_compiler production_effects -- --nocapture` | Wave 0 RED files now exist for draft_commands, engine_core, render_graph, audio_engine, ffmpeg_compiler | RED pending implementation |
| PRODFX-02 | Transition adjacency, overlap, undoable commands, preview/export support or explicit degraded diagnostics | Rust unit/integration plus visible timeline E2E when controls are enabled | `cargo test -p draft_commands transition && cargo test -p render_graph transition && cargo test -p realtime_preview_runtime transition && cargo test -p ffmpeg_compiler transition` | Wave 0 RED files now exist for draft_commands/render_graph/realtime_preview_runtime/ffmpeg_compiler | RED pending implementation |
| PRODFX-03 | Capability registry maps semantic effect/filter/transition intent to preview/export support states | Rust capability matrix plus schema/contract checks | `cargo test -p draft_model capability && cargo test -p render_graph capability && cargo test -p realtime_preview_runtime capability_matrix` | Wave 0 RED files now exist in draft_model/render_graph/realtime_preview_runtime | RED pending implementation |
| PRODFX-04 | Masks, blends, blur, filters, and complex effects use GPU preview where supported and classify unsupported export paths | Rust GPU/offscreen tests, compiler diagnostics, desktop E2E | `cargo test -p realtime_preview_runtime production_effects -- --nocapture && cargo test -p ffmpeg_compiler production_effects -- --nocapture && pnpm --filter @video-editor/desktop exec playwright test tests/production-effects.spec.ts --reporter=line` | Wave 0 RED preview/compiler/desktop files now exist | RED pending implementation |
| PRODFX-05 | Kaipai-like fixtures verify template import preview/export parity, compatibility reports, and performance budgets | `testkit` plus desktop template import/product E2E | `cargo test -p testkit production_effects -- --nocapture && pnpm --filter @video-editor/desktop exec playwright test tests/template-import.spec.ts --reporter=line` | Wave 0 RED testkit preview/export files now exist | RED pending implementation |

---

## Wave 0 Requirements

- [x] `scripts/phase19-source-guards.sh` blocks renderer-owned FFmpeg strings, retime source mapping, transition overlap validation, effect evaluation, dirty-range/cache semantics, fallback success, provider-native IDs as internal semantics, and per-mousemove save/revision/undo loops.
- [x] Root `package.json` contains `test:phase19-rust`, `test:phase19-source-guards`, `test:phase19-desktop`, and `test:phase19`.
- [x] Rust RED tests exist before implementation for retiming, audio graph retiming, transition semantics, effect capability registry, GPU preview/export classification, and template fixture parity.
- [x] Playwright E2E is added before visible controls are treated as product-complete.
- [ ] UI implementation is followed by an independent UI review artifact from `gsd-ui-auditor`, the GSD UI review workflow, or another separate reviewer/subagent; if no independent reviewer path is available, sign-off remains blocked rather than falling back to self-audit.

## Wave 0 RED Commands

The following commands were added as executable RED gates in Plan 19-01. They are expected to fail until later Phase 19 implementation plans replace string-only semantics and wire Rust-owned production behavior:

```bash
pnpm run test:phase19-source-guards -- --wave0
cargo test -p draft_model production_effects_contracts -- --nocapture
cargo test -p draft_commands retiming_commands -- --nocapture
cargo test -p draft_commands transition_commands -- --nocapture
cargo test -p engine_core retiming -- --nocapture
cargo test -p audio_engine dsp_timeline -- --nocapture
cargo test -p render_graph production_effects -- --nocapture
cargo test -p realtime_preview_runtime production_effects -- --nocapture
cargo test -p ffmpeg_compiler production_effects -- --nocapture
cargo test -p testkit production_effects -- --nocapture
pnpm --filter @video-editor/desktop exec playwright test tests/production-effects.spec.ts --reporter=line
```

`nyquist_compliant` and `wave_0_complete` intentionally remain `false` in frontmatter. Plan 19-15 owns final closeout after all Phase 19 implementation gates and the independent UI audit pass.

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
