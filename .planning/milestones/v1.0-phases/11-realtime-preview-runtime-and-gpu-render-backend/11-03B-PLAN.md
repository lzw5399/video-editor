---
phase: "11-realtime-preview-runtime-and-gpu-render-backend"
plan: "03B"
type: execute
wave: 4
depends_on:
  - "11-02"
  - "11-03"
files_modified:
  - "crates/realtime_preview_runtime/Cargo.toml"
  - "crates/realtime_preview_runtime/src/lib.rs"
  - "crates/realtime_preview_runtime/src/gpu/mod.rs"
  - "crates/realtime_preview_runtime/src/gpu/device.rs"
  - "crates/realtime_preview_runtime/src/gpu/compositor.rs"
  - "crates/realtime_preview_runtime/src/gpu/pipelines.rs"
  - "crates/realtime_preview_runtime/src/gpu/surface.rs"
  - "crates/realtime_preview_runtime/src/gpu/texture_cache.rs"
  - "crates/realtime_preview_runtime/tests/gpu_subset.rs"
  - "crates/realtime_preview_runtime/tests/offscreen_compositor.rs"
autonomous: true
requirements:
  - RTPREV-02
  - RTPREV-03
  - RTPREV-05
user_setup: []
must_haves:
  truths:
    - "The realtime runtime has a `wgpu` device/offscreen compositor path targeting D3D12 on Windows and Metal on macOS through backend selection."
    - "Canvas background and image/video CPU frames from 11-03 upload into runtime-owned textures and render as graph-ordered textured quads."
    - "Default automated tests use mock/offscreen paths and do not require a physical GPU."
    - "Real D3D12/Metal adapter smoke remains opt-in manual/platform CI, not a default automated task gate."
  artifacts:
    - path: "crates/realtime_preview_runtime/src/gpu/device.rs"
      provides: "wgpu instance/adapter/device/queue setup"
    - path: "crates/realtime_preview_runtime/src/gpu/compositor.rs"
      provides: "offscreen canvas and textured quad compositor"
    - path: "crates/realtime_preview_runtime/src/gpu/surface.rs"
      provides: "offscreen target abstraction for Phase 11 tests"
    - path: "crates/realtime_preview_runtime/src/gpu/texture_cache.rs"
      provides: "CPU frame to runtime texture upload/cache"
  key_links:
    - from: "crates/realtime_preview_runtime/src/frame_provider.rs"
      to: "crates/realtime_preview_runtime/src/gpu/texture_cache.rs"
      via: "CPU RGBA frames become runtime textures"
      pattern: "CpuRgba"
    - from: "crates/realtime_preview_runtime/src/capabilities.rs"
      to: "crates/realtime_preview_runtime/src/gpu/compositor.rs"
      via: "only supported graph states reach compositor"
      pattern: "RealtimePreviewGraphSupport::Supported"
---

<objective>
Implement the Phase 11 `wgpu` offscreen device/compositor subset on top of the frame provider contracts from 11-03.

Purpose: make RTPREV-02 executable through `wgpu` for canvas, image, video frame, transform, opacity, and sampled keyframe state without FFmpeg preview rendering.
Output: GPU dependency wiring, mockable device/offscreen target, texture upload/cache, compositor pipeline, and deterministic offscreen tests.
</objective>

<execution_context>
@/Users/zhiwen/.codex/get-shit-done/workflows/execute-plan.md
@/Users/zhiwen/.codex/get-shit-done/templates/summary.md
</execution_context>

<context>
@AGENTS.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/STATE.md
@.planning/notes/production-editor-architecture-decisions.md
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-CONTEXT.md
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-RESEARCH.md
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-DESIGN.md
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-03-SUMMARY.md
@crates/realtime_preview_runtime/src/frame_provider.rs
@crates/realtime_preview_runtime/src/capabilities.rs
@crates/render_graph/src/graph.rs
</context>

## Artifacts this plan produces

- `RealtimePreviewGpuDevice`
- `RealtimePreviewGpuTarget`
- `RealtimePreviewCompositor`
- `RealtimePreviewTextureCache`
- offscreen render tests for canvas and textured quads
- opt-in manual `VIDEO_EDITOR_TEST_WGPU=1` real adapter smoke

<tasks>

