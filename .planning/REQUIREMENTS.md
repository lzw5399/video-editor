# Requirements: Video Editor

**Defined:** 2026-06-17
**Core Value:** Users can reliably import media, edit segments on a familiar Jianying-style timeline, preview the result, save the draft, and export a video through one consistent editing and rendering model.

## v1 Requirements

### Foundation

- [x] **FOUND-01**: Developer can build a Rust workspace and Electron desktop shell from a clean checkout.
- [x] **FOUND-02**: Electron can call the Rust core through a typed binding/API boundary.
- [ ] **FOUND-03**: The app can discover configured FFmpeg and ffprobe binaries and report actionable errors when unavailable.
- [ ] **FOUND-04**: The repository includes deterministic fixtures and golden test harnesses before feature work depends on media rendering.

### Draft Format

- [ ] **DRAFT-01**: User can create a new `.veproj` draft bundle.
- [ ] **DRAFT-02**: User can open and save a draft without semantic changes in a round trip.
- [ ] **DRAFT-03**: Draft schema uses Jianying-aligned concepts: draft, material, track, segment, target/source time range, main-track magnet, canvas adjustment, keyframe, sticker, text bubble, text effect, filter, and transition.
- [ ] **DRAFT-04**: Draft stores semantic state only in `project.json`; thumbnails, waveforms, preview caches, render graphs, FFmpeg scripts, and exports are derived artifacts.
- [ ] **DRAFT-05**: Draft versioning and migration hooks exist for future schema changes.

### Materials

- [ ] **MAT-01**: User can import video, image, and audio materials into the draft.
- [ ] **MAT-02**: Imported materials receive stable IDs and retain URI, duration, fps, size, stream, and audio metadata from ffprobe.
- [ ] **MAT-03**: Material bin displays imported materials with basic metadata and generated thumbnails where applicable.
- [ ] **MAT-04**: App detects missing material files and presents a recovery/error state without corrupting the draft.

### Timeline And Commands

- [ ] **TIME-01**: Draft supports at least one sequence with video, audio, and text tracks.
- [ ] **TIME-02**: User can add material segments to tracks with explicit source and target time ranges.
- [ ] **TIME-03**: User can select, move, split, trim, and delete timeline segments.
- [ ] **TIME-04**: User can undo and redo every committed timeline edit.
- [ ] **TIME-05**: Main-track magnet/snapping behavior is implemented in the Rust core, not in UI-only state.
- [ ] **TIME-06**: Invalid edits are rejected atomically without partially mutating the draft.
- [ ] **TIME-07**: Track stacking/z-index and per-track mute state are represented in the draft model.

### Text And Audio

- [ ] **TEXT-01**: User can add text/subtitle segments to a text track.
- [ ] **TEXT-02**: User can edit text content, font size, color, alignment, stroke, shadow, and background for MVP text segments.
- [ ] **TEXT-03**: Text layout uses pinned fonts and deterministic settings for preview/export parity.
- [ ] **AUD-01**: User can add audio/BGM materials to an audio track.
- [ ] **AUD-02**: User can adjust segment volume and track mute state.

### Preview

- [ ] **PREV-01**: User can preview the current draft in the center player.
- [ ] **PREV-02**: User can seek/scrub the playhead and request a deterministic preview frame.
- [ ] **PREV-03**: User can play a short preview segment using a cache generated from the same render path as export.
- [ ] **PREV-04**: Preview cache invalidates only affected ranges after timeline or text edits.

### Export

- [ ] **EXP-01**: User can export the draft to H.264 MP4 with a small preset set.
- [ ] **EXP-02**: Export uses the same normalized draft, resolved frame state, render graph, and FFmpeg compilation path as preview.
- [ ] **EXP-03**: Export reports progress, supports cancel, captures logs, and classifies common FFmpeg errors.
- [ ] **EXP-04**: Export output is validated for duration, fps, resolution, audio stream, and file existence.

### Desktop UI

