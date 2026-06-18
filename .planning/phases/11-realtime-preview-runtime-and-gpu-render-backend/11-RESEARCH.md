# Phase 11: Realtime Preview Runtime And GPU Render Backend - Research

**Researched:** 2026-06-18  
**Domain:** Rust realtime preview runtime, `wgpu` compositor, Electron desktop embedding  
**Confidence:** MEDIUM-HIGH

## User Constraints (from CONTEXT.md)

### Locked Architecture Decisions

- Build for Windows and macOS desktop first.
- Preserve mobile/server extension seams, but do not build full iOS/Android apps in Phase 11.
- Use a Rust-side `RealtimePreviewRuntime`.
- Use `wgpu` as the GPU abstraction.
- Target D3D12 on Windows and Metal on macOS through `wgpu`.
- Electron/React owns UI controls and layout, but not realtime composition semantics.
- FFmpeg remains export/transcode/compatibility fallback, not the supported interactive preview path.
- The runtime consumes accepted draft semantics and render graph intent from Rust-owned core layers.
- The renderer must not construct FFmpeg commands, render graphs, GPU command lists, cache keys, or timeline state.
- Preview runtime must be designed around a shared `TimelineClock`.
- Timeline position uses integer microseconds.
- Frame rates and playback rates use rational values.
- `PlaybackGeneration` changes after seek, pause/resume, or accepted edits so stale preview/audio/task results can be rejected.
- Phase 11 may use existing FFmpeg-derived frame artifacts as a fallback while the new preview runtime is introduced.
- The API shape must not block Phase 12 native media IO/hardware decode: Windows Media Foundation/DXVA/D3D texture interop and macOS AVFoundation/VideoToolbox/CoreVideo/Metal texture interop.
- Phase 11 should introduce only the minimal preview task queue/cancellation shape needed for realtime preview.
- Full priority-aware `task_runtime` is Phase 16, but Phase 11 APIs must carry target timeline time and playback generation from the start.

### Phase 11 Planning Notes

- Start with a small renderable subset: canvas background, image/video layer placement, opacity, text overlay intent where feasible, and diagnostics for unsupported operations.
- Keep preview/export parity visible: GPU preview and FFmpeg export share engine/render graph semantics, and any divergence must be classified.
- Initial success should be measured by first-frame latency, seek latency, dropped/repeated frames, fallback count, and stale-generation rejection.
- Do not attempt full production hardware decode, audio DSP, scheduler, or complex effects inside Phase 11; those are subsequent phases.

## Project Constraints (from AGENTS.md)

- UI emits commands; Rust core owns project and timeline semantics; UI code may not directly construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is canonical; render graphs, FFmpeg scripts, thumbnails, waveforms, proxy files, and preview caches are derived artifacts. [VERIFIED: AGENTS.md]
- Product, Rust domain, IPC, docs, schema, and tests should use Jianying concepts such as draft, material, track, segment, keyframe, filter, and transition. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates; persisted semantics must avoid naked floating-point time. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg, and FFmpeg Runtime executes jobs without deciding editing behavior. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are conceptual references only; do not copy GPL code, assets, XML, presets, or UI implementation. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution must be reviewed for LGPL/GPL/nonfree options, notices, and commercial obligations before shipping redistributed binaries. [VERIFIED: AGENTS.md]
- The user explicitly limited writes to this research/design pair and requested no commit. [VERIFIED: user request]

## Summary

Phase 11 should introduce a new Rust-owned `RealtimePreviewRuntime` that reuses the existing pipeline through draft normalization, frame/range resolution, and `render_graph::RenderGraph`, then branches before FFmpeg compilation into a `wgpu` compositor path. [VERIFIED: `crates/preview_service/src/service.rs`, `crates/render_graph/src/graph.rs`] The existing `preview_service` already performs draft -> engine -> render graph -> FFmpeg artifact work; Phase 11 should preserve that as fallback and add a realtime backend for supported states. [VERIFIED: `crates/preview_service/src/service.rs`]

