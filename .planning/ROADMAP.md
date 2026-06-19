# Roadmap: Video Editor

## Overview

This milestone builds a desktop-first Jianying-style video editor MVP on a Rust-owned core. The path is intentionally layered: establish the engineering and test harness, define the draft/material model, implement command-driven timeline editing, build the desktop workspace, connect preview/export through one render path, then harden and package the app.

After Phase 10.1 closes the usable MVP gap, the roadmap shifts from MVP delivery to production-grade editor architecture. The next phases address the known architectural ceiling directly: realtime GPU preview, hardware media IO, incremental render graph updates, coherent derived artifacts, resource management, an independent audio pipeline, task scheduling, mobile/server bindings, and then production retiming/effects/transitions on top of that foundation. FFmpeg remains a final export/encoding runtime, not the default realtime preview renderer for supported interactive editing paths.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation And Golden Harness** - Rust/Electron workspace, binding path, FFmpeg discovery, and deterministic test fixtures (completed 2026-06-16)
- [x] **Phase 2: Draft And Material System** - `.veproj` draft bundle, Jianying-aligned schema, material import/probing, and save/open validation (completed 2026-06-17)
- [x] **Phase 3: Timeline Command Core** - Track/segment model, command API, undo/redo, snapping, text/audio basics (completed 2026-06-17)
- [x] **Phase 4: Jianying-Style Desktop Workspace** - Editor UI shell matching Jianying workspace structure and command-only core integration (completed 2026-06-17)
- [x] **Phase 04.1: Professional Jianying Workspace UI Refinement** - Higher-density Jianying Pro-like desktop workspace refinement (completed 2026-06-17)
- [x] **Phase 5: Preview And Export Pipeline** - Shared render graph path for preview frames, preview cache, and MP4 export (completed 2026-06-18)
- [x] **Phase 6: MVP Hardening And Packaging** - End-to-end verification, packaged app smoke tests, license manifest, and release readiness (completed 2026-06-17)
- [x] **Phase 7: Project Canvas Space And Coordinate System** - Draft-level aspect ratio, canvas size, frame rate, background, and normalized coordinate semantics (completed 2026-06-18)
- [x] **Phase 8: Segment Transform And Visual Compositing** - Jianying-style 画面/基础/变换 controls, layer ordering, visibility, fit/fill/stretch, and composition semantics (completed 2026-06-18)
- [x] **Phase 9: Complete Text And Subtitle System** - Font references, text box layout, line height, letter spacing, safe areas, and multi-segment subtitle/text parity (completed 2026-06-18)
- [x] **Phase 10: Typed Keyframe And Animation System** - Typed animated values, easing curves, and frame-time animation evaluation for transform/text/sticker/effect parameters (completed 2026-06-18)
- [x] **Phase 10.1: Usable Editor MVP Completion** - System file import, real preview image display, playhead seeking, and basic video/audio/text/subtitle editing usability (completed 2026-06-18)
- [x] **Phase 11: Realtime Preview Runtime And GPU Render Backend** - Production realtime preview runtime, GPU compositor backend, frame pacing, and preview/export parity diagnostics (completed 2026-06-18)
- [x] **Phase 12: Media IO, Hardware Decode, And Frame/Texture Interop** - Platform media reader/decoder abstraction, hardware decode capability reporting, frame pools, and low-copy texture handoff (completed 2026-06-19)
- [x] **Phase 13: Incremental Render Graph, Dirty Ranges, And Cache Coherence** - Stable graph IDs, graph diffing, dirty range propagation, undo/redo-aware graph snapshots, and cache invalidation contracts (completed 2026-06-19)
- [x] **Phase 14: Asset Resource Manager And Derived Artifact Store** - Material/resource index, proxy/thumbnail/waveform pipelines, artifact manifests, versioning, replacement invalidation, and cache GC (completed 2026-06-19)
- [x] **Phase 15: Audio Engine And DSP Timeline Pipeline** - Low-latency audio graph, DSP timeline semantics, preview playback sync, waveform integration, and export parity (completed 2026-06-19)
- [ ] **Phase 15.1: P0 Basic Editing Chain Repair** - Production playback, baseline text/audio preview parity, first-material canvas adaptation, multitrack editing, and full user-chain acceptance before scheduler work (INSERTED)
- [ ] **Phase 15.2: P0 Jianying-Style Production UI Convergence** - Remove debug-console UI, align the five-zone Jianying-style production workspace, modal export, focused inspector, and screenshot-backed regression before scheduler work (INSERTED)
- [ ] **Phase 16: Task Scheduler, Job Isolation, And Performance Telemetry** - Priority queues, cancellation, backpressure, thread-pool isolation, export/preview/cache separation, and performance budgets
- [ ] **Phase 17: Mobile/Server Binding Architecture And Runtime Ports** - Node-API/C ABI/JNI/Swift binding split, lifecycle and permission contracts, texture/file handles, and server runtime boundary
- [ ] **Phase 18: Production Effects, Retiming, And Transition Semantics** - Restore retiming, effects, filters, masks, and transitions on top of the production preview/cache/audio/runtime foundation

## Phase Details

### Phase 1: Foundation And Golden Harness

**Goal**: Create the buildable repo foundation and test harness that every later phase depends on.
**Depends on**: Nothing (first phase)
**Requirements**: FOUND-01, FOUND-02, FOUND-03, FOUND-04, TEST-01
**Success Criteria** (what must be TRUE):

  1. Developer can run one command to build the Rust workspace and Electron desktop shell.
  2. Electron can call a minimal Rust binding and receive a typed response.
  3. FFmpeg and ffprobe discovery works and reports a clear error when binaries are missing.
  4. Golden fixture structure exists and CI can run schema validation plus one tiny render smoke.

