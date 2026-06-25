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

## FFmpeg Runtime Scope

Desktop builds use a bundled FFmpeg runtime. Discovery is single-source:

- Electron configures the native binding with the app-local bundled runtime directory.
- `apps/desktop-electron/runtime/ffmpeg/<platform>-<arch>` during local
  development.
- `process.resourcesPath/ffmpeg/<platform>-<arch>` in packaged Electron builds.

The runtime crate does not search `PATH` and does not accept separate
per-binary runtime variables. Product Electron startup also does not honor
external runtime directory overrides. Electron is responsible for provisioning
and packaging FFmpeg/ffprobe resources before launch.

The bundled runtime engineering manifest records exact versions and checksums.
Public redistribution still requires legal review of the exact FFmpeg build,
notices, source-offer obligations, build flags, and commercial product
constraints before shipping externally.

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
The H.264 software video frame provider/cache and diagnostics such as
`TextParityUnsupported` are runtime capability evidence, not product playback
success evidence.

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

## Phase 18 Portable Runtime And Binding Boundaries

Phase 18 promotes the desktop-first Rust core into a portable runtime surface.
The ownership split is destructive by design: shared project, export, handle,
and lifecycle semantics live below every adapter in `editor_runtime`. Desktop
Node-API, portable C ABI, future Android JNI, future iOS Swift/ObjC, and server
entrypoints are transport layers over that shared Rust authority.

### Shared Runtime Ownership Map

| Layer | Owns | Does Not Own |
|-------|------|--------------|
| `editor_runtime` | Runtime sessions, project sessions, project-store calls, Node-shaped project-session semantics, export service, render graph build, FFmpeg job compilation, scheduler state, export telemetry, handle registry, owner/generation/ref/lease/release/cascade diagnostics | N-API transport, C ABI buffer layout, Electron IPC, Android lifecycle callbacks, iOS permission UX, server CLI argument parsing |
| `bindings_node` | Desktop Node-API and JSON transport, explicit N-API function names, serde conversion, desktop resource wiring | Project-session registry, draft mutation semantics, export scheduler policy, render graph or FFmpeg compilation, portable handle lifetime policy |
| Electron main/preload/renderer | UI commands, desktop IPC validation, native binding loading, preview host geometry, product display | Draft/project/export semantics, FFmpeg command construction, render graph construction, fallback success decisions, handle metadata |
| `bindings_c` | Stable C ABI transport, generated `video_editor_runtime.h`, `repr(C)` status/runtime/handle/buffer/texture structs, bounded diagnostic buffers | Draft semantics, project lifecycle policy, export scheduler policy, handle retain/release metadata |
| Future Android JNI adapter | JNI thread attachment, Activity/process lifecycle forwarding, Java/Kotlin wrappers around C ABI handles, platform permission prompts | Rust resource metadata, fabricated handles, garbage-collection-only release, duplicated project/export semantics |
| Future iOS Swift/ObjC adapter | C header import, Swift/ObjC wrappers around opaque handles, security-scoped resource coordination, app lifecycle forwarding | Rust resource metadata, fabricated handles, ARC-only release, duplicated project/export semantics |
| `server_runtime` | Electron-free `.veproj` open, export start/status/cancel/wait entrypoints, JSON CLI events, bundle-relative material resolution before export | Electron, BrowserWindow, preload IPC, DOM state, desktop UI view models, independent render/export scheduler |

`editor_runtime::EDITOR_RUNTIME_CONTRACT_VERSION` names the shared Rust
contract. Adapter-specific versioning may wrap it, but adapters must not fork
draft, project, render, export, scheduler, or handle semantics.

### Project And Export Boundary

`.veproj/project.json` remains the canonical semantic source of truth across
desktop, C ABI, mobile contracts, and server runtime. `editor_runtime` opens and
saves bundles through `project_store`, then builds export jobs through
`engine_core`, `render_graph`, `ffmpeg_compiler`, `task_runtime`, and
`media_runtime` services. Adapters pass requests and receive typed responses;
they do not construct FFmpeg commands or render graphs.

`server_runtime` is the first non-Electron entrypoint over the shared export
path. It opens `.veproj` bundles, resolves bundle-relative filesystem materials
at export time without mutating `project.json`, starts exports through
`editor_runtime::ExportService`, reports structured progress/status/error JSON,
supports cancellation, and validates output media through the same runtime path.

### Opaque Handles And Mobile Contracts

Runtime sessions, project sessions, media handles, frame handles, texture
handles, and artifact handles are Rust-owned opaque tokens. Public tokens carry
only kind, ID, owner session, owner generation, and generation facts. Rust stores
the resource metadata, retain/release state, lease expiry, texture/device facts,
and leak diagnostics.

Future JNI and Swift/ObjC adapters import the C ABI contract in
`crates/bindings_c/include/video_editor_runtime.h` and follow
[`docs/mobile-runtime-contracts.md`](mobile-runtime-contracts.md). Phase 18
documents mobile lifecycle, background/foreground, sandboxed permission
invalidation, file handle, texture/device, cancellation, explicit release, and
cascading close rules. It does not ship full Android/iOS apps, mobile UI,
permission UX, platform packaging, or store deployment.

### Phase 18 Gate Scripts

The root Phase 18 gate is:

```bash
pnpm run test:phase18
```

It composes:

