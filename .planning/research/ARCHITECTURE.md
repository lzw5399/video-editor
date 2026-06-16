# Architecture Research

## Architecture Direction

The editor should have one semantic spine:

```text
draft/project.json
  -> command
  -> normalized draft
  -> resolved frame state
  -> render graph
  -> FFmpeg job
  -> preview/export
```

Every layer should use Jianying-aligned concepts where possible: draft, material, track, segment, source/target time range, main-track magnet, canvas adjustment, keyframe, sticker, text bubble, text effect, filter, transition.

## Proposed Repository Layout

```text
apps/
  desktop-electron/

crates/
  draft_model/
  draft_commands/
  engine_core/
  render_graph/
  ffmpeg_compiler/
  media_runtime/
  media_runtime_desktop/
  preview_service/
  project_store/
  bindings_node/
  testkit/
  adapter_jianying/        # post-MVP

schemas/
  draft.schema.json
  command.schema.json

fixtures/
goldens/
docs/
tools/
```

## Project Bundle

Use a directory bundle:

```text
example.veproj/
  project.json
  cache/
    thumbnails/
    waveforms/
    previews/
  exports/
```

`project.json` is the only semantic source of truth. Cache files, render graphs, FFmpeg scripts, ffprobe output, thumbnails, waveforms, and exports are derived artifacts.

## Layer Responsibilities

| Layer | Owns | Does Not Own |
|-------|------|--------------|
| Electron shell | windows, menu, file dialogs, permissions, packaging | draft semantics, FFmpeg commands |
| Renderer UI | layout, drag gestures, selection, panels, timeline zoom | direct draft mutation |
| Node binding | stable IPC/API between UI and Rust | business rules |
| `draft_model` | draft/material/track/segment schema, time, migrations | editing decisions |
| `draft_commands` | add/move/split/trim/delete, undo/redo, snapping | rendering |
| `engine_core` | normalization, time mapping, track stacking, frame state | FFmpeg syntax |
| `render_graph` | typed render plan | process execution |
| `ffmpeg_compiler` | inputs, filter scripts, subtitles, encode args | editing semantics |
| `media_runtime` | ffprobe/ffmpeg execution, progress, cancel, errors | timeline behavior |
| `preview_service` | preview frames/segments, thumbnails, waveform cache | project source of truth |

## Kdenlive Lessons

- A single authoritative timeline backend should accept or reject UI edit requests atomically.
- Stable IDs matter more than UI row positions.
- Project profile and render preset should be separate.
- Project bin/material registry should be separate from timeline segments.
- Validation and migration are part of the editor, not optional tooling.
- Export should be a job with logs, progress, cancellation, and serialized input.

## MLT Lessons

Adopt concepts, not runtime:

- Producer -> material/source/generator.
- Playlist -> ordered track lane.
- Tractor/multitrack -> composed draft sequence.
- Filter -> single-input effect/filter.
- Transition -> two-input compositor/mixer.
- Consumer -> preview/export sink.
- Profile -> draft/render profile.

Use a typed graph instead of mutable stringly typed service networks.

## Jianying Lessons

- Match concepts and terminology directly.
- Do not treat `resource_id` or `effect_id` as internal render semantics.
- Convert coordinate/sign/unit differences at adapter boundaries.
- Text effects, bubbles, filters, and proprietary resources need graceful degradation.