**Plans**: 9 plans
Plans:

- [x] 01-01-PLAN.md - Create root tooling, workspace manifests, and command entrypoints
- [x] 01-02-PLAN.md - Create pure Rust semantic crate shells and command contracts
- [x] 01-03-PLAN.md - Create service-boundary crate shells and runtime-boundary docs
- [x] 01-04-PLAN.md - Implement the Node-API binding crate
- [x] 01-05-PLAN.md - Implement FFmpeg/ffprobe discovery and command-envelope runtime probe
- [x] 01-06-PLAN.md - Generate schema/TypeScript contracts and validate fixtures
- [x] 01-07-PLAN.md - Add tiny FFmpeg render smoke harness
- [x] 01-08-PLAN.md - Create minimal Electron shell and binding smoke
- [x] 01-09-PLAN.md - Finalize `just` build/test gates and CI

### Phase 2: Draft And Material System

**Goal**: Establish `.veproj` drafts, Jianying-aligned schema concepts, material import/probing, and save/open integrity.
**Depends on**: Phase 1
**Requirements**: DRAFT-01, DRAFT-02, DRAFT-03, DRAFT-04, DRAFT-05, MAT-01, MAT-02, MAT-03, MAT-04
**Success Criteria** (what must be TRUE):

  1. User can create, save, close, and reopen a `.veproj` draft without semantic changes.
  2. Draft schema uses Jianying terms consistently across Rust model, schema, commands, tests, and docs.
  3. User can import video, image, and audio materials and see probed metadata in the material bin.
  4. Missing material detection surfaces a recoverable state without corrupting the draft.

**Plans**: 6 plans
Plans:
**Wave 1**

- [x] 02-01-PLAN.md - Define draft/material/track/segment schema and migration hooks

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 02-02-PLAN.md - Implement project bundle persistence, path helpers, and round-trip tests
- [x] 02-03-PLAN.md - Implement media-runtime probing and generated material test fixtures

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 02-04-PLAN.md - Implement material import service, missing diagnostics, and draft contracts

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 02-05-PLAN.md - Expose material commands through bindings and Electron smoke

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 02-06-PLAN.md - Add draft/material fixtures and final Phase 2 gates

### Phase 3: Timeline Command Core

**Goal**: Implement Rust-owned timeline editing semantics and command/undo behavior before rich UI relies on it.
**Depends on**: Phase 2
**Requirements**: TIME-01, TIME-02, TIME-03, TIME-04, TIME-05, TIME-06, TIME-07, TEXT-01, TEXT-02, AUD-01, AUD-02, TEST-02
**Success Criteria** (what must be TRUE):

  1. User-visible edits are represented as typed commands and cannot mutate the draft partially on failure.
  2. User can add, select, move, split, trim, and delete video/audio/text segments.
  3. Undo/redo works for every committed MVP edit.
  4. Main-track magnet/snapping is computed in Rust core and covered by command tests.

**Plans**: 5/5 plans complete

Plans:

**Wave 1**

- [x] 03-01-PLAN.md - Implement track/segment timeline model and overlap/stacking rules

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03-02-PLAN.md - Implement add, move, split, trim, delete, select, and invalid-edit rejection commands

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 03-03-PLAN.md - Implement undo/redo, snapping/main-track magnet, and command event output

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 03-04-PLAN.md - Implement MVP text/audio semantic commands and direct tests

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 03-05-PLAN.md - Add fixtures, source guards, and final Phase 3 gates

### Phase 4: Jianying-Style Desktop Workspace

**Goal**: Build the desktop editor workspace with Jianying-like structure and command-only integration to the Rust core.
**Depends on**: Phase 3
**Requirements**: UI-01, UI-02, UI-03, UI-04, UI-05, UI-06, TEST-06
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. User sees a Jianying-like workspace: top feature categories, left material/function panel, center preview, right inspector, bottom timeline.
  2. User can import materials, add/edit segments, edit text/audio values, and see draft state update through Rust commands.
  3. UI consistently uses Jianying concepts and does not expose alternate internal jargon.
  4. Desktop UI uses Simplified Chinese for user-visible copy by default.
  5. Timeline and panel layout remain stable during selection, hover, drag, and playback updates.

**Plans**: 4 plans
Plans:
**Wave 1**

- [x] 04-01-PLAN.md - Build desktop workspace shell, layout system, and feature category navigation

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 04-02-PLAN.md - Implement material, text, and audio panels plus right inspector

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 04-03-PLAN.md - Implement timeline interaction surface wired only through command API

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 04-04-PLAN.md - Add Electron IPC contracts, Playwright smoke flow, and visual layout checks

### Phase 04.1: Professional Jianying Workspace UI Refinement (INSERTED)

