# Phase 11: Realtime Preview Runtime And GPU Render Backend - Context

**Gathered:** 2026-06-18
**Status:** Ready for research and planning after Phase 10.1 completion

<domain>
## Task Boundary

Phase 11 starts the production-grade architecture sequence after Phase 10.1. It replaces the supported realtime preview path's dependence on per-frame FFmpeg processes with a Rust-side realtime preview runtime and GPU compositor. The first implementation target is Windows and macOS desktop.

</domain>

<decisions>
## Locked Architecture Decisions

### Desktop Target

- Build for Windows and macOS desktop first.
- Preserve mobile/server extension seams, but do not build full iOS/Android apps in Phase 11.

### Preview Runtime

- Use a Rust-side `RealtimePreviewRuntime`.
- Use `wgpu` as the GPU abstraction.
- Target D3D12 on Windows and Metal on macOS through `wgpu`.
- Electron/React owns UI controls and layout, but not realtime composition semantics.
- FFmpeg remains export/transcode/compatibility fallback, not the supported interactive preview path.

### Runtime Inputs

- The runtime consumes accepted draft semantics and render graph intent from Rust-owned core layers.
- The renderer must not construct FFmpeg commands, render graphs, GPU command lists, cache keys, or timeline state.

### Clock And Generation

- Preview runtime must be designed around a shared `TimelineClock`.
- Timeline position uses integer microseconds.
- Frame rates and playback rates use rational values.
- `PlaybackGeneration` changes after seek, pause/resume, or accepted edits so stale preview/audio/task results can be rejected.

### Media Decode Boundary

- Phase 11 may use existing FFmpeg-derived frame artifacts as a fallback while the new preview runtime is introduced.
- The API shape must not block Phase 12 native media IO/hardware decode: Windows Media Foundation/DXVA/D3D texture interop and macOS AVFoundation/VideoToolbox/CoreVideo/Metal texture interop.

### Task Boundary

- Phase 11 should introduce only the minimal preview task queue/cancellation shape needed for realtime preview.
- Full priority-aware `task_runtime` is Phase 16, but Phase 11 APIs must carry target timeline time and playback generation from the start.

</decisions>

<specifics>
## Phase 11 Planning Notes

- Start with a small renderable subset: canvas background, image/video layer placement, opacity, text overlay intent where feasible, and diagnostics for unsupported operations.
- Keep preview/export parity visible: GPU preview and FFmpeg export share engine/render graph semantics, and any divergence must be classified.
- Initial success should be measured by first-frame latency, seek latency, dropped/repeated frames, fallback count, and stale-generation rejection.
- Do not attempt full production hardware decode, audio DSP, scheduler, or complex effects inside Phase 11; those are subsequent phases.

</specifics>

<canonical_refs>
## Canonical References

- `.planning/notes/production-editor-architecture-decisions.md`
- `.planning/ROADMAP.md`
- `.planning/REQUIREMENTS.md`
- `docs/runtime-boundaries.md`
- `crates/preview_service/src/service.rs`
- `crates/render_graph/src/graph.rs`

</canonical_refs>
