<!-- GSD:project-start source:PROJECT.md -->

## Project

**Video Editor**

Video Editor is a desktop-first video editing application with a Jianying/CapCut-like editing experience and a self-owned Rust editing/rendering core. The first product is an Electron desktop editor, but the project is structured so the same draft semantics, timeline behavior, render graph, and FFmpeg compilation path can later serve mobile apps and server rendering.

This is a general-purpose editor, not an AI talking-head or oral-video product. AI workflows, Jianying draft compatibility, mobile clients, and cloud rendering are future extensions built on top of the same editor core.

**Core Value:** Users can reliably import media, edit clips on a familiar Jianying-style timeline, preview the result, save the draft, and export a video through one consistent editing and rendering model.

### Constraints

- **Architecture**: UI emits commands; Rust core owns project and timeline semantics. No UI code may directly construct FFmpeg commands.
- **Project format**: `.veproj/project.json` is the canonical source of truth. Render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts.
- **Terminology**: Product language, desktop code, Rust domain types, IPC commands, docs, schema, and tests should follow Jianying concepts wherever possible. Prefer draft/material/track/segment/keyframe/filter/transition-style terms over invented equivalents.
- **Time model**: Core time math must use integer microseconds, frame indices, or rational frame rates. Avoid naked floating-point time in persisted semantics.
- **Rendering**: Render Graph isolates editing semantics from FFmpeg. FFmpeg Runtime executes jobs and reports progress/errors; it does not decide editing behavior.
- **References**: Kdenlive and MLT are conceptual references only. Do not copy GPL code, assets, XML definitions, presets, or UI implementation.
- **Compatibility**: External drafts go through adapters and produce compatibility reports. Proprietary IDs are external references, not internal render semantics.
- **Testing**: Each roadmap phase must define executable gates before implementation is considered complete.
- **Licensing**: FFmpeg distribution must be reviewed for LGPL/GPL/nonfree build options, notices, and commercial product obligations.

<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->

## Technology Stack

## Recommendation

## Core Stack

| Area | Recommendation | Rationale |
|------|----------------|-----------|
| Desktop shell | Electron | Fastest path to a Jianying-like desktop editor, native menus/filesystem, mature packaging ecosystem |
| Renderer UI | React + TypeScript | Mature component model for complex editor UI; easy Playwright coverage |
| Rust binding | Node-API via a thin binding crate | Keeps editing semantics in Rust while Electron calls a stable API |
| Core language | Rust | Strong ownership, deterministic model code, serde/schema support, cross-platform FFI path |
| Project format | `.veproj/project.json` | Durable project bundle with canonical semantic source of truth |
| Media execution | FFmpeg/ffprobe desktop binary for MVP | Debuggable, reproducible, good logs; later mobile runtime can swap implementation |
| Testing | Rust tests + golden fixtures + Playwright Electron E2E | Tests the full editor pipeline from model to packaged app |

## Rust Workspace

- `draft_model`: draft/material/track/segment schema, time model, migrations.
- `draft_commands`: split, trim, move, delete, undo/redo, snapping/main-track magnet.
- `engine_core`: normalize draft, resolve segment timing, evaluate frame state.
- `render_graph`: typed graph and render intents.
- `ffmpeg_compiler`: render graph to FFmpeg inputs, filter scripts, ASS subtitles, encode settings.
- `media_runtime`: ffprobe/ffmpeg traits, job state, progress, cancellation, errors.
- `media_runtime_desktop`: desktop binary implementation.
- `preview_service`: thumbnails, waveforms, preview frames/segments, cache invalidation.
- `project_store`: `.veproj` open/save/autosave and relative path handling.
- `bindings_node`: Electron-facing Node-API surface.
- `testkit`: fixture generation, golden comparisons, render smoke helpers.

Later crates:

- `adapter_jianying`: Jianying/CapCut draft import/export subset and compatibility reports.
- `bindings_c`: mobile/server FFI boundary.
- `media_runtime_ios`, `media_runtime_android`, `media_runtime_server`.

## Version Policy

- Electron version and builder/package choice.
- Node-API/napi-rs or equivalent binding choice.
- Rust toolchain version.
- FFmpeg/ffprobe distribution and license posture.
- Playwright/Electron test harness version.

## Notes From References

- Kdenlive shows why project bin, timeline model, monitor, jobs, and render/export should be separate subsystems.
- MLT shows useful media abstractions but should not become runtime dependency.
- pyJianYingDraft shows vocabulary and data concepts worth aligning with: draft, material, track, segment, source/target time range, keyframe, text, sticker, filter, transition.

<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:

- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