**Goal:** Upgrade the existing Jianying-style MVP workspace into a higher-density professional desktop editor UI that is closer to Jianying Pro's workstation structure while preserving original assets and command-only Rust integration.
**Requirements**: UI-07, UI-08, UI-09, UI-10, UI-11, UI-12, TEST-08
**Depends on:** Phase 4
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Top feature area, left resource panel, center preview shell, right inspector, and bottom timeline all use compact professional editor density with Simplified Chinese Jianying-style terminology.
  2. Deferred categories such as stickers, effects, transitions, subtitles, filters, adjustment, templates, and digital human remain visible with Chinese not-yet-connected states rather than disappearing.
  3. Timeline editing controls still route through `window.videoEditorCore.executeCommand`; renderer code does not mutate draft tracks/segments/timeranges or own undo/redo behavior.
  4. Preview and export UI shells are ready for Phase 5 integration but do not implement real render graph or FFmpeg execution in the renderer.
  5. Playwright Electron checks cover 1280x800 and 1120x720 with no region overlap/clipping and source guards enforce the UI/Rust/render boundary.

**Plans:** 4/4 plans complete

Plans:

- [x] 04.1-01: Refine top feature area and left resource/function panel density
- [x] 04.1-02: Refine preview monitor shell and Jianying-style inspector tabs/controls
- [x] 04.1-03: Refine timeline toolbar, track headers, segment visuals, ruler, playhead, snapping, and zoom states
- [x] 04.1-04: Add professional workspace Playwright coverage, source guards, and visual regression gates

### Phase 5: Preview And Export Pipeline

**Goal**: Connect the Rust editing model to a shared render graph path for preview and final MP4 export.
**Depends on**: Phase 04.1
**Requirements**: TEXT-03, PREV-01, PREV-02, PREV-03, PREV-04, EXP-01, EXP-02, EXP-03, EXP-04, TEST-03, TEST-04, TEST-05
**Success Criteria** (what must be TRUE):

  1. User can seek and preview deterministic frames generated from the same resolved draft semantics used for export.
  2. User can play a short cached preview segment and edits invalidate only affected cache ranges.
  3. User can export H.264 MP4 with progress, cancellation, logs, and classified errors.
  4. Golden tests cover normalized draft, frame state, render graph, FFmpeg script, preview/export parity, and output metadata.

**Plans**: 9/9 plans complete

Plans:

**Wave 1**

- [x] 05-01-PLAN.md - Implement engine_core normalization, frame state, and deterministic text layout

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 05-02-PLAN.md - Implement the typed renderer-neutral render graph

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 05-03-PLAN.md - Implement FFmpeg compiler jobs, filters, ASS sidecars, and snapshots

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 05-04-PLAN.md - Implement preview_service cache, generation, and invalidation
- [x] 05-07-PLAN.md - Implement export runtime progress, cancel, logs, and output validation

**Wave 5** *(blocked on Wave 4 preview completion)*

- [x] 05-05-PLAN.md - Add preview command contracts, binding routes, and renderer envelope helpers

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 05-06-PLAN.md - Add preview UI, source guards, and automated screenshots

**Wave 7** *(blocked on Wave 6 and export runtime completion)*

- [x] 05-08-PLAN.md - Add export contracts, binding registry, UI, and automated screenshots

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 05-09-PLAN.md - Add preview/export parity, final source guards, and root gates

### Phase 6: MVP Hardening And Packaging

**Goal**: Verify the full import-edit-preview-export workflow in dev and packaged desktop builds.
**Depends on**: Phase 5
**Requirements**: TEST-06, TEST-07
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Packaged app launches offline and loads the Rust binding and FFmpeg runtime.
  2. Electron E2E test imports material, edits timeline, previews, exports, and verifies output.
  3. Release artifacts include FFmpeg license/build manifest and third-party notices.
  4. MVP has a documented known-limits list and clear next-phase backlog for adapters, advanced effects, and platform expansion.

**Plans**: 5 plans

Plans:

- [x] 06-01: Add packaged app build, native binding loading, and bundled runtime checks
- [x] 06-02: Add Rust-owned runtime capability report and generated command route
- [x] 06-03: Display Rust-owned runtime diagnostics in the Chinese preview shell
- [x] 06-04: Add dev and packaged no-mock import-preview-export workflow gates
- [x] 06-05: Document known limits, license posture, and post-MVP backlog

### Phase 7: Project Canvas Space And Coordinate System

**Goal**: Establish the draft-level project canvas space that all later transform, sticker, text, PIP, background, and render graph behavior depends on.
**Depends on**: Phase 6
**Requirements**: CANVAS-01, CANVAS-02, CANVAS-03, CANVAS-04
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Draft schema has a canonical project canvas/profile model for aspect ratio, canvas width/height, frame rate, and background.
  2. Canvas background supports black, solid color, blur fill, and image background as semantic options, with unsupported render modes classified.
  3. All visual coordinates use one documented normalized coordinate system that later sticker/text/PIP/transform/keyframe behavior can share.
  4. Desktop UI exposes Chinese project canvas settings using Jianying-style terms and routes changes through Rust commands.

**Plans**: 7 plans

Plans:

**Wave 1**

- [x] 07-01-PLAN.md - Define draft canvas model, validation, coordinate docs, and focused tests

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 07-02-PLAN.md - Update fixtures, schema export wiring, and generated canvas contracts

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 07-03-PLAN.md - Add undoable Rust canvas command and binding route
- [x] 07-04-PLAN.md - Propagate canvas profile through engine, render graph, and FFmpeg compiler

**Wave 4** *(blocked on Wave 3 render/compiler completion)*

- [x] 07-05-PLAN.md - Propagate canvas profile through preview and export services

**Wave 5** *(blocked on Wave 3 command route and Wave 4 service propagation)*

- [x] 07-06-PLAN.md - Add Chinese inspector and preview canvas UI with Playwright workspace gates

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 07-07-PLAN.md - Add Phase 07 source guards and public root gates

