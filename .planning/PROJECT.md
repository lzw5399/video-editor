# Video Editor

## What This Is

Video Editor is a desktop-first video editing application with a Jianying/CapCut-like editing experience and a self-owned Rust editing/rendering core. The first product is an Electron desktop editor, but the project is structured so the same draft semantics, timeline behavior, render graph, preview/runtime contracts, FFmpeg compilation path, and adapter boundaries can later serve mobile apps, server rendering, and external draft/template workflows.

This is a general-purpose editor, not an AI talking-head or oral-video product. AI workflows, full proprietary draft compatibility, mobile apps, and cloud rendering are future extensions built on top of the same editor core.

## Core Value

Users can reliably import media, edit clips on a familiar Jianying-style timeline, preview the result, save the draft, and export a video through one consistent editing and rendering model.

## Current State

**Shipped milestone:** v1.0 Production Core on 2026-06-26.

v1.0 completed 25 phases and 187 plans. The project now has a Rust-owned `.veproj` draft model, material system, timeline command semantics, undo/redo, snapping, canvas/transform/text/keyframe/effect semantics, render graph, FFmpeg compiler path, realtime preview/runtime boundaries, media IO contracts, artifact/cache systems, audio graph, scheduler, template import with offline Kaipai adapter, portable binding architecture, and production retime/effect/filter/mask/blend/transition semantics.

The desktop product flow now starts from create/open project, uses Simplified Chinese Jianying-style UI language, routes editing through Rust-owned project-session intents and interaction sessions, proves preview/export behavior through product E2E evidence, and rejects fallback/mock/artifact/CPU/DOM evidence as product success.

Milestone archive:

