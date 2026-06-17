---
phase: 05
slug: preview-and-export-pipeline
status: draft
nyquist_compliant: true
wave_0_complete: false
revised: 2026-06-18
---

# Phase 05 - Validation Strategy

Per-phase validation contract for the revised preview/export pipeline plans.

## Test Infrastructure

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test`, Playwright Electron, bash source guards |
| Config file | `Cargo.toml`, `package.json`, `justfile`, `apps/desktop-electron/playwright.config.ts` |
| Quick run command | `pnpm run test:phase5-render-core` |
| Full suite command | `just test` |
| Visual artifacts | Playwright screenshots written under `test-results/phase5/` for 1280x800 and 1120x720 |

## Sampling Rate

- After every task commit: run the narrowest affected Rust crate test plus `pnpm run test:phase5-source-guards` once that script exists.
- After every plan wave: run `pnpm run test:phase5-render-core` and any focused desktop preview/export test affected by the wave.
- Before verification: `just build`, `just test`, `pnpm run test:phase5-render-core`, `pnpm run test:phase5-source-guards`, and `git diff --exit-code schemas apps/desktop-electron/src/generated` must be green.
- UI visual checks are automated: Playwright must assert geometry at 1120x720 and 1280x800, save screenshots, and include notes that the Phase 04.1 compact scrollbar/proportion baseline still holds.

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 05-01 | 1 | TEST-03, EXP-02 | T-05-01 | Invalid draft ranges are classified before render planning | unit | `cargo test -p engine_core normalization -- --nocapture` | planned | pending |
| 05-01-02 | 05-01 | 1 | TEST-03, EXP-02 | T-05-02 | Frame/range state uses integer microseconds and rational frame rates | unit/snapshot | `cargo test -p engine_core frame_state -- --nocapture` | planned | pending |
| 05-01-03 | 05-01 | 1 | TEXT-03 | T-05-03 | Missing text profile/font data is classified | unit/snapshot | `cargo test -p engine_core text_layout -- --nocapture` | planned | pending |
| 05-02-01 | 05-02 | 2 | TEST-04, EXP-02 | T-05-04 | Render graph contains no process execution or FFmpeg syntax | unit/snapshot | `cargo test -p render_graph render_graph -- --nocapture` | planned | pending |
| 05-02-02 | 05-02 | 2 | PREV-02, PREV-03, EXP-01, EXP-02 | T-05-05 | Preview/export profiles share one graph family | unit/snapshot | `cargo test -p render_graph output_profiles -- --nocapture` | planned | pending |
| 05-03-01 | 05-03 | 3 | TEST-04, EXP-01, EXP-02 | T-05-06 | FFmpeg jobs are structured argument vectors, not shell strings | unit/snapshot | `cargo test -p ffmpeg_compiler ffmpeg_job -- --nocapture` | planned | pending |
| 05-03-02 | 05-03 | 3 | TEXT-03, TEST-04 | T-05-07 | ASS/font capability failures are classified | unit/snapshot | `cargo test -p ffmpeg_compiler ass -- --nocapture` | planned | pending |
| 05-03-03 | 05-03 | 3 | TEST-04 | T-05-06 | Stable filter/script snapshots are reviewable | unit/snapshot | `cargo test -p ffmpeg_compiler -- --nocapture` | planned | pending |
| 05-04-01 | 05-04 | 4 | PREV-03, PREV-04 | T-05-08 | Cache keys stay in preview_service and out of renderer/project JSON | unit | `cargo test -p preview_service cache -- --nocapture` | planned | pending |
| 05-04-02 | 05-04 | 4 | PREV-01, PREV-02, PREV-03, EXP-02 | T-05-09 | Preview artifacts come from the shared graph/compiler path | integration | `cargo test -p preview_service preview_generation -- --nocapture` | planned | pending |
| 05-04-03 | 05-04 | 4 | PREV-04 | T-05-08 | Only overlapping target ranges invalidate | unit | `cargo test -p preview_service invalidation -- --nocapture` | planned | pending |
| 05-05-01 | 05-05 | 5 | PREV-01, PREV-02, PREV-03 | T-05-10 | Rust-generated contracts own preview command shape | contract | `cargo test -p draft_model schema_exports_include_preview_command_contracts -- --nocapture && cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` | planned | pending |
| 05-05-02 | 05-05 | 5 | PREV-01, PREV-02, PREV-03, PREV-04 | T-05-11 | Binding composes preview_service and returns artifact metadata only | integration | `cargo test -p bindings_node preview_commands -- --nocapture` | planned | pending |
| 05-05-03 | 05-05 | 5 | PREV-01, PREV-02 | T-05-12 | Renderer helpers build envelopes only | unit/source guard | `pnpm run test:phase5-source-guards` | planned | pending |
| 05-06-01 | 05-06 | 6 | PREV-01, PREV-02, PREV-03 | T-05-12 | UI stores display state but no cache/render semantics | Playwright | `pnpm --filter @video-editor/desktop test:workspace -g "预览"` | planned | pending |
| 05-06-02 | 05-06 | 6 | PREV-01, PREV-02 | T-05-12 | Screenshot geometry gates cover 1120x720 and 1280x800 | Playwright/source guard | `pnpm run test:phase5-source-guards && pnpm --filter @video-editor/desktop test:workspace -g "预览"` | planned | pending |
| 05-07-01 | 05-07 | 4 | EXP-03 | T-05-13 | Runtime owns progress, cancellation, and bounded logs | runtime integration | `cargo test -p media_runtime export_job -- --nocapture` | planned | pending |
| 05-07-02 | 05-07 | 4 | EXP-04 | T-05-14 | Output validation uses ffprobe metadata | runtime integration | `cargo test -p media_runtime output_validation -- --nocapture` | planned | pending |
| 05-08-01 | 05-08 | 7 | EXP-01, EXP-03, EXP-04 | T-05-15 | Rust-generated contracts own export command shape | contract | `cargo test -p draft_model schema_exports_include_export_command_contracts -- --nocapture && cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture` | planned | pending |
| 05-08-02 | 05-08 | 7 | EXP-01, EXP-03, EXP-04 | T-05-16 | Binding registry owns Electron job IDs; renderer owns no process handles | integration | `cargo test -p bindings_node export_commands -- --nocapture` | planned | pending |
| 05-08-03 | 05-08 | 7 | EXP-01, EXP-03, EXP-04 | T-05-17 | Chinese export UI uses Rust commands only | Playwright | `pnpm --filter @video-editor/desktop test:workspace -g "导出"` | planned | pending |
| 05-09-01 | 05-09 | 8 | EXP-02, TEST-05 | T-05-18 | Preview/export parity uses one compiled path and documented tolerance | render golden | `cargo test -p testkit preview_export_parity -- --nocapture` | planned | pending |
| 05-09-02 | 05-09 | 8 | TEST-03, TEST-04, TEST-05 | T-05-19 | Final gates block renderer ownership and contract drift | aggregate | `pnpm run test:phase5-render-core && pnpm run test:phase5-source-guards && pnpm run test && just test` | planned | pending |

## Wave 0 Requirements

- `crates/engine_core/tests/normalization.rs` - normalization, track stacking, time mapping.
- `crates/engine_core/tests/frame_state_snapshots.rs` - frame-state and text layout snapshots.
- `crates/render_graph/tests/render_graph_snapshots.rs` - renderer-neutral graph/profile snapshots.
- `crates/ffmpeg_compiler/tests/ffmpeg_job_snapshots.rs` - FFmpeg job/script snapshots.
- `crates/ffmpeg_compiler/tests/ass_snapshots.rs` - deterministic text sidecar snapshots.
- `crates/ffmpeg_compiler/tests/capability_snapshots.rs` - font/filter/encoder capability classification.
- `crates/preview_service/tests/cache_invalidation.rs` - range invalidation.
- `crates/preview_service/tests/preview_generation.rs` - preview frame and short segment generation.
- `crates/media_runtime/tests/export_job.rs` - progress, logs, cancel, classified errors.
- `crates/media_runtime/tests/output_validation.rs` - ffprobe output metadata validation.
- `crates/testkit/tests/preview_export_parity.rs` - preview/export frame tolerance.
- `scripts/phase5-source-guards.sh` - renderer/core boundary checks.
- `package.json` scripts `test:phase5-render-core` and `test:phase5-source-guards`, chained into `test`.
- Desktop Playwright coverage for Chinese preview/export behavior and screenshots at 1120x720 and 1280x800.

## Automated Screenshot Artifacts

| Artifact | Created By | Requirements | Assertions |
|----------|------------|--------------|------------|
| `test-results/phase5/preview-1280x800.png` | 05-06 | PREV-01, PREV-02 | preview frame/status/control geometry, no overlap, dark compact scrollbar/proportion baseline |
| `test-results/phase5/preview-1120x720.png` | 05-06 | PREV-01, PREV-02 | narrow viewport layout, controls fit, Phase 04.1 timeline proportions preserved |
| `test-results/phase5/export-1280x800.png` | 05-08 | EXP-01, EXP-03, EXP-04 | output path/preset/progress/log/validation/cancel states fit |
| `test-results/phase5/export-1120x720.png` | 05-08 | EXP-01, EXP-03, EXP-04 | export panel and monitor remain visible without clipped Chinese text |

## Threat References

| Threat | Mitigation |
|--------|------------|
| T-05-01 | Validate draft ranges and material status in `engine_core` before render planning. |
| T-05-06 | `ffmpeg_compiler` emits `Vec<OsString>` argument plans and derived scripts; `media_runtime` executes them. |
| T-05-08 | Preview cache entries include target ranges, semantic fingerprints, output profile, and material dependencies. |
| T-05-12 | Source guards reject renderer-owned FFmpeg, render graph, cache keys, export scripts, and process APIs. |
| T-05-13 | Export runtime uses progress parsing, cancel tokens, timeouts, and bounded logs. |
| T-05-18 | Parity compares exact dimensions/time metadata plus documented pixel tolerance instead of byte-perfect output. |

## Validation Sign-Off

- All tasks have planned automated verify commands.
- No Phase 5 plan has checkpoint tasks.
- Screenshot checks are executable artifacts with geometry assertions.
- Requirements TEXT-03, PREV-01..04, EXP-01..04, and TEST-03..05 are mapped.
- `nyquist_compliant: true` set in frontmatter.
