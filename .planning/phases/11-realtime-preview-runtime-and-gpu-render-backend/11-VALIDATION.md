---
phase: 11
slug: realtime-preview-runtime-and-gpu-render-backend
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-18
---

# Phase 11 - Validation Strategy

> Per-phase validation contract for realtime preview runtime execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`; Playwright Electron workspace tests |
| **Config file** | `Cargo.toml`, `apps/desktop-electron/playwright.config.ts`, `package.json` |
| **Quick run command** | `cargo test -p realtime_preview_runtime -- --nocapture` |
| **Full suite command** | `pnpm run test:phase11` |
| **Estimated runtime** | ~180 seconds locally, excluding opt-in real GPU platform smoke |

---

## Sampling Rate

- **After every task commit:** Run the focused `<automated>` command from that task.
- **After every plan wave:** Run the plan `<verification>` block plus `cargo check --workspace --locked`.
- **Before `$gsd-verify-work`:** `pnpm run test:phase11` must be green.
- **Max feedback latency:** 300 seconds for default CI gates.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 11-01-01 | 01 | 1 | RTPREV-01/RTPREV-05 | T-11-01 | Clock uses integer microseconds and generation advances on timeline/runtime operations. | unit | `cargo test -p realtime_preview_runtime clock_generation -- --nocapture` | W0 | pending |
| 11-01-02 | 01 | 1 | RTPREV-01/RTPREV-05 | T-11-02/T-11-04 | Stale and canceled frame results are rejected before presentation and counted in telemetry. | unit | `cargo test -p realtime_preview_runtime stale_frame_rejection -- --nocapture`; `cargo test -p realtime_preview_runtime cancellation_telemetry -- --nocapture` | W0 | pending |
| 11-02-01 | 02 | 2 | RTPREV-01/RTPREV-04 | T-11-04 | Runtime graph preparation consumes Rust-owned normalized draft/render graph intent. | integration | `cargo test -p realtime_preview_runtime runtime_graph -- --nocapture` | W0 | pending |
| 11-02-02 | 02 | 2 | RTPREV-02/RTPREV-04 | T-11-05 | Support classifier rejects unsupported graph intent before GPU draw submission. | unit | `cargo test -p realtime_preview_runtime capability_classifier -- --nocapture` | W0 | pending |
| 11-03-01 | 03 | 3 | RTPREV-02/RTPREV-03 | T-11-07 | CPU frame provider validates image/static frame contracts. | unit | `cargo test -p realtime_preview_runtime frame_provider -- --nocapture` | W0 | pending |
| 11-03-02 | 03 | 3 | RTPREV-02/RTPREV-03 | T-11-09 | H.264 software frame provider initializes a session-owned CPU frame cache and preview requests read frames without per-frame FFmpeg artifact generation. | integration | `cargo test -p realtime_preview_runtime video_frame_provider -- --nocapture` | W0 | pending |
| 11-03B-01 | 03B | 4 | RTPREV-02 | T-11-03B-03 | `wgpu` device setup is mockable by default and backend selection is constrained to D3D12/Metal for production desktop targets. | unit/gpu-gated | `cargo test -p realtime_preview_runtime offscreen_compositor -- --nocapture` | W0 | pending |
| 11-03B-02 | 03B | 4 | RTPREV-02/RTPREV-03 | T-11-03B-02 | Canvas, image, and video CPU frames upload to GPU textures and render as graph-ordered quads. | unit/gpu-gated | `cargo test -p realtime_preview_runtime gpu_subset -- --nocapture` | W0 | pending |
| 11-04-01 | 04 | 5 | RTPREV-02/RTPREV-05 | T-11-10 | Native surface handles stay opaque and session-owned. | unit/platform-gated | `cargo test -p realtime_preview_runtime native_surface_contracts -- --nocapture` | W0 | pending |
| 11-04-02 | 04 | 5 | RTPREV-01/RTPREV-05 | T-11-12 | Node-API bindings expose only thin session/surface/request commands. | integration | `cargo test -p bindings_node realtime_preview_bindings -- --nocapture` | W0 | pending |
| 11-04B-01 | 04B | 6 | RTPREV-02/RTPREV-05 | T-11-13 | Electron main owns native handle acquisition; renderer sends only bounds and UI commands. | Playwright | `pnpm --filter @video-editor/desktop test:workspace -g "实时预览|native host"` | W0 | pending |
| 11-05-01 | 05 | 6 | RTPREV-01/RTPREV-03 | T-11-16 | Supported canvas/image/video graph states route to realtime backend and fake FFmpeg executor is not called per frame. | integration | `cargo test -p preview_service realtime_backend_no_ffmpeg -- --nocapture` | W0 | pending |
| 11-05-02 | 05 | 6 | RTPREV-03/RTPREV-05 | T-11-17 | Unsupported and canceled paths emit Rust-owned diagnostics without claiming realtime support. | integration | `cargo test -p preview_service fallback_ladder -- --nocapture`; `cargo test -p preview_service cancellation_telemetry -- --nocapture` | W0 | pending |
| 11-05B-01 | 05B | 7 | RTPREV-05 | T-11-19 | Renderer display model carries telemetry and cancellation state as data only. | unit/source | `pnpm --filter @video-editor/desktop test:workspace -g "telemetry"` | W0 | pending |
| 11-06-01 | 06 | 7 | RTPREV-02/RTPREV-04 | T-11-21 | Text parity gate either proves GPU text support or reports explicit unsupported diagnostics. | golden | `cargo test -p testkit realtime_preview_parity -- --nocapture` | W0 | pending |
| 11-07-01 | 07 | 8 | RTPREV-01..05 | T-11-24 | Source guards reject renderer-owned FFmpeg/render graph/GPU/cache/fallback/timeline semantics. | source guard | `bash scripts/phase11-source-guards.sh` | W0 | pending |
| 11-07-02 | 07 | 8 | RTPREV-01..05 | T-11-25 | Final root gate runs Rust, source guard, contract, and workspace smoke coverage. | integration | `pnpm run test:phase11` | W0 | pending |

---

## Wave 0 Requirements

- [ ] `crates/realtime_preview_runtime/tests/clock_generation.rs` - stubs for RTPREV-05.
- [ ] `crates/realtime_preview_runtime/tests/stale_frame_rejection.rs` - stubs for stale generation rejection.
- [ ] `crates/realtime_preview_runtime/tests/cancellation_telemetry.rs` - stubs for cancellation telemetry.
- [ ] `crates/realtime_preview_runtime/tests/runtime_graph.rs` - stubs for Rust-owned graph preparation.
- [ ] `crates/realtime_preview_runtime/tests/capability_classifier.rs` - stubs for supported/degraded/unsupported classification.
- [ ] `crates/realtime_preview_runtime/tests/video_frame_provider.rs` - H.264 fixture frame provider tests.
- [ ] `crates/realtime_preview_runtime/tests/gpu_subset.rs` - canvas/image/video CPU frame upload tests.
- [ ] `crates/realtime_preview_runtime/tests/offscreen_compositor.rs` - mock/offscreen compositor tests.
- [ ] `crates/preview_service/tests/realtime_backend_no_ffmpeg.rs` - supported path no-per-frame-FFmpeg test.
- [ ] `crates/preview_service/tests/fallback_ladder.rs` - fallback diagnostics test.
- [ ] `crates/preview_service/tests/cancellation_telemetry.rs` - cancellation propagation through preview service.
- [ ] `scripts/phase11-source-guards.sh` - ownership guard.
- [ ] `package.json` scripts `test:phase11-rust`, `test:phase11-source-guards`, `test:phase11-workspace`, `test:phase11`.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Windows D3D12 native surface smoke | RTPREV-02/RTPREV-03 | CI may not have Windows GPU desktop session. | On Windows, run `VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime real_wgpu_adapter -- --ignored --nocapture`, then run the Phase 11 Playwright native host smoke. |
| macOS Metal native surface smoke | RTPREV-02/RTPREV-03 | CI may not have macOS Metal desktop session. | On macOS, run `VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime real_wgpu_adapter -- --ignored --nocapture`, then run the Phase 11 Playwright native host smoke. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all missing test references.
- [x] No watch-mode flags.
- [x] Feedback latency < 300s for default CI gates.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** pending execution