<task type="auto" tdd="true">
  <name>Task 11-03B-01: Add `wgpu` device bootstrap and offscreen target</name>
  <files>crates/realtime_preview_runtime/Cargo.toml, crates/realtime_preview_runtime/src/lib.rs, crates/realtime_preview_runtime/src/gpu/mod.rs, crates/realtime_preview_runtime/src/gpu/device.rs, crates/realtime_preview_runtime/src/gpu/surface.rs, crates/realtime_preview_runtime/tests/offscreen_compositor.rs</files>
  <read_first>
    - `crates/realtime_preview_runtime/Cargo.toml`
    - `crates/realtime_preview_runtime/src/frame_provider.rs`
    - `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-RESEARCH.md`
    - `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-DESIGN.md`
  </read_first>
  <action>Add approved `wgpu`, `raw-window-handle`, and `pollster` dependencies per the Package Legitimacy Audit. Implement backend selection with D3D12 on Windows, Metal on macOS, `OffscreenOnly`, and `Mock` modes. Add an offscreen texture target with explicit width, height, scale factor millis, and format. Default tests must use mock/offscreen paths that do not require a real graphics adapter. Real adapter/device smoke must be implemented as an ignored/manual test documented in 11-VALIDATION and 11-07, not as a default automated task command.</action>
  <acceptance_criteria>
    Mock/offscreen tests pass in CI without a physical GPU; backend selection never picks unsupported desktop backends for the Phase 11 supported path; ignored real GPU smoke exists but is not run by default automated verification.
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p realtime_preview_runtime offscreen_compositor -- --nocapture</automated>
    <manual>VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime real_wgpu_adapter -- --ignored --nocapture</manual>
  </verify>
  <done>Task complete when GPU device setup is mockable, offscreen targets work in default tests, and real adapter tests are documented as opt-in platform gates.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 11-03B-02: Render canvas and textured quad subset through compositor</name>
  <files>crates/realtime_preview_runtime/src/gpu/compositor.rs, crates/realtime_preview_runtime/src/gpu/pipelines.rs, crates/realtime_preview_runtime/src/gpu/texture_cache.rs, crates/realtime_preview_runtime/tests/gpu_subset.rs</files>
  <read_first>
    - `crates/render_graph/src/graph.rs`
    - `crates/realtime_preview_runtime/src/capabilities.rs`
    - `crates/realtime_preview_runtime/src/frame_provider.rs`
    - `crates/realtime_preview_runtime/src/software_video_provider.rs`
  </read_first>
  <action>Implement the supported Phase 11 compositor subset: draw black/solid canvas background, upload CPU RGBA/static image and software video frames into textures, draw visual layers in render graph stack order as textured quads, and apply engine/render-graph supplied position, scale, opacity, and supported fit/fill/stretch values. Use sampled keyframe state from the graph; do not interpolate keyframes in GPU code. Emit diagnostics and fallback reasons when frame input, transform, target dimensions, or support classification prevents GPU render.</action>
  <acceptance_criteria>
    Tests prove a solid canvas produces deterministic pixels, a CPU RGBA image layer and a software video frame layer appear in expected quadrant/stack order, opacity affects output alpha/color, unsupported graph intent does not enter draw submission, and no FFmpeg executor or compiler type is imported by the GPU module.
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p realtime_preview_runtime gpu_subset -- --nocapture</automated>
    <automated>cargo test -p realtime_preview_runtime -- --nocapture</automated>
  </verify>
  <done>Task complete when the compositor renders the supported subset through mock/offscreen coverage and rejects unsupported intent before draw submission.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| frame provider -> GPU upload | Material-derived pixel buffers cross into GPU memory. |
| capability classifier -> compositor | Only supported graph state should reach draw submission. |
| GPU adapter/device -> runtime session | Backend/device failure must not crash the editor or masquerade as supported preview. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-11-03B-01 | Denial of service | CPU frame upload | mitigate | Validate dimensions and pixel length before texture upload; return typed provider errors from 11-03. |
| T-11-03B-02 | Tampering | GPU compositor input | mitigate | Accept only render_graph/engine resolved state; never evaluate keyframes or timeline semantics inside GPU modules. |
| T-11-03B-03 | Denial of service | adapter/device initialization | mitigate | Report `NoGpuAdapter`/backend diagnostics and keep real GPU smoke opt-in via platform gate. |
| T-11-SC | Tampering | `wgpu`, `raw-window-handle`, `pollster` installs | mitigate | Packages are approved in `11-RESEARCH.md` Package Legitimacy Audit; no additional GPU/text package may be added without that audit. |
</threat_model>

<verification>
<automated>cargo test -p realtime_preview_runtime offscreen_compositor -- --nocapture</automated>
<automated>cargo test -p realtime_preview_runtime gpu_subset -- --nocapture</automated>
<automated>cargo check --workspace --locked</automated>
<manual>VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime real_wgpu_adapter -- --ignored --nocapture</manual>
</verification>

<source_audit>
GOAL | Phase 11 | `wgpu` rendering path for supported interactive timeline subset | 11-03B | COVERED
REQ | RTPREV-02 | canvas, video/image visual layer, transform, opacity, keyframe state through `wgpu` with diagnostics | 11-03B | COVERED
REQ | RTPREV-03 | supported compositor path has no per-frame FFmpeg execution | 11-03B | COVERED
REQ | RTPREV-05 | compositor results carry telemetry/generation from shared contracts | 11-03B | COVERED
CONTEXT | CTX-RuntimeChoice | Rust `RealtimePreviewRuntime` plus `wgpu` primary supported realtime preview path | 11-03B | COVERED
RESEARCH | Resolved Surface Rollout | offscreen/mock first for Rust tests, native Windows/macOS child surface in later Phase 11 wave | 11-03B | COVERED
</source_audit>

<success_criteria>
The realtime runtime can render the Phase 11 supported visual subset into an offscreen `wgpu` target using CPU/static image and H.264 software video frame inputs, while default tests remain deterministic and real GPU tests are opt-in manual/platform gates.
</success_criteria>

<output>
Create `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-03B-SUMMARY.md` when done.
</output>
