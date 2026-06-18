# Requirements: Video Editor

**Defined:** 2026-06-17
**Core Value:** Users can reliably import media, edit segments on a familiar Jianying-style timeline, preview the result, save the draft, and export a video through one consistent editing and rendering model.

## v1 Requirements

### Foundation

- [x] **FOUND-01**: Developer can build a Rust workspace and Electron desktop shell from a clean checkout.
- [x] **FOUND-02**: Electron can call the Rust core through a typed binding/API boundary.
- [x] **FOUND-03**: The app can discover configured FFmpeg and ffprobe binaries and report actionable errors when unavailable.
- [x] **FOUND-04**: The repository includes deterministic fixtures and golden test harnesses before feature work depends on media rendering.

### Draft Format

- [x] **DRAFT-01**: User can create a new `.veproj` draft bundle.
- [x] **DRAFT-02**: User can open and save a draft without semantic changes in a round trip.
- [x] **DRAFT-03**: Draft schema uses Jianying-aligned concepts: draft, material, track, segment, target/source time range, main-track magnet, canvas adjustment, keyframe, sticker, text bubble, text effect, filter, and transition.
- [x] **DRAFT-04**: Draft stores semantic state only in `project.json`; thumbnails, waveforms, preview caches, render graphs, FFmpeg scripts, and exports are derived artifacts.
- [x] **DRAFT-05**: Draft versioning and migration hooks exist for future schema changes.

### Materials

- [x] **MAT-01**: User can import video, image, and audio materials into the draft.
- [x] **MAT-02**: Imported materials receive stable IDs and retain URI, duration, fps, size, stream, and audio metadata from ffprobe.
- [x] **MAT-03**: Material bin displays imported materials with basic metadata and generated thumbnails where applicable.
- [x] **MAT-04**: App detects missing material files and presents a recovery/error state without corrupting the draft.

### Timeline And Commands

- [x] **TIME-01**: Draft supports at least one sequence with video, audio, and text tracks.
- [x] **TIME-02**: User can add material segments to tracks with explicit source and target time ranges.
- [x] **TIME-03**: User can select, move, split, trim, and delete timeline segments.
- [x] **TIME-04**: User can undo and redo every committed timeline edit.
- [x] **TIME-05**: Main-track magnet/snapping behavior is implemented in the Rust core, not in UI-only state.
- [x] **TIME-06**: Invalid edits are rejected atomically without partially mutating the draft.
- [x] **TIME-07**: Track stacking/z-index and per-track mute state are represented in the draft model.

### Text And Audio

- [x] **TEXT-01**: User can add text/subtitle segments to a text track.
- [x] **TEXT-02**: User can edit text content, font size, color, alignment, stroke, shadow, and background for MVP text segments.
- [x] **TEXT-03**: Text layout uses pinned fonts and deterministic settings for preview/export parity.
- [x] **AUD-01**: User can add audio/BGM materials to an audio track.
- [x] **AUD-02**: User can adjust segment volume and track mute state.

### Preview

- [x] **PREV-01**: User can preview the current draft in the center player.
- [x] **PREV-02**: User can seek/scrub the playhead and request a deterministic preview frame.
- [x] **PREV-03**: User can play a short preview segment using a cache generated from the same render path as export.
- [x] **PREV-04**: Preview cache invalidates only affected ranges after timeline or text edits.

### Export

- [x] **EXP-01**: User can export the draft to H.264 MP4 with a small preset set.
- [x] **EXP-02**: Export uses the same normalized draft, resolved frame state, render graph, and FFmpeg compilation path as preview.
- [x] **EXP-03**: Export reports progress, supports cancel, captures logs, and classifies common FFmpeg errors.
- [x] **EXP-04**: Export output is validated for duration, fps, resolution, audio stream, and file existence.

### Desktop UI