The recommended desktop embedding path is a native preview child surface controlled by Rust and positioned by Electron, with an offscreen GPU-to-image fallback for platforms where native child view integration is not stable yet. [CITED: https://www.electronjs.org/docs/latest/api/browser-window#wingetnativewindowhandle] Electron exposes a native window handle for `BrowserWindow`, while `wgpu` surfaces are created from window/display handles through its surface APIs. [CITED: https://docs.rs/wgpu/29.0.3/wgpu/struct.Instance.html] [CITED: https://docs.rs/raw-window-handle/0.6.2/raw_window_handle/]

Phase 11 must not solve hardware decode, audio output, or full task scheduling. [VERIFIED: `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-CONTEXT.md`] Instead, it should add the handle, clock, generation, telemetry, capability, and fallback contracts that Phase 12 media IO, Phase 15 audio, and Phase 16 scheduler can consume without changing preview API shape. [VERIFIED: `.planning/notes/production-editor-architecture-decisions.md`]

**Primary recommendation:** add `realtime_preview_runtime` plus thin desktop surface/session bindings; render a small `wgpu` subset now, classify unsupported graph intent, and route fallback through existing preview artifacts without spawning FFmpeg per frame for supported states. [VERIFIED: `.planning/REQUIREMENTS.md` RTPREV-01..05]

## Phase Requirements

| ID | Requirement | Research Support |
|----|-------------|------------------|
| RTPREV-01 | Rust-owned `RealtimePreviewRuntime` separate from FFmpeg export compilation. | Reuse engine/render graph preparation, then branch before `compile_ffmpeg_job`. [VERIFIED: `crates/preview_service/src/service.rs`] |
| RTPREV-02 | Render supported video, image, text, layers, transforms, opacity, canvas, keyframes through `wgpu` on D3D12/Metal. | `wgpu` 29.0.3 exposes `dx12` and `metal` features and a cross-platform graphics API. [VERIFIED: crates.io] [CITED: https://docs.rs/wgpu/29.0.3/wgpu/] |
| RTPREV-03 | Seek/scrub/basic playback do not spawn FFmpeg per frame for supported states. | Add GPU render path for supported graphs; keep FFmpeg artifact fallback only for unsupported paths and cache misses. [VERIFIED: `crates/preview_service/src/service.rs`] |
| RTPREV-04 | Preview/export share engine/render graph semantics and report divergence. | Existing render graph contains canvas, materials, video layers, text overlays, sampled frames, sampled animation states, and diagnostics. [VERIFIED: `crates/render_graph/src/graph.rs`] |
| RTPREV-05 | Shared `TimelineClock` and `PlaybackGeneration` with telemetry. | Existing time model provides `Microseconds` and `RationalFrameRate`; Phase 11 should add playback/session state around them. [VERIFIED: `crates/draft_model/src/time.rs`, `crates/draft_model/src/material.rs`] |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Timeline semantics and frame state | Rust core (`engine_core`) | Rust preview runtime | Engine already resolves frame state from integer microseconds. [VERIFIED: `crates/engine_core/src/frame_state.rs`] |
| Render intent graph | Rust core (`render_graph`) | Runtime backends | Render graph is renderer-neutral and should feed both GPU preview and FFmpeg export. [VERIFIED: `crates/render_graph/src/lib.rs`] |
| Realtime GPU composition | Rust runtime (`realtime_preview_runtime`) | Desktop platform surface module | `wgpu` device/surface ownership belongs outside Electron renderer code. [VERIFIED: 11-CONTEXT.md] |
| Preview surface placement | Electron main/renderer layout | Rust desktop surface | Electron owns workspace layout; Rust owns pixels. [CITED: https://www.electronjs.org/docs/latest/api/browser-window#wingetnativewindowhandle] |
| Hardware decode / texture interop | Phase 12 media IO runtime | Realtime preview runtime consumer | Phase 11 API must accept future frame/texture handles but should not implement Media Foundation/VideoToolbox decode. [VERIFIED: 11-CONTEXT.md] |
| Audio sync | Phase 15 audio engine | `TimelineClock` | Phase 11 must define shared clock/generation, not audio output. [VERIFIED: production architecture decisions] |
| Priority scheduling | Phase 16 task runtime | Minimal Phase 11 queue | Phase 11 carries generation/time/cancel metadata and avoids full scheduler scope. [VERIFIED: 11-CONTEXT.md] |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `wgpu` | 29.0.3 | Cross-platform GPU API for preview compositor. | Current crates.io release includes `dx12` and `metal` features and is the locked project decision. [VERIFIED: crates.io] [CITED: https://docs.rs/wgpu/29.0.3/wgpu/] |
| `raw-window-handle` | 0.6.2 | Typed raw display/window handle interop. | `wgpu` surface creation uses raw window/display handle concepts; Electron supplies native handles at the shell boundary. [VERIFIED: crates.io] [CITED: https://docs.rs/raw-window-handle/0.6.2/raw_window_handle/] |
| existing `engine_core` | workspace | Normalize draft and resolve frame/range state. | Already resolves `FrameState`, sampled frames, source positions, text overlays, and keyframed visual state. [VERIFIED: `crates/engine_core/src/frame_state.rs`] |
| existing `render_graph` | workspace | Renderer-neutral render intent. | Already carries canvas, materials, visual layers, text overlays, sampled animation states, and diagnostics. [VERIFIED: `crates/render_graph/src/graph.rs`] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `glyphon` | 0.11.0 | `wgpu` text rendering. | Use for Phase 11 text overlay rendering if deterministic parity can be constrained by pinned fonts and tests. [VERIFIED: crates.io] [ASSUMED] |
| `pollster` | 0.4.0 | Blocking bootstrap for `wgpu` async initialization. | Use only during session/device startup if the binding surface remains synchronous. [VERIFIED: crates.io] |
| `windows-sys` | 0.61.2 currently present in workspace | Win32 handle/window API bindings. | Prefer project-consistent low-level Win32 bindings for child HWND operations if needed. [VERIFIED: `crates/project_store/Cargo.toml`] |
| `objc2-app-kit` | 0.3.2 | AppKit bindings for macOS native view parenting. | Use only if Phase 11 implements Rust-owned NSView/AppKit child view directly. [VERIFIED: crates.io] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Native child surface | Offscreen `wgpu` render then PNG/BGRA buffer into React | Easier Electron integration but adds readback/copy and should be fallback, not supported realtime path. [ASSUMED] |
| Native child surface | Electron offscreen shared texture | Electron documents offscreen shared texture structures for offscreen webContents rendering, not as the primary arbitrary Rust `wgpu` child surface path. [CITED: https://www.electronjs.org/docs/latest/api/structures/offscreen-shared-texture] |
| Rust `wgpu` runtime | Browser WebGPU inside React | Easier to embed but violates Rust ownership of realtime composition and render graph consumption. [VERIFIED: 11-CONTEXT.md] |
| Phase 11 CPU frame provider | Full native hardware decode | Full native decode is explicitly Phase 12 scope. [VERIFIED: 11-CONTEXT.md] |

**Installation sketch:**

```bash
cargo add -p realtime_preview_runtime wgpu raw-window-handle glyphon pollster
# Add platform target deps only where implemented:
cargo add -p realtime_preview_runtime --target 'cfg(windows)' windows-sys
cargo add -p realtime_preview_runtime --target 'cfg(target_os = "macos")' objc2-app-kit
```

## Package Legitimacy Audit

`slopcheck install --ecosystem crates.io wgpu raw-window-handle glyphon pollster` reported all four as `[OK]`; it then attempted `cargo add` and failed because the workspace package was unspecified, with no requested artifact or source file modified. [VERIFIED: slopcheck output, git status]

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `wgpu` | crates.io | created 2019-01-24 | 24,928,968 total / 6,692,907 recent | https://github.com/gfx-rs/wgpu | OK | Approved [VERIFIED: crates.io API] |
| `raw-window-handle` | crates.io | created 2019-07-25 | 73,618,668 total / 18,014,716 recent | https://github.com/rust-windowing/raw-window-handle | OK | Approved [VERIFIED: crates.io API] |
| `glyphon` | crates.io | created 2022-05-10 | 880,943 total / 234,561 recent | https://github.com/grovesNL/glyphon | OK | Approved, but text parity needs focused validation [VERIFIED: crates.io API] [ASSUMED] |
| `pollster` | crates.io | created 2020-04-07 | 20,024,805 total / 5,777,728 recent | https://github.com/zesterer/pollster | OK | Approved [VERIFIED: crates.io API] |
| `objc2-app-kit` | crates.io | version 0.3.2 | not collected | https://github.com/madsmtm/objc2 | OK | Approved if macOS native view module uses it [VERIFIED: cargo info + slopcheck] |

**Packages removed due to slopcheck [SLOP] verdict:** none when checking the correct `crates.io` ecosystem. [VERIFIED: slopcheck output]  
**Packages flagged as suspicious [SUS]:** none when checking the correct `crates.io` ecosystem. [VERIFIED: slopcheck output]

## Architecture Patterns

### System Architecture Diagram

```text
Electron React controls
  -> preload/main command envelope
  -> bindings_node session API
  -> RealtimePreviewRuntime session
      -> TimelineClock + PlaybackGeneration
      -> engine_core normalize/resolve frame or range
      -> render_graph build intent graph
      -> capability classifier
          -> supported: wgpu compositor -> native child surface present
          -> supported but no native surface: wgpu offscreen -> buffer/image fallback
          -> unsupported/no adapter/no frame provider: existing preview_service artifact fallback
      -> telemetry + parity diagnostics
  -> Electron displays telemetry and keeps layout/transport state
```

### Recommended Project Structure

```text
crates/
  realtime_preview_runtime/
    src/lib.rs              # public session/runtime API
    src/clock.rs            # TimelineClock, PlaybackGeneration
    src/capabilities.rs     # support/degraded/unsupported classifier
    src/frame_provider.rs   # CPU frame / future texture provider traits
    src/gpu/
      mod.rs                # wgpu device, pipelines, compositor
      surface.rs            # surface/offscreen targets
      text.rs               # glyphon or text texture bridge
    src/platform/
      windows.rs            # child HWND + raw handle creation
      macos.rs              # child NSView + raw handle creation
    tests/
      clock_generation.rs
      capability_matrix.rs
      stale_frame_rejection.rs
      parity_diagnostics.rs
```

### Pattern 1: Runtime Session Owns GPU State

**What:** create long-lived preview sessions with a `wgpu::Instance`, adapter/device/queue, optional surface, loaded draft/render graph snapshot, frame provider, telemetry buffer, and current playback generation. [CITED: https://docs.rs/wgpu/29.0.3/wgpu/struct.Instance.html]

**When to use:** every preview monitor instance gets a runtime session; seeks and edits update session state instead of rebuilding GPU state per frame. [ASSUMED]

### Pattern 2: Graph Capability Classifier Before Rendering

**What:** classify each render graph intent as supported, degraded, or unsupported for the realtime backend before enqueuing GPU work. [VERIFIED: `crates/render_graph/src/graph.rs` already uses `RenderIntentSupport` diagnostics]

**When to use:** every seek/playback request should produce `RealtimePreviewDiagnostics` even when fallback succeeds. [VERIFIED: RTPREV-04]

### Pattern 3: Generation-Gated Presentation

**What:** every render request carries `target_time: Microseconds` and `playback_generation: PlaybackGeneration`; the runtime rejects or withholds presentation if completion generation differs from current session generation. [VERIFIED: 11-CONTEXT.md]

**When to use:** seek, scrub, play, pause/resume, accepted edit, fallback artifact completion, and future audio/scheduler jobs. [VERIFIED: production architecture decisions]

## Electron Embedding Recommendation

| Option | Recommendation | Why |
|--------|----------------|-----|
| Native child window/view | Primary Phase 11 path | Electron exposes native window handle access, and OS child windows/views can host a Rust-owned `wgpu` surface while React reserves bounds. [CITED: https://www.electronjs.org/docs/latest/api/browser-window#wingetnativewindowhandle] |
| Offscreen `wgpu` render + copied preview image | Required fallback | Keeps feature testable on CI/headless/no-child-surface setups, but introduces copies and readback latency. [ASSUMED] |
| Shared texture between Rust and Electron | Defer | This is closer to Phase 12 frame/texture interop and risks coupling preview display to media IO before handle lifetimes are designed. [VERIFIED: 11-CONTEXT.md] |
| Browser WebGPU canvas | Reject | It moves composition ownership into renderer/web code and conflicts with locked Rust runtime ownership. [VERIFIED: 11-CONTEXT.md] |

Implementation notes:

- Windows: Rust can create/own a child HWND and parent it to the Electron `BrowserWindow` handle; Win32 `SetParent` exists for parent changes. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setparent]
- Windows bounds/DPI/z-order need explicit resize/update calls from Electron layout state; Win32 `SetWindowPos` is the standard API for size/position/z-order updates. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos]
- macOS: Rust can create an AppKit child `NSView` and add it below the preview region; `NSView` supports adding subviews. [CITED: https://developer.apple.com/documentation/appkit/nsview/addsubview(_:)]
- `raw-window-handle` is the right Rust vocabulary for passing platform window/display handles to surface creation. [CITED: https://docs.rs/raw-window-handle/0.6.2/raw_window_handle/]

## Minimal Phase 11 Renderable Subset

| Intent | Phase 11 Support | Fallback |
|--------|------------------|----------|
| Canvas black/solid background | GPU supported | none |
| Image material layer | GPU supported by CPU upload texture | existing preview artifact if image load fails |
| Video material layer | GPU supported when `PreviewFrameProvider` returns a CPU RGBA frame for source position, including generated H.264 material frames from the session-owned software cache | existing cached preview artifact / FFmpeg fallback only for unsupported or unavailable frames |
| Transform position/scale/opacity | GPU supported from engine-resolved sampled state | fallback if invalid matrix/input size |
| Rotation/crop/fit/fill/stretch | support only where already deterministic in render graph; otherwise classify degraded/unsupported | existing preview artifact |
| Text overlay | GPU supported if `glyphon` parity tests pass; otherwise texture/raster fallback with diagnostic | existing preview artifact |
| Keyframes | GPU consumes engine-resolved sampled state, not UI interpolation | fallback only when graph reports unsupported property |
| Filters/transitions/masks/blend modes | mostly unsupported/degraded in Phase 11 | existing preview artifact |
| Audio | no realtime audio output in Phase 11 | Phase 15 |

## Fallback Ladder

1. `wgpu` native surface present with supported graph intent and available CPU frame inputs from image/static/software-video frame providers. [VERIFIED: RTPREV-02]
2. `wgpu` offscreen target, then copied display artifact for Electron fallback composition. [ASSUMED]
3. Existing `preview_service` cached artifact if cache hit. [VERIFIED: `crates/preview_service/src/cache.rs`]
4. Existing `preview_service` FFmpeg artifact generation for unsupported states or no GPU adapter, with telemetry reason and no claim of realtime support. [VERIFIED: `crates/preview_service/src/service.rs`]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| GPU abstraction | Direct D3D12/Metal renderer from scratch | `wgpu` | Locked decision and portable backend abstraction. [VERIFIED: 11-CONTEXT.md] |
| Window handle vocabulary | Ad hoc integer pointer structs across APIs | `raw-window-handle` plus platform wrappers | Standard Rust handle interop avoids ambiguous HWND/NSView pointer handling. [CITED: https://docs.rs/raw-window-handle/0.6.2/raw_window_handle/] |
| Text shaping/rasterization | Custom font shaping in Phase 11 | `glyphon` if parity validates, or fallback artifact | Text layout/rendering edge cases are broad and parity-sensitive. [ASSUMED] |
| Playback freshness | Boolean cancel flags only | `PlaybackGeneration` carried by every result | Stale frame/audio/task overwrites are a known Phase 11/15/16 concern. [VERIFIED: 11-CONTEXT.md] |
| Scheduler | Full priority scheduler now | Minimal queue API with generation/time/cancel metadata | Full scheduler is Phase 16. [VERIFIED: 11-CONTEXT.md] |

## Common Pitfalls

### Pitfall 1: Treating GPU Preview As A Second Semantic Renderer

**What goes wrong:** GPU preview drifts from FFmpeg export because it interprets transforms, keyframes, text, and layer order separately. [ASSUMED]  
**How to avoid:** consume `engine_core` resolved state and `render_graph` intent only; add parity diagnostics for every divergence. [VERIFIED: RTPREV-04]

### Pitfall 2: Creating A New FFmpeg Process During Scrub

**What goes wrong:** Phase 11 appears to have a realtime runtime but supported seeks still call `request_preview_frame` cache miss generation. [VERIFIED: current `preview_service` runs FFmpeg on cache miss]  
**How to avoid:** supported graph classification must route to `RealtimePreviewRuntime`; fallback path must report reason and count. [VERIFIED: RTPREV-03, RTPREV-05]

### Pitfall 3: Letting Native Surface Geometry Drift From React Layout

**What goes wrong:** child HWND/NSView z-order, DPI, or bounds drift from the preview monitor. [ASSUMED]  
**How to avoid:** Electron sends explicit bounds/DPI updates from the preview canvas rect; Rust owns surface resize and telemetry. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos]

### Pitfall 4: Premature Shared Texture Coupling

**What goes wrong:** Phase 11 bakes in D3D/Metal texture handle lifetimes before Phase 12 defines media frame pools and texture handles. [VERIFIED: 11-CONTEXT.md]  
**How to avoid:** define `PreviewFrameProvider` / `TextureHandle` placeholder traits but implement CPU-frame upload first. [ASSUMED]

## Code Examples

### Generation-Gated Render Result

```rust
// Source: internal design derived from 11-CONTEXT.md and draft_model time types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlaybackGeneration(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimePreviewRequest {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub mode: PreviewRequestMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimePreviewFrameResult {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub presented: bool,
    pub stale_rejected: bool,
    pub fallback: Option<RealtimePreviewFallbackReason>,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
    pub telemetry: RealtimePreviewTelemetry,
}
```

### Backend Selection

```rust
// Source: wgpu 29.0.3 documents Backends and Instance-based adapter/device setup.
let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
    backends: if cfg!(target_os = "windows") {
        wgpu::Backends::DX12
    } else if cfg!(target_os = "macos") {
        wgpu::Backends::METAL
    } else {
        wgpu::Backends::empty()
    },
    ..Default::default()
});
```

## State Of The Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-frame FFmpeg PNG preview | Long-lived `wgpu` runtime for supported interactive preview | Phase 11 roadmap | Removes process startup from supported seek/scrub path. [VERIFIED: RTPREV-03] |
| Renderer displays artifact path/image only | Rust-owned surface plus telemetry and fallback display state | Phase 11 roadmap | Makes preview runtime measurable and keeps renderer non-semantic. [VERIFIED: RTPREV-05] |
| Preview cache invalidation only | Generation-gated in-flight frame rejection | Phase 11 roadmap | Prevents old seek/edit results from overwriting current preview. [VERIFIED: 11-CONTEXT.md] |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Native child HWND/NSView embedding is acceptable for Phase 11 despite focus/z-order complexity. | Electron Embedding Recommendation | Planner may need a larger UI/platform spike or choose offscreen fallback first. |
| A2 | `glyphon` can meet deterministic text parity requirements with pinned fonts. | Standard Stack, Renderable Subset | Text may need artifact fallback or a later dedicated text rendering plan. |
| A3 | CPU frame upload backed by a session-owned software video frame cache is acceptable before Phase 12 hardware decode. | Minimal Renderable Subset | Large or unsupported-codec video preview may not meet Phase 12 hardware performance targets, but supported H.264 test material still has a non-per-frame-FFmpeg realtime path in Phase 11. |
| A4 | Offscreen GPU readback fallback is sufficient for CI and unsupported native surface environments. | Fallback Ladder | Automated visual tests may need mock renderer instead of real pixels. |

## Resolved Questions

1. **[RESOLVED] Surface rollout order:** Phase 11 builds offscreen/mock GPU targets first for deterministic Rust tests, then introduces native Windows/macOS child surfaces in a separate rollout wave. Native child surfaces remain the production desktop path, while offscreen/mock targets provide CI coverage and fallback behavior. [VERIFIED: checker resolution, 11-CONTEXT.md]

2. **[RESOLVED] Text GPU support:** Text GPU rendering is parity-gated. If the text subset passes deterministic parity tests, it may use the `glyphon` GPU path; if parity fails, text must be classified as degraded or unsupported and routed through a fallback artifact/texture path with diagnostics. Silent approximate GPU text rendering is not allowed. [VERIFIED: checker resolution, RTPREV-02, RTPREV-04]

3. **[RESOLVED] Phase 11 frame provider source:** Phase 11 `PreviewFrameProvider` implements image/static CPU frames, a session-owned software video frame cache for generated H.264 MP4/MOV material fixtures, and future texture descriptor placeholders in the API. Real hardware decode and native texture interop remain Phase 12 scope, but supported video material preview in Phase 11 must not fall back to preview artifact generation or spawn FFmpeg per requested frame. [VERIFIED: checker revision, RTPREV-02, RTPREV-03]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust/Cargo | Rust crates and tests | yes | cargo 1.95.0 | none |
| Node.js | Electron build/test | yes | v24.12.0 | none |
| npm | package metadata checks | yes | 11.6.2 | pnpm scripts already configured |
| `slopcheck` | package legitimacy audit | yes | installed, no `--json` support | non-JSON output recorded |
| `ctx7` | documentation lookup fallback | no | - | official docs/web sources used |

**Missing dependencies with no fallback:** none for research/design. [VERIFIED: command probes]  
**Missing dependencies with fallback:** `ctx7`; official docs and registry commands were used. [VERIFIED: command probes]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Rust tests | `cargo test --workspace --locked`; focused package commands should be added for `realtime_preview_runtime`. [VERIFIED: `package.json`] |
| Electron tests | Playwright via `pnpm --filter @video-editor/desktop test:workspace`. [VERIFIED: `apps/desktop-electron/package.json`] |
| Existing render parity | `cargo test -p testkit preview_export_parity -- --nocapture`. [VERIFIED: `package.json`] |

### Phase Requirements To Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| RTPREV-01 | Runtime consumes draft/render graph without FFmpeg compile path | unit/integration | `cargo test -p realtime_preview_runtime runtime_graph -- --nocapture` | no, Wave 0 |
| RTPREV-02 | Supported canvas/image/video CPU-frame subset classifies and renders through `wgpu`/mock backend | unit/gpu-gated | `cargo test -p realtime_preview_runtime gpu_subset -- --nocapture` | no, Wave 0 |
| RTPREV-03 | Supported seek path, including H.264 material frame provider cache hits, does not call FFmpeg executor per frame | integration | `cargo test -p preview_service realtime_backend_no_ffmpeg -- --nocapture` | no, Wave 0 |
| RTPREV-04 | GPU/export parity diagnostics emitted | golden/integration | `cargo test -p testkit realtime_preview_parity -- --nocapture` | no, Wave 0 |
| RTPREV-05 | clock/generation/telemetry/stale rejection | unit | `cargo test -p realtime_preview_runtime clock_generation telemetry -- --nocapture` | no, Wave 0 |

### Wave 0 Gaps

- Add `crates/realtime_preview_runtime` crate and focused tests. [VERIFIED: workspace has no such crate]
- Add H.264 material frame provider tests that generate deterministic fixtures through `testkit`, initialize a session-owned CPU frame cache, then prove preview frame requests do not call FFmpeg per frame. [VERIFIED: `crates/testkit/src/lib.rs` fixture generation exists]
- Add mock/offscreen renderer so CI does not require real D3D12/Metal adapter. [ASSUMED]
- Add source guards preventing Electron renderer from owning GPU command lists, render graphs, FFmpeg fallback selection, timeline generation, or cache keys. [VERIFIED: existing source guard pattern in `package.json`]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Desktop local app scope; no auth in Phase 11. [ASSUMED] |
| V3 Session Management | yes, local runtime handles | Opaque preview session IDs with explicit close/release and generation checks. [VERIFIED: production architecture decisions] |
| V4 Access Control | yes, IPC boundary | Renderer may request preview actions only through preload/main/binding commands. [VERIFIED: AGENTS.md] |
| V5 Input Validation | yes | Validate surface handles, bounds, target microseconds, generation, graph support, and material paths before runtime use. [ASSUMED] |
| V6 Cryptography | no | No cryptographic feature in Phase 11. [ASSUMED] |

### Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stale frame overwrite after seek/edit | Tampering | `PlaybackGeneration` checked before present/respond. [VERIFIED: 11-CONTEXT.md] |
| Renderer bypasses Rust runtime semantics | Elevation of privilege | Source guards and narrow IPC command API. [VERIFIED: AGENTS.md] |
| Invalid native handle causes crash | Denial of service | Main-process-only handle acquisition, platform validation, session close cleanup. [ASSUMED] |
| Unbounded preview requests starve UI | Denial of service | Minimal cancel/backpressure in Phase 11; full scheduler in Phase 16. [VERIFIED: 11-CONTEXT.md] |

## Sources

### Primary

- Project context: `AGENTS.md`, `.planning/PROJECT.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md`, `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-CONTEXT.md`
- Architecture notes: `.planning/notes/production-editor-architecture-decisions.md`, `.planning/research/questions.md`, `docs/runtime-boundaries.md`
- Codebase APIs: `crates/preview_service/src/service.rs`, `crates/preview_service/src/lib.rs`, `crates/render_graph/src/graph.rs`, `crates/render_graph/src/profile.rs`, `crates/media_runtime/src/lib.rs`, `crates/engine_core/src/frame_state.rs`, `crates/draft_model/src/time.rs`
- `wgpu` docs: https://docs.rs/wgpu/29.0.3/wgpu/
- `raw-window-handle` docs: https://docs.rs/raw-window-handle/0.6.2/raw_window_handle/
- Electron BrowserWindow native handle docs: https://www.electronjs.org/docs/latest/api/browser-window#wingetnativewindowhandle
- Electron offscreen shared texture docs: https://www.electronjs.org/docs/latest/api/structures/offscreen-shared-texture
- Microsoft Win32 `SetParent`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setparent
- Microsoft Win32 `SetWindowPos`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos
- Apple AppKit `NSView.addSubview`: https://developer.apple.com/documentation/appkit/nsview/addsubview(_:)

### Registry And Tool Checks

- `cargo search`, `cargo info`, crates.io API for `wgpu`, `raw-window-handle`, `glyphon`, `pollster`, `objc2-app-kit`
- `slopcheck install --ecosystem crates.io ...`
- `npm view electron ...`, `npm view @napi-rs/cli ...`

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH for `wgpu`/`raw-window-handle`; MEDIUM for `glyphon` text parity. [VERIFIED: crates.io] [ASSUMED]
- Architecture: MEDIUM-HIGH; project constraints and current pipeline are clear, but native child embedding needs implementation validation. [VERIFIED: codebase + official docs] [ASSUMED]
- Pitfalls: MEDIUM; based on project boundaries plus known native surface/fallback risks. [VERIFIED: project docs] [ASSUMED]

**Research date:** 2026-06-18  
**Valid until:** 2026-07-18 for package versions and Electron/wgpu API details.