- `.planning/milestones/v1.0-ROADMAP.md`
- `.planning/milestones/v1.0-REQUIREMENTS.md`
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md`

## Requirements

### Validated

- ✓ Buildable Rust workspace and Electron desktop shell with typed Node-API boundary, FFmpeg discovery, deterministic fixtures, generated contracts, and GSD gates — v1.0.
- ✓ Self-owned `.veproj/project.json` draft bundle with Jianying-aligned draft/material/track/segment/time/keyframe/filter/transition vocabulary and migration hooks — v1.0.
- ✓ Rust-owned timeline command semantics for add/select/move/split/trim/delete, undo/redo, snapping/main-track magnet, text/audio basics, and invalid-edit rejection — v1.0.
- ✓ Jianying-style Simplified Chinese desktop workspace with material/resource panel, preview, inspector, timeline, project entry, top-right export modal, and command-only UI integration — v1.0.
- ✓ Preview/export share normalized draft, frame state, render graph, compiler/runtime, diagnostics, and product E2E verification instead of renderer-owned media or FFmpeg construction — v1.0.
- ✓ Production architecture foundations for realtime preview, native/GPU media IO, incremental graph/cache invalidation, artifact store, audio DSP, scheduler isolation, performance telemetry, and no-product-fallback gates — v1.0.
- ✓ Provider-neutral template import with offline Kaipai adapter, localized resources, adaptation reports, report navigation, preview/export evidence, and provider IDs kept out of internal render semantics — v1.0.
- ✓ Portable runtime surfaces for Node-API, C ABI, future JNI/Swift, server export, opaque handle lifetimes, and adapter-owned transport boundaries — v1.0.
- ✓ Production retiming, speed mapping, transitions, first-party effects/filters, masks, blends, capability registry, preview/export diagnostics, and desktop controls through Rust-owned semantics — v1.0.

### Active

No active v1.0 requirements remain. The next milestone must define a fresh `.planning/REQUIREMENTS.md` through `$gsd-new-milestone`.

Candidate next-milestone topics:

- Clean up planning traceability and verification artifact consistency identified by the v1.0 milestone audit.
- Finish release/package polish such as Electron app metadata/icon and Node version alignment.
- Close documented crop/export limitations and broaden production effect/export parity.
- Run deeper real-user product UAT on longer timelines and template-heavy drafts.
- Decide whether v1.1 should prioritize release hardening, editor usability depth, compatibility coverage, or mobile/server productization.

### Out of Scope

- AI oral-video workflows, ASR, automatic highlight detection, and template intelligence remain outside the current product identity.
- Jianying/CapCut/Kaipai drafts are external adapter inputs, not the primary project format.
- 100% proprietary effect/preset parity remains legally and technically constrained; unsupported external features must report degraded/unsupported status.
- Direct Kdenlive, MLT, or GPL editor runtime integration remains reference-only.
- Full mobile apps, cloud rendering product UX, marketplace preset libraries, and live provider integrations are future product scopes.

## Context

The product remains guided by `AI_Video_Editing_Single_Engine_Guideline.md` with the explicit correction that the active target is a general desktop video editor.

`reference/pyJianYingDraft` remains useful for vocabulary and compatibility concepts. Kdenlive and MLT remain architecture references only. Do not copy GPL code, assets, XML definitions, presets, or UI implementation.

## Constraints

- **Architecture**: UI emits commands, intents, or interaction-session updates; Rust core owns project, timeline, preview/export, cache, retime/effect/transition, and adapter semantics. No UI code may directly construct FFmpeg commands.
- **Production refactor policy**: Do not patch around a known-wrong ownership boundary. If preview, edit, render, session, media, or native-surface code is structurally wrong, replace the boundary with the long-term production architecture and delete the legacy path.
- **Project format**: `.veproj/project.json` is the canonical source of truth. Render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts.
- **Terminology**: Product language, desktop code, Rust domain types, IPC commands, docs, schema, and tests should follow Jianying concepts wherever possible.
- **Time model**: Core time math must use integer microseconds, frame indices, or rational frame rates. Avoid naked floating-point time in persisted semantics.
- **Rendering**: Render Graph isolates editing semantics from FFmpeg. FFmpeg Runtime executes jobs and reports progress/errors; it does not decide editing behavior.
- **No product fallback**: Product-facing paths must fail closed with explicit diagnostics when production implementation is unavailable.
- **Product E2E acceptance**: User-visible editor features are not complete until Playwright/Electron tests perform the real user workflow and verify visible preview, timeline state, saved draft, or exported media evidence.
- **Compatibility**: External drafts go through adapters and produce compatibility reports. Proprietary IDs are external references, not internal render semantics.
- **Testing**: Each roadmap phase must define executable gates before implementation is considered complete.
- **Licensing**: FFmpeg distribution must be reviewed for LGPL/GPL/nonfree build options, notices, and commercial product obligations.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Build a general-purpose desktop video editor, not an oral-video product | Current product value is a Jianying-like editor, not AI talking-head generation | Validated through v1.0 scope |
| Start with Rust core from day one | Editing semantics, schemas, rendering contracts, and adapters must be durable cross-platform assets | v1.0 established Rust-owned semantics and transport adapters |
| Use Electron for the first desktop shell | Fastest path to a production desktop editor UI while Rust owns core behavior | v1.0 shipped Electron workspace and product E2E gates |
| Use a self-owned `.veproj` format | Long-term control matters more than using external draft formats directly | v1.0 validated `.veproj/project.json` as canonical |
| Align vocabulary with Jianying concepts | Users and future compatibility work benefit from familiar terms | v1.0 aligned draft/material/track/segment/keyframe/filter/transition language |
| Disallow fallback as product success | Fallback hides missing production implementation and creates false confidence | v1.0 added no-product-fallback policy and guards |
| Treat high-frequency interaction as Rust-owned session semantics | Dragging, scrubbing, and inspector updates need live feedback without save/undo/revision storms | v1.0 added interaction sessions and coalesced commits |
| Keep external adapter IDs out of internal render semantics | Proprietary IDs are compatibility/report facts, not first-party effects | v1.0 Kaipai and Phase 19 boundaries enforce this |

## Next Milestone Goals

Run `$gsd-new-milestone` before implementation. Recommended decision points:

- Is v1.1 a release-hardening milestone or a deeper editor-capability milestone?
- Should planning traceability cleanup happen first, or stay as archived tech debt?
- Which remaining product limitations should become hard acceptance gates: crop export, app metadata/icon, long timeline performance, broader effect parity, or compatibility import/export?

---
*Last updated: 2026-06-26 after v1.0 Production Core milestone close*
