# Phase 11: Realtime Preview Runtime And GPU Render Backend - Design

**Designed:** 2026-06-18  
**Status:** Proposed for planning  
**Scope:** Rust realtime preview runtime, GPU compositor backend, Electron desktop preview surface, telemetry, fallback, and parity diagnostics.

## Design Goal

Phase 11 adds a production preview runtime path without changing canonical draft semantics. The runtime consumes accepted draft snapshots and render graph intent from Rust-owned layers, renders supported states with `wgpu`, rejects stale frames with `PlaybackGeneration`, and falls back through the existing preview artifact path only when capability classification requires it.

Do not implement Phase 12 native hardware decode, Phase 15 audio output, Phase 16 full scheduler, or Phase 18 complex effects in this phase.

## Proposed Crates And Modules

### `crates/realtime_preview_runtime`

Owns GPU preview state and renderer-independent preview session APIs.

```text
crates/realtime_preview_runtime/src/
  lib.rs
  clock.rs
  session.rs
  request.rs
  telemetry.rs
  diagnostics.rs
  capabilities.rs
  frame_provider.rs
  fallback.rs
  gpu/
    mod.rs
    device.rs
    compositor.rs
    pipelines.rs
    surface.rs
    texture_cache.rs
    text.rs
  platform/
    mod.rs
    windows.rs
    macos.rs
```

Core dependencies:

- `draft_model`, `engine_core`, `render_graph`
- `wgpu`, `raw-window-handle`, `serde`
- `glyphon` only if text parity gates are included in the implementation wave
- target dependencies for native child surface code: `windows-sys` on Windows, `objc2-app-kit` on macOS

### Existing Crates To Extend

`preview_service` remains the fallback coordinator and existing artifact/cache owner. Add a backend trait or wrapper so supported preview requests route to `RealtimePreviewRuntime`, while unsupported/no-adapter cases route to current `request_preview_frame` behavior.

`bindings_node` exposes preview session operations through Node-API. It should stay thin: create/close session, attach/detach surface, update bounds, seek/render, play/pause, and query telemetry.

`render_graph` may need small diagnostic additions only. Avoid moving GPU-specific types into `render_graph`; keep it renderer-neutral.

`media_runtime` should not become a hardware decode implementation in Phase 11. Add only generic frame input contracts if needed, such as `CpuVideoFrame` and `PreviewFrameProvider`, unless those fit better in `realtime_preview_runtime`.

## Ownership Boundaries

| Layer | Owns | Must Not Own |
|-------|------|--------------|
| Electron renderer | Controls, layout rect, transport button events, displayed telemetry | Draft mutation, render graphs, GPU command lists, FFmpeg selection, cache keys |
| Electron main/preload | Native window handle acquisition, IPC bridge, safe command routing | Preview composition semantics |
| `bindings_node` | Thin session API mapping and opaque IDs | GPU rendering logic or timeline math |
| `realtime_preview_runtime` | GPU device/surface, compositor, clock/generation, support classification, telemetry | Draft command semantics, media hardware decode, audio output, export compilation |
| `engine_core` | Normalized draft and frame/range state | Runtime surfaces, FFmpeg, GPU |
| `render_graph` | Renderer-neutral intent | `wgpu`, OS handles, FFmpeg process execution |
| `preview_service` | Artifact fallback and cache invalidation | Primary supported realtime path semantics |

## Public API Shape

