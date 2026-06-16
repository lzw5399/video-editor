# Stack Research

## Recommendation

Use a Rust-first core with an Electron desktop shell.

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

Initial crates:

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

Pin concrete versions during Phase 0 after the repo is scaffolded. The planning decision is architectural, not version-specific.

Phase 0 must record:

- Electron version and builder/package choice.
- Node-API/napi-rs or equivalent binding choice.
- Rust toolchain version.
- FFmpeg/ffprobe distribution and license posture.
- Playwright/Electron test harness version.

## Notes From References

- Kdenlive shows why project bin, timeline model, monitor, jobs, and render/export should be separate subsystems.
- MLT shows useful media abstractions but should not become runtime dependency.
- pyJianYingDraft shows vocabulary and data concepts worth aligning with: draft, material, track, segment, source/target time range, keyframe, text, sticker, filter, transition.

