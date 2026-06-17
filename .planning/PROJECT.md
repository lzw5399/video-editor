# Video Editor

## What This Is

Video Editor is a desktop-first video editing application with a Jianying/CapCut-like editing experience and a self-owned Rust editing/rendering core. The first product is an Electron desktop editor, but the project is structured so the same draft semantics, timeline behavior, render graph, and FFmpeg compilation path can later serve mobile apps and server rendering.

This is a general-purpose editor, not an AI talking-head or oral-video product. AI workflows, Jianying draft compatibility, mobile clients, and cloud rendering are future extensions built on top of the same editor core.

## Core Value

Users can reliably import media, edit clips on a familiar Jianying-style timeline, preview the result, save the draft, and export a video through one consistent editing and rendering model.

## Requirements

### Validated

- ✓ Provide a Jianying-like desktop editing workspace with media, preview, inspector, and multi-track timeline regions — Phase 4.
- ✓ Desktop app user-facing language is Simplified Chinese by default, including product copy, panel titles, controls, empty states, errors, and test-visible UI text — Phase 4.
- ✓ Use Jianying-style concepts consistently across desktop UI, Rust core, IPC commands, documentation, schema, and tests: draft, material, track, segment, target/source time range, main track magnet, canvas adjustment, keyframe, sticker, text bubble, text effect, filter, transition — Phases 2-4.

### Active

- [ ] Store project state in a self-owned `.veproj` project bundle with a canonical `project.json` source of truth.
- [ ] Implement Rust-owned project, media, timeline, command, undo/redo, snapping, render graph, and FFmpeg compilation layers.
- [ ] Support an MVP editor flow: import video/image/audio, arrange clips on tracks, split/trim/move/delete, add text/subtitle and BGM, preview, save/open, and export MP4.
- [ ] Keep preview and export on the same semantic path: project -> command -> normalized timeline -> resolved frame state -> render graph -> FFmpeg job.
- [ ] Add test gates at every phase, including schema/model tests, command tests, render graph snapshots, FFmpeg smoke renders, preview/export parity, and Electron E2E tests.

### Out of Scope

- AI oral-video workflows, ASR, automatic highlight detection, and template intelligence - not part of the current product identity.
- Jianying/CapCut/Kaipai draft import/export in MVP - adapter work is later and must not drive the primary internal format.
- 100% pixel-level recreation of Jianying, CapCut, or Kaipai proprietary effects - unsupported effects should degrade or map to local alternatives.
- Direct Kdenlive, MLT, or GPL editor runtime integration - these are references for architecture and concepts, not production dependencies.
- Mobile apps, server rendering, GPU real-time preview, nested sequences, OTIO, complex effects, advanced masks, and large preset libraries in MVP.

## Context

The project is guided by `AI_Video_Editing_Single_Engine_Guideline.md`, with the important correction that the current target is a general desktop video editor, not a talking-head editor. Kdenlive is the main reference for editor capability boundaries, project bin/timeline/monitor/export organization, and backend-model discipline. MLT is the main reference for media engine abstractions such as producer, playlist, tractor/multitrack, filter, transition, consumer, and profile.

`reference/pyJianYingDraft` is used as a vocabulary and compatibility reference. Its draft, material, track, segment, source/target time range, canvas adjustment, keyframe, text, sticker, effect, filter, transition, and template concepts should shape the desktop UI, Rust core domain model, IPC command names, project schema, tests, and documentation. Do not invent a second conceptual vocabulary when Jianying terminology is already clear.

The application should feel like a restrained Jianying-style editor rather than a generic dashboard. The first screen should be the editor workspace: top feature categories, left material/effect library, center preview, right property inspector, and bottom multi-track timeline. MVP may leave advanced categories as placeholders, but the layout and interaction model should be established early.

The Electron desktop UI should use Simplified Chinese as the default user-facing language. Internal Rust, IPC, schema, and test identifiers can remain code-friendly, but visible copy should be Chinese and should preserve Jianying-style terminology.

## Constraints

- **Architecture**: UI emits commands; Rust core owns project and timeline semantics. No UI code may directly construct FFmpeg commands.
- **Project format**: `.veproj/project.json` is the canonical source of truth. Render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts.
- **Terminology**: Product language, desktop code, Rust domain types, IPC commands, docs, schema, and tests should follow Jianying concepts wherever possible. Prefer draft/material/track/segment/keyframe/filter/transition-style terms over invented equivalents.
- **Time model**: Core time math must use integer microseconds, frame indices, or rational frame rates. Avoid naked floating-point time in persisted semantics.
- **Rendering**: Render Graph isolates editing semantics from FFmpeg. FFmpeg Runtime executes jobs and reports progress/errors; it does not decide editing behavior.
- **References**: Kdenlive and MLT are conceptual references only. Do not copy GPL code, assets, XML definitions, presets, or UI implementation.
- **Compatibility**: External drafts go through adapters and produce compatibility reports. Proprietary IDs are external references, not internal render semantics.
- **Testing**: Each roadmap phase must define executable gates before implementation is considered complete.
- **Licensing**: FFmpeg distribution must be reviewed for LGPL/GPL/nonfree build options, notices, and commercial product obligations.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Build a general-purpose desktop video editor, not an oral-video product | The current goal is a Jianying-like editor; oral-video language in the guideline is not the active product scope | - Pending |
| Start with Rust Core from day one | Editing semantics, schema, render graph, and tests should become durable cross-platform assets immediately | Phase 1 established the Rust workspace, command contracts, Node-API binding, and runtime/test boundaries |
| Use Electron for the first desktop shell | Electron gives the fastest path to a desktop editor UI while Rust owns core logic | Phase 1 established the Electron shell, preload bridge, and Rust binding smoke |
| Use a self-owned `.veproj` format | Long-term cross-platform control matters more than direct use of Jianying or Kdenlive formats | - Pending |
| Align product and schema concepts with Jianying | Users and future compatibility work benefit from familiar vocabulary; avoid creating a parallel terminology layer | Phases 2-4 validated Jianying-aligned draft/material/timeline/schema/IPC/UI/test vocabulary |
| Use Simplified Chinese for the desktop UI | The first desktop product targets a Chinese Jianying-style editing experience, so visible copy should match the user's language and editing vocabulary | Phase 4 validated Simplified Chinese visible copy and source guards |
| Treat Kdenlive and MLT as references, not runtimes | Their architecture and abstractions are useful, but direct integration creates licensing, mobility, and product-control problems | - Pending |
| Test every layer of the pipeline | A video editor fails through subtle time, render, preview, and packaging drift; phase gates must catch this early | Phase 1 established `just build`, `just test`, schema drift checks, Electron smoke, FFmpeg discovery tests, and render smoke |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `$gsd-transition`):
1. Requirements invalidated? -> Move to Out of Scope with reason
2. Requirements validated? -> Move to Validated with phase reference
3. New requirements emerged? -> Add to Active
4. Decisions to log? -> Add to Key Decisions
5. "What This Is" still accurate? -> Update if drifted

**After each milestone** (via `$gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check - still the right priority?
3. Audit Out of Scope - reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-06-17 after Phase 4 verification*
