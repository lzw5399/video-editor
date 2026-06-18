---
title: Production Editor Architecture Decisions
date: 2026-06-18
context: Post-Phase-10.1 production-grade desktop editor planning
status: confirmed
---

# Production Editor Architecture Decisions

This note captures the confirmed architecture direction for production-grade editor work after Phase 10.1. The first implementation target is Windows and macOS desktop. iOS and Android remain future extension targets through portable runtime boundaries, not near-term product deliverables.

## Confirmed Decisions

### 1. Desktop First, Mobile Ready

- Primary target: Windows and macOS desktop editor.
- Electron/React remains the desktop UI shell.
- Rust remains the owner of project semantics, timeline commands, render graph intent, preview/runtime sessions, and export orchestration.
- iOS and Android are not built as full apps in the next production phases; the architecture preserves portable runtime, media, GPU, and binding contracts for future ports.

### 2. Realtime Preview Runtime

- Phase 11 uses a Rust-side `RealtimePreviewRuntime` as the main interactive preview path.
- GPU rendering uses `wgpu`.
- Windows preview targets D3D12 through `wgpu`; macOS preview targets Metal through `wgpu`.
- Electron/React controls the UI and sends commands; it does not own realtime composition.
- FFmpeg remains for export, transcode, compatibility fallback, and diagnostics, not the supported realtime preview path.

### 3. Media IO And Hardware Decode

- Platform-native media IO and hardware decode are the desktop main path after Phase 11.
- Windows path: Media Foundation / DXVA / D3D texture interop.
- macOS path: AVFoundation / VideoToolbox / CoreVideo / Metal texture interop.
- FFmpeg remains a fallback/probe/export/transcode implementation.
- Upper layers consume `DecodedVideoFrame` or `TextureHandle` abstractions rather than FFmpeg-produced preview image/video files.

### 4. Incremental Render Graph

- `draft_commands` should emit a `CommandDelta` after accepted edits.
- `CommandDelta` carries changed entity IDs, changed timeranges, and changed domains such as timing, visual, text, audio, material, and effect.
- `render_graph` should use stable node identities derived from semantic entities, not content hashes alone.
- Node fingerprints decide whether content changed; node identities decide what entity a graph node represents.
- Dirty range propagation drives preview, export preparation, audio, thumbnail, waveform, proxy, and cache invalidation.

### 5. Artifact Store And Cache Coherence

- `.veproj/project.json` remains the only canonical semantic source of truth.
- `.veproj/derived/` stores rebuildable artifacts.
- Use a project-local SQLite artifact index plus blob directories instead of large JSON manifests.
- Artifact keys include source material fingerprint, semantic node fingerprint, dirty range, output profile, runtime capability fingerprint, artifact schema version, and generator version.
- Replacement, relink, timeline edit, and runtime capability changes invalidate exact affected artifacts.

### 6. Audio Engine And Shared Timeline Clock

- Add an independent `audio_engine` for low-latency preview playback.
- Windows output uses WASAPI; macOS output uses CoreAudio.
- Preview audio is mixed in realtime; FFmpeg remains usable for export or fallback mixdown.
- Audio, `wgpu` preview rendering, and scheduled preview tasks must share one `TimelineClock`.
- Timeline position uses integer microseconds; frame rate and playback rate use rational values.
- `PlaybackGeneration` invalidates stale audio buffers, preview frames, and queued tasks after seek, pause, resume, or timeline changes.

### 7. Unified Task Runtime

- Introduce a `task_runtime` / `JobScheduler` for preview, decode, artifact, proxy, waveform, probe, export, and IO work.
- Interactive jobs such as seek, scrub, preview frame, and inspector recompute have highest priority.
- Export, proxy, waveform, and cache jobs cannot starve interactive preview.
- Jobs carry `target_timeline_time_us` and `playback_generation` so stale work cannot overwrite current preview/audio state.
- Scheduler reports queue latency, cancellation, fallback, first-frame time, dropped/repeated frames, cache hit rate, and resource budget telemetry.

### 8. Portable Runtime And Handle Registry

- Phase 17 creates portable runtime and binding contracts, not full mobile apps.
- Desktop Node-API remains, but should become a thin bridge.
- Add a C ABI shape with opaque handles for future mobile/server bindings.
- Sessions, media handles, frame handles, texture handles, and artifact handles use a unified registry.
- Handles have owner session, generation, reference count, explicit release, session-close cascading release, and debug leak diagnostics.
- GPU texture/frame handles must be bound to their device/context lifetime and thread-safety contract.

### 9. Production Effects Recovery

- Retiming, transitions, filters, masks, blends, and effects return after realtime preview, media IO, graph/cache, audio, scheduler, and binding foundations.
- Phase 18 order: capability registry, retiming/speed, transitions, visual effects, template fidelity gates.
- The product does not chase 100% proprietary effect parity.
- Effects are self-owned semantics with preview/export support matrices and explicit supported/degraded/unsupported reports.

## Architecture Shape

```text
Electron UI
  -> bindings_node
  -> editor_core_runtime sessions
  -> draft_commands / engine_core
  -> render_graph snapshots and dirty ranges
  -> realtime_preview_runtime + wgpu
  -> media_runtime_windows / media_runtime_macos / media_runtime_ffmpeg
  -> audio_engine + TimelineClock
  -> artifact_store + task_runtime
```

## Deferred By Design

- Full iOS/Android product UI.
- 100% proprietary Jianying/Kaipai effect parity.
- Moving canonical project semantics into cache/artifact manifests.
- Renderer-owned FFmpeg commands, timeline mutation, render graph construction, or cache invalidation.