### IDs And Clock

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PreviewSessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlaybackGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PreviewCancellationToken(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Stopped,
    Paused,
    Playing,
    Scrubbing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackRate {
    pub numerator: i32,
    pub denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineClock {
    pub position: Microseconds,
    pub frame_rate: RationalFrameRate,
    pub playback_rate: PlaybackRate,
    pub state: PlaybackState,
    pub generation: PlaybackGeneration,
}
```

Generation rules:

- Increment on seek, scrub start/commit, play, pause, resume, stop, accepted edit, draft reload, material relink, surface detach, and runtime reset.
- Every queued render/fallback result includes the generation it was requested with.
- Present only when `result.generation == session.clock.generation`.
- Count rejected frames in telemetry.

### Session API

```rust
pub struct RealtimePreviewRuntime {
    // registry of sessions, adapter/device policy, fallback hooks
}

pub struct RealtimePreviewSessionConfig {
    pub session_label: String,
    pub preferred_backend: PreviewGpuBackend,
    pub max_surface_width: u32,
    pub max_surface_height: u32,
    pub enable_text_gpu_path: bool,
}

pub enum PreviewGpuBackend {
    Auto,
    D3d12,
    Metal,
    OffscreenOnly,
    Mock,
}

impl RealtimePreviewRuntime {
    pub fn create_session(
        &mut self,
        config: RealtimePreviewSessionConfig,
    ) -> Result<PreviewSessionId, RealtimePreviewError>;

    pub fn close_session(&mut self, session_id: PreviewSessionId);

    pub fn update_draft_snapshot(
        &mut self,
        session_id: PreviewSessionId,
        draft: Draft,
    ) -> Result<PlaybackGeneration, RealtimePreviewError>;

    pub fn seek(
        &mut self,
        session_id: PreviewSessionId,
        target_time: Microseconds,
    ) -> Result<PlaybackGeneration, RealtimePreviewError>;

    pub fn request_frame(
        &mut self,
        session_id: PreviewSessionId,
        request: RealtimePreviewFrameRequest,
    ) -> Result<RealtimePreviewFrameResult, RealtimePreviewError>;
}
```

### Surface API

```rust
pub struct PreviewSurfaceBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor_millis: u32,
}

pub enum PreviewSurfaceDescriptor {
    NativeChild {
        parent_window_handle: NativeParentWindowHandle,
        bounds: PreviewSurfaceBounds,
    },
    Offscreen {
        width: u32,
        height: u32,
        scale_factor_millis: u32,
    },
}

pub enum NativeParentWindowHandle {
    WindowsHwnd(u64),
    MacosNsView(u64),
}
```

Only Electron main should acquire `BrowserWindow.getNativeWindowHandle()` and pass it to the binding. The renderer should report the preview rect and receive telemetry/display state, not raw OS handles.

### Render Request/Result

```rust
pub struct RealtimePreviewFrameRequest {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub cancellation_token: Option<PreviewCancellationToken>,
    pub mode: PreviewRequestMode,
}

pub enum PreviewRequestMode {
    Seek,
    Scrub,
    PlaybackTick,
    FirstFrame,
}

pub struct RealtimePreviewFrameResult {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub presented: bool,
    pub stale_rejected: bool,
    pub canceled: bool,
    pub cancellation_token: Option<PreviewCancellationToken>,
    pub backend: RealtimePreviewBackendUsed,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
    pub telemetry: RealtimePreviewTelemetry,
}
```

## Frame Provider Boundary

Phase 11 needs an input abstraction without implementing full media IO.

```rust
pub trait PreviewFrameProvider {
    fn provider_name(&self) -> &'static str;

    fn frame_for(
        &mut self,
        material_id: &MaterialId,
        source_position: Microseconds,
        generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError>;
}

pub enum PreviewFrameInput {
    CpuRgba(CpuVideoFrame),
    StaticImage(CpuVideoFrame),
    FutureTextureHandle(TextureHandleDescriptor),
    Unavailable { reason: String },
}

pub struct CpuVideoFrame {
    pub width: u32,
    pub height: u32,
    pub stride_bytes: u32,
    pub color: FrameColorInfo,
    pub pixels: Vec<u8>,
}

pub struct TextureHandleDescriptor {
    pub handle_id: u64,
    pub owner_generation: PlaybackGeneration,
    pub backend: PreviewGpuBackend,
}
```

Phase 11 implementation must support images, static frames, and generated H.264 MP4/MOV video material frames through a session-owned software CPU frame cache. Fixture generation or cache initialization may use existing testkit/desktop media utilities, but `request_frame`, seek, scrub, and playback-tick requests for supported timeline states must not spawn FFmpeg per frame or route through preview artifact generation. `FutureTextureHandle` is intentionally a descriptor placeholder for Phase 12; do not implement platform hardware decode or D3D/Metal texture import in this phase.

## GPU Compositor Design

The compositor should render into either a configured native surface or an offscreen texture.

Pipeline stages:

1. Prepare `EngineProfile` from draft canvas.
2. Normalize draft.
3. Resolve a single-frame `RenderRangeState` for target time.
4. Build `RenderGraph`.
5. Classify graph support against runtime capabilities.
6. Acquire/upload source textures through `PreviewFrameProvider`, including H.264 material frames from the session-owned software frame cache.
7. Draw canvas background.
8. Draw visual layers in graph stack order.
9. Apply position/scale/opacity and supported fit/fill/stretch transforms.
10. Draw text overlays if GPU text path is enabled and parity-supported.
11. Present to native surface or copy offscreen fallback output.
12. Emit telemetry and diagnostics.

Initial shader scope:

- Solid color full-screen quad.
- Textured quads with alpha.
- Transform matrix from engine-resolved integer/millis values.
- Nearest/linear sampling policy documented per preview profile.
- No custom effects, masks, transitions, or advanced color management in Phase 11.

## Capability And Diagnostic Types

```rust
pub enum RealtimePreviewSupport {
    Supported,
    Degraded { reason: String },
    Unsupported { reason: String },
}

pub struct RealtimePreviewDiagnostic {
    pub entity_id: Option<String>,
    pub domain: RealtimePreviewDiagnosticDomain,
    pub support: RealtimePreviewSupport,
    pub reason: String,
    pub fallback_used: bool,
}

pub enum RealtimePreviewDiagnosticDomain {
    Canvas,
    MaterialFrame,
    VisualLayer,
    Transform,
    Text,
    Keyframe,
    Effect,
    Surface,
    Runtime,
}
```

Diagnostic policy:

- Do not silently fake unsupported graph intent.
- Preserve existing `RenderIntentSupport` diagnostics and add realtime-specific reasons.
- A fallback can still be successful, but the result must say it was fallback.
- Parity diagnostics should be serializable for test snapshots.

## Telemetry

```rust
pub struct RealtimePreviewTelemetry {
    pub first_frame_latency_ms: Option<u64>,
    pub seek_latency_ms: Option<u64>,
    pub queue_latency_ms: u64,
    pub render_duration_ms: u64,
    pub presented_frame_count: u64,
    pub dropped_frame_count: u64,
    pub repeated_frame_count: u64,
    pub stale_rejected_count: u64,
    pub canceled_request_count: u64,
    pub fallback_count: u64,
    pub cache_hit_count: u64,
    pub target_time: Microseconds,
    pub generation: PlaybackGeneration,
}
```

Phase 11 should collect telemetry in memory per session and expose a binding query such as `getRealtimePreviewTelemetry(sessionId)`. Persisting telemetry or building a global profiler belongs in Phase 16.

## Electron Integration

### Main Process

Add a main-process preview host service:

- Create a preview runtime session after the editor window is ready.
- Call `BrowserWindow.getNativeWindowHandle()` in main, not renderer.
- Track preview canvas bounds from renderer messages.
- Attach/detach/update native child surface through bindings.
- Forward preview commands with session ID, target microseconds, and generation.
- Destroy preview session before BrowserWindow close.

### Renderer

Renderer changes should stay UI-only:

- Reserve a stable `.preview-native-host` box inside the preview monitor.
- Send rect/scale updates to main when layout changes.
- Continue sending seek/play commands through typed helpers.
- Display runtime diagnostics and telemetry in Chinese UI copy.
- Use image fallback display only when runtime reports offscreen/artifact fallback.

### Binding Commands

Add command routes or direct binding APIs:

- `createRealtimePreviewSession`
- `closeRealtimePreviewSession`
- `attachRealtimePreviewSurface`
- `updateRealtimePreviewSurfaceBounds`
- `updateRealtimePreviewDraftSnapshot`
- `seekRealtimePreview`
- `requestRealtimePreviewFrame`
- `getRealtimePreviewTelemetry`

Keep opaque session IDs in Rust. Do not expose GPU device pointers, command encoders, surface internals, or frame cache keys to TypeScript.

## Fallback Design

Fallback decisions happen in Rust:

```text
request frame
  -> generation check
  -> build graph
  -> classify graph
  -> if supported and surface/device/frame input available: GPU render
  -> else if offscreen GPU available: offscreen render + copy fallback
  -> else if preview artifact cache hit: return artifact fallback
  -> else: run existing preview_service FFmpeg fallback and mark fallback reason
```

Fallback reasons:

- `NoGpuAdapter`
- `SurfaceUnavailable`
- `SurfaceLost`
- `UnsupportedGraphIntent`
- `FrameProviderUnavailable`
- `TextParityUnsupported`
- `NativeChildWindowFailed`
- `OffscreenReadbackRequired`
- `PreviewArtifactCacheHit`
- `FfmpegArtifactGenerated`

Supported Phase 11 paths must not call FFmpeg per frame. Tests should use a fake `FfmpegExecutor` that fails if called for a supported graph.

## Avoiding Phase 12/15/16 Blocking

### Phase 12 Media IO

Do:

- Define frame inputs as `CpuVideoFrame` and future `TextureHandleDescriptor`.
- Keep color metadata and lifetimes explicit.
- Keep texture handle IDs opaque.

Do not:

- Implement Media Foundation, DXVA, AVFoundation, VideoToolbox, CoreVideo, or Metal texture import in Phase 11.
- Tie `TextureHandleDescriptor` to one platform-specific concrete pointer shape yet.

### Phase 15 Audio

Do:

- Make `TimelineClock` and `PlaybackGeneration` independent of video rendering.
- Include playback rate and state.
- Ensure generation increments on operations that will later invalidate audio buffers.

Do not:

- Add WASAPI/CoreAudio output, DSP graph, or audio master clock.
- Make video runtime the permanent clock owner; it is a clock participant.

### Phase 16 Scheduler

Do:

- Carry target timeline microseconds, generation, request mode, cancellation token, and queue latency in every request/result.
- Implement a minimal single-session queue with latest-request-wins behavior for scrub.

Do not:

- Build global priority queues, thread pool isolation, artifact scheduling, export scheduling, or resource budget policy.

## Rollout Waves

### Wave 0: Crate Shell, Clock, Contracts

- Add `realtime_preview_runtime` crate to workspace.
- Add clock/generation/request/result/telemetry/diagnostic types.
- Add mock runtime tests for generation increment and stale rejection.
- Add source guard plan for renderer ownership boundaries.

### Wave 1: Render Graph Preparation And Capability Classifier

- Add helper that prepares normalized draft, one-frame render range, and graph from a target time.
- Add support matrix for canvas, image/video layer, opacity, transform, text, and unsupported graph intent.
- Add parity diagnostic snapshots.

### Wave 2: `wgpu` Device And Offscreen Renderer

- Initialize `wgpu` adapter/device/queue.
- Add offscreen surface/texture target and mockable compositor.
- Add H.264 fixture-backed software video frame provider and session-owned CPU frame cache.
- Render black/solid canvas and textured quads from image and video CPU frame inputs.
- Add tests that can run with mock backend by default and real GPU only when environment allows.

### Wave 3: Desktop Native Surface Embedding

- Add Windows child HWND host.
- Add macOS child NSView host.
- Add binding APIs for attach/detach/update bounds.
- Add Electron main/renderer surface rect bridge.
- Add Playwright smoke that verifies command routing, non-overlap layout, fallback state, and telemetry display.

### Wave 4: Runtime/Fallback Integration

- Integrate realtime backend into preview request flow.
- Keep current preview artifact path as fallback.
- Add fake FFmpeg executor tests proving supported canvas/image/video paths do not spawn FFmpeg per frame.
- Add fallback ladder diagnostics.

### Wave 5: Text, Parity, And Final Gates

- Add GPU text path if `glyphon` parity is acceptable; otherwise classify text fallback explicitly.
- Add preview/export parity diagnostics for golden drafts.
- Add final source guards and root phase script.
- Update docs/runtime boundaries with Phase 11 runtime boundary.

## Testing Strategy

### Rust Unit Tests

- `clock_generation_increments_on_seek_play_pause_edit`
- `stale_generation_result_is_rejected`
- `frame_index_uses_rational_frame_rate`
- `capability_classifier_marks_supported_canvas_image_opacity`
- `capability_classifier_marks_effects_masks_transitions_unsupported`
- `fallback_reason_serializes_for_binding`

### Rust Integration Tests

- `supported_graph_does_not_call_ffmpeg_executor`
- `h264_material_frame_provider_serves_cpu_frames_without_per_frame_ffmpeg`
- `unsupported_graph_routes_to_preview_service_fallback`
- `realtime_and_export_share_render_graph_snapshot`
- `telemetry_records_first_frame_seek_fallback_stale`

### GPU Tests

- Default CI: mock/offscreen backend tests only.
- Opt-in local: real `wgpu` adapter test behind environment flag such as `VIDEO_EDITOR_TEST_WGPU=1`.
- Platform compile/smoke: `cargo check` must cover platform-gated native surface modules on the current target; Windows D3D12 and macOS Metal native surface attach, resize, and present are required manual/CI platform gates before closing Phase 11.

### Electron/Playwright Tests

- Workspace still has five stable regions at 1280x800 and 1120x720.
- Preview host rect is non-zero and does not overlap timeline/inspector.
- Seek sends integer microsecond target and generation.
- Runtime telemetry appears after first frame/seek.
- Fallback image display appears when native surface is unavailable.
- Source guards reject renderer-owned `wgpu`, render graph construction, FFmpeg fallback selection, and direct cache key logic.

## Source Guard Additions

Add a Phase 11 guard script that rejects:

- `wgpu`, `GPUDevice`, `GPUCanvasContext`, or WebGPU renderer ownership in `apps/desktop-electron/src/renderer`.
- `build_render_graph`, `compile_ffmpeg_job`, `FfmpegExecutor`, cache key construction, or GPU command list construction in renderer code.
- Floating point persisted time names in generated contracts or runtime request types.
- Direct draft track/segment mutation in preview UI.

Allow:

- DOM measurement APIs in renderer for preview rect.
- Main-process native handle acquisition.
- Binding route names and generated TypeScript command/result types.

## Resolved Planning Decisions

1. Native child surface work is split by responsibility, not by platform: Rust/native surface contracts and Node-API bindings are planned separately from Electron main/preload/renderer bridge work. Windows and macOS platform modules remain in the same Rust-native plan because they share the same surface contract and validation tests.
2. Text GPU rendering is parity-gated. Phase 11 may implement a GPU text path only when deterministic parity tests pass; otherwise text is explicitly classified as degraded or unsupported and routed through the Rust-owned fallback diagnostics. Silent approximate GPU text rendering is not an accepted outcome.
3. `PreviewFrameProvider` lives in `realtime_preview_runtime` for Phase 11. It supports image/static CPU frames, generated H.264 video material frames from a session-owned software frame cache, and future texture descriptor placeholders. Native hardware decode and texture interop remain Phase 12 scope.
4. First-frame and seek latency budgets are measurement gates in Phase 11, not hard pass/fail thresholds. Phase 11 must record first-frame latency, seek latency, frame pacing, dropped/repeated frames, stale-generation rejection, cancellation, fallback, and cache-hit telemetry so later phases can set production budgets from observed baselines.
5. Offscreen fallback is a Rust-owned diagnostic/display path and must not become the production success path. It may return an existing preview artifact/display reference for UI inspection; full-resolution raw BGRA frame transport through JS is not accepted for the production path.

## Definition Of Done

- `RealtimePreviewRuntime` exists as a Rust-owned runtime with session APIs.
- Supported Phase 11 graph subset renders through `wgpu` or a mock/offscreen backend in tests.
- Supported seek/scrub path does not invoke FFmpeg per frame.
- Native desktop surface path is implemented or explicitly falls back with diagnostics on both Windows and macOS.
- `TimelineClock` and `PlaybackGeneration` are shared request/result types.
- Stale frame rejection is tested.
- Telemetry includes first-frame, seek, pacing/drop/repeat, stale rejection, cancellation/fallback/cache counters.
- Preview/export parity diagnostics exist for golden drafts.
- Renderer remains UI-only under source guards.
