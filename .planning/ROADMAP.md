# Roadmap: Video Editor

## Overview

This milestone builds a desktop-first Jianying-style video editor MVP on a Rust-owned core. The path is intentionally layered: establish the engineering and test harness, define the draft/material model, implement command-driven timeline editing, build the desktop workspace, connect preview/export through one render path, then harden and package the app.

After the MVP is packaged, the roadmap continues with core editor capability phases needed for Jianying/Kaipai-like template fidelity: project canvas space, segment transform, visual compositing, complete text, typed keyframes, retiming, effects, and transitions. These are core semantics, not adapter-only compatibility shims.

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
- [ ] **Phase 5: Preview And Export Pipeline** - Shared render graph path for preview frames, preview cache, and MP4 export
- [ ] **Phase 6: MVP Hardening And Packaging** - End-to-end verification, packaged app smoke tests, license manifest, and release readiness
- [ ] **Phase 7: Project Canvas Space And Coordinate System** - Draft-level aspect ratio, canvas size, frame rate, background, and normalized coordinate semantics
- [ ] **Phase 8: Segment Transform And Visual Compositing** - Jianying-style 画面/基础/变换 controls, layer ordering, visibility, fit/fill/stretch, and composition semantics
- [ ] **Phase 9: Complete Text And Subtitle System** - Font references, text box layout, line height, letter spacing, safe areas, and multi-segment subtitle/text parity
- [ ] **Phase 10: Typed Keyframe And Animation System** - Typed animated values, easing curves, and frame-time animation evaluation for transform/text/sticker/effect parameters
- [ ] **Phase 11: Retiming And Speed System** - Segment speed, source/target time mapping, audio follow-speed policy, and deferred reverse/curve-speed boundaries
- [ ] **Phase 12: Filter Adjustment And Effect Semantics** - First-party filter/adjustment/effect parameter schemas plus supported/degraded/unsupported capability boundaries
- [ ] **Phase 13: Transition Semantics And Timeline Integration** - Transition attachment, duration/type/params, overlap/trim/snapping effects, and render graph representation

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

**Plans**: 5 plans

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

**Plans**: 9 plans

Plans:

**Wave 1**

- [x] 05-01-PLAN.md - Implement engine_core normalization, frame state, and deterministic text layout

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 05-02-PLAN.md - Implement the typed renderer-neutral render graph

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 05-03-PLAN.md - Implement FFmpeg compiler jobs, filters, ASS sidecars, and snapshots

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 05-04-PLAN.md - Implement preview_service cache, generation, and invalidation
- [ ] 05-07-PLAN.md - Implement export runtime progress, cancel, logs, and output validation

**Wave 5** *(blocked on Wave 4 preview completion)*

- [ ] 05-05-PLAN.md - Add preview command contracts, binding routes, and renderer envelope helpers

**Wave 6** *(blocked on Wave 5 completion)*

- [ ] 05-06-PLAN.md - Add preview UI, source guards, and automated screenshots

**Wave 7** *(blocked on Wave 6 and export runtime completion)*

- [ ] 05-08-PLAN.md - Add export contracts, binding registry, UI, and automated screenshots

**Wave 8** *(blocked on Wave 7 completion)*

- [ ] 05-09-PLAN.md - Add preview/export parity, final source guards, and root gates

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

**Plans**: 3 plans

Plans:

- [ ] 06-01: Add packaged app build, native binding loading, and bundled runtime checks
- [ ] 06-02: Add packaged import-preview-export smoke tests and release gates
- [ ] 06-03: Document known limits, license posture, and post-MVP backlog

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

**Plans**: TBD

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

**Plans**: TBD

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

**Plans**: TBD

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

**Plans**: TBD

### Phase 11: Retiming And Speed System

**Goal**: Add segment speed semantics so templates can express rhythm changes beyond trim/split/move.
**Depends on**: Phase 10
**Requirements**: SPEED-01, SPEED-02, SPEED-03
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Segment speed changes are represented as typed semantics that define source/target time mapping after retiming.
  2. Audio follow-speed policy is explicit and renderable/degradable through the shared preview/export path.
  3. Reverse playback and curve speed have explicit deferred or degraded capability boundaries until implemented.
  4. Timeline trim/split/move validation understands retimed segments and remains atomic/undoable.

**Plans**: TBD

### Phase 12: Filter Adjustment And Effect Semantics

**Goal**: Define first-party effect semantics and capability reporting instead of stuffing native effects into opaque strings.
**Depends on**: Phase 11
**Requirements**: FX-01, FX-02, FX-03
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Draft schema distinguishes filter, adjustment, effect, and transition concepts using Jianying-aligned names and typed parameter schemas.
  2. render_graph and ffmpeg_compiler classify each effect parameter as supported, degraded, or unsupported.
  3. Jianying/Kaipai private native effect IDs are external references with compatibility reports, not internal render semantics.
  4. Desktop UI can show deferred/unsupported states in Chinese without pretending the effect is fully renderable.

**Plans**: TBD

### Phase 13: Transition Semantics And Timeline Integration

**Goal**: Implement transition semantics as first-class timeline relationships with deterministic edit and render behavior.
**Depends on**: Phase 12
**Requirements**: TRN-01, TRN-02, TRN-03
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Transitions attach to the correct adjacent or overlapping segment relationship with type, duration, and parameters.
  2. Timeline validation defines how transitions affect overlap, trim, snapping, and main-track magnet behavior.
  3. render_graph represents transition windows deterministically for preview/export compilation.
  4. Unsupported proprietary transitions degrade or report incompatibility rather than becoming opaque supported strings.

**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 04.1 -> 5 -> 6 -> 7 -> 8 -> 9 -> 10 -> 11 -> 12 -> 13

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation And Golden Harness | 9/9 | Complete    | 2026-06-17 |
| 2. Draft And Material System | 6/6 | Complete    | 2026-06-17 |
| 3. Timeline Command Core | 5/5 | Complete    | 2026-06-17 |
| 4. Jianying-Style Desktop Workspace | 4/4 | Complete    | 2026-06-17 |
| 04.1 Professional Jianying Workspace UI Refinement | 4/4 | Complete    | 2026-06-17 |
| 5. Preview And Export Pipeline | 4/9 | In Progress|  |
| 6. MVP Hardening And Packaging | 0/3 | Not started | - |
| 7. Project Canvas Space And Coordinate System | 0/TBD | Not started | - |
| 8. Segment Transform And Visual Compositing | 0/TBD | Not started | - |
| 9. Complete Text And Subtitle System | 0/TBD | Not started | - |
| 10. Typed Keyframe And Animation System | 0/TBD | Not started | - |
| 11. Retiming And Speed System | 0/TBD | Not started | - |
| 12. Filter Adjustment And Effect Semantics | 0/TBD | Not started | - |
| 13. Transition Semantics And Timeline Integration | 0/TBD | Not started | - |