### Phase 8: Segment Transform And Visual Compositing

**Goal**: Implement Jianying-style segment-level 画面 / 基础 / 变换 semantics and deterministic visual layer composition.
**Depends on**: Phase 7
**Requirements**: XFORM-01, XFORM-02, XFORM-03, LAYER-01, LAYER-02, LAYER-03
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Segment semantics support position x/y, scale, rotation, opacity, crop, anchor, and fit/fill/stretch modes using typed persisted values.
  2. Background fill handles vertical media in horizontal canvases and similar aspect-ratio mismatches without renderer-owned layout math.
  3. Video, image, sticker, and text layers have explicit stacking and visibility semantics that render_graph can compile deterministically.
  4. Blend mode and mask are represented as capability-aware semantic boundaries even if first implementation degrades or defers them.
  5. UI inspector controls use Jianying-style Chinese labels and all edits route through Rust commands with undo/redo coverage.

**Plans**: 5/5 plans complete

Plans:

**Wave 1**

- [x] 08-01-PLAN.md - Add typed visual segment model and Rust-owned updateSegmentVisual command

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 08-02-PLAN.md - Propagate segment visual semantics through engine frame state and render graph

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 08-03-PLAN.md - Compile supported transform subset and clear stale derived UI state after visual edits

**Wave 4** *(blocked on Wave 3 render/compiler completion)*

- [x] 08-04-PLAN.md - Expose selected-segment visual controls in the desktop inspector

**Wave 5** *(blocked on Wave 4 UI completion)*

- [x] 08-05-PLAN.md - Add Phase 08 source guards, public gates, and verification closure

### Phase 9: Complete Text And Subtitle System

**Goal**: Upgrade MVP text into a complete Jianying-style text/subtitle model suitable for real templates.
**Depends on**: Phase 8
**Requirements**: TEXT2-01, TEXT2-02, TEXT2-03
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Text semantics include font references, font size, color, stroke, shadow, background, alignment, text box width/height, line height, letter spacing, and safe-area/layout region.
  2. Multiple text and subtitle segments render through the shared preview/export path with stable layout snapshots.
  3. Text inspector UI uses Chinese Jianying-style terms and does not invent a separate internal vocabulary.
  4. Unsupported text effects or proprietary text bubbles are classified as degraded/unsupported rather than silently treated as supported.

**Plans**: 5 plans

Plans:

**Wave 1**

- [x] 09-01-PLAN.md - Extend text/subtitle schema, validation, and generated contracts

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 09-02-PLAN.md - Propagate complete text semantics through engine, render graph, and ASS compiler

**Wave 3** *(blocked on Wave 1 and Wave 2 completion)*

- [x] 09-03-PLAN.md - Add Rust-owned subtitle SRT import and binding coverage

**Wave 4** *(blocked on Wave 1 and Wave 3 completion)*

- [x] 09-04-PLAN.md - Add Jianying-style text/subtitle desktop controls and Playwright coverage

**Wave 5** *(blocked on Wave 4 UI completion)*

- [x] 09-05-PLAN.md - Add Phase 09 source guards, public gates, and verification closure

### Phase 10: Typed Keyframe And Animation System

**Goal**: Replace string-like keyframe placeholders with typed animated values and easing so templates can express dynamic motion.
**Depends on**: Phase 9
**Requirements**: ANIM-01, ANIM-02, ANIM-03
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Draft schema supports typed animated values for position, scale, rotation, opacity, text parameters, sticker parameters, filter parameters, and volume where applicable.
  2. Keyframes include time, value type, interpolation policy, and easing curve with deterministic evaluation at frame time.
  3. engine_core and render_graph evaluate animation without UI-owned interpolation or naked floating-point persisted time.
  4. Desktop UI exposes keyframe controls in the inspector and timeline while routing all mutations through Rust commands.

**Plans**: 5 plans

Plans:

**Wave 1**

- [x] 10-01-PLAN.md - Define typed keyframe schema, validation, and generated contracts

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 10-02-PLAN.md - Add Rust-owned keyframe commands and binding coverage

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 10-03-PLAN.md - Evaluate keyframes in engine/render graph and compiler diagnostics

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 10-04-PLAN.md - Expose Jianying-style keyframe controls in desktop inspector and timeline

**Wave 5** *(blocked on Wave 4 UI completion)*

- [x] 10-05-PLAN.md - Add Phase 10 source guards, public gates, and verification closure

### Phase 10.1: Usable Editor MVP Completion (INSERTED)

**Goal:** Close the gap between a demo workspace and a truly usable Jianying/CapCut-style MVP editor by making import, timeline seeking, real preview display, and basic video/audio/text/subtitle editing work end-to-end through Rust commands.
**Requirements**: MVPEDIT-01, MVPEDIT-02, MVPEDIT-03, MVPEDIT-04, MVPEDIT-05, MVPEDIT-06, MVPEDIT-07, MVPEDIT-08, MVPEDIT-09
**Depends on:** Phase 10
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. User can click `导入素材`, choose video/audio/image files through a system file chooser, and see imported materials with kind, duration, dimensions or audio metadata, and availability in the left material panel.
  2. User can add compatible video/image/audio/text/subtitle materials to timeline tracks, select segments, move, trim, split, delete, undo, redo, and keep snapping/main-track magnet behavior Rust-owned.
  3. User can click/drag the timeline playhead, use time input, and step previous/next frame; playhead changes request and display the current real preview frame.
  4. The preview monitor displays the returned PNG frame inside the canvas at the draft canvas aspect ratio, with black/empty canvas fallback when no visible segment exists.
  5. Video/image/text transform, text styling, audio volume, and track mute edits are exposed in the UI and route through `updateSegmentVisual`, `editTextSegment`, `setSegmentVolume`, and `setTrackMute`.
  6. SRT import is Rust-parsed through `importSubtitleSrt`, creates editable text/subtitle segments, and renderer code never constructs subtitle cue segments directly.
  7. Selection state is visible in the timeline, preview selection overlay, and inspector; no-selection state shows draft parameters.
  8. Electron renderer remains sandboxed: no direct Node/Electron access, no direct draft mutation, no FFmpeg/ffprobe/render graph/export script construction.
  9. Playwright Electron tests cover real file import, add-to-timeline, preview image display, playhead request/update, transform, audio, text, and SRT flows at 1280x800 and 1120x720.