- [x] **UI-01**: Desktop editor first screen uses a Jianying-like workspace: top feature categories, left material/function panel, center preview, right inspector, and bottom multi-track timeline.
- [x] **UI-02**: MVP UI implements media/material, text, and audio panels while reserving visible categories for sticker, effect, transition, filter, and adjustment.
- [x] **UI-03**: UI uses Jianying-style terms consistently and does not expose alternate internal jargon.
- [x] **UI-04**: UI emits typed commands to Rust and cannot mutate the draft directly.
- [x] **UI-05**: Timeline controls have stable dimensions and do not shift layout during selection, hover, or playback updates.
- [x] **UI-06**: Desktop UI user-facing language is Simplified Chinese by default, including panel titles, controls, empty states, errors, and test-visible copy.
- [x] **UI-07**: Top feature area uses compact icon-plus-text entries with Jianying-style categories and restrained cyan active states.
- [x] **UI-08**: Left resource/function panel uses a narrow category tree plus content area with import, search, sort/filter, compact material cards, and visible deferred feature states.
- [x] **UI-09**: Center preview shell has a professional monitor structure with title bar, black video canvas, bottom playback controls, fit/ratio/fullscreen controls, and reserved preview-frame integration points.
- [x] **UI-10**: Right inspector uses Jianying-style primary tabs, compact rows, sliders, numeric inputs, switches, color swatches, keyframe placeholders, draft parameters when nothing is selected, and segment-specific controls when selected.
- [x] **UI-11**: Timeline uses a compact icon toolbar, track headers with lock/visibility/mute state, realistic video/audio/text segment blocks, stable ruler/playhead/snapping/zoom controls, and no layout jump on hover or selection.
- [x] **UI-12**: Professional workspace refinement preserves the command-only renderer boundary and does not introduce renderer-owned draft mutation, undo/redo semantics, FFmpeg commands, render graphs, export scripts, or preview cache semantics.

### Testing And Quality

- [x] **TEST-01**: Schema and model tests validate every golden draft fixture.
- [x] **TEST-02**: Command tests cover split, trim, move, delete, snapping, undo, redo, text edit, and volume edit.
- [x] **TEST-03**: Engine tests cover normalization, time mapping, track stacking, text layout, and frame-state snapshots.
- [x] **TEST-04**: Render graph and FFmpeg compiler outputs have snapshot tests.
- [x] **TEST-05**: Preview frame and exported frame match within documented tolerance for golden drafts.
- [x] **TEST-06**: Electron E2E test imports material, edits a timeline, previews, exports, and verifies output.
- [x] **TEST-07**: Packaged app smoke test launches offline and completes import-preview-export.
- [x] **TEST-08**: Electron workspace tests and source guards verify the professional UI at 1280x800 and 1120x720, including five-region visibility, command-only timeline updates, and no renderer-owned media/render semantics.

## v2 Requirements

### Usable MVP Completion

- **MVPEDIT-01**: Desktop import uses an Electron main/preload system file chooser for video, audio, and image files; renderer code receives selected paths through a narrow sandboxed API and then calls `importMaterial`.
- **MVPEDIT-02**: Imported materials appear in the left material panel with Jianying-style Chinese labels for kind, duration, dimensions or audio stream metadata, and availability.
- **MVPEDIT-03**: Timeline operations for add, select, move, trim, split, delete, undo, redo, and copy where implemented remain command-only and keep snapping/main-track magnet behavior in Rust.
- **MVPEDIT-04**: Timeline playhead can be changed by ruler click, playhead drag, time input, and previous/next frame controls; each seek requests a preview frame for the current integer-microsecond time.
- **MVPEDIT-05**: Preview monitor displays returned PNG frame pixels inside the draft canvas aspect ratio instead of only showing artifact paths, with a black/empty canvas fallback.
- **MVPEDIT-06**: Video/image/text visual controls for position, scale, rotation, opacity, crop, fit/fill/stretch, and visibility route through `updateSegmentVisual` and are reflected in preview state.
- **MVPEDIT-07**: Audio editing exposes segment volume and track mute through `setSegmentVolume` and `setTrackMute`, with waveform shown as a P0 placeholder and real waveform cache explicitly deferred.
- **MVPEDIT-08**: Text and SRT subtitle workflows add/edit text segments through `addTextSegment`, `editTextSegment`, and Rust-owned `importSubtitleSrt`; renderer code never constructs subtitle cue segments.
- **MVPEDIT-09**: Playwright Electron coverage verifies system import, add-to-timeline, preview image display, playhead request/update, transform, audio, text, and SRT flows at 1280x800 and 1120x720.

### Core Editing Expansion

