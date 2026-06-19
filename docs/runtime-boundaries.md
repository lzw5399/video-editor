# Runtime Boundaries

Phase 1 establishes the service-boundary shape for runtime, filesystem, preview,
and test harness integration. It does not implement product editing behavior,
packaged runtime management, mobile backends, server rendering, or hardware
encoder selection.

## Trait Placement

Platform traits live at the consuming service boundary:

- `media_runtime::FfmpegExecutor` owns the FFmpeg and ffprobe process execution
  boundary.
- `project_store::PlatformFileSystem` owns filesystem access for `.veproj`
  project bundle persistence.
- `preview_service::PreviewRenderer` reserves the future preview rendering
  boundary for frames, segments, thumbnails, waveform cache, and invalidation.

There is no generic `platform` crate. Electron, future iOS, future Android, and
future server backends should inject implementations at the app shell or service
boundary rather than leaking platform traits into semantic crates.

## Pure Semantic Crates

`draft_model`, `draft_commands`, and `engine_core` must remain pure semantic
crates. They may define draft/material/track/segment/time concepts and editing
semantics, but they must not depend on:

- `media_runtime::FfmpegExecutor`
- `project_store::PlatformFileSystem`
- `preview_service::PreviewRenderer`
- OS process execution details
- Electron, mobile, server, or filesystem runtime abstractions

Render graph and FFmpeg compiler crates also stay separated from process
execution: render semantics compile into plans, while runtime crates execute
jobs and report progress or errors.

## FFmpeg Scope In Phase 1

Phase 1 discovers local FFmpeg and ffprobe binaries through explicit
configuration and the host environment only:

- `VE_FFMPEG_PATH`
- `VE_FFPROBE_PATH`
- `PATH`

Phase 1 does not download FFmpeg, does not install FFmpeg, does not bundle
FFmpeg, and does not redistribute FFmpeg. Because this phase does not ship or
redistribute FFmpeg binaries, it also does not perform FFmpeg distribution
license review, third-party notice generation, or LGPL/GPL/nonfree build-option
selection.

If a later packaged desktop app, mobile app, server renderer, or release process
distributes FFmpeg binaries, that later work must review license posture,
notices, source-offer obligations, build flags, and commercial product
constraints before shipping.

## Desktop Runtime

`media_runtime_desktop::DesktopFfmpegExecutor` is the desktop implementation
shell for `media_runtime::FfmpegExecutor`. It represents the Electron desktop
backend injection point.

`media_runtime` already owns FFmpeg/ffprobe discovery, version probes,
structured missing-binary and probe errors, checked paths, bounded stdout/stderr
summaries, and command/payload-safe runtime probe contracts. Desktop execution
uses argument-array process launches through `DesktopFfmpegExecutor`; process
waits are bounded so renderer-triggered probes and testkit smoke runs cannot
hang indefinitely.

Deferred runtime work includes packaged binary management, per-job cancellation,
progress streams, app-level timeout policy, and license review for any later
redistributed FFmpeg build.

## Project Store Runtime

`project_store::StdPlatformFileSystem` is the standard desktop filesystem shell
for `.veproj` persistence. `.veproj/project.json` remains the canonical source of
truth. Render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files,
preview caches, and exported videos are derived artifacts.

`project_store` classifies and resolves material URIs for `.veproj` bundles, but
it does not import materials, assign material IDs, mutate the material registry,
run ffprobe, or decide editing behavior. Binding-facing command/API services
coordinate those operations by combining `project_store` path helpers,
`media_runtime` probing, pure `draft_model` registry helpers, draft validation,
and project-bundle saves.

Material import persists normalized semantic material fields only: stable ID,
URI, display name, material kind, duration, dimensions, rational frame rate,
stream flags, audio sample rate/channel count, status, and bounded probe error
text. Thumbnails, waveform data, raw probe JSON, preview caches, render graphs,
FFmpeg scripts, proxy files, and export outputs are derived artifacts outside
`.veproj/project.json`.

Missing local materials are recoverable draft state. The material entry remains
in the draft with its original URI and status, while binding-facing services
return classified diagnostics with last-known resolved path details for future
relink UI.

## Preview Runtime

`preview_service::PreviewRenderer` is boundary-only in Phase 1. It reserves where
future preview frame and segment generation can be injected without letting
preview runtime concerns enter draft or timeline semantics.

## Phase 11 Realtime Preview Runtime

Phase 11 promotes realtime preview from a boundary placeholder into a Rust-owned
runtime path. Rust-owned session, clock, generation, capability classification,
telemetry, and GPU composition stay in `realtime_preview_runtime` and
`preview_service`; Electron/React remains the desktop UI shell.

### Ownership Map