**Plans:** 7/7 plans complete

Plans:

- [x] 10.1-01-PLAN.md - Add system file chooser import and material list usability gates
- [x] 10.1-02-PLAN.md - Display real preview PNG frames and empty canvas fallback
- [x] 10.1-03-PLAN.md - Make timeline playhead seek/click/drag/frame-step request preview frames
- [x] 10.1-04-PLAN.md - Complete video/image transform editing and preview selection overlay
- [x] 10.1-05-PLAN.md - Complete text add/edit/style display and preview parity
- [x] 10.1-06-PLAN.md - Complete audio segment volume, track mute, and waveform placeholder usability
- [x] 10.1-07-PLAN.md - Complete SRT import/edit flow, source guards, and phase gates

### Phase 11: Realtime Preview Runtime And GPU Render Backend

**Goal**: Introduce a Rust-side production realtime preview runtime that renders supported interactive timelines through `wgpu` on Windows/macOS instead of spawning FFmpeg/filter graphs for every preview frame.
**Depends on**: Phase 10.1
**Requirements**: RTPREV-01, RTPREV-02, RTPREV-03, RTPREV-04, RTPREV-05
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Preview requests flow through a Rust-owned `RealtimePreviewRuntime` that consumes accepted draft semantics and render graph intent.
  2. Supported video/image/text/layer transform, opacity, canvas, and keyframe state renders through `wgpu`, targeting D3D12 on Windows and Metal on macOS.
  3. Seeking, scrubbing, and basic playback do not spawn a new FFmpeg process per frame for supported preview paths.
  4. Realtime preview, audio preview, and scheduled preview tasks use a shared integer-microsecond `TimelineClock` plus `PlaybackGeneration` so stale frames are rejected after seek/edit.
  5. Realtime preview and export continue to share engine/render graph semantics, with parity tests and diagnostics for known divergences.
  6. Preview runtime reports measurable first-frame, seek, frame pacing, stale-generation rejection, cancellation, and fallback telemetry.

**Plans:** 10/10 plans complete

Plans:

- [x] 11-01 - Realtime preview runtime crate shell, `TimelineClock`, `PlaybackGeneration`, session/request/result/telemetry contracts
- [x] 11-02 - Render graph preparation, capability classifier, and preview/export parity diagnostics
- [x] 11-03 - Frame provider contracts and H.264 software video frame cache
- [x] 11-03B - `wgpu` device/offscreen compositor and supported canvas/image/video textured quad subset
- [x] 11-04 - Rust native surface contracts and thin Node-API realtime preview bindings
- [x] 11-04B - Electron main/preload/renderer native preview host bridge and rect smoke tests
- [x] 11-05 - Preview service realtime backend integration, no-per-frame-FFmpeg supported path, fallback ladder diagnostics
- [x] 11-05B - Desktop telemetry/fallback display without renderer-owned fallback decisions
- [x] 11-06 - Text parity gate and realtime/export diagnostic coverage
- [x] 11-07 - Source guards, runtime boundary docs, and final Phase 11 gate scripts

### Phase 12: Media IO, Hardware Decode, And Frame/Texture Interop

**Goal**: Split media reading/decoding from FFmpeg-specific execution and introduce Windows/macOS native media IO, hardware decode, frame pools, and texture ownership contracts.
**Depends on**: Phase 11
**Requirements**: MEDIAIO-01, MEDIAIO-02, MEDIAIO-03, MEDIAIO-04, MEDIAIO-05
**Success Criteria** (what must be TRUE):

  1. Media runtime exposes reader/decoder traits and capability reports rather than binding preview/decode semantics directly to FFmpeg process execution.
  2. Windows runtime reports Media Foundation / DXVA / D3D texture capabilities; macOS runtime reports AVFoundation / VideoToolbox / CoreVideo / Metal texture capabilities.
  3. Decoded frames use explicit frame pools, lifetimes, color metadata, and CPU/GPU handle types instead of ad hoc byte buffers.
  4. Realtime preview can consume frame or texture handles without full 4K pixel buffers crossing the JS/Rust boundary.
  5. FFmpeg remains available as fallback/probe/export/transcode implementation, with unsupported codecs, pixel formats, color spaces, and hardware paths degrading predictably.

**Plans:** 9/9 plans executed

Plans:

- [x] 12-01 - Shared media IO traits, frame pool leases, color metadata, texture/device identity, fallback reason contracts
- [x] 12-02 - Desktop native/FFmpeg media IO capability reporting
- [x] 12-02B - Binding/schema/source guards and platform dependency verification checkpoint
- [x] 12-03 - FFmpeg CPU frame fallback decoder and structured fallback ladder
- [x] 12-04 - macOS AVFoundation/VideoToolbox/CoreVideo/Metal frame and texture path
- [x] 12-05 - Windows Media Foundation/DXVA/D3D frame and texture path
- [x] 12-06 - Phase 11 media IO handoff adapter
- [x] 12-06B - Handle-based preview decode binding/release contracts
- [x] 12-06C - Release/session-close leak tests, final Phase 12 gates, and manual platform verification notes

### Phase 13: Incremental Render Graph, Dirty Ranges, And Cache Coherence

**Goal**: Make the semantic timeline and render graph update incrementally so large drafts do not require full graph regeneration and cache invalidation after every edit.
**Depends on**: Phase 12
**Requirements**: INCR-01, INCR-02, INCR-03, INCR-04, INCR-05
**Success Criteria** (what must be TRUE):

  1. Render graph nodes have stable identities tied to semantic draft entities, not content hashes alone.
  2. Accepted draft commands emit `CommandDelta` data with changed entity IDs, changed domains, and changed integer-microsecond time ranges.
  3. Dirty range propagation spans preview, export preparation, audio, thumbnails, waveforms, proxies, and preview cache without naked floating-point time.
  4. Undo/redo restores semantic state and either restores matching graph/cache snapshots or invalidates affected ranges deterministically.
  5. Large-timeline tests cover graph diff cost, cache invalidation correctness, and preview/export consistency after edits.

**Plans:** 8/8 plans complete

Plans:

- [x] 13-01 - Validation harness, source guards, package scripts, and large-timeline fixture helpers
- [x] 13-02 - `CommandDelta` core types, range helpers, and simple command delta emission
- [x] 13-02B - Schema/TypeScript/generated contract export for delta types
- [x] 13-03 - Text/audio/visual/keyframe/canvas/material domain coverage and undo/redo invalidation
- [x] 13-04 - Stable render graph node IDs, fingerprints, graph snapshots, and graph diff helpers
- [x] 13-05 - Preview cache key v2, invalidation request v2, dirty consumer expansion, and export-prep dirty facts
- [x] 13-05B - Binding-safe invalidation contracts and generated schema/TypeScript updates
- [x] 13-06 - Large-timeline, preview/export parity, source guard, and final Phase 13 gates

### Phase 14: Asset Resource Manager And Derived Artifact Store

**Goal**: Add a production resource layer backed by a project-local SQLite artifact index and derived blob store for materials, fonts, effects, proxies, thumbnails, waveforms, graph snapshots, and preview artifacts.
**Depends on**: Phase 13
**Requirements**: ASSET-01, ASSET-02, ASSET-03, ASSET-04, ASSET-05
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Asset manager indexes materials, proxies, thumbnails, waveforms, fonts, and supported effect resources with stable IDs and project-relative references.
  2. `.veproj/derived/artifact-store.sqlite` tracks derived artifacts, dependencies, dirty state, generation status, schema version, runtime capability fingerprint, source material fingerprint, graph fingerprint, and generation parameters.
  3. Replacing, relinking, renaming, or deleting source media invalidates or regenerates exactly the affected artifacts.
  4. Proxy, thumbnail, and waveform generation is chunked, resumable, cancellable, and isolated from interactive preview responsiveness.
  5. Cache garbage collection, storage quotas, and optional cloud/server synchronization manifests are defined before remote rendering depends on them.

**Plans**: 7/7 plans complete
Plans:

- [x] 14-01 - Artifact store crate, SQLite schema, blob paths, and initial guards
- [x] 14-02 - Resource index and dependency rows
- [x] 14-03 - Dependency-driven artifact invalidation
- [x] 14-04 - Generation jobs plus proxy, thumbnail, and waveform artifact generation
- [x] 14-05 - GC, quota, and local sync manifest semantics
- [x] 14-06 - Generated artifact contracts and Node binding commands
- [x] 14-07 - Production resource status UI and final gates

### Phase 15: Audio Engine And DSP Timeline Pipeline

**Goal**: Introduce an independent low-latency audio graph and DSP timeline synchronized to the same `TimelineClock` and `PlaybackGeneration` used by `wgpu` preview rendering.
**Depends on**: Phase 14
**Requirements**: AUDIO2-01, AUDIO2-02, AUDIO2-03, AUDIO2-04
**Success Criteria** (what must be TRUE):

  1. Audio preview playback uses a dedicated audio graph with shared `TimelineClock`, seek, pause, cancel, and buffering behavior independent from FFmpeg preview frame generation.
  2. Segment gain, track mute, pan, fades, keyframed volume, and future audio effects have typed DSP semantics with integer/rational timeline mapping.
  3. Windows preview audio output uses WASAPI; macOS preview audio output uses CoreAudio.
  4. Waveform and peak data from the artifact store drive UI display without becoming canonical audio semantics.
  5. Export audio mixdown remains parity-tested against the preview audio graph with classified differences.

**Plans**: 7 plans

Plans:

**Wave 1**

- [x] 15-01-PLAN.md - Define audio DSP semantic contracts and command-owned audio edit deltas

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 15-02-PLAN.md - Create audio_engine DSP timeline, mix intent, sessions, buffers, output traits, and telemetry

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 15-03-PLAN.md - Add desktop CoreAudio/WASAPI output boundary behind package legitimacy and native-proof gates
- [x] 15-06-PLAN.md - Connect audio mix intent to export compilation and preview/export parity diagnostics