- **CANVAS-01**: Draft has a canonical project canvas/profile model for aspect ratio, canvas width/height, and rational frame rate.
- **CANVAS-02**: Draft supports semantic canvas background modes: black, solid color, blur fill, and image background.
- **CANVAS-03**: Visual coordinate semantics are documented and shared by transform, sticker, text, PIP, keyframe, preview, and export paths.
- **CANVAS-04**: Desktop UI exposes project canvas settings with Simplified Chinese Jianying-style terminology and Rust command ownership.

- **XFORM-01**: Segment-level 画面/基础/变换 semantics support position x/y, scale, rotation, opacity, crop, and anchor using typed persisted values.
- **XFORM-02**: Segment transform supports fit, fill, stretch, and background fill behavior for aspect-ratio mismatches.
- **XFORM-03**: Transform edits are Rust-owned commands with validation, undo/redo, generated contracts, and no renderer-owned draft mutation.

- **LAYER-01**: Visual layer semantics distinguish video, image, sticker, and text layers with explicit layer ordering and visibility.
- **LAYER-02**: engine_core and render_graph evaluate visual composition order deterministically for preview/export.
- **LAYER-03**: Blend mode and mask are represented with supported/degraded/unsupported capability boundaries even when full rendering is deferred.

- **TEXT2-01**: Complete text semantics include font references, text box width/height, line height, letter spacing, safe area/layout region, font size, color, stroke, shadow, background, and alignment.
- **TEXT2-02**: Multiple text/subtitle segments render through the shared preview/export path with stable layout snapshots.
- **TEXT2-03**: Unsupported proprietary text bubbles, text effects, or font resources produce degraded/unsupported reports rather than silent fake support.

- **ANIM-01**: Draft stores typed animated values for position, scale, rotation, opacity, text parameters, sticker parameters, filter parameters, and volume where applicable.
- **ANIM-02**: Keyframes include integer/rational time, typed values, interpolation policy, and easing curve.
- **ANIM-03**: engine_core and render_graph evaluate animated values at frame time without UI-owned interpolation or naked floating-point persisted semantics.

### Production Realtime Preview And Rendering

- **RTPREV-01**: Realtime preview has a Rust-owned `RealtimePreviewRuntime` separate from FFmpeg export compilation and can consume accepted draft semantics plus render graph intent.
- **RTPREV-02**: Supported video, image, text, visual layer, transform, opacity, canvas, and keyframe state can render through `wgpu`, targeting D3D12 on Windows and Metal on macOS, with explicit unsupported/degraded diagnostics.
- **RTPREV-03**: Seek, scrub, and basic playback preview do not spawn a new FFmpeg process per frame for supported timeline states.
- **RTPREV-04**: Realtime preview and export share engine/render graph semantics and produce parity diagnostics for known divergence.
- **RTPREV-05**: Preview runtime uses shared integer-microsecond `TimelineClock` and `PlaybackGeneration` values, and reports first-frame, seek latency, frame pacing, dropped frame, stale-generation rejection, cancellation, fallback, and cache-hit telemetry.

### Media IO And Hardware Decode

- **MEDIAIO-01**: Media reading and decoding are behind runtime traits/capability reports rather than directly binding preview decode semantics to FFmpeg process execution.
- **MEDIAIO-02**: Desktop runtime reports Windows Media Foundation / DXVA / D3D texture capabilities and macOS AVFoundation / VideoToolbox / CoreVideo / Metal texture capabilities with fallback reasons.
- **MEDIAIO-03**: Decoded media frames have explicit frame-pool, lifetime, color metadata, CPU frame, and GPU texture handle contracts.
- **MEDIAIO-04**: Preview and binding layers avoid full-frame JS/Rust copies for 4K media when handle-based frame or texture paths are available.
- **MEDIAIO-05**: FFmpeg remains available as fallback/probe/export/transcode implementation, and unsupported codecs, pixel formats, color spaces, and hardware paths degrade predictably with test coverage.

### Incremental Graph And Cache Coherence

