---
phase: 15
slug: audio-engine-and-dsp-timeline-pipeline
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-19
---

# Phase 15 - Validation Strategy

Per-phase validation contract for the audio engine and DSP timeline pipeline.

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`; Playwright Electron workspace tests; shell source guards with `rg` |
| **Config file** | `Cargo.toml`, `package.json`, `scripts/phase15-source-guards.sh` after Wave 0 |
| **Quick run command** | `cargo test -p audio_engine -- --nocapture` after crate creation |
| **Full suite command** | `pnpm run test:phase15 && pnpm run test:contracts` after scripts are added |
| **Estimated runtime** | ~180 seconds for focused phase gate, excluding optional native audio |

## Sampling Rate

- **After every task commit:** Run the focused test for the touched layer, for example `cargo test -p audio_engine dsp_timeline -- --nocapture`.
- **After every plan wave:** Run `pnpm run test:phase15-rust && pnpm run test:phase15-source-guards`.
- **Before `$gsd-verify-work`:** Run `pnpm run test:phase15 && pnpm run test:contracts`; run optional native output proof only when the host and env allow it.
- **Max feedback latency:** 180 seconds for the required automated phase gate.

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 15-W0-01 | 15-01 | 1 | AUDIO2-01 | T-15-01 | Audio session responses are generation-gated and stale/canceled buffers cannot present. | unit/integration | `cargo test -p audio_engine audio_session_generation -- --nocapture` | ❌ W0 | ⬜ pending |
| 15-W0-02 | 15-01 | 1 | AUDIO2-02 | T-15-02 | Gain, track mute, pan, fades, volume keyframes, and effect slots use integer/rational mapping. | unit/snapshot | `cargo test -p audio_engine dsp_timeline -- --nocapture` | ❌ W0 | ⬜ pending |
| 15-W0-03 | 15-02 | 1 | AUDIO2-03 | T-15-03 | CoreAudio/WASAPI are behind Rust traits; CI uses mock output by default; no native handles cross TypeScript. | unit/capability | `cargo test -p audio_output_desktop audio_output_capabilities -- --nocapture` | ❌ W0 | ⬜ pending |
| 15-W0-04 | 15-03 | 2 | AUDIO2-03 | T-15-04 | Waveform/peak display consumes artifact-store summaries and never canonicalizes derived artifacts. | source guard / Playwright | `pnpm run test:phase15-source-guards && pnpm --filter @video-editor/desktop test:workspace -g "音频预览|波形|播放状态"` | ❌ W0 | ⬜ pending |
| 15-W0-05 | 15-04 | 2 | AUDIO2-04 | T-15-05 | Preview/export audio mix parity differences are classified through Rust-owned diagnostics. | parity | `cargo test -p testkit audio_preview_export_parity -- --nocapture` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

## Wave 0 Requirements

- [ ] `crates/audio_engine/tests/audio_session_generation.rs` - shared `TimelineClock`/`PlaybackGeneration`, cancel, seek, pause, buffering, stale rejection.
- [ ] `crates/audio_engine/tests/dsp_timeline.rs` - integer/rational gain, mute, pan, fades, volume keyframes, future effect slots.
- [ ] `crates/audio_output_desktop/tests/audio_output_capabilities.rs` - mock default output and platform capability labels for CoreAudio/WASAPI.
- [ ] `crates/testkit/tests/audio_preview_export_parity.rs` - deterministic typed mix/sample summary parity.
- [ ] `scripts/phase15-source-guards.sh` - renderer boundary guard for audio graphs, sample mixing, native device handles, FFmpeg audio filters, waveform artifact paths, SQLite, fingerprints, and dirty ranges.
- [ ] `apps/desktop-electron/tests/workspace.spec.ts` additions - mock audio preview/waveform/status UI coverage.
- [ ] `package.json` scripts - `test:phase15-rust`, `test:phase15-source-guards`, `test:phase15-workspace`, and `test:phase15`.

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Native macOS CoreAudio playback | AUDIO2-03 | CI may not have a reliable output device or permission to open one. | On macOS with an output device, run `VIDEO_EDITOR_TEST_NATIVE_AUDIO=1 cargo test -p audio_output_desktop native_audio -- --nocapture` after the native test exists. |
| Native Windows WASAPI playback | AUDIO2-03 | Current development host is macOS; Windows WASAPI proof needs a Windows runner or manual machine. | On Windows with an output device, run `VIDEO_EDITOR_TEST_NATIVE_AUDIO=1 cargo test -p audio_output_desktop native_audio -- --nocapture` after the native test exists. |

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all MISSING references.
- [x] No watch-mode flags.
- [x] Feedback latency < 180s for required gate.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** approved 2026-06-19