**Wave 4** *(blocked on Wave 3 output completion)*

- [x] 15-04-PLAN.md - Expose generated audio preview, device, and waveform binding contracts

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 15-05-PLAN.md - Add production audio preview, waveform, device-status, and audio editing UI

**Wave 6** *(blocked on Waves 3-5 completion)*

- [x] 15-07-PLAN.md - Add final Phase 15 source guards and aggregate verification gates

### Phase 15.1: P0 Basic Editing Chain Repair (INSERTED)

**Goal**: Make the default editor chain behave like a production video editor before scheduler work: realtime playback must be the normal path, baseline video/text/audio preview must not rely on fallback success, first media should establish an appropriate canvas, multitrack editing must be explicit, and import/edit/play/save/reopen/export must be a hard user-flow gate.
**Depends on**: Phase 15
**Requirements**: P0-EDIT-01, P0-EDIT-02, P0-EDIT-03, P0-EDIT-04, P0-EDIT-05, P0-EDIT-06
**Success Criteria** (what must be TRUE):

  1. Clicking play does not continuously trigger `requestPreviewFrame`; playback, pause, seek, and scrub route through the formal realtime preview runtime, with `requestPreviewFrame` reserved for paused single-frame preview or developer diagnostics.
  2. 1080p video with baseline text and audio can play continuously on the supported path; baseline video/image/text/audio editing does not treat FFmpeg PNG/segment fallback as normal success.
  3. Text preview and export use the same stable bundled open-source font registry entry, with explicit license/file/glyph validation.
  4. Empty drafts adopt the first video/image material's orientation, aspect ratio, and video frame rate unless the user has manually changed the canvas; default segment visual fit avoids stretch distortion.
  5. Users can add/select target video/audio/text tracks, stack video/image/text layers, mix audio tracks, and rename/lock/show/mute tracks through Rust-owned commands.
  6. A real fixture E2E proves import vertical video, auto vertical canvas, play, add bundled-font text, add music, trim, split, undo/redo, save, reopen, and export.

**Plans:** 6 plans

Plans:

**Wave 1**

- [x] 15.1-01-PLAN.md - Repair realtime playback control chain and stop playback from driving repeated preview-frame artifact requests

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 15.1-02-PLAN.md - Enforce baseline video/image/text/audio realtime preview capability and productized unsupported states
- [x] 15.1-03-PLAN.md - Add bundled font registry shared by text preview and export

**Wave 3** *(blocked on Wave 1 completion)*

- [ ] 15.1-04-PLAN.md - Add first-material canvas adaptation and non-stretch default visual fit
- [ ] 15.1-05-PLAN.md - Add Rust-owned multitrack commands and minimal timeline controls

**Wave 4** *(blocked on Waves 1-3 completion)*

- [ ] 15.1-06-PLAN.md - Add full import-edit-play-save-reopen-export user-chain gate and source guards

### Phase 15.2: P0 Jianying-Style Production UI Convergence (INSERTED)

**Goal**: Converge the desktop workspace from an engineering/debug console into a Jianying-style production editor UI while keeping the product feature set smaller: default UI shows editing controls only, export is a right-side/top modal flow, and diagnostics stay behind developer mode.
**Depends on**: Phase 15.1
**Requirements**: P0-UI-01, P0-UI-02, P0-UI-03, P0-UI-04, P0-UI-05, P0-UI-06
**Success Criteria** (what must be TRUE):

  1. Default UI does not show FFmpeg, ffprobe, runtime/backend telemetry, artifact/cache paths, raw diagnostics, graph/cache internals, or developer-only status readouts.
  2. The five-zone layout matches the intended Jianying-style model: top product/category/export bar, left resource library by category, center preview canvas/playback controls, right focused inspector, and bottom timeline.
  3. Export is opened from the top/right export button in a modal containing path, resolution, frame rate, bitrate, audio options, progress, cancel, and open-location actions; preview no longer hosts a permanent export panel.
  4. Preview defaults to画面, timecode, play/pause, previous/next frame, fit/aspect/fullscreen controls, with realtime status shown only as productized exception copy.
  5. Left resource pages behave like a material library, text/audio entry panel, and lightweight generation status surface instead of a debug task dashboard.
  6. Timeline defaults to editor interactions: drag move, edge trim, draggable playhead, undo/redo, split, delete, snapping, add-track, and zoom; numeric move/trim inputs are not default toolbar controls.
  7. Right inspector is contextual: unselected draft/canvas parameters, selected video/image visual controls, selected text controls, and selected audio controls; developer details are hidden unless developer mode is enabled.
  8. Regression tests compare against `docs/ui-reference/jianying-pro/screenshots/` for layout/information hierarchy and assert 1280x800 and 1120x720 stability with no debug copy in default mode.

**Plans:** 0 plans

Plans:

- [ ] TBD (run $gsd-plan-phase 15.2 to break down)

### Phase 16: Task Scheduler, Job Isolation, And Performance Telemetry

**Goal**: Add a production job scheduler that isolates preview, decode, cache, IO, export, and analysis work while aligning all time-sensitive jobs to the shared timeline clock.
**Depends on**: Phase 15.2
**Requirements**: SCHED-01, SCHED-02, SCHED-03, SCHED-04
**Success Criteria** (what must be TRUE):

  1. Preview, decode, artifact generation, export, media probing, and filesystem IO run through priority-aware queues with cancellation and backpressure.
  2. Export and heavy artifact jobs cannot block playhead scrubbing, inspector edits, or preview frame delivery on supported hardware.
  3. Time-sensitive jobs carry target timeline microseconds and `PlaybackGeneration` so stale work cannot overwrite current preview/audio state.
  4. Thread-pool/resource limits are explicit, configurable for desktop development, and ready to map onto mobile/server runtimes.
  5. Performance telemetry records queue latency, job duration, cancellation, fallback, cache hit rate, first-frame time, and dropped-frame budgets.