- **INCR-01**: Render graph nodes have stable identities tied to semantic draft entities rather than content hashes alone, with fingerprints for current content, inputs, and runtime capabilities.
- **INCR-02**: Accepted draft commands emit `CommandDelta` data with changed entity IDs, changed domains, and changed integer-microsecond ranges for incremental graph updates or targeted invalidation.
- **INCR-03**: Dirty range propagation spans preview, export preparation, audio, thumbnails, waveforms, proxies, and preview cache using integer/rational time.
- **INCR-04**: Undo/redo restores semantic state and either restores matching graph/cache snapshots or invalidates affected ranges deterministically.
- **INCR-05**: Large-timeline tests verify graph diff cost, dirty range accuracy, and preview/export consistency after localized edits.

### Asset Resource And Derived Artifact Store

- **ASSET-01**: Asset manager indexes materials, proxies, thumbnails, waveforms, fonts, and supported effect resources with stable IDs and project-relative references.
- **ASSET-02**: Derived artifacts are tracked in `.veproj/derived/artifact-store.sqlite` with schema version, runtime capability fingerprint, source material fingerprint, graph fingerprint, generation parameters, dependency rows, dirty state, and generation status.
- **ASSET-03**: Replacing, relinking, renaming, or deleting source media invalidates or regenerates exactly the affected artifacts.
- **ASSET-04**: Proxy, thumbnail, and waveform generation is chunked, resumable, cancellable, and isolated from interactive preview responsiveness.
- **ASSET-05**: Cache garbage collection, storage quotas, and optional cloud/server synchronization manifests are defined before remote rendering depends on them.

### Audio Engine And DSP Pipeline

- **AUDIO2-01**: Audio preview playback uses a dedicated audio graph synchronized to the shared `TimelineClock` and `PlaybackGeneration`, with seek, pause, cancel, and buffering behavior independent from FFmpeg preview frame generation.
- **AUDIO2-02**: Segment gain, track mute, pan, fades, keyframed volume, and future audio effects have typed DSP semantics with integer/rational timeline mapping.
- **AUDIO2-03**: Windows preview audio output uses WASAPI and macOS preview audio output uses CoreAudio, while waveform and peak data from the artifact store drive UI display without becoming canonical audio semantics.
- **AUDIO2-04**: Export audio mixdown remains parity-tested against the preview audio graph with classified differences.

### Scheduler And Performance

- **SCHED-01**: Preview, decode, artifact generation, export, media probing, and filesystem IO run through priority-aware queues with cancellation, backpressure, target timeline microseconds, and `PlaybackGeneration`.
- **SCHED-02**: Export and heavy artifact jobs cannot block playhead scrubbing, inspector edits, or preview frame delivery on supported hardware.
- **SCHED-03**: Thread-pool and resource limits are explicit, configurable for desktop development, and ready to map onto mobile/server runtimes.
- **SCHED-04**: Performance telemetry records queue latency, job duration, cancellation, fallback, cache hit rate, first-frame time, and dropped-frame budgets.

### Binding Runtime Expansion

- **BIND-01**: Binding architecture separates desktop Node-API, portable C ABI, future Android JNI, future iOS Swift/ObjC, and server entrypoints without duplicating draft semantics.
- **BIND-02**: Runtime sessions, project sessions, media handles, frame handles, texture handles, and artifact handles use opaque IDs with owner session, generation, reference count, explicit release, cascading session-close release, and debug leak diagnostics.
- **BIND-03**: Large media frames and preview outputs cross language boundaries through handle-based or low-copy paths whenever supported, with GPU texture/frame handles bound to their device/context lifetime.
- **BIND-04**: Server runtime can open `.veproj`, resolve materials, run render/export jobs, and report progress without Electron.
- **BIND-05**: ABI, serialization, and binding smoke tests protect contract drift across desktop, mobile prototypes, and server rendering.

### Production Effects And Retiming

- **PRODFX-01**: Retiming/speed curves are typed draft semantics evaluated by engine_core and represented in render graph/audio graph without renderer-owned time math.
- **PRODFX-02**: Transitions between adjacent visual segments have typed semantics, preview/export implementations or explicit degraded diagnostics, and undoable commands.
- **PRODFX-03**: Filters/effects use a capability registry that maps semantic effect intent to GPU preview and export/compiler implementations where supported, before retiming, transition, and effect implementation expands.
- **PRODFX-04**: Masks, blend modes, blur, and complex effects use the production GPU preview path for realtime interaction and classify unsupported export paths.
- **PRODFX-05**: Complex Jianying/Kaipai-like template fixtures verify preview/export parity, fallback reports, and performance budgets for production editing scenarios.