- `pnpm run test:phase18-rust`
- `pnpm run test:phase18-source-guards`
- `pnpm run test:phase18-abi`
- `pnpm run test:phase18-server`
- `pnpm run test:phase18-mobile-contracts`
- `cargo check --workspace --locked`
- `pnpm run test:no-product-fallback`
- `pnpm run test:contracts`

The source guard blocks duplicated adapter semantics, C ABI dependency on the
desktop Node adapter, server Electron dependencies, adapter-owned handle
lifetime policy, Electron render/export construction, and fallback/mock/artifact
success evidence. The ABI drift guard regenerates the checked-in C header
through project-local pinned `cbindgen 0.29.4`. The mobile contract guard checks
the contract document and `bindings_c` smoke tests for owner, generation,
device, release, and cascading close coverage.

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

## Phase 19 Production Effect, Retiming, And Transition Ownership

Phase 19 promotes retiming, transitions, effects, filters, masks, blends, and
high-frequency editor manipulation into Rust-owned production semantics. The
desktop UI may expose controls only after Rust reports capability-backed
support, degraded support, or unsupported diagnostics through the shared runtime
contracts.

### Semantic Ownership Map

| Layer | Owns | Does Not Own |
|-------|------|--------------|
| `draft_model` | First-party draft schema for retiming, transitions, production effects, filters, masks, blends, capability states, external compatibility references, and integer/rational timing fields | Provider-private IDs as internal semantics, FFmpeg filter strings, UI preview shortcuts |
| `draft_commands` | Undoable commands for retime, transition, effect, filter, mask, blend, and project interaction begin/update/commit/cancel flows | Renderer drag math, per-pointer save loops, direct project persistence from UI samples |
| `engine_core` | Accepted draft normalization, integer-microsecond source/target mapping, transition relationship resolution, and frame-state evaluation inputs | Electron-local time mapping, FFmpeg filter compilation, provider-native behavior |
| `audio_engine` | Audio graph retime intent, follow-speed classification, source sample mapping, and parity diagnostics | UI-owned audio speed math, silent audio fallback success |
| `render_graph` | Typed render intents, capability-aware graph nodes, semantic fingerprints, dirty domains, cache invalidation inputs, and preview/export graph parity | FFmpeg process execution, DOM/CSS effect rendering, adapter-private effect semantics |
| `realtime_preview_runtime` | GPU/native preview support classification, production effect/filter/mask/blend/transition preview execution, interaction-generation telemetry, and unavailable diagnostics | Mock/artifact/CPU/DOM evidence as product preview success, renderer-side effect evaluation |
| `ffmpeg_compiler` | Compilation from typed render graph intents into FFmpeg filter scripts and export diagnostics for supported/degraded paths | Editing behavior decisions, Electron-constructed FFmpeg commands, provider-native passthrough semantics |
| `editor_runtime` | Project session interaction routing, coalesced interaction updates, commit/cancel authority, project save/revision ownership, and runtime capability reports | N-API transport details, React state management, default UI copy |

The root invariant is unchanged: `.veproj/project.json` is the canonical
semantic source of truth. Render graphs, compiled FFmpeg scripts, thumbnails,
waveforms, preview caches, and exported media remain derived artifacts.

### Desktop UI Boundary

Electron renderer code displays state returned from Rust and emits typed project
intents or project interaction events. It may keep immediate ghost/provisional
preview state for pointer responsiveness, but draft mutation, undo/revision
creation, save timing, source-to-target mapping, transition validity, effect
evaluation, capability decisions, dirty ranges, cache fingerprints, render graph
construction, and FFmpeg command generation remain below the binding boundary.

High-frequency controls such as effect strength sliders, filter strength
sliders, mask handles, blend opacity, transition duration handles, retime
handles, and keyframe drags must follow the Rust interaction route:

```text
beginProjectInteraction -> coalesced updateProjectInteraction -> commitProjectInteraction / cancelProjectInteraction
```

Renderer-side `requestAnimationFrame` coalescing is allowed for reducing pointer
traffic and drawing ghost state. It must not create one save, undo entry,
revision increment, or semantic commit per pointer sample. Committed product
state is accepted only from the Rust response/generation.

### External Adapter Boundary

Kaipai, Jianying, CapCut, and other external draft adapters may carry proprietary
or provider-native effect/filter/transition IDs only as external compatibility
references in import/export reports. Those IDs do not become internal render
semantics, capability keys, or default UI labels. Adapters translate the
supported subset into first-party draft semantics and report unsupported,
degraded, or approximate mappings explicitly.

### Phase 19 Gate Scripts

The root Phase 19 gate is:

```bash
pnpm run test:phase19
```

It composes:

- `pnpm run test:phase19-source-guards`
- `pnpm run test:no-product-fallback`
- `pnpm run test:phase19-rust`
- `pnpm run test:phase19-desktop`
- `cargo check --workspace --locked`
- `pnpm run test:contracts`

`test:phase19-source-guards` runs `scripts/phase19-source-guards.sh` in default
aggregate mode. The guard blocks Electron-owned FFmpeg construction, renderer
retime mapping, transition validation, effect/filter/mask/blend evaluation,
cache/fingerprint semantics, provider-native IDs as internal semantics,
fallback/mock/artifact/CPU/DOM success evidence, and high-frequency pointer
samples that directly save, push undo, increment revisions, or commit project
intents.

## Deferred Hardware Encoder Boundary

`HardwareEncoder` is documented only and is not implemented as a Rust type in
Phase 1. Hardware encoder discovery and selection belong with real preview and
export pipeline work after encode presets, runtime capabilities, and packaging
constraints are defined.
