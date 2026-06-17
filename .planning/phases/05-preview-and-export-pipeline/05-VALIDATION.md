---
phase: 05
slug: preview-and-export-pipeline
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-17
---

# Phase 05 — Validation Strategy

> Per-phase validation contract for preview/export pipeline execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`, Playwright Electron, bash source guards |
| **Config file** | `Cargo.toml`, `package.json`, `justfile`, `apps/desktop-electron/playwright.config.ts` |
| **Quick run command** | `pnpm run test:phase5-render-core` |
| **Full suite command** | `just test` |
| **Estimated runtime** | ~180-300 seconds once render gates exist |

---

## Sampling Rate

- **After every task commit:** Run the narrowest affected Rust crate test plus `pnpm run test:phase5-source-guards` once that script exists.
- **After every plan wave:** Run `pnpm run test:phase5-render-core` and any focused desktop preview/export test affected by the wave.
- **Before `$gsd-verify-work`:** `just build`, `just test`, `pnpm run test:phase5-render-core`, `pnpm run test:phase5-source-guards`, and `git diff --exit-code schemas apps/desktop-electron/src/generated` must be green.
- **Max feedback latency:** Keep narrow crate checks under 60 seconds where possible; full render gates may exceed this.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 05-01 | 1 | TEST-03 | — | N/A | unit/snapshot | `cargo test -p engine_core -- --nocapture` | ❌ W0 | ⬜ pending |
| 05-01-02 | 05-01 | 1 | TEXT-03 | — | Missing font/capability is classified | unit/snapshot | `cargo test -p engine_core text_layout -- --nocapture` | ❌ W0 | ⬜ pending |
| 05-02-01 | 05-02 | 1 | TEST-04, EXP-02 | — | Renderer-neutral graph contains no process execution | unit/snapshot | `cargo test -p render_graph -- --nocapture` | ❌ W0 | ⬜ pending |
| 05-02-02 | 05-02 | 1 | TEST-04, EXP-01 | T-05-01 | FFmpeg args are vectors/scripts, not shell strings | unit/snapshot | `cargo test -p ffmpeg_compiler -- --nocapture` | ❌ W0 | ⬜ pending |
| 05-03-01 | 05-03 | 2 | PREV-01, PREV-02 | — | Preview requests return derived artifact metadata only | integration | `cargo test -p preview_service preview_frame -- --nocapture` | ❌ W0 | ⬜ pending |
| 05-03-02 | 05-03 | 2 | PREV-03, PREV-04 | T-05-02 | Cache invalidates overlapping ranges only | unit/integration | `cargo test -p preview_service preview_segment_cache -- --nocapture` | ❌ W0 | ⬜ pending |
| 05-03-03 | 05-03 | 2 | PREV-01, PREV-02, UI-06 | — | Renderer does not construct FFmpeg/render graph/cache keys | Playwright/source guard | `pnpm run test:phase5-source-guards && pnpm --filter @video-editor/desktop test:workspace -g "预览"` | ❌ W0 | ⬜ pending |
| 05-04-01 | 05-04 | 3 | EXP-01, EXP-03 | T-05-03 | Export progress/cancel/log state stays Rust-owned | runtime integration | `cargo test -p media_runtime export_job_runtime -- --nocapture` | ❌ W0 | ⬜ pending |
| 05-04-02 | 05-04 | 3 | EXP-04 | — | Output validation uses ffprobe metadata | runtime integration | `cargo test -p media_runtime output_validation -- --nocapture` | ❌ W0 | ⬜ pending |
| 05-04-03 | 05-04 | 3 | TEST-05, EXP-02 | — | Preview/export parity uses one compiled path | render golden | `cargo test -p testkit preview_export_parity -- --nocapture` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/engine_core/tests/normalization.rs` — normalization, track stacking, time mapping.
- [ ] `crates/engine_core/tests/frame_state_snapshots.rs` — frame-state and text layout snapshots.
- [ ] `crates/render_graph/tests/render_graph_snapshots.rs` — renderer-neutral graph snapshots.
- [ ] `crates/ffmpeg_compiler/tests/ffmpeg_job_snapshots.rs` — FFmpeg job/script snapshots.
- [ ] `crates/ffmpeg_compiler/tests/ass_snapshots.rs` — deterministic text sidecar snapshots.
- [ ] `crates/preview_service/tests/cache_invalidation.rs` — range invalidation.
- [ ] `crates/preview_service/tests/preview_generation.rs` — preview frame and short segment generation.
- [ ] `crates/media_runtime/tests/export_job.rs` — progress, logs, cancel, classified errors.
- [ ] `crates/media_runtime/tests/output_validation.rs` — ffprobe output metadata validation.
- [ ] `crates/testkit/tests/preview_export_parity.rs` — preview/export frame tolerance.
- [ ] `scripts/phase5-source-guards.sh` — renderer/core boundary checks.
- [ ] `package.json` scripts `test:phase5-render-core` and `test:phase5-source-guards`, chained into `test`.
- [ ] Desktop Playwright coverage for Chinese preview/export UI behavior.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Visual polish of preview/export controls in Chinese desktop workspace | PREV-01, EXP-01, UI-06 | Automated tests verify presence/geometry, but design quality still needs human visual check | Open the Electron app after Phase 5, seek preview, start/cancel export, and confirm Chinese labels, progress, logs, and errors fit without overlap |

---

## Threat References

| Threat | Mitigation |
|--------|------------|
| T-05-01 Shell injection through FFmpeg args | Use `Command::new(...).args(Vec<OsString>)`, filter scripts/sidecars written by Rust, and source guards preventing renderer FFmpeg construction |
| T-05-02 Stale preview cache after timeline/text edit | Cache entries include target ranges and semantic fingerprints; invalidation runs in `preview_service` after accepted Rust command responses |
| T-05-03 Unbounded export logs or hung process | Runtime stores bounded logs, parses progress, and kills the child process on cancel/timeout |

---

## Validation Sign-Off

- [x] All tasks have planned automated verify commands or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all currently missing Phase 5 tests.
- [x] No watch-mode flags.
- [x] Feedback latency target documented.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** approved 2026-06-17 for planning
