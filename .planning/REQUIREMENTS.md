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

- **SPEED-01**: Segment speed/变速 is represented as typed semantics that define source/target time mapping after retiming.
- **SPEED-02**: Reverse playback and curve speed have explicit deferred/degraded capability boundaries until implemented.
- **SPEED-03**: Audio follow-speed behavior is explicit and renderable/degradable through the shared preview/export path.

- **FX-01**: Draft distinguishes filter, adjustment, effect, and transition concepts with Jianying-aligned names and typed parameter schemas.
- **FX-02**: First-party filter/adjustment/effect parameters report supported, degraded, or unsupported capabilities through render_graph/compiler outputs.
- **FX-03**: Jianying/Kaipai private native effect IDs remain external compatibility references and are not treated as internal render semantics.

- **TRN-01**: Transitions attach to the correct adjacent or overlapping segment relationship with type, duration, and typed parameters.
- **TRN-02**: Timeline validation defines transition effects on overlap, trim, snapping, and main-track magnet behavior.
- **TRN-03**: render_graph represents transition windows deterministically for preview/export, with unsupported proprietary transitions classified.

### Compatibility

- **COMP-01**: User can import a supported Jianying/CapCut draft subset into `.veproj`.
- **COMP-02**: User receives a compatibility report listing supported, degraded, and unsupported external draft features.
- **COMP-03**: User can export a supported Jianying-compatible draft subset when feasible.

### Platform Expansion

- **PLAT-01**: Rust core exposes a C/FFI boundary for mobile/server shells.
- **PLAT-02**: Server renderer can render a `.veproj` without Electron.
- **PLAT-03**: iOS and Android prototypes can open and lightly edit the same draft semantics.

## Out of Scope

| Feature | Reason |
|---------|--------|
| AI oral-video workflows | Current product is a general desktop video editor |
| Jianying draft as primary project format | Conflicts with self-owned cross-platform core and long-term schema control |
| 100% proprietary effect parity | Private resources/effects are unstable, unavailable, or legally constrained |
| Kdenlive/MLT runtime integration | References only; direct integration creates mobility and licensing constraints |
| GPU real-time effects engine | Too large for MVP; preview cache and FFmpeg path come first |
| Mobile apps and cloud rendering in MVP | Architecture should prepare for them, but desktop editor must prove the core first |

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
| TEXT2-01 | Phase 9 | Planned |
| TEXT2-02 | Phase 9 | Planned |
| TEXT2-03 | Phase 9 | Planned |
| ANIM-01 | Phase 10 | Planned |
| ANIM-02 | Phase 10 | Planned |
| ANIM-03 | Phase 10 | Planned |
| SPEED-01 | Phase 11 | Planned |
| SPEED-02 | Phase 11 | Planned |
| SPEED-03 | Phase 11 | Planned |
| FX-01 | Phase 12 | Planned |
| FX-02 | Phase 12 | Planned |
| FX-03 | Phase 12 | Planned |
| TRN-01 | Phase 13 | Planned |
| TRN-02 | Phase 13 | Planned |
| TRN-03 | Phase 13 | Planned |
| COMP-01 | Post-MVP | Deferred |
| COMP-02 | Post-MVP | Deferred |
| COMP-03 | Post-MVP | Deferred |
| PLAT-01 | Post-MVP | Deferred |
| PLAT-02 | Post-MVP | Deferred |
| PLAT-03 | Post-MVP | Deferred |

**Coverage:**

- v1 requirements: 52 total
- Mapped to phases: 52
- Unmapped: 0
- v2/post-MVP requirements: 31 total, 25 planned in Phases 7-13 and 6 deferred

---
*Requirements defined: 2026-06-17*
*Last updated: 2026-06-18 after Phase 08 verification*
