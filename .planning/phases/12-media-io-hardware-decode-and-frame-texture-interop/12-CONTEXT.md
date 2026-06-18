# Phase 12: Media IO, Hardware Decode, And Frame/Texture Interop - Context

**Gathered:** 2026-06-18
**Status:** Ready for research and design; implementation planning should wait for Phase 11 plan shape if APIs change.

<domain>
## Task Boundary

Phase 12 splits media reading and decoding from FFmpeg process execution. It introduces Rust-owned media IO abstractions for probing, reading, video decode, audio decode, frame pools, decoded frame metadata, texture handles, and runtime capability reporting.

Phase 12 is a service-boundary phase. It feeds Phase 11 realtime preview with decoded per-material frames or texture handles, but it must not take ownership of timeline evaluation, visual composition, render graph construction, preview scheduling, export compilation, or UI state.
</domain>

<decisions>
## Locked Architecture Decisions

### Project Constraints

- UI emits commands; Rust core owns project and timeline semantics.
- No UI or binding code may directly construct FFmpeg commands.
- `.veproj/project.json` remains the canonical semantic source of truth.
- Render graphs, FFmpeg scripts, thumbnails, waveforms, proxy files, preview caches, decoded frame caches, and texture handles are derived runtime artifacts.
- Time math uses integer microseconds, frame indices, or rational frame rates. Persisted semantics must not use naked floating-point time.
- Kdenlive and MLT remain conceptual references only. Do not copy GPL code, assets, XML, presets, or UI implementation.
- FFmpeg distribution/licensing posture must be reviewed before any redistributed FFmpeg binary ships.

### Media Runtime Boundary

- Add a media IO boundary beside the existing FFmpeg process boundary, not inside draft, command, engine, render graph, or UI crates.
- Keep `media_runtime` as the shared trait/type surface for `MediaProbeService`, `MediaReader`, `VideoDecoder`, `AudioDecoder`, `FramePool`, `DecodedVideoFrame`, `DecodedAudioFrame`, `TextureHandle`, and `RuntimeCapabilities`.
- Keep existing FFmpeg process execution available for fallback, export, transcode, diagnostics, and existing probe behavior.
- `media_runtime_desktop` remains a desktop injection point and may dispatch to platform-specific implementations.

### Platform Paths

- Windows main path: Media Foundation media reading, DXVA hardware decode, and D3D texture interop.
- macOS main path: AVFoundation media reading, VideoToolbox hardware decode, CoreVideo pixel buffers, and Metal texture interop.
- Unsupported codecs, pixel formats, color spaces, encrypted media, device mismatches, allocation failures, or interop failures must return classified fallback reasons rather than panics or silent CPU copies.

### Frame And Texture Ownership

- Decoded frames must be represented as explicit leases from a `FramePool`.
- A decoded video frame carries source material ID/session, source PTS in integer microseconds or frame index, duration, dimensions, pixel format, color metadata, storage kind, and a release path.
- A texture handle is opaque outside the Rust runtime and is bound to a platform backend, device/context identity, owner session, generation, and explicit release/cascading session-close release.
- Preview/binding APIs must avoid transferring full 4K frame byte buffers across the JS/Rust boundary when a frame handle or texture handle path is available.

### Phase 11 Integration

- Phase 11 realtime preview remains the consumer of decoded frame/texture handles.
- Phase 12 decoders do not evaluate timeline state, keyframes, transforms, layer order, effects, text layout, transitions, or export semantics.
- The handoff shape is: render graph/material intent from Rust core -> media runtime source-time decode -> `DecodedVideoFrame` or `TextureHandle` -> Phase 11 realtime compositor.

### Fallback Ladder

- Prefer native platform hardware decode to platform GPU texture.
- Fall back to native platform software decode to CPU frame when hardware decode or texture interop fails.
- Fall back to FFmpeg decode/probe paths for unsupported platform media paths.
- Fall back to existing FFmpeg-generated preview artifacts only when handle/frame decode cannot satisfy the request.
- Every fallback must include a structured reason and must be visible in runtime capability reports and preview diagnostics.
</decisions>

<requirements>
## Phase Requirements

| ID | Requirement |
|----|-------------|
| MEDIAIO-01 | Media reading and decoding are behind runtime traits/capability reports rather than directly binding preview decode semantics to FFmpeg process execution. |
| MEDIAIO-02 | Desktop runtime reports Windows Media Foundation / DXVA / D3D texture capabilities and macOS AVFoundation / VideoToolbox / CoreVideo / Metal texture capabilities with fallback reasons. |
| MEDIAIO-03 | Decoded media frames have explicit frame-pool, lifetime, color metadata, CPU frame, and GPU texture handle contracts. |
| MEDIAIO-04 | Preview and binding layers avoid full-frame JS/Rust copies for 4K media when handle-based frame or texture paths are available. |
| MEDIAIO-05 | FFmpeg remains available as fallback/probe/export/transcode implementation, and unsupported codecs, pixel formats, color spaces, and hardware paths degrade predictably with test coverage. |
</requirements>

<boundaries>
## Out Of Scope

- Full iOS or Android media runtime implementation.
- Full production task scheduler; Phase 16 owns priority-aware scheduling.
- Audio output engine, WASAPI/CoreAudio playback, or audio/video clock drift policy; Phase 15 owns audio playback.
- Render graph dirty range propagation and cache coherence; Phase 13 owns incremental graph and cache invalidation.
- GPU effects, masks, transitions, retiming, and production effect registry.
- Export renderer rewrite or hardware encoder selection.
- Packaging or redistributing FFmpeg binaries.
- Direct renderer/UI access to native D3D, Metal, CVPixelBuffer, or decoder pointers.
</boundaries>

<canonical_refs>
## Canonical References

- `AGENTS.md`
- `.planning/PROJECT.md`
- `.planning/ROADMAP.md`
- `.planning/REQUIREMENTS.md`
- `.planning/notes/production-editor-architecture-decisions.md`
- `.planning/research/questions.md`
- `docs/runtime-boundaries.md`
- `crates/media_runtime/src/lib.rs`
- `crates/media_runtime/src/probe.rs`
- `crates/media_runtime/src/capabilities.rs`
- `crates/media_runtime_desktop/src/lib.rs`
- `crates/preview_service/src/service.rs`
- `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-CONTEXT.md`
</canonical_refs>
