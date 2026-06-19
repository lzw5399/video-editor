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
- [ ] After MVP packaging, expand core editor semantics for project canvas space, transform/compositing, complete text, typed keyframes, retiming, effects, and transitions with Jianying-aligned terminology across Rust, schema, IPC, docs, and UI.

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
- **No product fallback**: Product-facing paths must fail closed with explicit diagnostics when the production implementation is unavailable. Debug, mock, artifact, CPU, legacy, or approximate paths may not be reported as successful product behavior. Reviewers must apply `docs/no-product-fallback-policy.md` and run `pnpm run test:no-product-fallback` when touching preview/playback/rendering or other success-evidence paths.
- **No legacy compatibility by default**: This is a greenfield editor. Refactors should replace incomplete historical implementations with the current intended architecture instead of preserving old product paths. Reviewers must apply `docs/refactor-and-legacy-cleanup-policy.md` and check that obsolete fallback, mock, debug, and alias paths are removed or gated from normal UI flows.
- **Product E2E acceptance**: User-visible editor features are not complete until Playwright/Electron tests perform the real user workflow and verify product evidence: visible preview, timeline state, saved project, or exported media. Reviews must apply `docs/product-e2e-acceptance-policy.md`; unit tests and binding contracts are necessary but insufficient for production completion.
- **Production UI flow**: The default desktop flow starts at a project entry state where the user creates a new project or opens an existing project before importing materials. Export is a top-right product action that opens a modal; permanent export panels inside the preview area are not the target production interaction.
- **UI icon sourcing**: New production UI iconography should first use SVG assets selected from `/Users/zhiwen/code/video-editor/icons` and copy the chosen assets into the application before use. Only create new icons when the provided set has no suitable match.
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
| Treat product E2E as the completion gate for visible editing features | Code-level correctness can still produce a confusing or nonfunctional editor; normal user workflows must prove preview/playback/edit/export behavior end to end | `docs/product-e2e-acceptance-policy.md` is a mandatory review reference starting before Phase 15.2 |
| Disallow fallback as product success | Fallback output hides missing production implementation and creates false confidence in playback/preview/export | `docs/no-product-fallback-policy.md` is a mandatory review reference and `pnpm run test:no-product-fallback` remains a required gate for success-evidence paths |
| Prefer replacing legacy paths over preserving compatibility | The editor is being built from scratch; keeping old partial paths creates hidden fallback behavior and weakens production validation | `docs/refactor-and-legacy-cleanup-policy.md` is mandatory for refactors and review must check for obsolete code that should be removed or gated |
| Use project-entry and top-right export as production workflow anchors | Jianying-style usage starts from a draft/project context and keeps export as a global product action, not a preview-panel debug tool | Phase 15.3 must add create/open project entry before import and move export into a top-right modal flow |
| Treat canvas, transform, compositing, text, keyframes, retiming, effects, and transitions as core semantics | Template fidelity depends on shared draft/render behavior, not adapter-only strings or renderer-local state | Phases 7-13 added after MVP packaging to build these capabilities in Rust/domain/schema/IPC/UI with Jianying-aligned terms |

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
*Last updated: 2026-06-20 after Phase 15.2/15.3 P0 production preview and UI-flow corrections*
