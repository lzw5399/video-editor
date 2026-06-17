---
phase: 07
slug: project-canvas-space-and-coordinate-system
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-18
---

# Phase 07 - Validation Strategy

> Per-phase validation contract for draft canvas schema, Rust-owned canvas commands, preview/export propagation, Chinese desktop UI, and source ownership guards.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`; generated contract drift tests; Playwright Electron via `@playwright/test`; shell source guards |
| **Config file** | `Cargo.toml`, `apps/desktop-electron/playwright.config.ts`, `package.json`, `justfile` |
| **Quick run command** | `pnpm run test:phase7` |
| **Full suite command** | `pnpm run test && /Users/zhiwen/.cargo/bin/just test` |
| **Estimated runtime** | ~3-12 minutes depending on Electron startup, FFmpeg, and root Rust tests |

---

## Sampling Rate

- **After every task commit:** Run the focused automated command listed for that task.
- **After every plan wave:** Run `pnpm run test:phase7` once the script exists; before that, run the focused commands from the completed wave.
- **Before `$gsd-verify-work`:** Run `pnpm run test`, `/Users/zhiwen/.cargo/bin/just test`, and `git diff --exit-code schemas apps/desktop-electron/src/generated`.
- **Max feedback latency:** 5 minutes for Rust/model/command/source-guard tasks, 12 minutes for Electron workspace gates.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 07-01-01 | 01 | 1 | CANVAS-01, CANVAS-02, CANVAS-03 | T-07-01 / T-07-02 | Malformed canvas dimensions, fps, aspect ratio, color, and image references are rejected in Rust | rust unit | `cargo test -p draft_model canvas -- --nocapture` | ❌ W0 | ⬜ pending |
| 07-01-02 | 01 | 1 | CANVAS-01, CANVAS-02, CANVAS-04 | T-07-03 | Generated schema and TypeScript contracts expose Rust-owned canvas payloads | schema + contract | `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` | ❌ W0 | ⬜ pending |
| 07-02-01 | 02 | 2 | CANVAS-01, CANVAS-02, CANVAS-04 | T-07-05 / T-07-06 | Canvas edits clone, validate, and commit through Rust command history | rust command | `cargo test -p draft_commands canvas -- --nocapture` | ❌ W0 | ⬜ pending |
| 07-02-02 | 02 | 2 | CANVAS-01, CANVAS-02, CANVAS-04 | T-07-07 | Node binding only routes typed `updateDraftCanvasConfig` envelopes | rust binding | `cargo test -p bindings_node canvas_commands -- --nocapture` | ❌ W0 | ⬜ pending |
| 07-03-01 | 03 | 2 | CANVAS-01, CANVAS-03 | T-07-08 | Engine profile width, height, and frame rate resolve from `Draft.canvasConfig` | rust unit | `cargo test -p engine_core canvas -- --nocapture` | ❌ W0 | ⬜ pending |
| 07-03-02 | 03 | 2 | CANVAS-02, CANVAS-03 | T-07-09 | Render graph and compiler carry background support/degraded/unsupported diagnostics | rust snapshot | `cargo test -p render_graph canvas -- --nocapture` | ❌ W0 | ⬜ pending |
| 07-03-03 | 03 | 2 | CANVAS-01, CANVAS-02, CANVAS-03 | T-07-10 | Preview/export production paths no longer rely on MVP hard-coded canvas profile | rust service | `cargo test -p preview_service canvas -- --nocapture` | ❌ W0 | ⬜ pending |
| 07-04-01 | 04 | 3 | CANVAS-04 | T-07-11 | Renderer builds command envelopes only and applies Rust command responses | e2e | `pnpm --filter @video-editor/desktop test:workspace -g "画布"` | ❌ W0 | ⬜ pending |
| 07-04-02 | 04 | 3 | CANVAS-01, CANVAS-02, CANVAS-04 | T-07-12 | Inspector and preview show Chinese canvas settings without direct draft mutation | e2e + layout | `pnpm --filter @video-editor/desktop test:workspace -g "草稿参数|画布"` | ❌ W0 | ⬜ pending |
| 07-04-03 | 04 | 3 | CANVAS-04 | T-07-13 | 1280x800 and 1120x720 workspaces keep five regions visible without white scrollbars | e2e screenshot | `pnpm --filter @video-editor/desktop test:workspace -g "草稿参数|画布"` | ❌ W0 | ⬜ pending |
| 07-05-01 | 05 | 4 | CANVAS-01, CANVAS-02, CANVAS-03, CANVAS-04 | T-07-14 | Source guards block renderer-owned canvas, render, export, and preview semantics | source guard | `bash scripts/phase7-source-guards.sh` | ❌ W0 | ⬜ pending |
| 07-05-02 | 05 | 4 | CANVAS-01, CANVAS-02, CANVAS-03, CANVAS-04 | T-07-15 | Root gates run Phase 07 checks and generated contracts have no drift | root gate | `pnpm run test:phase7 && pnpm run test && /Users/zhiwen/.cargo/bin/just test` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/draft_model/src/canvas.rs` plus `crates/draft_model/tests/canvas_config.rs` for schema defaults, validation, and coordinate conversion.
- [ ] Canvas fixture updates in `fixtures/draft/positive/*/project.json` and `fixtures/draft/negative/*canvas*/project.json`.
- [ ] `crates/draft_commands/src/canvas.rs` and command tests for apply/undo/redo/invalid canvas payloads.
- [ ] `crates/engine_core/tests/*canvas*` or normalization tests proving non-default canvas profile resolution.
- [ ] `crates/render_graph/tests/*canvas*`, `crates/ffmpeg_compiler/tests/*canvas*`, `crates/preview_service/tests/*canvas*`, and binding export tests for draft-driven canvas semantics.
- [ ] `apps/desktop-electron/tests/workspace.spec.ts` canvas UI command-routing and layout coverage at `1280x800` and `1120x720`.
- [ ] `scripts/phase7-source-guards.sh` plus `package.json` / `justfile` wiring for `test:phase7`.
- [ ] `docs/canvas-coordinate-system.md` documenting center-origin, `+X` right, `+Y` up, and UI pixel conversion.

Existing infrastructure covers Rust tests, contract generation, Playwright Electron, Phase 4/5 source guard patterns, and root package scripts.

---

## Manual-Only Verifications

All Phase 07 behaviors have automated verification. Visual quality still requires screenshot inspection if Playwright produces artifacts, but the gate is automated through the workspace tests and source guards.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all missing references.
- [x] No watch-mode flags.
- [x] Feedback latency < 12 minutes.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** approved 2026-06-18
