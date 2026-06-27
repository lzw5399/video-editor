---
phase: 04
slug: jianying-style-desktop-workspace
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-17
---

# Phase 04 - Validation Strategy

Per-phase validation contract for the Jianying-style Electron desktop workspace, Simplified Chinese UI, command-only Rust integration, and stable timeline/panel layout.

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Playwright Test `1.61.0` for Electron UI; TypeScript build/source guards; existing Rust/cargo gates through root build/test |
| **Config file** | `apps/desktop-electron/playwright.config.ts`, `apps/desktop-electron/package.json`, `package.json`, `justfile` |
| **Quick run command** | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts` after desktop build, or the Phase 4 focused script added in Wave 0 |
| **Full suite command** | `PATH="$HOME/.cargo/bin:$PATH" just test` if `just` is installed; otherwise `pnpm run test` plus the generated drift check |
| **Estimated runtime** | ~120-300 seconds after Phase 4 Playwright and guard scripts are added |

## Sampling Rate

- **After every task commit:** Run the narrowest affected desktop build or focused Playwright test, plus `pnpm run test:phase4-source-guards` once the guard script exists.
- **After every plan wave:** Run `pnpm --filter @video-editor/desktop test` and the Phase 4 focused workspace/layout test command.
- **Before `$gsd-verify-work`:** Run `PATH="$HOME/.cargo/bin:$PATH" just build`, `PATH="$HOME/.cargo/bin:$PATH" just test`, `pnpm run test:phase4-source-guards`, Phase 4 Playwright layout tests, and `git diff --exit-code schemas apps/desktop-electron/src/generated`.
- **Max feedback latency:** 300 seconds for the full Phase 4 suite on the local machine.

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 04-01-W0 | 04-01 | 1 | UI-01, UI-03, UI-06 | T-04-01 / T-04-04 | Workspace opens directly to Chinese Jianying-style regions without English smoke labels or alternate Asset/Clip terminology | Electron E2E + source guard | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts -g "Chinese editor workspace" && pnpm run test:phase4-source-guards` | Wave 0 | pending |
| 04-01-W1 | 04-01 | 1 | UI-05 | T-04-04 | Top categories, left panel, preview, inspector, and timeline use stable dimensions at 1280x800 and 1120x720 | layout/visual | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts -g "layout stability"` | Wave 0 | pending |
| 04-02-W0 | 04-02 | 2 | UI-02, UI-03, UI-06 | T-04-01 / T-04-05 | Material, text, audio, deferred-category, and inspector panels show Chinese states and no renderer-only semantics | Electron E2E | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts -g "workspace panels"` | Wave 0 | pending |
| 04-02-W1 | 04-02 | 2 | UI-04 | T-04-02 / T-04-03 | Inspector text/audio/volume/mute actions call generated command envelopes and apply Rust responses only | Electron E2E + source guard | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts -g "inspector commands" && pnpm run test:phase4-source-guards` | Wave 0 | pending |
| 04-03-W0 | 04-03 | 3 | UI-04, UI-05 | T-04-02 / T-04-03 / T-04-04 | Timeline add/select/move/split/trim/delete/undo/redo surface updates from `TimelineCommandResponse` without direct draft mutation | Electron E2E + source guard | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts -g "command-only timeline"` | Wave 0 | pending |
| 04-03-W1 | 04-03 | 3 | TEST-06 | T-04-02 / T-04-05 | Phase 4 subset of import/edit/preview-shell flow is proven while preview/export remain explicitly deferred | Electron E2E | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts -g "Phase 4 editor flow"` | Wave 0 | pending |
| 04-04-W0 | 04-04 | 4 | UI-01, UI-02, UI-03, UI-04, UI-05, UI-06, TEST-06 | T-04-01 / T-04-02 / T-04-03 / T-04-04 / T-04-05 | Final source guards, generated drift checks, Chinese workspace E2E, and layout/visual checks are named and included in root gates | full gate | `pnpm run test:phase4-source-guards && pnpm --filter @video-editor/desktop playwright test && git diff --exit-code schemas apps/desktop-electron/src/generated && PATH="$HOME/.cargo/bin:$PATH" just test` | Wave 0 | pending |

*Status: pending until the corresponding Phase 4 plan creates tests/scripts and the command exits green.*

## Threat References

| Ref | Threat | Required Mitigation |
|-----|--------|---------------------|
| T-04-01 | English smoke UI or invented terminology ships as the editor workspace | Chinese copy/source guards and Playwright role/text assertions for `顶部功能区`, `素材面板`, `预览窗口`, `属性检查器`, and `时间线` |
| T-04-02 | Renderer mutates `Draft.tracks`, segments, undo/redo, snapping, or timeranges directly | Source guards plus E2E assertions that accepted edits apply only `TimelineCommandResponse` |
| T-04-03 | UI bypasses the preload/native boundary or imports Electron/Node APIs in renderer | Preserve `window.videoEditorCore` as the only renderer bridge and guard `electron`, `node:*`, `fs`, `path`, `child_process`, and raw `ipcRenderer` imports |
| T-04-04 | Layout shifts, overlaps, or clips controls during selection/hover/playback state | Fixed dimensions and Playwright geometry/screenshot checks at 1280x800 and 1120x720 |
| T-04-05 | UI hides rejected Rust commands or locally repairs invalid edits | Keep prior accepted draft state and show Chinese command errors from `CommandResultEnvelope.error` |

## Wave 0 Requirements

- [ ] Add Phase 4 Playwright coverage for Chinese workspace regions, panel states, command-only timeline flow, and layout stability.
- [ ] Add `test:phase4-source-guards` at the root and include it in the public `pnpm run test` / `just test` path.
- [ ] Add source guards for direct renderer draft mutation, renderer Electron/Node imports, renderer FFmpeg/ffprobe construction, generated contract edits, and English-only key workspace labels.
- [ ] Add or update focused desktop test scripts for Phase 4 workspace/layout checks.
- [ ] Generate and commit screenshot baselines if visual snapshots are used as hard gates; otherwise use deterministic bounding-box geometry checks plus optional screenshots.

## Manual-Only Verifications

All Phase 4 MVP workspace behaviors should have automated verification. Subjective polish may still be reviewed during `$gsd-ui-review`, but the phase cannot rely on manual-only checks for region visibility, Chinese copy, command-only edits, or layout stability.

## Validation Sign-Off

- [x] All tasks have automated verify targets or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all missing test references.
- [x] No watch-mode flags.
- [x] Feedback latency target is below 300 seconds.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** pending execution