**Plans**: TBD

Plans:

- [ ] TBD - Plan after Phase 15.2 completion

### Phase 17: Mobile/Server Binding Architecture And Runtime Ports

**Goal**: Turn the desktop-first Rust core into a portable runtime surface with explicit Node-API, C ABI, future JNI/Swift contracts, server entrypoints, and reference-counted opaque handle lifetimes.
**Depends on**: Phase 16
**Requirements**: PLAT-01, PLAT-02, PLAT-03, BIND-01, BIND-02, BIND-03, BIND-04, BIND-05
**Success Criteria** (what must be TRUE):

  1. Binding architecture separates desktop Node-API, portable C ABI, future Android JNI, future iOS Swift/ObjC, and server entrypoints without duplicating draft semantics.
  2. Runtime sessions, project sessions, media handles, frame handles, texture handles, and artifact handles use opaque IDs with owner session, generation, reference count, explicit release, cascading session-close release, and leak diagnostics.
  3. Mobile lifecycle, sandboxed media permissions, file handles, texture handles, memory ownership, and cancellation are represented as contracts, but full iOS/Android apps are deferred.
  4. Large media frames and preview outputs do not cross language boundaries as unnecessary copies when a handle-based path is available.
  5. Server runtime can open `.veproj`, resolve materials, run render/export jobs, and report progress without Electron.
  6. ABI, serialization, and binding smoke tests protect contract drift across desktop, mobile contracts, and server rendering.

**Plans**: TBD

Plans:

- [ ] TBD - Plan after Phase 16 completion

### Phase 18: Production Effects, Retiming, And Transition Semantics

**Goal**: Restore retiming, effects, filters, masks, blends, and transitions as production editor semantics once realtime preview, media IO, graph/cache, audio, and scheduling foundations exist.
**Depends on**: Phase 17
**Requirements**: PRODFX-01, PRODFX-02, PRODFX-03, PRODFX-04, PRODFX-05
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Retiming/speed curves are typed draft semantics evaluated by engine_core and represented in render graph/audio graph without renderer-owned time math.
  2. Transitions between adjacent visual segments have typed semantics, preview/export implementations or explicit degraded diagnostics, and undoable commands.
  3. Filters/effects use a capability registry that maps semantic effect intent to GPU preview and export/compiler implementations where supported.
  4. Masks, blend modes, blur, and complex effects use the production GPU preview path for realtime interaction and classify unsupported export paths.
  5. Complex Jianying/Kaipai-like template fixtures verify preview/export parity, fallback reports, and performance budgets for production editing scenarios.

**Plans**: TBD

Plans:

- [ ] TBD - Plan after Phase 17 completion; sequence capability registry, retiming/speed, transitions, visual effects, then template fidelity gates; use `ROADMAP_PHASES_11_13_ARCHIVE.md` as historical input for retiming/effects/transition scope

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 04.1 -> 5 -> 6 -> 7 -> 8 -> 9 -> 10 -> 10.1 -> 11 -> 12 -> 13 -> 14 -> 15 -> 16 -> 17 -> 18

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation And Golden Harness | 9/9 | Complete    | 2026-06-17 |
| 2. Draft And Material System | 6/6 | Complete    | 2026-06-17 |
| 3. Timeline Command Core | 5/5 | Complete    | 2026-06-17 |
| 4. Jianying-Style Desktop Workspace | 4/4 | Complete    | 2026-06-17 |
| 04.1 Professional Jianying Workspace UI Refinement | 4/4 | Complete    | 2026-06-17 |
| 5. Preview And Export Pipeline | 9/9 | Complete   | 2026-06-18 |
| 6. MVP Hardening And Packaging | 5/5 | Complete    | 2026-06-17 |
| 7. Project Canvas Space And Coordinate System | 7/7 | Complete    | 2026-06-18 |
| 8. Segment Transform And Visual Compositing | 5/5 | Complete | 2026-06-18 |
| 9. Complete Text And Subtitle System | 5/5 | Complete | 2026-06-18 |
| 10. Typed Keyframe And Animation System | 5/5 | Complete    | 2026-06-18 |
| 10.1 Usable Editor MVP Completion | 7/7 | Complete | 2026-06-18 |
| 11. Realtime Preview Runtime And GPU Render Backend | 10/10 | Complete   | 2026-06-18 |
| 12. Media IO, Hardware Decode, And Frame/Texture Interop | 9/9 | Complete | 2026-06-19 |
| 13. Incremental Render Graph, Dirty Ranges, And Cache Coherence | 8/8 | Complete   | 2026-06-19 |
| 14. Asset Resource Manager And Derived Artifact Store | 7/7 | Complete    | 2026-06-19 |
| 15. Audio Engine And DSP Timeline Pipeline | 7/7 | Complete   | 2026-06-19 |
| 16. Task Scheduler, Job Isolation, And Performance Telemetry | TBD | Not planned | - |
| 17. Mobile/Server Binding Architecture And Runtime Ports | TBD | Not planned | - |
| 18. Production Effects, Retiming, And Transition Semantics | TBD | Not planned | - |
