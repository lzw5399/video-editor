# Roadmap: Video Editor

## Overview

This milestone builds a desktop-first Jianying-style video editor MVP on a Rust-owned core. The path is intentionally layered: establish the engineering and test harness, define the draft/material model, implement command-driven timeline editing, build the desktop workspace, connect preview/export through one render path, then harden and package the app.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation And Golden Harness** - Rust/Electron workspace, binding path, FFmpeg discovery, and deterministic test fixtures (completed 2026-06-16)
- [ ] **Phase 2: Draft And Material System** - `.veproj` draft bundle, Jianying-aligned schema, material import/probing, and save/open validation
- [ ] **Phase 3: Timeline Command Core** - Track/segment model, command API, undo/redo, snapping, text/audio basics
- [ ] **Phase 4: Jianying-Style Desktop Workspace** - Editor UI shell matching Jianying workspace structure and command-only core integration
- [ ] **Phase 5: Preview And Export Pipeline** - Shared render graph path for preview frames, preview cache, and MP4 export
- [ ] **Phase 6: MVP Hardening And Packaging** - End-to-end verification, packaged app smoke tests, license manifest, and release readiness

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

**Plans**: 4 plans

Plans:

- [ ] 02-01: Define draft/material/track/segment schema and migration hooks
- [ ] 02-02: Implement project bundle open/save/autosave and round-trip tests
- [ ] 02-03: Implement material import, ffprobe metadata, IDs, thumbnails, and missing-material checks
- [ ] 02-04: Add draft/material golden fixtures and schema/model tests

### Phase 3: Timeline Command Core

**Goal**: Implement Rust-owned timeline editing semantics and command/undo behavior before rich UI relies on it.
**Depends on**: Phase 2
**Requirements**: TIME-01, TIME-02, TIME-03, TIME-04, TIME-05, TIME-06, TIME-07, TEXT-01, TEXT-02, AUD-01, AUD-02, TEST-02
**Success Criteria** (what must be TRUE):

  1. User-visible edits are represented as typed commands and cannot mutate the draft partially on failure.
  2. User can add, select, move, split, trim, and delete video/audio/text segments.
  3. Undo/redo works for every committed MVP edit.
  4. Main-track magnet/snapping is computed in Rust core and covered by command tests.

**Plans**: 4 plans

Plans:

- [ ] 03-01: Implement track/segment timeline model and overlap/stacking rules
- [ ] 03-02: Implement add, move, split, trim, delete, select, and invalid-edit rejection commands
- [ ] 03-03: Implement undo/redo, snapping/main-track magnet, and command event output
- [ ] 03-04: Implement MVP text/audio commands and command test coverage

### Phase 4: Jianying-Style Desktop Workspace

**Goal**: Build the desktop editor workspace with Jianying-like structure and command-only integration to the Rust core.
**Depends on**: Phase 3
**Requirements**: UI-01, UI-02, UI-03, UI-04, UI-05, TEST-06
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. User sees a Jianying-like workspace: top feature categories, left material/function panel, center preview, right inspector, bottom timeline.
  2. User can import materials, add/edit segments, edit text/audio values, and see draft state update through Rust commands.
  3. UI consistently uses Jianying concepts and does not expose alternate internal jargon.
  4. Timeline and panel layout remain stable during selection, hover, drag, and playback updates.

**Plans**: 4 plans

Plans:

- [ ] 04-01: Build desktop workspace shell, layout system, and feature category navigation
- [ ] 04-02: Implement material, text, and audio panels plus right inspector
- [ ] 04-03: Implement timeline interaction surface wired only through command API
- [ ] 04-04: Add Electron IPC contracts, Playwright smoke flow, and visual layout checks

### Phase 5: Preview And Export Pipeline

**Goal**: Connect the Rust editing model to a shared render graph path for preview and final MP4 export.
**Depends on**: Phase 4
**Requirements**: TEXT-03, PREV-01, PREV-02, PREV-03, PREV-04, EXP-01, EXP-02, EXP-03, EXP-04, TEST-03, TEST-04, TEST-05
**Success Criteria** (what must be TRUE):

  1. User can seek and preview deterministic frames generated from the same resolved draft semantics used for export.
  2. User can play a short cached preview segment and edits invalidate only affected cache ranges.
  3. User can export H.264 MP4 with progress, cancellation, logs, and classified errors.
  4. Golden tests cover normalized draft, frame state, render graph, FFmpeg script, preview/export parity, and output metadata.

**Plans**: 4 plans

Plans:

- [ ] 05-01: Implement normalization, resolved frame state, and text layout determinism
- [ ] 05-02: Implement typed render graph and FFmpeg compiler snapshots
- [ ] 05-03: Implement preview frame/segment generation and cache invalidation
- [ ] 05-04: Implement MP4 export job, progress/cancel/errors, and render golden tests

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

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation And Golden Harness | 9/9 | Complete    | 2026-06-17 |
| 2. Draft And Material System | 0/4 | Not started | - |
| 3. Timeline Command Core | 0/4 | Not started | - |
| 4. Jianying-Style Desktop Workspace | 0/4 | Not started | - |
| 5. Preview And Export Pipeline | 0/4 | Not started | - |
| 6. MVP Hardening And Packaging | 0/3 | Not started | - |
