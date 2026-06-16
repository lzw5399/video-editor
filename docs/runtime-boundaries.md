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
backend injection point. Later discovery work will add version probes,
structured missing-binary errors, checked paths, bounded stderr summaries, and
argument-array process execution through this boundary.

## Project Store Runtime

`project_store::StdPlatformFileSystem` is the standard desktop filesystem shell
for `.veproj` persistence. `.veproj/project.json` remains the canonical source of
truth. Render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files,
preview caches, and exported videos are derived artifacts.

## Preview Runtime

`preview_service::PreviewRenderer` is boundary-only in Phase 1. It reserves where
future preview frame and segment generation can be injected without letting
preview runtime concerns enter draft or timeline semantics.

## Deferred Hardware Encoder Boundary

`HardwareEncoder` is documented only and is not implemented as a Rust type in
Phase 1. Hardware encoder discovery and selection belong with real preview and
export pipeline work after encode presets, runtime capabilities, and packaging
constraints are defined.
