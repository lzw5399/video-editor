---
phase: 12
slug: media-io-hardware-decode-and-frame-texture-interop
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-18
---

# Phase 12 - Validation Strategy

Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` plus existing Electron/Node contract tests |
| Config file | Cargo workspace, package scripts in `package.json`, generated schema export tests |
| Quick run command | `cargo test -p media_runtime -p media_runtime_desktop` |
| Full suite command | `pnpm run test:phase12` |
| Estimated runtime | Focused Rust gates should stay under 60 seconds each; final `test:phase12` may exceed 60 seconds because it aggregates Rust, binding, source guard, and generated contract checks |

---

## Sampling Rate

- After every task commit: run the task-level `<automated>` commands from the plan.
- After every plan wave: run all automated commands for plans completed in that wave.
- Before `$gsd-verify-work`: run `pnpm run test:phase12` and the platform manual checks listed below.
- Max feedback latency: one failing task command before continuing to the next task; no three consecutive tasks may proceed without an automated command.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 12-01-01 | 01 | 1 | MEDIAIO-01, MEDIAIO-05 | T-12-01-01 / T-12-01-03 | Shared contracts serialize opaque handles and fallback reasons without platform pointers | unit | `cargo test -p media_runtime media_io_contracts -- --nocapture && cargo test -p media_runtime fallback_reasons -- --nocapture` | no - task creates tests | [ ] pending |
| 12-01-02 | 01 | 1 | MEDIAIO-03 | T-12-01-02 | Frame pool acquire/release/session close records owner session and generation | unit | `cargo test -p media_runtime frame_pool -- --nocapture` | no - task creates tests | [ ] pending |
| 12-02-01 | 02 | 2 | MEDIAIO-02, MEDIAIO-05 | T-12-02-01 / T-12-02-03 | Capability reports preserve FFmpeg fields and do not claim texture readiness without device compatibility | unit/platform-gated | `cargo test -p media_runtime runtime_capability -- --nocapture && cargo test -p media_runtime_desktop capabilities -- --nocapture` | no - task creates tests | [ ] pending |
| 12-02B-01 | 02B | 3 | MEDIAIO-02, MEDIAIO-04 | T-12-02B-01 / T-12-02B-02 | Binding/schema contracts expose metadata, not full-frame bytes or native pointers | contract/source guard | `cargo test -p bindings_node runtime_capabilities -- --nocapture && cargo test -p draft_model schema_exports -- --nocapture && pnpm run test:phase12-source-guards && git diff --exit-code schemas` | no - task creates tests/script | [ ] pending |
| 12-02B-02 | 02B | 3 | MEDIAIO-05 | T-12-02B-SC | Platform dependency set is approved before native implementation relies on it | metadata + human checkpoint | `cargo metadata --locked --format-version 1 >/tmp/video-editor-phase12-cargo-metadata.json` | n/a - checkpoint records summary | [ ] pending |
| 12-03-01 | 03 | 4 | MEDIAIO-01, MEDIAIO-03, MEDIAIO-05 | T-12-03-01 / T-12-03-02 | FFmpeg fallback uses argument arrays and returns CPU frame leases with diagnostics | integration | `cargo test -p media_runtime_desktop ffmpeg_fallback -- --nocapture && cargo test -p media_runtime material_probe -- --nocapture` | no - task creates tests | [ ] pending |
| 12-03-02 | 03 | 4 | MEDIAIO-05 | T-12-03-03 | Fallback ladder records selected path and preserves preview/export behavior | integration/regression | `cargo test -p media_runtime_desktop fallback_ladder -- --nocapture && cargo test -p preview_service preview -- --nocapture && cargo test -p media_runtime export_job -- --nocapture` | no - task creates tests | [ ] pending |
| 12-04-01 | 04 | 5 | MEDIAIO-02, MEDIAIO-03, MEDIAIO-05 | T-12-04-01 | macOS native path returns leases and explicit fallback diagnostics for unproven codecs | platform-gated | `cargo test -p media_runtime_desktop macos -- --nocapture && VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop macos_native -- --nocapture` | no - task creates tests | [ ] pending |
| 12-04-02 | 04 | 5 | MEDIAIO-03, MEDIAIO-04 | T-12-04-02 / T-12-04-03 | Metal texture handles include device identity and use CPU fallback when compatibility is unproven | platform-gated/unit | `cargo test -p media_runtime_desktop macos_texture -- --nocapture && cargo test -p media_runtime frame_pool -- --nocapture` | no - task creates tests | [ ] pending |
| 12-05-01 | 05 | 5 | MEDIAIO-02, MEDIAIO-03, MEDIAIO-05 | T-12-05-01 | Windows native path returns leases and explicit fallback diagnostics for unproven codecs | platform-gated | `cargo test -p media_runtime_desktop windows -- --nocapture && VIDEO_EDITOR_TEST_NATIVE_MEDIA=1 cargo test -p media_runtime_desktop windows_native -- --nocapture` | no - task creates tests | [ ] pending |
| 12-05-02 | 05 | 5 | MEDIAIO-03, MEDIAIO-04 | T-12-05-02 / T-12-05-03 | D3D texture handles include device identity and use CPU fallback when compatibility is unproven | platform-gated/unit | `cargo test -p media_runtime_desktop windows_texture -- --nocapture && cargo test -p media_runtime frame_pool -- --nocapture` | no - task creates tests | [ ] pending |
| 12-06-01 | 06 | 6 | MEDIAIO-01, MEDIAIO-02, MEDIAIO-05 | T-12-06-01 / T-12-06-02 | Preview adapter passes resolved source-time requests and device compatibility without owning timeline/render semantics | integration | `cargo test -p realtime_preview_runtime media_io_handoff -- --nocapture && cargo test -p realtime_preview_runtime stale_generation -- --nocapture` | no - task creates tests | [ ] pending |
| 12-06B-01 | 06B | 7 | MEDIAIO-02, MEDIAIO-03, MEDIAIO-04, MEDIAIO-05 | T-12-06B-01 / T-12-06B-02 | Preview decode/release bindings validate session/generation and omit full-frame/native pointer payloads | contract | `cargo test -p bindings_node preview_decode -- --nocapture && cargo test -p draft_model schema_exports -- --nocapture && git diff --exit-code schemas apps/desktop-electron/src/generated` | no - task creates tests | [ ] pending |
| 12-06C-01 | 06C | 8 | MEDIAIO-03, MEDIAIO-05 | T-12-06C-01 | Session close releases or reports CPU, platform-opaque, and texture handle leaks | unit/integration | `cargo test -p media_runtime_desktop session_leaks -- --nocapture && cargo test -p media_runtime frame_pool -- --nocapture` | no - task creates tests | [ ] pending |
| 12-06C-02 | 06C | 8 | MEDIAIO-04, MEDIAIO-05 | T-12-06C-02 / T-12-06C-03 | Final guards reject source-boundary violations and final script records platform verification notes | source guard/full gate | `pnpm run test:phase12-source-guards && pnpm run test:phase12` | no - task updates script | [ ] pending |

Status key: [ ] pending, [green] green, [red] red, [flaky] flaky.

---

## Wave 0 Requirements

Phase 12 does not have a standalone Wave 0 plan. Test scaffolds are created in the first task that introduces each behavior, using the TDD blocks in the revised plans:

- `crates/media_runtime/tests/media_io_contracts.rs` - MEDIAIO-01 contracts.
- `crates/media_runtime/tests/frame_pool.rs` - MEDIAIO-03 frame lease lifecycle.
- `crates/media_runtime/tests/fallback_reasons.rs` - MEDIAIO-05 fallback serialization.
- `crates/media_runtime_desktop/tests/capabilities.rs` - MEDIAIO-02 platform capability reporting.
- `crates/bindings_node/tests/runtime_capabilities.rs` - binding capability contract.
- `scripts/phase12-source-guards.sh` - MEDIAIO-04 source-boundary guard.
- `crates/media_runtime_desktop/tests/ffmpeg_fallback.rs` and `fallback_ladder.rs` - FFmpeg CPU fallback and ladder behavior.
- `crates/media_runtime_desktop/tests/macos_media_io.rs` - macOS native/skipped tests.
- `crates/media_runtime_desktop/tests/windows_media_io.rs` - Windows native/skipped tests.
- `crates/realtime_preview_runtime/tests/media_io_handoff.rs` - Phase 11 handoff adapter behavior.
- `crates/bindings_node/tests/preview_commands.rs` - preview decode/release binding behavior.
- `crates/media_runtime_desktop/tests/session_leaks.rs` - release/session-close leak diagnostics.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Platform dependency legitimacy approval | MEDIAIO-05 | Cargo-aware slopcheck is unavailable; package audit requires user confirmation | In 12-02B, verify `windows` and `objc2-*` or approved equivalents against crates.io/source repos, then approve or record replacements. |
| macOS native decode and texture compatibility | MEDIAIO-02, MEDIAIO-03, MEDIAIO-04, MEDIAIO-05 | Native VideoToolbox/CoreVideo/Metal behavior requires real macOS hardware and a local fixture | On macOS, run the native media tests with an H.264 MP4/MOV fixture, confirm capability report, first-frame lease, fallback diagnostics, texture compatibility, and leak diagnostics. |
| Windows native decode and texture compatibility | MEDIAIO-02, MEDIAIO-03, MEDIAIO-04, MEDIAIO-05 | Native Media Foundation/DXVA/D3D behavior requires real Windows hardware and a local fixture | On Windows, run the native media tests with an H.264 MP4/MOV fixture, confirm capability report, first-frame lease, fallback diagnostics, texture compatibility, and leak diagnostics. |
| Final platform evidence notes | MEDIAIO-05 | CI may skip native hardware paths; verification evidence must be recorded | In `12-06C-SUMMARY.md`, record OS, fixture, selected path, fallback reason, texture compatibility state, and session-close leak diagnostic outcome. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify commands or a blocking human checkpoint with an automated metadata command.
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify.
- [ ] Test scaffolds are created by the task that first introduces each behavior.
- [ ] No watch-mode flags.
- [ ] Comment-sensitive source guards filter comments before count-based gates.
- [ ] `nyquist_compliant: true` set in frontmatter.

Approval: pending