- [ ] **UI-01**: Desktop editor first screen uses a Jianying-like workspace: top feature categories, left material/function panel, center preview, right inspector, and bottom multi-track timeline.
- [ ] **UI-02**: MVP UI implements media/material, text, and audio panels while reserving visible categories for sticker, effect, transition, filter, and adjustment.
- [ ] **UI-03**: UI uses Jianying-style terms consistently and does not expose alternate internal jargon.
- [ ] **UI-04**: UI emits typed commands to Rust and cannot mutate the draft directly.
- [ ] **UI-05**: Timeline controls have stable dimensions and do not shift layout during selection, hover, or playback updates.

### Testing And Quality

- [x] **TEST-01**: Schema and model tests validate every golden draft fixture.
- [ ] **TEST-02**: Command tests cover split, trim, move, delete, snapping, undo, redo, text edit, and volume edit.
- [ ] **TEST-03**: Engine tests cover normalization, time mapping, track stacking, text layout, and frame-state snapshots.
- [ ] **TEST-04**: Render graph and FFmpeg compiler outputs have snapshot tests.
- [ ] **TEST-05**: Preview frame and exported frame match within documented tolerance for golden drafts.
- [ ] **TEST-06**: Electron E2E test imports material, edits a timeline, previews, exports, and verifies output.
- [ ] **TEST-07**: Packaged app smoke test launches offline and completes import-preview-export.

## v2 Requirements

### Advanced Editing

- **ADV-01**: User can work with multiple sequences or nested drafts.
- **ADV-02**: User can add sticker segments and image/GIF/WebP/video overlays.
- **ADV-03**: User can add transform keyframes for position, scale, rotation, opacity, and volume.
- **ADV-04**: User can add masks, blend modes, segment filters, track filters, and transitions.
- **ADV-05**: User can use text bubbles and text effects with local fallback rendering.

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
| FOUND-03 | Phase 1 | Pending |
| FOUND-04 | Phase 1 | Pending |
| DRAFT-01 | Phase 2 | Pending |
| DRAFT-02 | Phase 2 | Pending |
| DRAFT-03 | Phase 2 | Pending |
| DRAFT-04 | Phase 2 | Pending |
| DRAFT-05 | Phase 2 | Pending |
| MAT-01 | Phase 2 | Pending |
| MAT-02 | Phase 2 | Pending |
| MAT-03 | Phase 2 | Pending |
| MAT-04 | Phase 2 | Pending |
| TIME-01 | Phase 3 | Pending |
| TIME-02 | Phase 3 | Pending |
| TIME-03 | Phase 3 | Pending |
| TIME-04 | Phase 3 | Pending |
| TIME-05 | Phase 3 | Pending |
| TIME-06 | Phase 3 | Pending |
| TIME-07 | Phase 3 | Pending |
| TEXT-01 | Phase 3 | Pending |
| TEXT-02 | Phase 3 | Pending |
| TEXT-03 | Phase 5 | Pending |
| AUD-01 | Phase 3 | Pending |
| AUD-02 | Phase 3 | Pending |
| PREV-01 | Phase 5 | Pending |
| PREV-02 | Phase 5 | Pending |
| PREV-03 | Phase 5 | Pending |
| PREV-04 | Phase 5 | Pending |
| EXP-01 | Phase 5 | Pending |
| EXP-02 | Phase 5 | Pending |
| EXP-03 | Phase 5 | Pending |
| EXP-04 | Phase 5 | Pending |
| UI-01 | Phase 4 | Pending |
| UI-02 | Phase 4 | Pending |
| UI-03 | Phase 4 | Pending |
| UI-04 | Phase 4 | Pending |
| UI-05 | Phase 4 | Pending |
| TEST-01 | Phase 1 | Complete |
| TEST-02 | Phase 3 | Pending |
| TEST-03 | Phase 5 | Pending |
| TEST-04 | Phase 5 | Pending |
| TEST-05 | Phase 5 | Pending |
| TEST-06 | Phase 6 | Pending |
| TEST-07 | Phase 6 | Pending |

**Coverage:**

- v1 requirements: 45 total
- Mapped to phases: 45
- Unmapped: 0

---
*Requirements defined: 2026-06-17*
*Last updated: 2026-06-17 after initial definition*