### Compatibility

- **COMP-01**: User can import a supported Jianying/CapCut draft subset into `.veproj`.
- **COMP-02**: User receives a compatibility report listing supported, degraded, and unsupported external draft features.
- **COMP-03**: User can export a supported Jianying-compatible draft subset when feasible.

### Platform Expansion

- **PLAT-01**: Rust core exposes a C/FFI boundary for mobile/server shells.
- **PLAT-02**: Server renderer can render a `.veproj` without Electron.
- **PLAT-03**: iOS and Android extension points are represented by ABI/JNI/Swift contract documents and smoke-level handle/session tests, while full mobile apps remain deferred.

## Out of Scope

| Feature | Reason |
|---------|--------|
| AI oral-video workflows | Current product is a general desktop video editor |
| Jianying draft as primary project format | Conflicts with self-owned cross-platform core and long-term schema control |
| 100% proprietary effect parity | Private resources/effects are unstable, unavailable, or legally constrained |
| Kdenlive/MLT runtime integration | References only; direct integration creates mobility and licensing constraints |
| GPU real-time effects engine in MVP | Too large for MVP; Phase 11 and Phase 18 now plan realtime preview/effects after Phase 10.1 |
| Mobile apps and cloud rendering in MVP | Desktop MVP comes first; Phase 17 now plans portable binding/server runtime foundations after Phase 10.1 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| FOUND-01 | Phase 1 | Complete |
| FOUND-02 | Phase 1 | Complete |
| FOUND-03 | Phase 1 | Complete |
| FOUND-04 | Phase 1 | Complete |
| DRAFT-01 | Phase 2 | Complete |
| DRAFT-02 | Phase 2 | Complete |
| DRAFT-03 | Phase 2 | Complete |
| DRAFT-04 | Phase 2 | Complete |
| DRAFT-05 | Phase 2 | Complete |
| MAT-01 | Phase 2 | Complete |
| MAT-02 | Phase 2 | Complete |
| MAT-03 | Phase 2 | Complete |
| MAT-04 | Phase 2 | Complete |
| TIME-01 | Phase 3 | Complete |
| TIME-02 | Phase 3 | Complete |
| TIME-03 | Phase 3 | Complete |
| TIME-04 | Phase 3 | Complete |
| TIME-05 | Phase 3 | Complete |
| TIME-06 | Phase 3 | Complete |
| TIME-07 | Phase 3 | Complete |
| TEXT-01 | Phase 3 | Complete |
| TEXT-02 | Phase 3 | Complete |
| TEXT-03 | Phase 5 | Complete |
| AUD-01 | Phase 3 | Complete |
| AUD-02 | Phase 3 | Complete |
| PREV-01 | Phase 5 | Complete |
| PREV-02 | Phase 5 | Complete |
| PREV-03 | Phase 5 | Complete |
| PREV-04 | Phase 5 | Complete |
| EXP-01 | Phase 5 | Complete |
| EXP-02 | Phase 5 | Complete |
| EXP-03 | Phase 5 | Complete |
| EXP-04 | Phase 5 | Complete |
| UI-01 | Phase 4 | Complete |
| UI-02 | Phase 4 | Complete |
| UI-03 | Phase 4 | Complete |
| UI-04 | Phase 4 | Complete |
| UI-05 | Phase 4 | Complete |
| UI-06 | Phase 4 | Complete |
| UI-07 | Phase 04.1 | Complete |
| UI-08 | Phase 04.1 | Complete |
| UI-09 | Phase 04.1 | Complete |
| UI-10 | Phase 04.1 | Complete |
| UI-11 | Phase 04.1 | Complete |
| UI-12 | Phase 04.1 | Complete |
| TEST-01 | Phase 1 | Complete |
| TEST-02 | Phase 3 | Complete |
| TEST-03 | Phase 5 | Complete |
| TEST-04 | Phase 5 | Complete |
| TEST-05 | Phase 5 | Complete |
| TEST-06 | Phase 6 | Complete |
| TEST-07 | Phase 6 | Complete |
| TEST-08 | Phase 04.1 | Complete |
| CANVAS-01 | Phase 7 | Complete |
| CANVAS-02 | Phase 7 | Complete |
| CANVAS-03 | Phase 7 | Complete |
| CANVAS-04 | Phase 7 | Complete |
| XFORM-01 | Phase 8 | Complete |
| XFORM-02 | Phase 8 | Complete |
| XFORM-03 | Phase 8 | Complete |
| LAYER-01 | Phase 8 | Complete |
| LAYER-02 | Phase 8 | Complete |
| LAYER-03 | Phase 8 | Complete |
| TEXT2-01 | Phase 9 | Complete |
| TEXT2-02 | Phase 9 | Complete |
| TEXT2-03 | Phase 9 | Complete |
| ANIM-01 | Phase 10 | Complete |
| ANIM-02 | Phase 10 | Complete |
| ANIM-03 | Phase 10 | Complete |
| MVPEDIT-01 | Phase 10.1 | Complete |
| MVPEDIT-02 | Phase 10.1 | Complete |
| MVPEDIT-03 | Phase 10.1 | Complete |
| MVPEDIT-04 | Phase 10.1 | Complete |
| MVPEDIT-05 | Phase 10.1 | Complete |
| MVPEDIT-06 | Phase 10.1 | Complete |
| MVPEDIT-07 | Phase 10.1 | Complete |
| MVPEDIT-08 | Phase 10.1 | Complete |
| MVPEDIT-09 | Phase 10.1 | Complete |
| RTPREV-01 | Phase 11 | Planned |
| RTPREV-02 | Phase 11 | Planned |
| RTPREV-03 | Phase 11 | Planned |
| RTPREV-04 | Phase 11 | Planned |
| RTPREV-05 | Phase 11 | Planned |
| MEDIAIO-01 | Phase 12 | Planned |
| MEDIAIO-02 | Phase 12 | Planned |
| MEDIAIO-03 | Phase 12 | Planned |
| MEDIAIO-04 | Phase 12 | Planned |
| MEDIAIO-05 | Phase 12 | Planned |
| INCR-01 | Phase 13 | Planned |
| INCR-02 | Phase 13 | Planned |
| INCR-03 | Phase 13 | Planned |
| INCR-04 | Phase 13 | Planned |
| INCR-05 | Phase 13 | Planned |
| ASSET-01 | Phase 14 | Planned |
| ASSET-02 | Phase 14 | Planned |
| ASSET-03 | Phase 14 | Planned |
| ASSET-04 | Phase 14 | Planned |
| ASSET-05 | Phase 14 | Planned |
| AUDIO2-01 | Phase 15 | Planned |
| AUDIO2-02 | Phase 15 | Planned |
| AUDIO2-03 | Phase 15 | Planned |
| AUDIO2-04 | Phase 15 | Planned |
| SCHED-01 | Phase 16 | Planned |
| SCHED-02 | Phase 16 | Planned |
| SCHED-03 | Phase 16 | Planned |
| SCHED-04 | Phase 16 | Planned |
| BIND-01 | Phase 17 | Planned |
| BIND-02 | Phase 17 | Planned |
| BIND-03 | Phase 17 | Planned |
| BIND-04 | Phase 17 | Planned |
| BIND-05 | Phase 17 | Planned |
| PRODFX-01 | Phase 18 | Planned |
| PRODFX-02 | Phase 18 | Planned |
| PRODFX-03 | Phase 18 | Planned |
| PRODFX-04 | Phase 18 | Planned |
| PRODFX-05 | Phase 18 | Planned |
| COMP-01 | Post-MVP | Deferred |
| COMP-02 | Post-MVP | Deferred |
| COMP-03 | Post-MVP | Deferred |
| PLAT-01 | Phase 17 | Planned |
| PLAT-02 | Phase 17 | Planned |
| PLAT-03 | Phase 17 | Planned |

**Coverage:**

- v1 requirements: 52 total
- Mapped to phases: 52
- Unmapped: 0
- v2/post-MVP requirements: 69 total, 66 planned in Phases 7-18 and 3 deferred

---
*Requirements defined: 2026-06-17*
*Last updated: 2026-06-18 after adding production-grade architecture Phases 11-18 after Phase 10.1*