| Layer | Owns | Does Not Own |
|-------|------|--------------|
| Electron renderer | UI controls, DOM measurement, preview host rectangle reporting, Chinese telemetry display | FFmpeg commands, render graphs, GPU devices, GPU command lists, cache keys, dirty ranges, fallback selection, timeline mutation, keyframe evaluation |
| Electron main/preload | `BrowserWindow.getNativeWindowHandle()` acquisition, safe IPC routing, integer host bounds forwarding | Preview composition semantics, fallback decisions, graph interpretation |
| `bindings_node` | Thin JSON/Node-API route and type mapping, opaque session IDs | GPU rendering logic, native handle exposure to renderer, timeline math |
| `realtime_preview_runtime` | `TimelineClock`, `PlaybackGeneration`, sessions, `wgpu` device/surface/offscreen targets, compositor, diagnostics, telemetry | Draft command mutation, FFmpeg export compilation, hardware decode, audio output, priority scheduling |
| `preview_service` | Supported realtime routing and frame provider/cache boundaries | Renderer UI, primary GPU composition internals, export behavior decisions |
| `engine_core` / `render_graph` | Accepted draft normalization, integer-microsecond frame state, renderer-neutral graph intent | `wgpu`, OS handles, FFmpeg process execution |

Renderer responsibilities are UI-only. It may measure the
`.preview-native-host` rectangle with `getBoundingClientRect`, send rounded
integer bounds and scale millis through preload, and display Simplified Chinese
status/telemetry returned from main/Rust. It must not construct FFmpeg commands,
render graphs, GPU command lists, cache keys, dirty ranges, fallback ladders, or
timeline/keyframe semantics.

### Runtime Inputs And No-Fallback Product Policy

Realtime preview consumes accepted draft snapshots, engine-resolved frame state,
and renderer-neutral `RenderGraph` intent from Rust-owned layers. Supported
seek, scrub, first-frame, and playback-tick requests use the
`RealtimePreviewRuntime` path and report `TimelineClock` plus
`PlaybackGeneration` telemetry so stale results are rejected before presentation.

Product realtime preview follows [No Product Fallback Policy](no-product-fallback-policy.md):
normal playback must not report success through mock output, preview PNG loops,
preview artifacts, FFmpeg artifacts, FFmpeg CPU decoded fingerprints, offscreen
readback, or synthetic DOM/frame-token evidence. If the true
GPU/native-texture/composited/present path is unavailable, the product must fail
closed with a clear unavailable diagnostic.

Low-level capability reports may still name fallback reasons to explain why a
path is unavailable. They must not continue product playback or satisfy product
E2E evidence.

### Downstream Phase Exclusions

- Phase 12 owns platform-native media IO and hardware decode: Windows Media Foundation/DXVA/D3D texture interop and macOS AVFoundation/VideoToolbox/CoreVideo/Metal texture interop.
- Phase 15 owns realtime audio, audio output, and audio/video synchronization on
  the shared `TimelineClock`.
- Phase 16 owns priority scheduling, queue fairness, background jobs, and full
  cancellation policy beyond the minimal generation/cancel request fields used
  in Phase 11.
- Phase 18 owns complex effects, retiming, filters, masks, and transitions with explicit supported/degraded/unsupported matrices.

### Phase 11 Gate Scripts

The root Phase 11 gate is:

```bash
pnpm run test:phase11
```

It composes:

- `pnpm run test:phase11-rust`
- `pnpm run test:phase11-source-guards`
- `pnpm run test:phase11-workspace`
- `pnpm run test:contracts`

`test:phase11-source-guards` blocks renderer-owned FFmpeg, render graph, GPU
command, cache key, dirty range, fallback, timeline mutation, keyframe
evaluation, and floating-point persisted timeline request fields while allowing
DOM measurement, Chinese telemetry display, main-process handle acquisition, and
binding route/type names.

### Manual Platform Smoke

These platform smokes are required before declaring a release build ready, but
they are not run by default in CI because they require real desktop GPU adapters
and native surfaces.

Windows D3D12:

```bash
VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime real_wgpu_adapter -- --ignored --nocapture
pnpm --filter @video-editor/desktop test:workspace -g "实时预览 native preview host rectangle reports integer bounds and telemetry"
```

macOS Metal:

```bash
VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime real_wgpu_adapter -- --ignored --nocapture
pnpm --filter @video-editor/desktop test:workspace -g "实时预览 native preview host rectangle reports integer bounds and telemetry"
```

## Deferred Hardware Encoder Boundary

`HardwareEncoder` is documented only and is not implemented as a Rust type in
Phase 1. Hardware encoder discovery and selection belong with real preview and
export pipeline work after encode presets, runtime capabilities, and packaging
constraints are defined.
