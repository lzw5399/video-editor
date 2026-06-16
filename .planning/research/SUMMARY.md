# Research Summary

## Stack

Use Electron for the desktop shell and Rust for the editor core from day one. The MVP should call Rust through a thin Node-API binding and execute desktop FFmpeg/ffprobe binaries through a runtime abstraction. This preserves future iOS, Android, and server paths without slowing the first desktop editor too much.

## Product And UI

The product should feel like a reduced Jianying desktop editor:

- Top feature categories.
- Left material/function panel.
- Center preview.
- Right inspector.
- Bottom multi-track timeline.

MVP can implement fewer panels, but the editor workspace shape should be established immediately.

## Terminology

Use Jianying concepts everywhere: desktop UI, Rust domain model, IPC commands, schema, docs, and tests. The core vocabulary is draft, material, track, segment, source/target time range, main-track magnet, canvas adjustment, keyframe, sticker, text bubble, text effect, filter, and transition.

## Architecture

The durable semantic path is:

```text
draft/project.json
  -> command
  -> normalized draft
  -> resolved frame state
  -> render graph
  -> FFmpeg job
  -> preview/export
```

Kdenlive informs editor boundaries and model discipline. MLT informs media abstractions. pyJianYingDraft informs vocabulary and draft compatibility concepts.

## Testing

Every phase must have executable gates. The first milestone should create deterministic fixtures and golden tests early, then grow coverage layer by layer:

- schema/model round trips
- command and undo/redo snapshots
- normalized draft and frame-state goldens
- render graph and FFmpeg script snapshots
- preview/export parity
- packaged Electron import-preview-export smoke tests

## MVP Shape

MVP means a user can create/open/save a draft, import materials, arrange segments on tracks, split/trim/move/delete, add text/subtitles and BGM, preview, and export MP4. Complex effects, external draft adapters, AI workflows, and mobile/server runtimes are post-MVP.

